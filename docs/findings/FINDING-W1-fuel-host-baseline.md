# FINDING-W1 — Fuel cost of a representative frame, measured off-device

**Spike:** W1
**Status:** Draft — **partial**. The fuel half of W1 is answered; the wall-time
half is not and cannot be, without hardware.
**Date:** 2026-08-05
**Settles:** the `fuel-per-frame` floor for R-02-24. Does **not** settle
ADR-S002, which stays Provisional.
**Spike code:** `spikes/w1-wasmi/` (throwaway). Reproduce with `./run.sh`.

---

## Why a partial finding exists at all

W1 asks for "wall time and fuel consumed". Those two halves have different
portability:

- **Fuel** is charged by the `wasmi` translator from a fixed cost table
  (`wasmi_core::FuelCostsProvider`: base, instance, load, store, call, simd,
  bytes-per-fuel). It is a property of the compiled `.wasm` and the `wasmi`
  version, not of the CPU underneath.
- **Wall time** is a property of the CPU, the clock, the caches, and PSRAM.

So the fuel figure can be established without a device and the wall time
cannot. This finding reports the first and explicitly refuses the second.

---

## Measurement conditions

| | |
|---|---|
| Build profile — guest | `release`, `opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, target `wasm32-unknown-unknown` |
| Build profile — harness | `release`, `opt-level = 3` |
| Toolchain | `rustc 1.97.0-nightly (f53b654a8 2026-04-30)` — the pinned `nightly-2026-05-01` |
| Runtime | `wasmi` **1.1.0**, `default-features = false`, `Config::default()` plus `consume_fuel(true|false)`, default (lazy) translation |
| Hosts | (a) x86_64 Linux, native. (b) `riscv64gc-unknown-linux-gnu` under `qemu-riscv64-static` 8.2.2 |
| Board | **none — no ESP32-P4 was involved in this measurement** |
| Module under test | `spikes/w1-wasmi/artifacts/w1_guest.wasm`, 11,555 bytes, sha256 `48fc0c7ab837cded944e252c3aa97794d1ea214d575b8c9c3e66865286b9dcf5` — committed, because the fuel figures are a property of these exact bytes |
| CPU clock | n/a |
| PSRAM part / mode / clock | n/a |
| Cache config | n/a |
| Sample count | wall time: 2000 native / 200 cross. Fuel: 32 consecutive calls, compared for equality |
| Warm or cold | warm — 8 calls discarded before recording. First-call figures reported separately because they differ |
| Measured by | `wasmi::Store::get_fuel` (fuel); in-process `Instant` (wall time) |

---

## Observed

### Fuel per call, steady state

| Export | Returns | Fuel (steady) | Fuel (first call) | First-call premium |
|---|---:|---:|---:|---:|
| `bench_tree` — build + commit 300-node tree | 300 nodes | **12,634** | 21,118 | +67.1% |
| `bench_markdown` — 60 lines, style runs | 40 runs | **85,850** | 111,043 | +29.3% |
| `bench_fountain` — 60 lines, style runs | 32 runs | **47,496** | 67,355 | +41.8% |
| `bench_frame` — all three, one frame | 372 | **145,987** | 194,413 | +33.2% |
| `bench_noop` — true negative | 0 | **2** | 30 | — |

Fuel was identical across all 32 post-warm-up calls for every export.

### Fuel is host-architecture-independent — checked, not assumed

Every figure in the table above is **byte-identical** when the same `.wasm`
runs on x86_64 and on riscv64 under qemu. All five exports, both the
steady-state and first-call columns. `run.sh` diffs the two columns and exits
non-zero if they ever diverge, so this stays checked rather than remembered.

### Work actually performed at the host boundary

| | |
|---|---|
| Nodes committed | 300 — matches the figure PLAN.md specifies |
| Encoded tree size | 6,763 bytes = **2.6%** of the R-02-07 256 KiB cap; 300 of 4096 nodes = 7.3% |
| Markdown style runs | 40 over 60 lines |
| Fountain style runs | 32 over 60 lines |

### `wasmi` builds `no_std` for the P4's target

`wasmi` 1.1.0 with `default-features = false` compiles cleanly for
**`riscv32imafc-unknown-none-elf`** on the pinned `nightly-2026-05-01`, both as
a bare dependency and as the full device runner in `spikes/w1-wasmi/device/`.
No fork, no patch, no C shim.

This matters more than it looks: it removes the possibility that R-05's
mitigation ladder gets reached for the wrong reason. If `wasmi` had not built
for the target, "the interpreter is too slow" and "the interpreter does not
compile" would have been easy to conflate under schedule pressure.

The device runner is written and its logic is tested — 17 tests, including
five that inject faults and confirm the run refuses to report. It has never
executed on silicon.

### Fuel-metering overhead, wall time, x86_64 native

| Export | metered median | unmetered median | overhead |
|---|---:|---:|---:|
| `bench_tree` | 34.6 µs | 32.1 µs | +7.7% |
| `bench_markdown` | 51.4 µs | 48.1 µs | +6.8% |
| `bench_fountain` | 60.4 µs | 55.9 µs | +8.1% |
| `bench_frame` | 157.1 µs | 146.7 µs | **+7.1%** |

---

## Not established

The list is long and that is the honest state of it.

1. **Anything about the ESP32-P4.** No device was involved. The wall-time
   columns are x86_64 and emulated-riscv64 numbers. They are in the report
   because the harness prints what it measured, **not** because they bear on
   the 16.6 ms frame budget. Do not quote them as W1 timings.
2. **Whether `wasmi` is fast enough** — the actual R-05 question. Fuel does not
   convert to time without a device measurement of fuel-per-second.
3. **The ~4 ms tree-build threshold** in PLAN.md is a *time* threshold. The
   12,634-fuel tree build cannot be compared against it yet.
4. **Metering overhead on the device.** +7.1% is an x86_64 figure. Branch cost
   and I-cache pressure differ on a 400 MHz RV32IMAFC core.
5. **32-bit pointer width.** Both hosts were 64-bit. Fuel is charged per wasm
   instruction so it should not depend on host pointer width — but "should"
   is not "was measured".
6. **`no_std` at run time.** `wasmi` now demonstrably *compiles* `no_std` for
   `riscv32imafc-unknown-none-elf`, and so does the device runner. Neither has
   *executed* in that configuration. Compiling is not running: the allocator,
   the clock source, and every trap path are untested on hardware.
7. **Workload realism.** The classifiers, the tree encoder, and the corpus in
   `spikes/w1-wasmi/` are my construction. The real Markdown and Fountain
   classifiers (SPEC-04) and the real tree codec (`crates/abi`) do not exist,
   and fuel scales with whatever they turn out to be. This number is
   *representative*, not *predictive*.
8. **Allocator cost.** The guest uses a static bump arena and never calls
   `alloc`. A real SDK allocator adds fuel that is not counted here.
9. **The mitigation ladder** — keyed diffing, host widgets, AOT — is untouched.
10. **Eager vs lazy translation.** Left at the `wasmi` default; the first-call
    premium may be an artifact of lazy translation rather than of cold caches.
11. **The `mcycle` CSR on ESP32-P4.** The device runner's default clock reads
    `mcycle`/`mcycleh`. That is machine-mode standard RISC-V and the HP cores
    are RV32IMAFC, so it is the expected source — but it is unverified on this
    silicon, and a stuck counter reads as an infinitely fast device. The
    runner self-checks for exactly that before measuring; the fallback is a
    `SYSTIMER`/`TIMG` alarm through `esp-hal`.

---

## True negative

**Fault injected:** `bench_noop`, an export that does no work and calls no host
function, measured through the identical path.

**Detector response:** 2 fuel against `bench_frame`'s 145,987 — a ratio of
~73,000×. Host-boundary counters stayed at zero for it while `bench_frame`
recorded 248 commits and 496 style-run pushes.

**Verdict:** trips correctly. The harness is measuring the workload, not call
overhead. `report::verify` fails the run if the no-op ever comes within 1% of a
frame, if fuel varies across identical calls, if the tree is not ~300 nodes, or
if `bench_frame` never reaches the host boundary.

Worth recording: **this check found three real defects on first run.** The tree
encoder was silently returning zero nodes and committing nothing; fuel varied
run-to-run because call one was being counted; the node count was 298, not 300.
All three produced output that looked plausible. Without the paired no-op and
the boundary counters, the first version of this finding would have reported a
confident fuel figure for a workload that never ran.

---

## Consequences

- **ADR-S002 stays Provisional.** W1 is not closed. Do not let this partial
  result be read as clearing the interpreter.
- **`fuel-per-frame` has a measured floor.** A legitimate frame of this shape
  costs ~146,000 fuel steady-state and ~194,000 on its first call. R-02-24's
  per-frame refill must clear the **first-call** figure with margin, or a cold
  app exhausts fuel on frame one and gets suspended for being new rather than
  for being hostile. That is a design consequence found off-device, and it is
  the most useful thing in this document.
- **Budgets affected:** none. No budget in `CLAUDE.md` §5 is expressed in fuel,
  and nothing here justifies moving one.
- **Spec amendments required:** none yet.
- **Noted, not acted on:** the workspace manifest still carries
  `wasmi = "0.4x"` with a "pin in Phase 3" comment. The current release is
  1.1.0, and this spike used it. Pinning is Phase 3's job; flagging it is this
  finding's.

## What would settle it

**Step 1 is written and waiting.** `spikes/w1-wasmi/device/` is a `no_std`
library that runs this exact `.wasm`, times it, self-checks, and derives
fuel-per-second. It compiles for `riscv32imafc-unknown-none-elf` today. The
board must supply four things and call `run`:

| Needed | Notes |
|---|---|
| Entry point | `riscv-rt` or `esp-hal`, clocks configured to the rate passed as `hz` |
| Global allocator | `wasmi` needs `alloc`. Internal SRAM suffices |
| A `Clock` | `CycleClock` reads `mcycle`; `self_check` refuses a stuck counter |
| A `core::fmt::Write` sink | `defmt` adapter, UART, or RTT |

**It does not need PSRAM.** The module is 11,555 bytes and `wasmi`'s working
set for it is small, so the measurement should fit in internal SRAM — which
means this does **not** block on Spike B1 and is the cheapest useful thing to
run on a newly arrived board, before display, C6, or PSRAM work.

Then:

1. Run it at 400 MHz, recording the clock config, and the PSRAM mode per
   R-01-05 if PSRAM is up by then. Report median and p99, warm.
2. Read **fuel-per-second** off the report. That single constant makes every
   fuel figure in this document convertible, including future ones, without
   re-running the workload. The runner prints it, and prints `not computed`
   rather than a fabricated zero when the self-checks fail.
3. Compare `bench_tree` alone against the ~4 ms trigger in PLAN.md. If it
   exceeds, work the mitigation ladder in the fixed order — keyed diffing,
   host widgets, then AOT — before revisiting ADR-S002.
4. Re-measure the metered/unmetered delta on device; the +7.1% here is not
   transferable.
5. Repeat once the real classifiers and the real `crates/abi` codec exist.
   The number will move.
