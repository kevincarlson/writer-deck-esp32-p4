// SPDX-License-Identifier: Apache-2.0
//! Spike W1 device runner — the on-device half of W1.
//!
//! Produces the one number the whole spike is waiting for: **device
//! fuel-per-second**. Once that is known, every fuel figure in
//! `docs/findings/FINDING-W1-fuel-host-baseline.md` converts to time without
//! re-running any workload.
//!
//! This is a library on purpose. It compiles and unit-tests on a machine with
//! no board attached; the board supplies four things and calls [`run`]:
//!
//! 1. **An entry point.** `riscv-rt`'s `#[entry]`, `esp-hal`'s, or your own
//!    reset handler. Clocks configured to the rate you pass in `hz`.
//! 2. **A global allocator.** `wasmi` needs `alloc`. Internal SRAM is enough
//!    — see the note on PSRAM below.
//! 3. **A [`Clock`].** [`clock::CycleClock`] reads `mcycle` and is the
//!    expected source, but it is UNVERIFIED on P4 silicon. [`run`] calls
//!    [`clock::self_check`] first and refuses to measure a stuck counter.
//! 4. **Somewhere to write.** [`Report::write`] takes any `core::fmt::Write`
//!    — a `defmt` adapter, a UART, RTT.
//!
//! ## This does not need PSRAM
//!
//! The guest module is 11,555 bytes and `wasmi`'s working set for it is small.
//! The measurement should fit in the P4's internal SRAM, which means **it does
//! not block on Spike B1**. That is deliberate: it is the cheapest useful
//! thing to run on a new board, before the display, the C6, or PSRAM work.
//!
//! ## What this measures and what it does not
//!
//! It measures wall time for a workload whose fuel cost is already known and
//! is host-independent. It does not measure anything about the real writer —
//! see the "Not established" list in the finding, which applies unchanged.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod clock;
pub mod runner;
pub mod stats;
pub mod verify;

use alloc::string::String;
use alloc::vec::Vec;
use clock::{Clock, ClockFault};
use core::fmt::Write;
use runner::Bench;
use stats::Stats;
use verify::Problem;

/// The exact module the host-side fuel figures were measured against.
/// sha256 48fc0c7ab837cded944e252c3aa97794d1ea214d575b8c9c3e66865286b9dcf5
///
/// Committed rather than rebuilt because the fuel numbers are a property of
/// these bytes. Rebuilding the guest with a different toolchain changes them,
/// and then the device figures are not comparable to the host ones.
pub const GUEST_WASM: &[u8] = include_bytes!("../../artifacts/w1_guest.wasm");

/// PLAN.md's workload, plus the paired true negative.
pub const BENCHES: [&str; 5] = [
    "bench_tree",
    "bench_markdown",
    "bench_fountain",
    "bench_frame",
    "bench_noop",
];

pub struct Report {
    pub hz: u64,
    pub benches: Vec<Bench>,
    pub problems: Vec<Problem>,
    /// `None` when it could not be computed — never a fabricated zero.
    pub fuel_per_second: Option<u64>,
}

impl Report {
    /// True only when every self-check passed. Numbers from a report where
    /// this is false must not be recorded.
    pub fn trustworthy(&self) -> bool {
        self.problems.is_empty()
    }
}

#[derive(Debug)]
pub enum RunError {
    Clock(ClockFault),
    Wasm(String),
}

/// Run the whole measurement.
///
/// `samples` of 200 is plenty; the fuel loop adds 40 calls per export on top.
pub fn run<C: Clock>(clock: &C, samples: usize) -> Result<Report, RunError> {
    // Before anything else: is the clock real? A stuck counter would make
    // every measurement below look impossibly good.
    clock::self_check(clock, 100_000).map_err(RunError::Clock)?;

    let mut benches = Vec::new();
    for name in BENCHES {
        benches.push(runner::measure(GUEST_WASM, name, samples, clock).map_err(RunError::Wasm)?);
    }

    let mut found = [Problem::FuelUnstable; 8];
    let n = verify::verify(&benches, &mut found);
    let problems: Vec<Problem> = found[..n].to_vec();

    // Only derive the headline constant from a run that passed its checks.
    let fuel_per_second = if problems.is_empty() {
        benches
            .iter()
            .find(|b| b.name == "bench_frame")
            .and_then(|f| stats::fuel_per_second(f.fuel, f.metered.median, clock.hz()))
    } else {
        None
    };

    Ok(Report {
        hz: clock.hz(),
        benches,
        problems,
        fuel_per_second,
    })
}

impl Report {
    pub fn write<W: Write>(&self, w: &mut W) -> core::fmt::Result {
        writeln!(w, "Spike W1 — on device, clock {} Hz", self.hz)?;
        writeln!(
            w,
            "{:<16} {:>7} {:>10} {:>10} {:>10} {:>10}",
            "export", "ret", "fuel", "med us", "p99 us", "unmet us"
        )?;
        for b in &self.benches {
            writeln!(
                w,
                "{:<16} {:>7} {:>10} {:>10} {:>10} {:>10}",
                b.name,
                b.ret,
                b.fuel,
                Stats::ticks_to_us(b.metered.median, self.hz),
                Stats::ticks_to_us(b.metered.p99, self.hz),
                Stats::ticks_to_us(b.unmetered.median, self.hz),
            )?;
        }

        match self.fuel_per_second {
            Some(fps) => {
                writeln!(w, "\nFUEL PER SECOND: {fps}")?;
                if let Some(frame) = self.benches.iter().find(|b| b.name == "bench_frame") {
                    let us = Stats::ticks_to_us(frame.metered.median, self.hz);
                    writeln!(w, "frame: {us} us against a 16600 us budget")?;
                    writeln!(
                        w,
                        "first-call fuel {} => {} us",
                        frame.first_call_fuel,
                        stats::fuel_to_us(frame.first_call_fuel, fps).unwrap_or(0)
                    )?;
                }
                if let Some(tree) = self.benches.iter().find(|b| b.name == "bench_tree") {
                    let us = Stats::ticks_to_us(tree.metered.median, self.hz);
                    writeln!(w, "tree build alone: {us} us against the ~4000 us trigger")?;
                }
            }
            None => writeln!(w, "\nFUEL PER SECOND: not computed")?,
        }

        if !self.problems.is_empty() {
            writeln!(w, "\nRUN INVALID — do not record these numbers:")?;
            for p in &self.problems {
                writeln!(w, "  - {}", p.describe())?;
            }
        }
        Ok(())
    }
}
