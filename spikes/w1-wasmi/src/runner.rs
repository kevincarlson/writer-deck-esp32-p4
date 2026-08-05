// SPDX-License-Identifier: Apache-2.0
//! Instantiates the guest and measures one export.

use std::time::{Duration, Instant};
use wasmi::{Caller, Config, Engine, Extern, Linker, Module, Store, TypedFunc};

/// What the host learned while the guest ran. Exists so the harness can prove
/// the workload actually reached the host boundary rather than being folded
/// away — see `report::verify`.
#[derive(Default)]
pub struct HostCalls {
    pub commits: u32,
    pub tree_bytes: u32,
    pub run_pushes: u32,
    pub run_bytes: u32,
}

pub struct Measured {
    pub name: String,
    pub ret: u32,
    pub fuel: u64,
    pub first_call_fuel: u64,
    pub fuel_stable: bool,
    pub calls: HostCalls,
    pub metered: Vec<Duration>,
    pub unmetered: Vec<Duration>,
}

fn config(fuel: bool) -> Config {
    let mut c = Config::default();
    c.consume_fuel(fuel);
    c
}

type Prepared = (Store<HostCalls>, TypedFunc<(), u32>);

fn prepare(wasm: &[u8], name: &str, fuel: bool) -> Result<Prepared, String> {
    let engine = Engine::new(&config(fuel));
    let module = Module::new(&engine, wasm).map_err(|e| format!("module: {e}"))?;
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
        .map_err(|e| format!("link commit_tree: {e}"))?;

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
        .map_err(|e| format!("link set_style_runs: {e}"))?;

    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|e| format!("instantiate: {e}"))?;

    let f = instance
        .get_typed_func::<(), u32>(&store, name)
        .map_err(|e| format!("export {name}: {e}"))?;

    Ok((store, f))
}

/// Touch every byte the guest handed over, so the host side is not optimised
/// into a no-op and a bad (ptr, len) is caught rather than ignored.
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
    // Returned so the sum cannot be discarded.
    sum | 1
}

/// A generous per-call fuel grant. Larger than any plausible frame so the
/// measurement is of consumption, not of a limit.
const FUEL_GRANT: u64 = 1_000_000_000;

/// Calls discarded before recording. Lazy translation makes call one cold.
const WARMUP: usize = 8;

pub fn measure(wasm: &[u8], name: &str, samples: usize) -> Result<Measured, String> {
    // --- fuel, and whether it is stable across runs ---
    //
    // The first call is discarded. wasmi translates lazily by default, so call
    // one is not steady state — the exact warm-up artifact CLAUDE.md §3 names.
    // `first_call_fuel` is kept because the difference is itself a finding: a
    // cold app's first frame costs more than its steady-state frames.
    let (mut store, f) = prepare(wasm, name, true)?;
    let mut fuel_seen: Option<u64> = None;
    let mut fuel_stable = true;
    let mut first_call_fuel = 0u64;
    let mut ret = 0;

    for i in 0..WARMUP + 32 {
        store.set_fuel(FUEL_GRANT).map_err(|e| e.to_string())?;
        ret = f.call(&mut store, ()).map_err(|e| format!("call: {e}"))?;
        let used = FUEL_GRANT - store.get_fuel().map_err(|e| e.to_string())?;
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

    // --- metered wall time ---
    let mut metered = Vec::with_capacity(samples);
    for _ in 0..WARMUP {
        store.set_fuel(FUEL_GRANT).map_err(|e| e.to_string())?;
        f.call(&mut store, ()).map_err(|e| format!("call: {e}"))?;
    }
    for _ in 0..samples {
        store.set_fuel(FUEL_GRANT).map_err(|e| e.to_string())?;
        let t = Instant::now();
        f.call(&mut store, ()).map_err(|e| format!("call: {e}"))?;
        metered.push(t.elapsed());
    }
    let calls = std::mem::take(store.data_mut());

    // --- unmetered wall time, same module, fuel disabled ---
    let (mut store2, f2) = prepare(wasm, name, false)?;
    let mut unmetered = Vec::with_capacity(samples);
    for _ in 0..WARMUP {
        f2.call(&mut store2, ()).map_err(|e| format!("call: {e}"))?;
    }
    for _ in 0..samples {
        let t = Instant::now();
        f2.call(&mut store2, ()).map_err(|e| format!("call: {e}"))?;
        unmetered.push(t.elapsed());
    }

    Ok(Measured {
        name: name.to_string(),
        ret,
        fuel: fuel_seen.unwrap_or(0),
        first_call_fuel,
        fuel_stable,
        calls,
        metered,
        unmetered,
    })
}
