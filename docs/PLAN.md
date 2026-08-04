# Sindri Writer Deck — Phased Implementation Plan

**Status:** Draft v0.1.0
**Governs:** all implementation work
**Companion documents:** `CLAUDE.md`, `docs/specs/SPEC-00`..`SPEC-05`

---

## 0. How to read this plan

Phases have **exit criteria**, not dates. A phase is complete when every
criterion is demonstrably met, including the `[PRESENCE]` items that require a
human with the hardware in front of them.

Phase 0 is **blocking**. It exists because this project depends on several
under-documented ESP32-P4 behaviours, and the fastest way to lose a month is
to design around an assumption that turns out false in week six. Every spike
in Phase 0 either confirms a design premise or forces a specified fallback.

Spikes produce a written finding in the **Observed / Not established / What
would settle it** form (`CLAUDE.md` §3) and, where relevant, an ADR. A spike
that produces working code but no written finding is not complete.

---

## Phase 0 — Spikes (blocking)

**Purpose:** convert the Medium-confidence rows in `SPEC-00 §2` and the
top risks in `SPEC-00 §9` into facts.

Spikes are throwaway. Write them in a scratch workspace, not in `crates/`.
The deliverable is knowledge, and code written to answer a question is
usually the wrong code to keep.

### Spike B1 — Rust PSRAM init and flash mapping (R-01)

Bring up PSRAM from Rust on the P4: controller config, part-specific init,
timing calibration, cache/MMU mapping. Then run the destructive memory test
from R-01-04, **including its true-negative case** — deliberately mis-map a
region and confirm the test fails.

- Settles: whether the bootloader can be Rust end-to-end, or needs a narrow C
  shim for the PSRAM sequence (SPEC-01 §8).
- Budget generously. This is the single highest-risk item in the project.
- Output: ADR-S021 recording the achieved mode, clock, and approach.

### Spike D1 — Display path and frame rate (R-02, R-03)

MIPI-DSI panel bring-up at 1024x600 RGB565. Measure sustained frame rate for:
full-screen redraw, dirty-rect partial redraw of a typical text-editing
update, and a full-page scroll. Double-buffered in PSRAM, then single-buffered
for comparison. Establish whether the PPA and 2D-DMA are usable from Rust or
need drivers written against the TRM.

- Settles: 60 Hz vs 30 Hz target, double vs single buffer, and how much of the
  `esp-hal` P4 surface exists versus needs writing.
- `[PRESENCE]` Tearing and scroll feel must be judged by eye.
- Output: ADR-S022 with the measured numbers and conditions.

### Spike W1 — `wasmi` throughput and fuel (R-05)

Run a representative workload on-device: build and commit a 300-node UI tree,
classify 60 lines of Markdown and 60 of Fountain, emit style runs. Measure
wall time and fuel consumed. Check fuel-metering overhead by running with and
without.

- Settles: ADR-S002 (interpreter vs AOT) and the default `fuel-per-frame`.
- If the tree build alone exceeds ~4 ms, the mitigation order is: shrink
  per-frame app work via keyed diffing, move more into host widgets, then
  evaluate AOT. Try them in that order before concluding the interpreter is
  too slow.

### Spike N1 — C6 link, HTTP, TLS (R-06, R-07)

Bring up the host↔C6 link from bare-metal Rust. Establish whether ESP-HOSTED's
host protocol is practical to implement outside ESP-IDF, or whether a custom
minimal protocol is the better answer. Then measure TLS handshake time and
peak memory for a real feed fetch.

- Settles: OQ-3 (SDIO vs SPI), R-06 fallback, R-07 TLS choice.
- Output: ADR-S023.

### Spike T1 — Text stack memory (informs SPEC-00 §6)

Shape and lay out a 10,000-paragraph document with the intended stack.
Measure: glyph atlas steady-state size, shaping cache behaviour, layout index
size, and the cost of a re-layout after a font-size change.

- Settles: the 1.0 MiB glyph/shaping budget and the 8-byte index claim.
- Measurement rule from `CLAUDE.md` §5 applies: high-water after 30 s idle.

### Spike L1 — LVGL bindings in `no_std` (R-04)

Generate LVGL 9 bindings in-tree with `bindgen`. Confirm a `no_std` build,
custom allocator hookup, and a rendered frame. Assess whether `lvgl-rs` is
usable or whether in-tree generation is the answer.

### Spike I1 — Input path (OQ-1, R-12)

Prototype the **two viable paths** and measure both: USB HID host via the P4's
USB OTG-HS, and a matrix keyboard on a dedicated MCU over I2C/UART. BLE HID via
the C6 is measured only if both others fail — it puts the radio in the critical
path of typing and is not recommended (SPEC-00 §2).

Measure, per path:

- Keypress → glyph, p99 over a sustained typing run, against the ≤ 50 ms budget.
  Measured at the point where the host TextView draws, since R-04-01 puts typing
  below the app; a measurement taken at event dispatch is not the budget.
- The summon-key interception path (R-03-01) and what it contributes to the
  120 ms summon budget (R-03-02).
