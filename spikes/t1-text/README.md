# Spike T1 — Text stack memory

**Status:** not started
**Informs:** `SPEC-00 §6` memory budget
**Output owed:** finding in `docs/findings/`

---

## The question

The text stack is the largest allocation in the system and the reason
ADR-S012 puts editing state host-side, under the budget enforcer's control.
`SPEC-00 §6` claims 1.00 MiB for glyph atlas plus shaping cache, and 8 bytes
per paragraph for the layout index. Are those numbers right?

## Work

Shape and lay out a **10,000-paragraph document** with the intended stack and
measure:

- Glyph atlas steady-state size.
- Shaping cache behaviour — hit rate and growth, not just final size.
- Layout index size (the claim is 80 KiB for 10k paragraphs).
- The cost of a **re-layout after a font-size change**. This is the expensive
  case and it is also a correctness case: R-05-18 requires reading position to
  survive a font-size change.

Layout is windowed (R-02-10): viewport plus one screen of margin in each
direction, and nothing else. If the measurement quietly lays out the whole
document, it is measuring a system we are not building.

## Measurement rule — this one is easy to get wrong

`CLAUDE.md` §5: report **resident PSRAM high-water after a 30-second idle
following a full document load** — not peak-during-load, not first-frame.
State the document used.

And `CLAUDE.md` §3's warning applies directly here: *a memory number that
looks stable because the allocator never released* is the failure mode. Say
how the number was obtained — allocator high-water, region walk, or heap
instrumentation — and what it would look like if the allocator were simply
never returning memory. If those two are indistinguishable in your
measurement, the measurement is not finished.

## Settles

Whether the 1.0 MiB glyph/shaping budget and the 8-byte index claim survive
contact. If either moves, `SPEC-00 §6` is amended in its own commit
(`CLAUDE.md` §2) — the budget is not raised to fit the result without that.

## Downstream

Phase 5 exits on typing latency ≤ 50 ms p99 on a 10k-paragraph document, and
Phase 3 exits on the text stack meeting *these* numbers. Whatever this spike
reports becomes the target both are checked against, so an optimistic number
here is not a kindness.
