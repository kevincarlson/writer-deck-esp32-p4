# Spike I1 — Input path

**Status:** not started
**Resolves:** OQ-1, and the R-12 dependency ("input path unresolved
invalidates latency budgets")
**Governing spec:** `SPEC-00 §2` (OQ-1), `SPEC-03 §2` (R-03-01, R-03-02),
`SPEC-04 §3` (R-04-01), `CLAUDE.md` §5 and §9
**Output owed:** finding in `docs/findings/`, **ADR-S024**

---

## The question

Which input path: USB HID host via the P4's USB OTG-HS, or a matrix keyboard
on a dedicated MCU over I2C/UART?

BLE HID via the C6 is the third option on paper and is **not** a candidate
here. It puts the radio in the critical path of typing, which is the opposite
of the power model in `SPEC-00 §7` — the C6 rail is off between windows. Test
it only if both other paths fail, and if it wins, that is a finding that
should make somebody uncomfortable rather than a result to accept quietly.

## Why this is Phase 0 and not Phase 1

OQ-1 determines two things that Phase 1 builds on top of rather than
discovers:

- Whether ≤ 50 ms p99 keypress → glyph is reachable at all on the chosen path.
- Whether the launcher can be summoned while the radio is gated — which is a
  property of the input hardware, not of the launcher.

`services/input` in Phase 1 is written against the answer. Writing it first
and measuring after is how the budget becomes the thing that gets adjusted.

## Work

Prototype both viable paths and measure each:

1. **Keypress → glyph, p99**, over a sustained typing run — not a single
   keystroke, not the first ten after boot.

   Measure at the point the **host TextView draws the glyph**. R-04-01 puts
   typing entirely below the app: the host inserts, re-lays out the affected
   lines, and redraws, and the app is told afterwards. A number taken at event
   dispatch measures the easy half and will look comfortably inside budget
   while the real path is not. This is the trap in this spike.

2. **The summon path.** R-03-01 requires the summon gesture be intercepted by
   the input service *before* dispatch to the foreground app, so the app
   cannot consume, delay, or veto it. Establish that each path can do that at
   all — a HID stack that delivers a whole report queue in order, with no
   priority path, constrains how R-03-02's 120 ms is spent. Report what
   interception costs.

3. **Power.** Idle and active current with the C6 rail off. This is what
   actually answers "can the launcher be summoned while the radio is gated".

4. **Chord and dedicated-key feasibility.** R-03-01 leaves the summon gesture
   itself open ("a dedicated key, or a chord — resolves with OQ-1"). A chord
   needs reliable simultaneous-key reporting, which is the same property as
   item 5. Say which one the chosen path can support.

## `[PRESENCE]`

**Ghosting and n-key rollover on the chosen input path** (`CLAUDE.md` §9).
This cannot be closed from a log. A matrix without diodes will report
plausible-looking keys that were never pressed, under exactly the conditions a
scripted test does not produce — fast touch-typing with rollover. Someone has
to type on it.

## Hardware dependency — state it in the finding

USB HID host needs the devkit and a keyboard. The matrix path needs an MCU and
a physical matrix that may not exist yet. **If only one path can be measured,
the finding says so and OQ-1 does not close on it.** A one-sided comparison
recorded as a decision is worse than an open question, because the open
question is at least visible.

If the second path is blocked on procurement rather than on engineering, that
is a schedule fact worth surfacing early — it is the most likely reason this
spike slips, and it is the reason to start it before the ones that only need
silicon already on the desk.

## Settles

OQ-1, ADR-S024, and the input half of `SPEC-00 §2`'s confidence table. If the
chosen path cannot hold ≤ 50 ms p99, that is a budget conversation in a spec
amendment (`CLAUDE.md` §5) — not a number quietly relaxed to match what was
measured.