- Idle and active current with the C6 rail off, which is what decides whether
  the launcher can be summoned while the radio is gated.

- Settles: OQ-1, and the R-12 latency-budget dependency.
- `[PRESENCE]` Ghosting and n-key rollover on the chosen path (CLAUDE.md §9).
- Output: ADR-S024.

### Phase 0 exit criteria

- All seven spikes have written findings in `docs/findings/`.
- ADRs S021–S024 (and any others arising) are recorded.
- `SPEC-00 §2` confidence column is updated; every Medium row is now High or
  has a named fallback.
- OQ-1 (input path), OQ-2 (PSRAM size), OQ-3 (transport) are **resolved**.
- The memory budget in `SPEC-00 §6` is revised against measured numbers, or
  explicitly confirmed.
- Risk register updated with measured likelihoods.

---

## Phase 1 — Board bring-up and kernel skeleton

Interim: ESP-IDF C bootloader (ADR-S010). Do not block on B1.

**Work**

- Workspace scaffold, `rust-toolchain.toml`, `riscv32imafc` targets, CI.
- `sindri-hal`: clocks, GPIO, SPI, SDMMC, I2C, timers, DMA. `unsafe` permitted,
  `SAFETY` comments mandatory.
- `sindri-panel`: MIPI-DSI framebuffer, per D1's conclusions.
- `kernel`: cooperative scheduler, time, panic handler, `defmt` logging, power
  states.
- `services/display`, `services/input` (per resolved OQ-1),
  `services/storage` (FAT/exFAT on SD).
- `ui/lvgl-sys` + a native "hello" screen. No WASM yet.
- `tools/xtask`: build, flash, monitor, `budget`.

**Exit criteria**

- Boots to a native LVGL screen showing live input.
- SD read/write verified, including power-loss-during-write behaviour.
- Frame rate meets the target set by D1.
- `xtask budget` runs in CI and reports system PSRAM use.
- `[PRESENCE]` Input latency and scroll feel judged acceptable on device.

---

## Phase 2 — Brokkr (Rust bootloader)

**Work**

- `crates/boot`: allocation-free, ≤ 48 KiB, per SPEC-01.
- Partition table, `bootctl`, A/B slots, rollback counter, trial flag.
- Image format tooling in `xtask` (ESP image format, not raw ELF).
- Verification: SHA-256 + the signature scheme confirmed in B1.
- PSRAM validation with its true-negative test.
- Recovery image and both entry triggers.
- Secure Boot v2 key provisioning workflow, documented for manufacture.

**Exit criteria**

- All six items in `SPEC-01 §8` demonstrated on hardware.
- ESP-IDF bootloader removed from the build.
- Deliberate flash corruption of slot A fails over to B, observed.
- Anti-rollback refuses a downgrade, observed.
- `[PRESENCE]` Cold-boot-to-launcher timed externally.

---

## Phase 3 — Runtime, ABI, and lifecycle

The most consequential phase. **The ABI is draft throughout and stays draft
until Phase 6 exits** (R-09).

**Work**

- `crates/abi`: shared types, tree codec, conformance harness skeleton.
- `crates/runtime/host`: host function implementations, fuel accounting,
  memory cap, trap containment, per-app crash log.
- `crates/runtime/loader`: `.wdapp` parse, Ed25519 verify, manifest validate,
  budget refusal (R-02-21).
- `ui/tree`: retained tree, keyed differ, LVGL driver. Host-testable.
- `ui/textview`: piece table, layout index, shaping, windowed layout, undo log.
- Lifecycle supervisor: three states, two warm slots, arena reset.
- `sdk/sindri-app` and the **desktop simulator** (R-02-29).
- Decide WIT/component-model vs hand-written host functions; record the ADR.

**Build fuel metering here, not later** (R-02-24). It is what makes the
launcher-always-reachable promise enforceable.

**Exit criteria**

- A trivial WASM app runs, commits a tree, suspends, and resumes.
- Fuel exhaustion in a deliberate infinite loop returns to a native stub
  within the R-03-02 budget of 120 ms.
- A deliberate trap is contained, logged, and attributed to the app.
- Arena high-water returns to baseline after reset, and the escape-detection
  true-negative fires when an escape is injected.
- Conformance suite passes on device and in the simulator.
- Text stack meets the T1 memory numbers with a 10k-paragraph document.

---

## Phase 4 — Launcher and installation

**Work**

- Launcher app (WASM) with the five screens in `SPEC-03 §4`.
- `writerdeck:launcher@1.0.0` privileged interface.
- Install/uninstall, capability disclosure, sideload from SD.
- Native fallback UI (R-03-19).
- `tools/wdapp`: package, sign, verify CLI.
- Recents index, theme, locale switching, Fluent bundle loading.

**Exit criteria**

- Install, launch, suspend, resume, and uninstall an app from SD end to end.
- Summon budget (≤ 120 ms) holds while a hostile test app spins, allocates,
  and commits oversized trees.
