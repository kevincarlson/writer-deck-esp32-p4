# Spike L1 — LVGL bindings in `no_std`

**Status:** not started
**Risk:** R-04 — LVGL Rust bindings are stale; LVGL 9 + `bindgen` in `no_std`
may need a maintained fork
**Output owed:** finding in `docs/findings/`

---

## The question

Generate LVGL 9 bindings **in-tree** with `bindgen` and confirm three things:

1. A `no_std` build.
2. Custom allocator hookup — LVGL's allocations must come from an allocator we
   control, because `SPEC-00 §6` budgets 0.75 MiB for LVGL draw plus the
   retained tree, with draw buffers preferentially in internal SRAM.
3. A rendered frame.

Then assess whether `lvgl-rs` is usable, or whether in-tree generation is the
answer.

## Why in-tree is the default

R-04's mitigation is explicit: generate bindings in-tree rather than depending
on `lvgl-rs`, and contain the result in `lvgl-sys`. A stale external binding
crate is a dependency on someone else's schedule for a component sitting
directly under every pixel on the device. The spike may still conclude
`lvgl-rs` is fine — but the burden of proof is on using it, not on rejecting it.

## Containment

`sindri-lvgl-sys` is one of the four crates permitted to contain `unsafe`
(`CLAUDE.md` §4). Everything it exposes past its module boundary must be safe;
callers outside the crate never see `unsafe`. The spike should sketch what that
boundary looks like, because if it can't be drawn cleanly that is itself the
finding.

Note also ADR-S003: apps never touch LVGL handles. They submit a declarative
keyed tree and the host owns every native object. That is what makes LVGL
replaceable later, and it is why this spike is about bindings, not about an
API surface to expose.

## Interacts with D1

D1 decides the framebuffer strategy (double vs single, PPA and 2D-DMA
availability). LVGL's draw-buffer configuration follows from that. Run L1
independently, but state which framebuffer arrangement was in place when a
frame was rendered.
