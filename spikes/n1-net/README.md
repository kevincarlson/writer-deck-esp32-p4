# Spike N1 — C6 link, HTTP, TLS

**Status:** not started
**Risks:** R-06 (ESP-HOSTED outside ESP-IDF needs the host protocol
reimplemented), R-07 (on-device TLS footprint and cert-store maintenance)
**Settles:** OQ-3, the R-06 fallback, the R-07 TLS choice
**Output owed:** finding in `docs/findings/`, **ADR-S023**

---

## The question

Bring up the host↔C6 link from bare-metal Rust. Is ESP-HOSTED's host protocol
practical to implement outside ESP-IDF, or is a custom minimal protocol the
better answer?

Then: measure TLS handshake time and peak memory for a real feed fetch.

## Work

1. **Transport — OQ-3.** SDIO is faster; SPI is simpler and frees pins.
   `SPEC-00 §2` recommends SPI as the default pending this spike, on the
   grounds that all network use is scheduled batch fetching (`SPEC-00 §7`).
   Confirm or refute against a real fetch, don't just assert it.
2. **Protocol — R-06.** If reimplementing ESP-HOSTED's host protocol is
   disproportionate, the named fallback is to treat the C6 as a dumb modem
   with a custom minimal protocol we define. Either answer is acceptable;
   an unstated one is not.
3. **TLS — R-07.** `embedded-tls` or `rustls` `no_std`. Measure handshake time
   and **peak** memory during handshake, not steady state — the handshake is
   the high-water mark and it has to coexist with everything in `SPEC-00 §6`.
   R-05-04 requires exactly one TLS path in the system; this spike picks it.

## Constraints that bound the answer

These are structural (`CLAUDE.md` §6) and the spike must not quietly design
around them:

- **No sockets in the app ABI**, ever. Whatever this spike builds sits behind
  `feed.*`, `fetch.document`, and `sync.*` (ADR-S004).
- **The radio is off between windows** (R-05-01). The C6 rail is powered down,
  not idle. Bring-up cost from cold rail is therefore part of the measurement,
  not an artifact to be warmed away.
- The pinned root store is updated with firmware, not fetched (R-07).

## `[PRESENCE]` adjacency

Phase 6 requires the radio be shown measurably off between windows, with a
meter. This spike does not have to close that item, but it should note the
rail-control mechanism it used so the Phase 6 measurement has something to
attach to.
