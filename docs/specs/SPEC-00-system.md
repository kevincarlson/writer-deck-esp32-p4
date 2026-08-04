# SPEC-00 — Sindri Writer Deck: System Architecture

**Status:** Draft v0.1.0
**Supersedes:** none
**Depends on:** nothing
**Depended on by:** SPEC-01, SPEC-02, SPEC-03, SPEC-04, SPEC-05

---

## 1. Purpose and non-goals

### Purpose

A distraction-resistant writing and reading device. Long-form drafting in
Markdown and Fountain, reading of RSS and EPUB, and an open third-party app
ecosystem — where "open" does not compromise the distraction properties,
because the distracting shapes are absent from the interface rather than
discouraged by policy.

### Non-goals

- General-purpose computing. There is no browser, no terminal, no package
  manager on-device.
- Multitasking. One app is resident at a time (SPEC-03 §3).
- Real-time guarantees beyond the input-latency budget.
- iOS/Android/desktop companions beyond a file-sync bridge.

### Naming

The system firmware is **Sindri**. The bootloader is **Brokkr**. Neither name
appears in user-visible strings; the product name is set at build time.

---

## 2. Hardware baseline

| Element | Assumption | Confidence |
|---|---|---|
| SoC | ESP32-P4, dual RISC-V RV32IMAFC HP cores @ 400 MHz, LP core @ 40 MHz | High |
| Internal SRAM | ~768 KiB HP L2MEM, plus TCM; L2 cache configurable | Medium — exact split is config-dependent |
| PSRAM | 32 MiB, via the SoC PSRAM interface | **Assumption — see OQ-2** |
| Display | 7", 1024x600, MIPI-DSI, RGB565 | High |
| Radio | ESP32-C6, over SDIO, via ESP-HOSTED-MCU | Medium — transport choice is OQ-3 |
| Storage | SD card (SDMMC) for documents; SPI NOR flash for firmware | High |
| Accelerators | 2D-DMA, PPA (pixel processing accelerator), hardware JPEG codec | Medium — Rust driver availability is R-04 |
| Audio | I2S codec to speakers | High |
| Input | **Unresolved — see OQ-1** | — |

**Everything marked Medium or lower is a Phase 0 spike input, not a
design premise.** The ESP32-P4 is recent enough that bare-metal Rust support
is incomplete in places; the plan is built around discovering exactly where.

### Open questions blocking Phase 1

- **OQ-1 — Input path.** USB HID host via the P4's USB OTG-HS, an integrated
  matrix keyboard on a dedicated MCU over I2C/UART, or BLE HID via the C6?
  This determines input latency, power behaviour, and whether the launcher can
  be summoned while the radio is gated. BLE HID is the worst of the three for
  the distraction model (it puts the radio in the critical path of typing) and
  is not recommended.
- **OQ-2 — PSRAM size and mode.** 16 vs 32 MiB, and the clock/mode achievable.
  The memory budget in §6 assumes 32 MiB. At 16 MiB the warm-slot count drops
  to one and EPUB page caching shrinks; the design survives but the budgets
  change.
- **OQ-3 — Host↔C6 transport.** SDIO is faster; SPI is simpler and frees pins.
  Given that all network use is scheduled batch fetching, SPI is likely
  sufficient and is the recommended default pending Spike N1.

---

## 3. Architecture overview

