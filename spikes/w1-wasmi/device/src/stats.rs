// SPDX-License-Identifier: Apache-2.0
//! Sample statistics, and the fuel-to-time conversion this whole spike exists
//! to produce.

extern crate alloc;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub min: u64,
    pub median: u64,
    pub p99: u64,
    pub max: u64,
    pub count: u32,
}

impl Stats {
    pub fn from(mut samples: Vec<u64>) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        samples.sort_unstable();
        let n = samples.len();
        let idx = |p: f64| -> usize {
            let i = ((n - 1) as f64 * p) as usize;
            if i >= n {
                n - 1
            } else {
                i
            }
        };
        Self {
            min: samples[0],
            median: samples[idx(0.50)],
            p99: samples[idx(0.99)],
            max: samples[n - 1],
            count: n as u32,
        }
    }

    /// Convert a tick count to microseconds. Integer maths throughout — no
    /// float formatting on a `no_std` target, and no rounding surprises.
    pub fn ticks_to_us(ticks: u64, hz: u64) -> u64 {
        if hz == 0 {
            return 0;
        }
        ticks.saturating_mul(1_000_000) / hz
    }
}

/// The headline constant. Once this is known, every fuel figure in
/// `FINDING-W1` converts to time without re-running anything.
///
/// Returns fuel per second, or `None` if the inputs cannot produce one.
pub fn fuel_per_second(fuel: u64, ticks: u64, hz: u64) -> Option<u64> {
    if fuel == 0 || ticks == 0 || hz == 0 {
        return None;
    }
    // fuel / (ticks / hz) == fuel * hz / ticks, done in u128 to avoid
    // overflowing on a 400 MHz clock and a six-figure fuel count.
    let v = (fuel as u128).checked_mul(hz as u128)? / (ticks as u128);
    if v > u64::MAX as u128 {
        None
    } else {
        Some(v as u64)
    }
}

/// Microseconds a given fuel amount would take at a measured rate. This is
/// what makes the 16.6 ms frame budget checkable against a fuel figure.
pub fn fuel_to_us(fuel: u64, fuel_per_sec: u64) -> Option<u64> {
    if fuel_per_sec == 0 {
        return None;
    }
    Some(((fuel as u128 * 1_000_000) / fuel_per_sec as u128) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn stats_pick_the_right_samples() {
        let s = Stats::from((1..=100).collect::<Vec<u64>>());
        assert_eq!(s.min, 1);
        assert_eq!(s.max, 100);
        assert_eq!(s.median, 50);
        assert_eq!(s.p99, 99);
        assert_eq!(s.count, 100);
    }

    #[test]
    fn empty_samples_do_not_panic() {
        assert_eq!(Stats::from(vec![]), Stats::default());
    }

    #[test]
    fn fuel_rate_round_trips() {
        // 145_987 fuel in 10 ms at 400 MHz => 4_000_000 ticks.
        let rate = fuel_per_second(145_987, 4_000_000, 400_000_000).unwrap();
        assert_eq!(rate, 14_598_700);
        // Converting back gives the 10 ms we started from.
        assert_eq!(fuel_to_us(145_987, rate).unwrap(), 10_000);
    }

    #[test]
    fn degenerate_inputs_return_none_not_zero() {
        assert_eq!(fuel_per_second(0, 1, 1), None);
        assert_eq!(fuel_per_second(1, 0, 1), None);
        assert_eq!(fuel_per_second(1, 1, 0), None);
        assert_eq!(fuel_to_us(1, 0), None);
    }

    #[test]
    fn ticks_convert_to_microseconds() {
        assert_eq!(Stats::ticks_to_us(400_000, 400_000_000), 1_000);
        assert_eq!(Stats::ticks_to_us(1, 0), 0);
    }
}
