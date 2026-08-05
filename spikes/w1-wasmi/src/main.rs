// SPDX-License-Identifier: Apache-2.0
//! Spike W1 harness. Runs the guest workload under `wasmi` and reports fuel
//! and wall time.
//!
//! Read `../README.md` §"What this can and cannot answer off-device" before
//! quoting any number this prints.

mod report;
mod runner;

use std::path::PathBuf;
use std::process::ExitCode;

const WASM: &str = "guest/target/wasm32-unknown-unknown/release/w1_guest.wasm";

/// PLAN.md's workload, plus the paired true negative.
const BENCHES: [&str; 5] = [
    "bench_tree",
    "bench_markdown",
    "bench_fountain",
    "bench_frame",
    "bench_noop",
];

fn main() -> ExitCode {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join(WASM);
    let wasm = match std::fs::read(&path) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("cannot read {}: {e}", path.display());
            eprintln!("build the guest first:  ./run.sh");
            return ExitCode::FAILURE;
        }
    };

    let samples: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    let mut results = Vec::new();
    for name in BENCHES {
        match runner::measure(&wasm, name, samples) {
            Ok(r) => results.push(r),
            Err(e) => {
                eprintln!("{name}: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    report::print(&results, samples);

    match report::verify(&results) {
        Ok(()) => ExitCode::SUCCESS,
        Err(problems) => {
            eprintln!("\nHARNESS INVALID — do not record these numbers:");
            for p in problems {
                eprintln!("  - {p}");
            }
            ExitCode::FAILURE
        }
    }
}
