# Sindri — Writer Deck Firmware

Bare-metal Rust firmware for an ESP32-P4 writer deck: 7" 1024x600 display,
ESP32-C6 companion radio, SD storage. Rust second-stage bootloader, a
single-app-at-a-time supervisor with Android-style activity lifecycles, and a
WASM sandbox hosting first- and third-party apps.

**Status: pre-implementation.** Phase 0 spikes are blocking. See
`docs/PLAN.md`.

---

## Where to start

| If you want to | Read |
|---|---|
| Contribute code | `CLAUDE.md` — the working agreement, first |
| Understand the architecture | `docs/specs/SPEC-00-system.md` |
| Write an app | `docs/specs/SPEC-02-host-abi.md`, then `sdk/` |
| Know what happens when | `docs/PLAN.md` |
| Know why a decision was made | `docs/adr/` |
| See where Phase 0 stands | `docs/PHASE-0.md`, then `spikes/` |
| Read a measurement | `docs/findings/` |

---

## What this device deliberately cannot do

These are structural properties enforced in the ABI, not settings:

- No sockets in the app interface. A chat client is unimplementable.
- No push, no notifications, no unread badges, no attention-getting API.
- No arbitrary audio playback. TTS and procedural ambience only.
- No filesystem paths across the sandbox boundary.
- No unbounded feed history. Retention is capped and there is no "load older".
- The radio is powered down between scheduled windows.

`CLAUDE.md` §6 is the authoritative list. If a feature needs one of these
broken, it does not belong on this device.

---

## Layout

```
crates/boot        Brokkr — Rust second-stage bootloader (no alloc)
crates/hal         MMIO, DMA, cache            [unsafe permitted]
crates/kernel      scheduler, time, power, panic
crates/panel       MIPI-DSI                     [unsafe permitted]
crates/services/   display input storage net feed audio crypto
crates/ui/         lvgl-sys, tree (retained + differ), textview
crates/runtime/    host (ABI, fuel, lifecycle), loader (.wdapp)
crates/abi/        shared types, codec, conformance suite
apps/              launcher, writer, reader  (wasm32-unknown-unknown)
sdk/               sindri-app crate, desktop simulator, templates
firmware/c6/       ESP-HOSTED slave build
tools/xtask        build · flash · monitor · budget · image
tools/wdapp        package · sign · verify
boards/            pin maps, panel timings, partitions.csv
tests/device       probe-rs + defmt-test
tests/golden       parser corpora, fuzz seeds
```

Only four crates may contain `unsafe`: `boot`, `hal`, `lvgl-sys`, `panel`.
Every other crate carries `#![forbid(unsafe_code)]`. Adding a fifth requires
an ADR.

---

## Building

```bash
cargo xtask build --board devkit-p4-7in --profile release
cargo xtask image                       # ESP image format, signed
cargo xtask flash --port /dev/ttyUSB0   # full flash incl. partitions
cargo xtask monitor                     # defmt over RTT
cargo xtask budget                      # memory + size gates (CI-enforced)
```

Apps build separately for `wasm32-unknown-unknown`:

```bash
cargo xtask app build writer
cargo wdapp package target/wasm32-unknown-unknown/release/writer.wasm \
      --manifest apps/writer/manifest.toml --key dev.ed25519
cargo xtask app install writer.wdapp    # over USB, or copy to SD
```

Develop without hardware:

```bash
cargo run -p sindri-simulator -- apps/writer
```

The simulator runs the same ABI implementation against a windowed LVGL, so
third parties can build and conformance-check an app without a device
(R-02-29).

---

## Budgets

CI fails on regression. Do not raise a budget to make a build pass.

| Budget | Value |
|---|---|
| Warm app resume | ≤ 250 ms |
| Cold app launch | ≤ 600 ms |
| Launcher summon | ≤ 120 ms |
| System reserved PSRAM | ≤ 6.0 MiB |
| Bootloader image | ≤ 48 KiB |
| Keypress → glyph, p99 | ≤ 50 ms |

Measurement conditions matter and are specified in `CLAUDE.md` §5. A number
without its conditions is not a finding.

---

## License

Apache-2.0. SPDX header on line 1 of every source file.
