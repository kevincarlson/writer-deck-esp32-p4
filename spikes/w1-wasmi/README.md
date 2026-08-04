# Spike W1 — `wasmi` throughput and fuel

**Status:** not started
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