```
┌──────────────────────────────────────────────────────────┐
│ ROM bootloader (mask ROM, not ours)                       │
└───────────────┬──────────────────────────────────────────┘
                │ verifies + loads
┌───────────────▼──────────────────────────────────────────┐
│ Brokkr — Rust second-stage bootloader (SPEC-01)           │
│  flash MMU/cache map · PSRAM init · A/B slot select        │
│  image verify · rollback counter · recovery entry          │
└───────────────┬──────────────────────────────────────────┘
                │ jumps to
┌───────────────▼──────────────────────────────────────────┐
│ Sindri kernel (no_std, single address space)              │
│  ┌────────────┬────────────┬───────────┬───────────────┐  │
│  │ hal        │ scheduler  │ time      │ power/gating  │  │
│  └────────────┴────────────┴───────────┴───────────────┘  │
│  ┌──────────────────────── services ─────────────────────┐│
│  │ display · input · storage/FS · net (C6) · feed        ││
│  │ audio/TTS · docstore · crypto · l10n                  ││
│  └───────────────────────────────────────────────────────┘│
│  ┌──────────────── host-side UI + text ──────────────────┐│
│  │ LVGL (C, via sindri-lvgl-sys) · retained tree         ││
│  │ TextView widget: piece table + layout index + shaping ││
│  └───────────────────────────────────────────────────────┘│
└───────────────┬──────────────────────────────────────────┘
                │ host ABI  writerdeck:app@1.0.0  (SPEC-02)
┌───────────────▼──────────────────────────────────────────┐
│ WASM runtime (wasmi) — one instance resident              │
│  fuel metering · memory cap · trap containment            │
│  ┌─────────┬──────────┬──────────┬────────────────────┐   │
│  │Launcher │ Writer   │ Reader   │ third-party apps   │   │
│  │(SPEC-03)│(SPEC-04) │(SPEC-05) │                    │   │
│  └─────────┴──────────┴──────────┴────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

The load-bearing structural claim: **expensive things are native, policy is
WASM.** Text shaping, layout, the piece table, LVGL rendering, feed parsing,
and TLS all run as host code. The app decides *what* to show and *what to do
next*. An interpreter is fast enough for that, which is why `wasmi` may not
need AOT compilation (Spike W1 settles it).

---

## 4. ADR ledger

Full text in `docs/adr/`. Summary:

| ID | Decision | Rationale | Status |
|---|---|---|---|
| ADR-S001 | Apps are WASM modules, not native ELF or static links | Fault containment across a trust boundary; stable ABI across firmware revisions; install without reflash | Accepted |
| ADR-S002 | `wasmi` as the interpreter, AOT deferred | `no_std`, fuel metering built in, small footprint. Revisit only if Spike W1 fails its budget | Provisional — W1 |
| ADR-S003 | Declarative keyed UI tree, not an LVGL handle API | ~20 host functions instead of ~400; no cross-boundary lifetimes; LVGL replaceable later; consistent look without contributor cooperation | Accepted |
| ADR-S004 | No socket primitive in the ABI; intent-shaped services only | Makes persistent-connection apps unimplementable; collapses TLS surface to one audited path | Accepted |
| ADR-S005 | Three lifecycle states (Foreground/Suspended/Cold), not Android's six | An opaque full-screen launcher collapses Paused and Stopped | Accepted |
| ADR-S006 | `Activity::suspend(self)` consumes the receiver | Makes "I kept a live handle across suspension" unrepresentable rather than discouraged | Accepted |
| ADR-S007 | Per-app arena reset wholesale at suspend | Guarantees a defragmentation event on every app switch; LVGL widget churn otherwise fragments slowly | Accepted |
| ADR-S008 | Framebuffers, glyph cache, dictionary, feed store are system-owned | Keeps them out of app budgets; lets a sync survive a reader suspend | Accepted |
| ADR-S009 | Bootloader written in Rust, allocation-free, `unsafe` permitted | Stated project requirement. Risk accepted and mitigated by the ESP-IDF fallback in ADR-S010 | Accepted |
| ADR-S010 | ESP-IDF C bootloader is a permitted *interim* for Phases 1–2 | Decouples app-layer progress from the highest-uncertainty item. Structural deferral with a named exit, not abandonment | Accepted |
| ADR-S011 | Ed25519 for app package signing; ESP Secure Boot v2 for firmware | Separate trust domains. App signing needs a small pure-Rust verifier; firmware signing is fixed by ROM | Accepted |
| ADR-S012 | Text editing state (piece table, layout index) lives host-side | It is the largest allocation in the system and must be under the budget enforcer's control; also keeps fuel cost off the typing path | Accepted |
| ADR-S013 | Syntax classification is app-side, pushed as style runs for the visible range only | Lets third parties add languages without an ABI change; bounds fuel cost to ~40 lines | Accepted |
| ADR-S014 | EPUB/HTML is converted app-side into a host block list, not rendered by a host HTML engine | Avoids an HTML engine on the device; reuses TextView layout in read-only mode | Accepted |
| ADR-S015 | Feed fetch and parse are host services; apps read a local snapshot | Parsing hostile XML must not be in an app's fuel budget, and the snapshot shape is what makes feeds non-infinite | Accepted |
| ADR-S016 | Two warm slots in PSRAM holding saved state *and* layout index | Alt-tab between draft and outline must not touch SD | Accepted |
| ADR-S017 | Marp support is "directive-compatible", not CSS-compatible | No CSS engine exists or will. Claiming Marp compatibility outright would overclaim | Accepted |
| ADR-S018 | Mermaid support ships sequence/state/Gantt first; layered graphs behind a spike | Flowchart quality depends on Sugiyama layered layout, a genuine project in itself | Accepted |
| ADR-S019 | No path strings cross the ABI; private dir handle + system picker only | Removes path traversal as a category; resolves file-association cleanly | Accepted |
| ADR-S020 | Fuel refilled per frame; exhaustion suspends the app with a recovery path | The launcher-always-reachable promise otherwise breaks on a third-party infinite loop | Accepted |

---

## 5. Trust model

Three domains:

1. **Firmware** — Brokkr and the kernel. Signed with the ESP Secure Boot v2
   key. Compromise is total.
2. **First-party apps** — signed with the vendor app key. Same format as
   third-party; no extra privilege beyond capability grants.
3. **Third-party apps** — signed with a developer key, or unsigned.

Sandbox boundary is the WASM instance. An app can: exhaust its own memory,
trap, burn fuel, write garbage to its own private directory. An app cannot:
read another app's data, hold a native pointer, open a socket, name a
filesystem path, or prevent the launcher from being summoned.

**Unsigned apps** install from SD after an explicit warning and are denied all
network capabilities. This is the sideload path and it must remain open — a
device that only runs vendor-blessed code is not the open system in the goal.

Capability grants are declared in the manifest, shown at install, and enforced
at the host-function boundary. There is no runtime prompt: a capability not
granted at install is simply absent, and the corresponding host functions trap.

---

## 6. Memory budget (32 MiB PSRAM assumption)

| Region | Size | Owner | Notes |
|---|---:|---|---|
| Framebuffer A | 1.20 MiB | display | 1024x600 RGB565 |
| Framebuffer B | 1.20 MiB | display | double buffer; see R-03 |
| LVGL draw + retained tree | 0.75 MiB | ui | draw buffers preferentially in internal SRAM |
| Glyph atlas + shaping cache | 1.00 MiB | text | system-owned (ADR-S008) |
| Feed store (working set) | 0.50 MiB | feed | snapshot on SD, hot slice in PSRAM |
| Kernel heap + service state | 0.75 MiB | kernel | |
| **System subtotal** | **5.40 MiB** | | budget gate: 6.0 MiB |
| Foreground app linear memory | 1.00 MiB | app | manifest default; max 4 MiB |
| Foreground app host-side state | ≤ 6.00 MiB | host | TextView piece table + layout index |
| Warm slot 1 (state + layout index) | ≤ 1.50 MiB | launcher | |
| Warm slot 2 (state + layout index) | ≤ 1.50 MiB | launcher | |
| Document page cache | 2.00 MiB | docstore | |
| **Total committed** | **~17.4 MiB** | | headroom for fragmentation and JPEG scratch |

Layout index sizing: 8 bytes per paragraph (byte offset + cached height).
A 10,000-paragraph novel is 80 KiB. Full line layout is computed for the
viewport plus one screen of margin in each direction, and nothing else.

**At 16 MiB PSRAM** (OQ-2 resolving unfavourably): drop to one warm slot, halve
the page cache, and consider single-buffered display with a tear-avoidance
strategy. Record as a spec amendment if it happens.

---

## 7. Power and the radio

The C6 is powered down by default. Network activity happens in windows:

- On a user-initiated sync or fetch.
- On a scheduled feed poll (default: twice daily, user-configurable, minimum
  interval 1 hour — the floor is deliberate).

Between windows the C6 rail is off. This is the physical expression of "no
push": there is no radio available to receive one. It is also the single
biggest battery lever.

The kernel exposes no host function that powers the radio on directly. Apps
request work (`fetch.document`, `sync.push`) and the net service decides when
to open a window, coalescing requests.

---

## 8. Repository layout

```
sindri/
├── CLAUDE.md
├── Cargo.toml                 # workspace
├── rust-toolchain.toml        # pinned nightly + riscv32imafc target
├── docs/
│   ├── PLAN.md
│   ├── adr/                   # ADR-S###.md
│   └── specs/                 # SPEC-00..05
├── crates/
│   ├── boot/                  # Brokkr — no alloc, unsafe permitted
│   ├── hal/                   # MMIO, DMA, cache — unsafe permitted
│   ├── kernel/                # scheduler, time, power, panic handler
│   ├── services/
│   │   ├── display/  input/  storage/  net/  feed/  audio/  crypto/
│   ├── ui/
│   │   ├── lvgl-sys/          # FFI — unsafe permitted
│   │   ├── tree/              # retained tree + differ (host-testable)
│   │   └── textview/          # piece table, layout index, shaping
│   ├── runtime/
│   │   ├── host/              # ABI implementation, fuel, lifecycle
│   │   └── loader/            # .wdapp parse, verify, instantiate
│   ├── abi/                   # shared types + codec + conformance suite
│   └── panel/                 # MIPI-DSI — unsafe permitted
├── apps/
│   ├── launcher/  writer/  reader/   # WASM, built as wasm32-unknown-unknown
├── sdk/
│   ├── sindri-app/            # third-party app crate
│   └── templates/             # cargo-generate templates
├── firmware/c6/               # ESP-HOSTED slave config + build
├── tools/
│   ├── xtask/                 # build, flash, budget, package, sign
│   └── wdapp/                 # package/sign/verify CLI
├── boards/
│   └── devkit-p4-7in/         # pin map, partitions.csv, panel timings
└── tests/
    ├── device/                # probe-rs + defmt-test
    └── golden/                # parser corpora, fuzz seeds
