# Spike W1 — `wasmi` throughput and fuel

**Status:** in progress — fuel half done off-device, wall-time half blocked on
hardware. See `docs/findings/FINDING-W1-fuel-host-baseline.md`.
**Risk:** R-05 — interpreter too slow for the UI tree build at 60 Hz
**Settles:** ADR-S002 (interpreter vs AOT), currently *Provisional*, and the
default `fuel-per-frame`
**Output owed:** finding in `docs/findings/`

---

## The question

The load-bearing structural claim of the whole architecture is in
`SPEC-00 §3`: **expensive things are native, policy is WASM.** An interpreter
is fast enough for policy. This spike is the test of that claim.

## Work

Run a representative workload on-device and measure wall time and fuel
consumed:

- Build and commit a 300-node UI tree.
- Classify 60 lines of Markdown and 60 lines of Fountain, emitting style runs
  (the R-04-02 shape: classification is app-side, visible range only —
  ADR-S013 bounds it to ~40 lines, so 60 is deliberately pessimistic).

Then measure fuel-metering overhead by running the same workload with metering
on and off. That difference is the price of the launcher-always-reachable
guarantee (ADR-S020), and it should be stated as a number rather than assumed
negligible.

## What settles it

The frame budget is 16.6 ms at 60 Hz. **If the tree build alone exceeds ~4 ms,
the mitigation order is fixed by `PLAN.md` and is not to be reordered:**

1. Shrink per-frame app work via keyed diffing (R-02-05 makes keys mandatory
   precisely so this is available).
2. Move more into host widgets — cheaper than an ABI function, and
   reimplementable later (`CLAUDE.md` §7).
3. *Then* evaluate AOT.

Try them in that order before concluding the interpreter is too slow. Reaching
for AOT first would be the expensive mistake here: it changes the trust and
memory story, not just the speed one.

## Also produces

The default `fuel-per-frame` value that Phase 3 implements against
(R-02-24). Report it with the workload it was derived from, not as a bare
number.

## Note on measurement

`CLAUDE.md` §3: the first frames after boot are not steady state. Report
median and p99 over a sustained run, and say whether the device was warm.

---

## What this can and cannot answer off-device

The two halves of W1 have different portability, and the harness is built
around that split.

**Fuel is portable.** `wasmi` charges fuel from a fixed cost table
(`wasmi_core::FuelCostsProvider`) applied by the translator. It is a property
of the compiled `.wasm` and the `wasmi` version, not of the host CPU. `run.sh`
proves this rather than asserting it: it runs the identical module on x86_64
and on riscv64 under qemu and diffs the fuel columns, failing if they differ.

**Wall time is not portable.** Every microsecond figure this harness prints is
the machine it ran on. None of it bears on the 16.6 ms frame budget. The
report labels this on every run; do not strip the label when quoting numbers.

So: the `fuel-per-frame` floor for R-02-24 is answerable here. Whether the
interpreter is fast enough — the actual R-05 question, and ADR-S002's
condition — is not, and this spike stays open until it runs on a P4.

## Running it

```bash
./run.sh              # guest build, native run, cross-arch fuel check
./run.sh 5000 500     # more samples
```

The harness **fails the run** rather than printing a plausible number when:
fuel varies across identical calls; the no-op true negative comes within 1% of
a frame; the tree is not ~300 nodes; or `bench_frame` never reaches the host
boundary. All four of those fired on the first run and each was a real defect.

The cross-architecture check needs `qemu-user-static` and the
`riscv64gc-unknown-linux-gnu` target; without them `run.sh` skips it and says
so rather than passing silently.

## Layout

```
guest/          the app workload, no_std, wasm32-unknown-unknown
  tree.rs       300-node UI tree + encoder   <-- NOT crates/abi's codec
  markdown.rs   incremental classifier with block-context cache (R-04-02)
  fountain.rs   element classification (R-04-07 shape)
  arena.rs      static bump arena, no `alloc`
corpus/         60+ lines each of Markdown and Fountain, chosen for the
                cases that break line-independent classification
src/            the harness: runner.rs measures, report.rs checks itself
```

`guest/src/tree.rs` mirrors the *shape* of SPEC-02 §4's encoding so the fuel
figure is representative. It is not the codec, it is not normative, and
copying it into `crates/abi` would be exactly the second-copy defect
`CLAUDE.md` §2 warns about.