- Unsigned app installs and is denied network, verified by attempting a fetch.
- Two consecutive launcher traps enter the native fallback.
- Warm-slot eviction ordering correct under a three-app rotation.

---

## Phase 5 — Writer

**Work**

- Markdown and Fountain incremental classifiers with the block-context cache.
- Style runs, outline, screenplay layout mode, focus modes.
- Fountain production reports and corkboard.
- Snapshot log with paragraph-level restore.
- Spell check against the DAWG; dictionary and thesaurus.
- Word-count goals and sprints.
- Markdown and PDF export; host PDF writer with subset fonts.
- Mermaid: sequence, state, Gantt, pie, mindmap only (R-04-14).
- Marp: directive-compatible with native templates (R-04-17).

**Exit criteria**

- Typing latency ≤ 50 ms p99 on a 10k-paragraph document.
- Style-run generation ≤ 2 ms p99 for 60 lines.
- Incremental classification is **byte-identical** to a from-scratch pass over
  the golden corpus. This is the true-negative for the cache and it is not
  optional.
- Warm resume ≤ 250 ms, cold ≤ 600 ms, per R-03-08 conditions.
- Snapshot restore verified including after a prune.
- Memory high-water within budget per the `CLAUDE.md` §5 rule.
- **ABI friction log written**: every place the interface fought the app.

---

## Phase 6 — Network, feeds, and Reader

**Work**

- `services/net`: C6 link, power windows, HTTP, TLS, pinned root store.
- `services/feed`: subscriptions, scheduled fetch, parsers, bounded snapshot,
  HTML→block normalisation at fetch time.
- Reader app: feed list, entry list, reading view, save-to-document.
- EPUB: container access, XHTML→blocks, pagination, position stability.
- Sync bridge (`sync.push`/`pull`) and the desktop side of it.

**Exit criteria**

- Radio measurably off between windows (`[PRESENCE]`, with a meter).
- Feed parsers survive the hostile corpus and a fuzzing run with no crashes.
- Window byte and time budgets enforced against an oversized feed.
- 2 MB EPUB cold-opens to first page ≤ 600 ms.
- Reading position survives a font-size change.
- **ABI friction log written**, merged with Phase 5's.

### ABI freeze gate

At the end of Phase 6, and not before: review both friction logs, make the
final additive and subtractive changes, run the full conformance suite, and
tag `writerdeck:app@1.0.0`. **After this point the interface is frozen.**

This gate is the reason the writer and reader are built before any third party
is invited. Getting this wrong is R-09, the highest-impact risk in the
register.

---

## Phase 7 — Hardening and the third-party path

**Work**

- Fuzzing in CI for every parser: RSS/Atom/JSON Feed, EPUB container, XHTML,
  Markdown, Fountain, manifest, tree codec, package header, partition table.
- Budget gates wired into CI as failures, not warnings.
- Per-app crash log surfaced in the launcher; repeat-offender handling.
- SDK: `cargo-generate` templates, simulator packaging, conformance harness a
  third party can run, publishing guide.
- Update index format and the opt-in update flow.
- Power tuning: sleep/wake, thermal behaviour at sustained load.
- Documentation: ABI reference, app author guide, the explicit list of what
  the device deliberately cannot do and why.

**Exit criteria**

- A third-party app written by someone without hardware, using only the SDK
  and simulator, installs and runs on device. This is the real test of Phase 7
  and it needs an actual second person.
- 72-hour soak with app rotation shows no PSRAM high-water growth.
- All budget gates enforced in CI.
- `[PRESENCE]` Battery life, thermals, and sleep/wake measured.

---

## Deferred beyond 1.0

Named so they are not smuggled into earlier phases.

| Item | Why deferred |
|---|---|
| Mermaid flowchart and class diagrams | Spike M1; layered layout is a project (R-10) |
| Marp CSS theming | No CSS engine, and there will not be one |
| Split live preview | Doubles layout cost, halves the writing column |
| Vertical writing modes / CJK vertical layout | Silent content loss risk (R-05-20) |
| SVG rendering | No renderer; `canvas` covers the diagram case |
| EPUB writing | Read-only by design (R-05-15) |
| Handwriting / stylus | Depends on OQ-1 and on hardware not assumed |
| CBZ/image viewer | Cheap given the JPEG codec, but the item most in tension with the distraction rule — decide deliberately, not by drift |
| Bibliography manager beyond autocomplete | Scope |
| Multi-device sync beyond the file bridge | Requires a service; out of scope |

---

## Cross-cutting standing items

Carried across every phase, checked at each exit:

1. **Budgets** — `CLAUDE.md` §5. Never raised to make a build pass.
2. **True negatives** — every detector has a case that should trip it and
   does. This project's recurring failure mode is a signal that looks like the
   healthy one, produced by a different mechanism.
3. **Findings form** — Observed / Not established / What would settle it, with
   measurement conditions.
4. **ADR per non-trivial decision.**
5. **`[PRESENCE]` items** — a phase does not close on CI alone.
6. **The forbidden list** — `CLAUDE.md` §6. If a feature requires breaking it,
   the feature does not belong on this device.
