// SPDX-License-Identifier: Apache-2.0
//! Formats results and checks the harness against its own true negative.

use crate::runner::Measured;
use std::time::Duration;

fn pct(v: &mut Vec<Duration>, p: f64) -> Duration {
    v.sort_unstable();
    if v.is_empty() {
        return Duration::ZERO;
    }
    let i = (((v.len() - 1) as f64) * p).round() as usize;
    v[i]
}

fn us(d: Duration) -> f64 {
    d.as_secs_f64() * 1e6
}

pub fn print(results: &[Measured], samples: usize) {
    println!("Spike W1 — wasmi workload, host baseline");
    println!("samples: {samples} per export, plus 32 warm-up calls for fuel\n");

    println!(
        "{:<16} {:>7} {:>12} {:>12} {:>6} {:>9} {:>9} {:>9} {:>10}",
        "export", "ret", "fuel", "1st call", "stable", "med us", "p99 us", "unmet us", "overhead"
    );
    println!("{}", "-".repeat(97));

    for r in results {
        let mut m = r.metered.clone();
        let mut u = r.unmetered.clone();
        let med = pct(&mut m, 0.50);
        let p99 = pct(&mut m, 0.99);
        let umed = pct(&mut u, 0.50);
        let overhead = if us(umed) > 0.0 {
            format!("{:+.1}%", (us(med) / us(umed) - 1.0) * 100.0)
        } else {
            "n/a".to_string()
        };
        println!(
            "{:<16} {:>7} {:>12} {:>12} {:>6} {:>9.2} {:>9.2} {:>9.2} {:>10}",
            r.name,
            r.ret,
            r.fuel,
            r.first_call_fuel,
            if r.fuel_stable { "yes" } else { "NO" },
            us(med),
            us(p99),
            us(umed),
            overhead
        );
    }

    println!("\nhost-boundary traffic (proves the workload reached the host):");
    for r in results {
        println!(
            "  {:<16} commits={} tree_bytes={} run_pushes={} run_bytes={}",
            r.name, r.calls.commits, r.calls.tree_bytes, r.calls.run_pushes, r.calls.run_bytes
        );
    }

    println!(
        "\nNOTE: wall time above is {} — NOT the ESP32-P4.",
        std::env::consts::ARCH
    );
    println!("      Fuel is host-independent; wall time and overhead% are not.");
}

/// The harness checks itself. A benchmark that measures call overhead, or one
/// whose workload was folded away, produces numbers that look fine.
pub fn verify(results: &[Measured]) -> Result<(), Vec<String>> {
    let mut problems = Vec::new();
    let find = |n: &str| results.iter().find(|r| r.name == n);

    for r in results {
        if !r.fuel_stable {
            problems.push(format!(
                "{}: fuel varied across identical runs — fuel is supposed to be \
                 deterministic, so either the workload has nondeterminism or the \
                 portability claim is wrong",
                r.name
            ));
        }
    }

    // True negative: the no-op must be orders of magnitude cheaper than a
    // frame. If it is not, the harness is measuring itself.
    match (find("bench_noop"), find("bench_frame")) {
        (Some(noop), Some(frame)) => {
            if noop.fuel * 100 >= frame.fuel {
                problems.push(format!(
                    "true negative failed: bench_noop burned {} fuel against \
                     bench_frame's {} — call overhead dominates and the workload \
                     numbers are not the workload",
                    noop.fuel, frame.fuel
                ));
            }
            if noop.calls.commits != 0 || noop.calls.run_pushes != 0 {
                problems.push(
                    "true negative failed: bench_noop reached the host boundary, \
                     so the per-export isolation is broken"
                        .to_string(),
                );
            }
            if frame.calls.commits == 0 || frame.calls.run_pushes == 0 {
                problems.push(
                    "bench_frame never reached the host boundary — the workload \
                     did not run"
                        .to_string(),
                );
            }
        }
        _ => problems.push("missing bench_noop or bench_frame".to_string()),
    }

    // The tree must actually have been built to 300 nodes.
    if let Some(t) = find("bench_tree") {
        if t.ret < 290 || t.ret > 310 {
            problems.push(format!(
                "bench_tree committed {} nodes, expected ~300 — the workload is \
                 not the one PLAN.md specifies",
                t.ret
            ));
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}