```

---

## 9. Risk register

| ID | Risk | Impact | Likelihood | Mitigation | Owner phase |
|---|---|---|---|---|---|
| R-01 | Rust bootloader cannot init PSRAM — the P4 sequence is documented mainly in ESP-IDF source, and translating it is nontrivial and easy to get subtly wrong | High | **High** | Spike B1 in Phase 0. ADR-S010 fallback keeps everything else moving. Budget a full sprint | 0, 2 |
| R-02 | `esp-hal` ESP32-P4 support is incomplete for MIPI-DSI, PPA, or JPEG | High | Medium-High | Spike D1. Be prepared to write drivers in `sindri-hal` against the TRM. Do not assume a crate exists | 0, 1 |
| R-03 | Double-buffered 1024x600 in PSRAM cannot sustain the frame budget over the DSI link | Medium | Medium | Spike D1 measures achievable rate. Fallbacks: partial refresh via LVGL dirty rects, 30 Hz target, single buffer | 0, 1 |
| R-04 | LVGL Rust bindings are stale; LVGL 9 + `bindgen` in `no_std` may need a maintained fork | Medium | Medium | Generate bindings in-tree rather than depending on `lvgl-rs`. Contain in `lvgl-sys` | 1 |
| R-05 | `wasmi` interpreter too slow for the UI tree build at 60 Hz | High | Medium | Spike W1. Mitigations in order: shrink per-frame app work via retained/keyed diffing, move more into host widgets, then evaluate AOT | 0, 3 |
| R-06 | ESP-HOSTED from bare-metal Rust (rather than ESP-IDF) requires reimplementing the host protocol | Medium | Medium-High | Spike N1. Fallback: treat the C6 as a dumb modem with a custom minimal protocol we define | 0, 6 |
| R-07 | TLS on-device: memory footprint and certificate store maintenance | Medium | Medium | `embedded-tls` or `rustls` `no_std`; measure in Spike N1. Pinned root store, updated with firmware | 6 |
| R-08 | Third-party apps degrade the experience or brick sessions | Medium | Medium | Fuel metering (ADR-S020), memory cap, arena reset, per-app crash log, manifest budget refusal | 3, 4 |
| R-09 | ABI proves wrong after third-party apps exist and cannot be changed | **Very High** | Medium | Small 1.0. Conformance suite. Widgets/services over functions. Prototype the writer and reader against a draft ABI *before* freezing | 3, 5, 6 |
| R-10 | Mermaid layered layout consumes the schedule | Medium | High | ADR-S018 defers it behind a spike. Do not start it inside a milestone that promises anything else | later |
| R-11 | Support burden from an open ecosystem lands on a solo maintainer | Medium | High | Fault containment, per-app crash logs attributable to the app, an SDK with a conformance harness so contributors self-diagnose | 7 |
| R-12 | Input path unresolved (OQ-1) invalidates latency budgets | Medium | Medium | Resolve before Phase 1 exit. Prototype the two viable paths | 1 |

---

## 10. Requirements index

Requirements are numbered `R-<spec>-<nn>` and live in their owning spec.
This index exists so PRs can cite them.

| Range | Spec |
|---|---|
| R-01-nn | SPEC-01 Bootloader |
| R-02-nn | SPEC-02 Host ABI |
| R-03-nn | SPEC-03 Launcher and lifecycle |
| R-04-nn | SPEC-04 Writer |
| R-05-nn | SPEC-05 Reader |
