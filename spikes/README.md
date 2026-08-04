# Spikes — scratch workspace

Phase 0 lives here. `PLAN.md`:

> Spikes are throwaway. Write them in a scratch workspace, not in `crates/`.
> The deliverable is knowledge, and code written to answer a question is
> usually the wrong code to keep.

So: **nothing in this directory is a deliverable.** It is not held to the
engineering standards in `CLAUDE.md` §4 — no 300-line ceiling, no
`#![forbid(unsafe_code)]`, no `thiserror`, no Fluent, no conformance tests.
Write the ugliest thing that answers the question.

Two rules do still apply, because they are the point of the exercise:

1. **SPDX header on line 1** of every source file. Cheap, and the licence
   posture should not have holes.
2. **The finding is the deliverable.** A spike that produces working code and
   no written finding in `docs/findings/` is not complete. Write it from
   `docs/findings/TEMPLATE.md`, in the Observed / Not established / What would
   settle it form.

When a spike's answer needs to become production code, it gets **rewritten**
in `crates/` against the spec, in a separate commit, citing the requirement
ID. It is not moved.

---

## Layout

One directory per spike, named `<id>-<slug>`:

```
spikes/b1-psram/     PSRAM init and flash mapping        (R-01)
spikes/d1-display/   MIPI-DSI panel and frame rate       (R-02, R-03)
spikes/w1-wasmi/     wasmi throughput and fuel           (R-05)
spikes/n1-net/       C6 link, HTTP, TLS                  (R-06, R-07)
spikes/t1-text/      text stack memory                   (SPEC-00 §6)
spikes/l1-lvgl/      LVGL 9 bindings in no_std           (R-04)
```

Each starts as a README stating the question and what would settle it. Code
lands beside it when the spike starts.

## Each spike is its own cargo workspace

The root `Cargo.toml` excludes `spikes`, but that alone is not enough — a
package here would still need to declare its own workspace root or cargo will
complain about the ambiguity. Every spike `Cargo.toml` therefore carries an
empty `[workspace]` table:

```toml
# SPDX-License-Identifier: Apache-2.0
[package]
name = "spike-b1-psram"
edition = "2021"
publish = false

[workspace]   # <-- required: keeps this out of the root workspace entirely
```

This is deliberate isolation, not tidiness. Spikes will pin different crate
versions, try competing HALs, and depend on things that must never reach a
firmware build. Keeping them out of the root lockfile is what makes that safe.

A spike may also need a toolchain other than the pinned
`nightly-2026-05-01` — for instance if a P4 HAL crate has not caught up. Put a
`rust-toolchain.toml` in the spike directory to override locally, and **record
the override in the finding's measurement conditions**. A number measured on a
different toolchain than the one it will be compared against is not
comparable, and that is exactly the class of mistake `CLAUDE.md` §3 is about.

## Build artifacts

`target/` is already gitignored at any depth. Spike source is committed —
being able to re-run a measurement is most of what makes it re-checkable — but
the code carries no forward compatibility promise and may be deleted wholesale
once Phase 0 closes.
