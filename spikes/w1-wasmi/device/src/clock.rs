// SPDX-License-Identifier: Apache-2.0
//! Tick source, and the check that it is real.
//!
//! A stuck cycle counter is the classic instance of the failure this project
//! keeps warning about: it reads as an infinitely fast device, which looks
//! exactly like a healthy result. `Clock::self_check` exists to catch that
//! before any measurement is recorded.

/// A free-running monotonic tick source.
pub trait Clock {
    /// Current tick count. Must be monotonic and must not wrap during a run.
    fn now(&self) -> u64;
    /// Ticks per second. On a P4 HP core driven from `mcycle`, this is the
    /// CPU clock — 400_000_000 at the target configuration.
    fn hz(&self) -> u64;
}

/// Why a clock was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockFault {
    /// `hz()` returned zero — nothing can be converted to time.
    ZeroHz,
    /// The counter did not advance across a busy loop. Almost always means
    /// the CSR is unimplemented or the peripheral was never enabled.
    Stuck,
    /// The counter went backwards.
    NonMonotonic,
}

/// Confirm the clock advances before trusting anything it reports.
///
/// `spin` should be large enough to take clearly more than one tick at the
/// device's clock rate. The default caller passes 100_000.
pub fn self_check<C: Clock>(clock: &C, spin: u32) -> Result<(), ClockFault> {
    if clock.hz() == 0 {
        return Err(ClockFault::ZeroHz);
    }
    let t0 = clock.now();
    let mut acc: u32 = 0;
    for i in 0..spin {
        // Written so the optimiser cannot remove it.
        acc = acc.wrapping_mul(31).wrapping_add(i);
    }
    let t1 = clock.now();
    core::hint::black_box(acc);

    if t1 < t0 {
        return Err(ClockFault::NonMonotonic);
    }
    if t1 == t0 {
        return Err(ClockFault::Stuck);
    }
    Ok(())
}

/// RISC-V `mcycle` / `mcycleh` cycle counter.
///
/// UNVERIFIED ON ESP32-P4. `mcycle` is machine-mode standard and the HP cores
/// are RV32IMAFC, so this is the expected source — but it has not been run on
/// silicon. If `self_check` returns `Stuck`, the CSR is not implemented or is
/// gated; fall back to a `SYSTIMER` or `TIMG` alarm through `esp-hal` and set
/// `hz` to that peripheral's rate rather than the CPU clock.
#[cfg(target_arch = "riscv32")]
pub struct CycleClock {
    pub hz: u64,
}

#[cfg(target_arch = "riscv32")]
impl Clock for CycleClock {
    fn now(&self) -> u64 {
        loop {
            let (hi, lo, hi2): (u32, u32, u32);
            // SAFETY: three CSR reads with no side effects. `mcycle`/`mcycleh`
            // are read-only counters in machine mode. The hi/lo/hi sequence
            // handles the low word wrapping between the two reads, which on a
            // 400 MHz core happens about every 10.7 seconds.
            unsafe {
                core::arch::asm!("csrr {0}, mcycleh", out(reg) hi, options(nomem, nostack));
                core::arch::asm!("csrr {0}, mcycle",  out(reg) lo, options(nomem, nostack));
                core::arch::asm!("csrr {0}, mcycleh", out(reg) hi2, options(nomem, nostack));
            }
            if hi == hi2 {
                return ((hi as u64) << 32) | (lo as u64);
            }
        }
    }

    fn hz(&self) -> u64 {
        self.hz
    }
}

#[cfg(test)]
pub struct FakeClock {
    pub ticks: core::cell::Cell<u64>,
    pub step: u64,
    pub hz: u64,
}

#[cfg(test)]
impl Clock for FakeClock {
    fn now(&self) -> u64 {
        let t = self.ticks.get();
        self.ticks.set(t + self.step);
        t
    }
    fn hz(&self) -> u64 {
        self.hz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stuck_clock_is_rejected() {
        struct Stuck;
        impl Clock for Stuck {
            fn now(&self) -> u64 {
                0
            }
            fn hz(&self) -> u64 {
                400_000_000
            }
        }
        assert_eq!(self_check(&Stuck, 10_000), Err(ClockFault::Stuck));
    }

    #[test]
    fn zero_hz_is_rejected() {
        struct NoHz;
        impl Clock for NoHz {
            fn now(&self) -> u64 {
                1
            }
            fn hz(&self) -> u64 {
                0
            }
        }
        assert_eq!(self_check(&NoHz, 10), Err(ClockFault::ZeroHz));
    }

    #[test]
    fn advancing_clock_passes() {
        let c = FakeClock {
            ticks: core::cell::Cell::new(0),
            step: 1000,
            hz: 400_000_000,
        };
        assert_eq!(self_check(&c, 10), Ok(()));
    }
}
