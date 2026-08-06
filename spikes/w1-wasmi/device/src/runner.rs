// SPDX-License-Identifier: Apache-2.0
//! Runs one export under `wasmi` and times it.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use wasmi::{Caller, Config, Engine, Extern, Linker, Module, Store, TypedFunc};

use crate::clock::Clock;
use crate::stats::Stats;

/// Proof that the workload reached the host boundary rather than being folded
/// away. Same role as in the desktop harness.
#[derive(Default, Clone, Copy)]
pub struct HostCalls {
    pub commits: u32,
    pub tree_bytes: u32,
    pub run_pushes: u32,
    pub run_bytes: u32,
}

pub struct Bench {
    pub name: &'static str,
    pub ret: u32,
    pub fuel: u64,
    pub first_call_fuel: u64,
    pub fuel_stable: bool,
    pub calls: HostCalls,
    pub metered: Stats,
    pub unmetered: Stats,
}

/// Discarded before recording. `wasmi` translates lazily, so call one is not
/// steady state.
pub const WARMUP: usize = 8;
const FUEL_GRANT: u64 = 1_000_000_000;
const FUEL_RUNS: usize = 32;

type Prepared = (Store<HostCalls>, TypedFunc<(), u32>);

fn prepare(wasm: &[u8], name: &str, fuel: bool) -> Result<Prepared, String> {
    let mut cfg = Config::default();
    cfg.consume_fuel(fuel);
    let engine = Engine::new(&cfg);
    let module = Module::new(&engine, wasm).map_err(|_| String::from("module decode failed"))?;
    let mut store = Store::new(&engine, HostCalls::default());
    let mut linker = Linker::new(&engine);

    linker
        .func_wrap(
            "host",
            "commit_tree",
            |mut caller: Caller<'_, HostCalls>, ptr: u32, len: u32| -> u32 {
                let ok = read_check(&mut caller, ptr, len);
                let st = caller.data_mut();
                st.commits += 1;
                st.tree_bytes += len;
                ok
            },
        )
        .map_err(|_| String::from("link commit_tree"))?;

    linker
        .func_wrap(
            "host",
            "set_style_runs",
            |mut caller: Caller<'_, HostCalls>, ptr: u32, len: u32| -> u32 {
                let ok = read_check(&mut caller, ptr, len);
                let st = caller.data_mut();
                st.run_pushes += 1;
                st.run_bytes += len;
                ok
            },
        )
        .map_err(|_| String::from("link set_style_runs"))?;

    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|_| String::from("instantiate failed"))?;
    let f = instance
        .get_typed_func::<(), u32>(&store, name)
        .map_err(|_| String::from("missing export"))?;
    Ok((store, f))
}

/// Touch every byte handed over, so the host side cannot be optimised away
/// and a bad (ptr, len) is caught rather than ignored.
fn read_check(caller: &mut Caller<'_, HostCalls>, ptr: u32, len: u32) -> u32 {
    let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
        return 0;
    };
    let data = mem.data(&caller);
    let (start, end) = (ptr as usize, ptr as usize + len as usize);
    if end > data.len() {
        return 0;
    }
    let mut sum: u32 = 0;
    for b in &data[start..end] {
        sum = sum.wrapping_add(*b as u32);
    }
    sum | 1
}

pub fn measure<C: Clock>(
    wasm: &[u8],
    name: &'static str,
    samples: usize,
    clock: &C,
) -> Result<Bench, String> {
    // --- fuel ---
    let (mut store, f) = prepare(wasm, name, true)?;
    let mut fuel_seen: Option<u64> = None;
    let mut fuel_stable = true;
    let mut first_call_fuel = 0u64;
    let mut ret = 0u32;

    for i in 0..WARMUP + FUEL_RUNS {
        store.set_fuel(FUEL_GRANT).map_err(|_| String::from("set_fuel"))?;
        ret = f.call(&mut store, ()).map_err(|_| String::from("trap"))?;
        let used = FUEL_GRANT - store.get_fuel().map_err(|_| String::from("get_fuel"))?;
        if i == 0 {
            first_call_fuel = used;
        }
        if i < WARMUP {
            continue;
        }
        match fuel_seen {
            None => fuel_seen = Some(used),
            Some(prev) if prev != used => fuel_stable = false,
            _ => {}
        }
    }

    // --- metered ticks ---
    for _ in 0..WARMUP {
        store.set_fuel(FUEL_GRANT).map_err(|_| String::from("set_fuel"))?;
        f.call(&mut store, ()).map_err(|_| String::from("trap"))?;
    }
    let mut metered = Vec::with_capacity(samples);
    for _ in 0..samples {
        store.set_fuel(FUEL_GRANT).map_err(|_| String::from("set_fuel"))?;
        let t0 = clock.now();
        f.call(&mut store, ()).map_err(|_| String::from("trap"))?;
        metered.push(clock.now().saturating_sub(t0));
    }
    let calls = *store.data();

    // --- unmetered ticks, same module, fuel disabled ---
    let (mut store2, f2) = prepare(wasm, name, false)?;
    for _ in 0..WARMUP {
        f2.call(&mut store2, ()).map_err(|_| String::from("trap"))?;
    }
    let mut unmetered = Vec::with_capacity(samples);
    for _ in 0..samples {
        let t0 = clock.now();
        f2.call(&mut store2, ()).map_err(|_| String::from("trap"))?;
        unmetered.push(clock.now().saturating_sub(t0));
    }

    Ok(Bench {
        name,
        ret,
        fuel: fuel_seen.unwrap_or(0),
        first_call_fuel,
        fuel_stable,
        calls,
        metered: Stats::from(metered),
        unmetered: Stats::from(unmetered),
    })
}
