# Findings

Written outputs of the Phase 0 spikes and of every later measurement,
benchmark, or investigation. One file per finding, named
`FINDING-<spike-id>-<slug>.md`, written from `TEMPLATE.md`.

The rules, from `CLAUDE.md` §3:

- Three parts, always: **Observed / Not established / What would settle it**.
- A number without its measurement conditions is not a finding.
- A benchmark run once is not a benchmark.
- Every detector needs a paired true negative.
- **A spike that produces working code but no written finding is not
  complete** (`PLAN.md` §"Phase 0").

## Phase 0 index

Status is one of: `not started`, `in progress`, `draft`, `complete`.

| Finding | Spike | Settles | Spike code | Status |
|---|---|---|---|---|
| — | B1 — Rust PSRAM init and flash mapping | R-01, ADR-S021, OQ-2 | `spikes/b1-psram/` | not started |
| — | D1 — Display path and frame rate | R-02, R-03, ADR-S022 | `spikes/d1-display/` | not started |
| — | W1 — `wasmi` throughput and fuel | R-05, ADR-S002 | `spikes/w1-wasmi/` | not started |
| — | N1 — C6 link, HTTP, TLS | R-06, R-07, OQ-3, ADR-S023 | `spikes/n1-net/` | not started |
| — | T1 — Text stack memory | SPEC-00 §6 budget | `spikes/t1-text/` | not started |
| — | L1 — LVGL bindings in `no_std` | R-04 | `spikes/l1-lvgl/` | not started |
| — | I1 — Input path | OQ-1, R-12, ADR-S024 | `spikes/i1-input/` | not started |

No finding has been written yet. Nothing in this directory should be read as a
result until its row above says `complete` and the file exists.

Phase 0 exit tracking lives in `docs/PHASE-0.md`.
