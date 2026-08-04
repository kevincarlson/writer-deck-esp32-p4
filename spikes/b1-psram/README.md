# Spike B1 — Rust PSRAM init and flash mapping

**Status:** not started
**Risk:** R-01 — High impact, **High** likelihood. The single highest-risk item
in the project. `PLAN.md` says budget generously; `SPEC-01` says read §8 before
estimating.
**Governing spec:** `SPEC-01 §4` (R-01-04, R-01-05), `SPEC-01 §8`
**Output owed:** finding in `docs/findings/`, **ADR-S021**

---

## The question

Can PSRAM be brought up on the ESP32-P4 from Rust — controller config,
part-specific init sequence, timing calibration, and cache/MMU mapping — or
does it need a narrow C shim?

Secondary, and answered on the way: OQ-2 (16 vs 32 MiB, and the achievable
clock and mode), plus the SRAM/L2-cache split that `SPEC-00 §2` currently
records as Medium confidence.

## Why it is hard

The sequence is part- and mode-dependent and documented mainly in ESP-IDF
source rather than in the TRM. `SPEC-01 §4` names the failure mode that
matters: **PSRAM can appear to work while returning corrupted data under
load.** A smoke test that reads back what it wrote from cache proves nothing.

## Work

1. Controller config, part-specific init, timing calibration, cache/MMU mapping.
2. Run the R-01-04 destructive memory test over the full range:
   - address-uniqueness (write the address at the address),
   - walking ones / walking zeros,
   - pseudo-random pattern with a fixed seed.
3. **Run its true-negative case.** Deliberately mis-map a region and confirm
   the test *fails*. This is not optional and it is the reason the spike
   exists in this form — a memory test that passes because the region was
   never actually written is the exact defect being guarded against.
4. Confirm, against the TRM and ESP-IDF source rather than by analogy with
   other ESP32 parts: the second-stage load offset, the image header layout
   the P4 ROM accepts, and which secure-boot signature scheme this silicon
   supports (RSA-PSS-3072 or ECDSA-P256 — `SPEC-01 §6`, R-01-08).

## What settles it

Whether the bootloader can be Rust end-to-end. If not, the fallback is named
and bounded: **a narrow C shim for the PSRAM sequence only**, called from
Rust, everything else in Rust (`SPEC-01 §8`). Record that as an ADR amendment
if it happens. Do not silently abandon the requirement, and do not let the
shim grow.

## Must be recorded in the finding

Per R-01-05 and `CLAUDE.md` §3 — PSRAM part number, achieved mode, achieved
clock, size, cache configuration, and CPU clock. Every later memory
measurement in this project is uncomparable without them.
