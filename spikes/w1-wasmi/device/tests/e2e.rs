// SPDX-License-Identifier: Apache-2.0
//! End-to-end run of the device path on the development machine.
//!
//! Everything here except the silicon: the committed `.wasm` is decoded, all
//! five exports execute under `wasmi`, the self-checks run, and the headline
//! constant is derived. Only the clock is a stand-in.
//!
//! The fuel assertions are the point. They tie the device runner to the exact
//! figures recorded in `docs/findings/FINDING-W1-fuel-host-baseline.md`, so if
//! the guest, the artifact, or the `wasmi` version drifts, this fails here
//! rather than silently on a bench with a board attached.

use std::time::Instant;
use w1_device::clock::Clock;

struct WallClock(Instant);

impl Clock for WallClock {
    fn now(&self) -> u64 {
        self.0.elapsed().as_nanos() as u64
    }
    fn hz(&self) -> u64 {
        1_000_000_000 // ticks are nanoseconds
    }
}

#[test]
fn device_path_runs_and_self_checks_pass() {
    let clock = WallClock(Instant::now());
    let report = w1_device::run(&clock, 100).expect("run failed");

    for p in &report.problems {
        eprintln!("problem: {}", p.describe());
    }
    assert!(
        report.trustworthy(),
        "self-checks failed on a machine where they should pass"
    );
    assert!(report.fuel_per_second.is_some());
}

#[test]
fn fuel_matches_the_recorded_finding() {
    let clock = WallClock(Instant::now());
    let report = w1_device::run(&clock, 32).expect("run failed");

    let f = |name: &str| -> (u64, u64) {
        let b = report
            .benches
            .iter()
            .find(|b| b.name == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        (b.fuel, b.first_call_fuel)
    };

    // FINDING-W1, "Fuel per call, steady state".
    assert_eq!(f("bench_tree"), (12_634, 21_118));
    assert_eq!(f("bench_markdown"), (85_850, 111_043));
    assert_eq!(f("bench_fountain"), (47_496, 67_355));
    assert_eq!(f("bench_frame"), (145_987, 194_413));
    assert_eq!(f("bench_noop"), (2, 30));
}

#[test]
fn the_committed_artifact_is_the_one_the_finding_measured() {
    // Guards against the artifact being refreshed without the finding being
    // updated — the two must move together.
    assert_eq!(w1_device::GUEST_WASM.len(), 11_555);
}

#[test]
fn report_renders_without_std() {
    let clock = WallClock(Instant::now());
    let report = w1_device::run(&clock, 16).expect("run failed");
    let mut s = String::new();
    std::fmt::Write::write_fmt(&mut s, format_args!("")).unwrap();
    report.write(&mut s).expect("write failed");
    assert!(s.contains("FUEL PER SECOND"));
    assert!(s.contains("bench_frame"));
    // The budget comparison must be present — it is the line a reader looks
    // for when the board finally runs this.
    assert!(s.contains("16600 us budget"));
}
