# Spike D1 — Display path and frame rate

**Status:** not started
**Risks:** R-02 (`esp-hal` P4 support incomplete for MIPI-DSI / PPA / JPEG),
R-03 (double-buffered 1024x600 in PSRAM may not sustain the frame budget)
**Output owed:** finding in `docs/findings/`, **ADR-S022**

---

## The question

MIPI-DSI panel bring-up at 1024x600 RGB565 — and then, what frame rate does
the path actually sustain?

## Work

Measure sustained frame rate for three cases, each double-buffered in PSRAM
and then single-buffered for comparison:

1. Full-screen redraw.
2. Dirty-rect partial redraw of a typical text-editing update (one line
   changing under a cursor — state the rect size used).
3. A full-page scroll.

Then establish whether the PPA and 2D-DMA are usable from Rust or need drivers
written against the TRM. Assume no crate exists until shown otherwise (R-02).

## What settles it

- 60 Hz vs 30 Hz as the frame target. The `CLAUDE.md` §5 budget is 16.6 ms at
  60 Hz, degrading to 30 Hz **before dropping input** — the ordering matters.
- Double vs single buffer. `SPEC-00 §6` currently spends 2.40 MiB on two
  framebuffers; dropping to one frees 1.20 MiB and forces a tear-avoidance
  strategy.
- How much of the `esp-hal` P4 surface exists versus has to be written in
  `sindri-hal`. This is a schedule input for Phase 1, not just a technical one.

## `[PRESENCE]`

**Tearing and scroll feel must be judged by eye.** A frame counter does not
show tearing, and "60 Hz achieved" alongside visible tearing is precisely the
healthy-looking signal produced by a different mechanism that `CLAUDE.md` §3
warns about. This item cannot be closed from CI or from a log.

## Depends on

PSRAM must be up. Until B1 lands, use whatever brings PSRAM up (including the
ESP-IDF path — ADR-S010 exists so this spike is not blocked), and **record the
PSRAM mode and clock in the finding**. Frame numbers measured at an unrecorded
PSRAM clock cannot be compared against anything later.
