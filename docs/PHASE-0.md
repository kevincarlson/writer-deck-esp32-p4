# Phase 0 — working tracker

**Governed by:** `PLAN.md` §"Phase 0 — Spikes (blocking)". That section is
authoritative; this file only tracks state against it.

Phase 0 is blocking. Nothing in Phase 1 starts until the exit criteria below
are met — with one deliberate exception named by the plan itself: the interim
ESP-IDF C bootloader (ADR-S010) means Phase 1 does **not** block on B1.

---

## 1. Spikes

Six spikes, all `not started`. Each produces a written finding in
`docs/findings/`; code alone does not close a spike.

| Spike | Question | Code | Finding | ADR owed | Status |
|---|---|---|---|---|---|
| B1 | Can PSRAM be brought up from Rust on the P4? | `spikes/b1-psram/` | — | S021 | not started |
| D1 | What frame rate does the DSI panel path sustain? | `spikes/d1-display/` | — | S022 | not started |
| W1 | Is `wasmi` fast enough for a per-frame UI tree build? | `spikes/w1-wasmi/` | — | — (settles S002) | not started |
| N1 | Is ESP-HOSTED practical outside ESP-IDF, and what does TLS cost? | `spikes/n1-net/` | — | S023 | not started |
| T1 | What does the text stack actually cost in memory? | `spikes/t1-text/` | — | — | not started |
| L1 | Do LVGL 9 bindings work in `no_std`? | `spikes/l1-lvgl/` | — | — | not started |

Ordering note: B1 is the highest-risk item and the plan says to budget it
generously, but it does not gate the others. D1, W1, T1, and L1 are
independent of it. B1 and D1 both touch PSRAM and cache configuration, so a
B1 result changes D1's measurement conditions — record D1's PSRAM mode
explicitly (R-01-05) or the frame numbers cannot be compared later.

---

## 2. Exit criteria

Copied from `PLAN.md` so this file can be checked off. None are met.

- [ ] All six spikes have written findings in `docs/findings/`.
- [ ] ADRs S021–S023 (and any others arising) are recorded.
- [ ] `SPEC-00 §2` confidence column updated; every Medium row is now High or
      has a named fallback.
- [ ] OQ-1 (input path) resolved.
- [ ] OQ-2 (PSRAM size and mode) resolved.
- [ ] OQ-3 (host↔C6 transport) resolved.
- [ ] `SPEC-00 §6` memory budget revised against measured numbers, or
      explicitly confirmed.
- [ ] Risk register updated with measured likelihoods.

### Open questions

| OQ | Question | Recommended default (pending evidence) | Settled by |
|---|---|---|---|
| OQ-1 | Input path: USB HID host, matrix keyboard on an MCU, or BLE HID via C6 | None. BLE HID is explicitly *not* recommended — it puts the radio in the critical path of typing | No spike is assigned to this in the plan — see §4 |
| OQ-2 | PSRAM size and mode: 16 vs 32 MiB, achievable clock | 32 MiB assumed by the `SPEC-00 §6` budget; 16 MiB has a named fallback (one warm slot, halved page cache) | B1 |
| OQ-3 | Host↔C6 transport: SDIO vs SPI | SPI — all network use is scheduled batch fetching | N1 |

### Confidence rows to close (`SPEC-00 §2`)

| Element | Current | Closed by |
|---|---|---|
| Internal SRAM split | Medium | B1 (cache/L2 config is chosen there) |
| PSRAM 32 MiB | Assumption | B1 / OQ-2 |
| Radio transport | Medium | N1 / OQ-3 |
| Accelerators (2D-DMA, PPA, JPEG) — Rust driver availability | Medium | D1 |
| Input | Unresolved | OQ-1 |

---

## 3. Rules that apply to every spike

1. **Spikes are throwaway.** They live in `spikes/`, never in `crates/`. The
   deliverable is knowledge; code written to answer a question is usually the
   wrong code to keep.
2. **Findings form is mandatory**: Observed / Not established / What would
   settle it, with measurement conditions (`CLAUDE.md` §3).
3. **True negatives.** Every detector gets a case that should trip it and
   does. B1's memory test has this as an explicit requirement (R-01-04); the
   others must state whether a detector is present at all.
4. **Budgets are not adjusted to fit a spike result.** A measurement that
   misses a budget is a finding and, if it stands, a spec amendment — in its
   own commit (`CLAUDE.md` §2, §5).
5. **Say when you are unsure**, in the same three-part form. A flagged unknown
   is cheap.

---

## 4. Known gap in the plan

**OQ-1 has no spike assigned to it.** `PLAN.md` lists OQ-1 as a Phase 0 exit
criterion, and `SPEC-00 §2` lists it as blocking Phase 1, but none of the six
spikes investigates the input path. R-12 assigns its mitigation to Phase 1
("prototype the two viable paths"), which contradicts resolving it at Phase 0
exit.

This is unresolved and is not resolved by this setup commit. It needs one of:
a seventh spike (I1) covering USB HID host on the P4 OTG-HS versus a matrix
keyboard on a dedicated MCU; or a plan amendment moving OQ-1's resolution to
Phase 1 alongside R-12. Either is a spec/plan amendment and belongs in its own
commit, not in an implementation commit.

Recorded here rather than decided, per `CLAUDE.md` §10.
