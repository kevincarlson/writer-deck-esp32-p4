# FINDING-<spike-id> — <one-line question this answers>

**Spike:** <B1 | D1 | W1 | N1 | T1 | L1>
**Status:** Draft | Complete
**Date:** <YYYY-MM-DD>
**Settles:** <risk / open question / ADR IDs — e.g. R-01, OQ-2, ADR-S021>
**Spike code:** `spikes/<dir>/` (throwaway; not a deliverable)

---

## Measurement conditions

A number without its conditions is not a finding (`CLAUDE.md` §3). Fill in
every row that applies; write "n/a" rather than deleting a row, so a reader can
tell the difference between "not relevant" and "not recorded".

| | |
|---|---|
| Build profile | `release` / `dev`, opt-level, LTO |
| Toolchain | rustc version + commit, from `rust-toolchain.toml` |
| Board | `devkit-p4-7in`, revision |
| CPU clock | e.g. 400 MHz |
| PSRAM part / mode / clock | e.g. <part no.>, octal, 200 MHz |
| Cache config | I/D cache size, line size, L2 split |
| Sample count | number of runs, not one |
| Warm or cold | device state at measurement |
| Measured by | internal timer / external instrument / eye |

---

## Observed

What was actually measured. Numbers with units and spread (min / median / p99),
not a single figure. Include the raw data or a path to it.

## Not established

What this measurement does **not** show. Be specific — "did not test at 16 MiB",
"single part, single board, no temperature sweep", "measured the tree build, not
the commit". Vagueness here is how a spike result gets over-applied later.

## What would settle it

The concrete next measurement, with the condition that would change the
conclusion.

---

## True negative

Every detector needs a case that *should* trip it and does (`CLAUDE.md` §3).
State the injected fault, and what the detector reported.

- **Fault injected:**
- **Detector response:**
- **Verdict:** trips / does not trip / not applicable — and if not applicable,
  why this measurement has no detector in it.

---

## Consequences

- **Design premise confirmed / refuted:**
- **Fallback triggered (if any):** name the fallback from the spec, not a new one.
- **Spec amendments required:** file and section, as a separate commit.
- **ADRs to record:**
- **Budgets affected:** before / after, per `CLAUDE.md` §5.
