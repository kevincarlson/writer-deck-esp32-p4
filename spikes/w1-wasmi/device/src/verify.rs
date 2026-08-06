// SPDX-License-Identifier: Apache-2.0
//! The run checks itself before its numbers are allowed to count.
//!
//! Mirrors the desktop harness's checks and adds the ones that only apply on
//! hardware: a stuck clock, and a metered/unmetered pair that came out
//! identical (which means fuel accounting was not actually on).

use crate::runner::Bench;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    FuelUnstable,
    TrueNegativeTooExpensive,
    NoopReachedHost,
    FrameNeverRan,
    TreeWrongSize,
    ZeroTicks,
    MeteringHadNoEffect,
}

impl Problem {
    pub fn describe(self) -> &'static str {
        match self {
            Problem::FuelUnstable => {
                "fuel varied across identical calls — fuel is meant to be \
                 deterministic, so either the workload is nondeterministic or \
                 the portability claim in FINDING-W1 is wrong"
            }
            Problem::TrueNegativeTooExpensive => {
                "bench_noop cost within 1% of bench_frame — call overhead \
                 dominates and the workload numbers are not the workload"
            }
            Problem::NoopReachedHost => {
                "bench_noop reached the host boundary — per-export isolation \
                 is broken"
            }
            Problem::FrameNeverRan => {
                "bench_frame never reached the host boundary — the workload \
                 did not run"
            }
            Problem::TreeWrongSize => {
                "bench_tree did not commit ~300 nodes — this is not the \
                 workload PLAN.md specifies"
            }
            Problem::ZeroTicks => {
                "an export measured zero elapsed ticks — the clock is not \
                 advancing, which reads as an infinitely fast device"
            }
            Problem::MeteringHadNoEffect => {
                "metered and unmetered timings are identical — fuel \
                 accounting was not actually enabled, so the overhead figure \
                 is meaningless"
            }
        }
    }
}

pub fn verify(benches: &[Bench], out: &mut [Problem; 8]) -> usize {
    let mut n = 0;
    let mut push = |p: Problem, n: &mut usize| {
        if *n < out.len() {
            out[*n] = p;
            *n += 1;
        }
    };

    let find = |name: &str| benches.iter().find(|b| b.name == name);

    for b in benches {
        if !b.fuel_stable {
            push(Problem::FuelUnstable, &mut n);
            break;
        }
    }

    for b in benches {
        if b.metered.median == 0 || b.unmetered.median == 0 {
            push(Problem::ZeroTicks, &mut n);
            break;
        }
    }

    if let Some(frame) = find("bench_frame") {
        if frame.metered.median == frame.unmetered.median {
            push(Problem::MeteringHadNoEffect, &mut n);
        }
        if frame.calls.commits == 0 || frame.calls.run_pushes == 0 {
            push(Problem::FrameNeverRan, &mut n);
        }
        if let Some(noop) = find("bench_noop") {
            if noop.fuel.saturating_mul(100) >= frame.fuel {
                push(Problem::TrueNegativeTooExpensive, &mut n);
            }
            if noop.calls.commits != 0 || noop.calls.run_pushes != 0 {
                push(Problem::NoopReachedHost, &mut n);
            }
        }
    }

    if let Some(t) = find("bench_tree") {
        if !(290..=310).contains(&t.ret) {
            push(Problem::TreeWrongSize, &mut n);
        }
    }

    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::HostCalls;
    use crate::stats::Stats;

    fn bench(name: &'static str, ret: u32, fuel: u64, med: u64, unmet: u64) -> Bench {
        Bench {
            name,
            ret,
            fuel,
            first_call_fuel: fuel * 2,
            fuel_stable: true,
            calls: HostCalls {
                commits: 1,
                tree_bytes: 6763,
                run_pushes: 2,
                run_bytes: 864,
            },
            metered: Stats {
                min: med,
                median: med,
                p99: med,
                max: med,
                count: 100,
            },
            unmetered: Stats {
                min: unmet,
                median: unmet,
                p99: unmet,
                max: unmet,
                count: 100,
            },
        }
    }

    fn noop(fuel: u64, med: u64) -> Bench {
        let mut b = bench("bench_noop", 0, fuel, med, med.saturating_sub(1));
        b.calls = HostCalls::default();
        b
    }

    #[test]
    fn a_healthy_run_reports_nothing() {
        let benches = [
            bench("bench_tree", 300, 12_634, 4000, 3800),
            bench("bench_frame", 372, 145_987, 40_000, 37_000),
            noop(2, 10),
        ];
        let mut out = [Problem::FuelUnstable; 8];
        assert_eq!(verify(&benches, &mut out), 0);
    }

    #[test]
    fn stuck_clock_is_caught() {
        let benches = [
            bench("bench_tree", 300, 12_634, 0, 0),
            bench("bench_frame", 372, 145_987, 0, 0),
            noop(2, 0),
        ];
        let mut out = [Problem::FuelUnstable; 8];
        let n = verify(&benches, &mut out);
        assert!(out[..n].contains(&Problem::ZeroTicks));
    }

    #[test]
    fn metering_that_did_nothing_is_caught() {
        let benches = [
            bench("bench_tree", 300, 12_634, 4000, 3800),
            bench("bench_frame", 372, 145_987, 40_000, 40_000),
            noop(2, 10),
        ];
        let mut out = [Problem::FuelUnstable; 8];
        let n = verify(&benches, &mut out);
        assert!(out[..n].contains(&Problem::MeteringHadNoEffect));
    }

    #[test]
    fn an_expensive_true_negative_is_caught() {
        let benches = [
            bench("bench_tree", 300, 12_634, 4000, 3800),
            bench("bench_frame", 372, 145_987, 40_000, 37_000),
            noop(100_000, 30_000),
        ];
        let mut out = [Problem::FuelUnstable; 8];
        let n = verify(&benches, &mut out);
        assert!(out[..n].contains(&Problem::TrueNegativeTooExpensive));
    }

    #[test]
    fn a_tree_that_never_built_is_caught() {
        let benches = [
            bench("bench_tree", 0, 12_634, 4000, 3800),
            bench("bench_frame", 372, 145_987, 40_000, 37_000),
            noop(2, 10),
        ];
        let mut out = [Problem::FuelUnstable; 8];
        let n = verify(&benches, &mut out);
        assert!(out[..n].contains(&Problem::TreeWrongSize));
    }
}
