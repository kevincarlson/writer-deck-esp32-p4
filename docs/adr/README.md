# Architecture Decision Records

One ADR per non-trivial decision, numbered `ADR-S###`, written from
`TEMPLATE.md`. "Non-trivial" means it constrains a future choice, or a
reasonable engineer would have chosen differently (`CLAUDE.md` §4).

## Current state of this directory

**ADR-S001 – ADR-S020 exist as ledger rows in `SPEC-00 §4`, not as full text
here.** The ledger carries ID, decision, rationale, and status; it is the
authoritative summary until the full text is backfilled. Do not read the
absence of a file here as the absence of a decision — check the ledger first.

Backfilling S001–S020 is not a Phase 0 exit criterion and is deliberately not
done as part of Phase 0 setup: transcribing twenty rationales invents detail
that was never written down. Backfill each one when work touches it, and cite
the ledger row in the interim.

## Reserved for Phase 0 outputs

These IDs are claimed by `PLAN.md` and must not be reused:

| ID | Owed by | Records |
|---|---|---|
| ADR-S021 | Spike B1 | Achieved PSRAM mode, clock, and init approach — and whether the bootloader is Rust end-to-end or needs the narrow C shim (SPEC-01 §8) |
| ADR-S022 | Spike D1 | Measured frame rate with conditions; 60 vs 30 Hz target; double vs single buffer |
| ADR-S023 | Spike N1 | Host↔C6 transport and protocol; TLS stack choice |
| ADR-S024 | Spike I1 | Input path (USB HID host vs. matrix keyboard on an MCU), and the summon gesture that follows from it |

Further ADRs arising from Phase 0 continue from S025.

Two ADRs already carry an open condition that Phase 0 resolves:

- **ADR-S002** (`wasmi`, AOT deferred) is *Provisional* — Spike W1 settles it.
- **ADR-S010** (ESP-IDF C bootloader as interim) has a named exit in
  `SPEC-01 §8`; B1 determines whether that exit is reachable as specified.

## Index

| ID | Decision | Status | Full text |
|---|---|---|---|
| S001–S020 | see `SPEC-00 §4` ledger | Accepted / Provisional | pending backfill |
| S021 | PSRAM init and mode | not written | owed by B1 |
| S022 | Display path and frame target | not written | owed by D1 |
| S023 | C6 transport and TLS | not written | owed by N1 |
| S024 | Input path and summon gesture | not written | owed by I1 |
