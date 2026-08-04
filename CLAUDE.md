# CLAUDE.md — Sindri Writer Deck

Working agreement for AI-assisted development in this repository. Read this
before touching any file. If an instruction here conflicts with a request in
the chat, say so and stop.

---

## 1. What this project is

A bare-metal (`no_std`) Rust firmware for an ESP32-P4 based writer deck with a
7" 1024x600 display and an ESP32-C6 companion radio. It ships a Rust
second-stage bootloader, a single-app-at-a-time supervisor with Android-style
activity lifecycles, and a WASM sandbox that hosts both first-party and
third-party applications.

**The system is open.** Third parties will write apps against the host ABI.
That single fact drives most of the constraints below.

Authoritative documents, in precedence order:

1. `docs/specs/SPEC-02-host-abi.md` — the frozen interface. Highest leverage
   document in the repo. Nothing may be added to it casually.
2. `docs/specs/SPEC-00-system.md` — architecture, ADR ledger, risk register.
3. `docs/PLAN.md` — phases, gates, exit criteria.
4. Remaining specs (`SPEC-01`, `SPEC-03`..`SPEC-05`).

---

## 2. Audit before implementation

Non-negotiable, and it is the rule most often violated under time pressure.

Before writing code for any task:

1. Read the governing spec section. Quote the requirement ID you are
   satisfying (`R-xx-nn`) in the PR description.
2. Search the tree for an existing implementation. This codebase has
   deliberate single points of truth; a second copy is a defect.
3. State what you are about to change and what could break. If the change
   touches `crates/abi/` or `crates/runtime/host/`, stop and ask.

If a spec is wrong or silent, **amend the spec first** in a separate commit,
then implement. Do not implement against an unwritten assumption.

---

## 3. Reporting findings

Every measurement, benchmark, or investigation is reported in three parts:

- **Observed** — what was actually measured, with conditions: build profile,
  clock config, PSRAM part and mode, cache config, sample count, whether the
  device was warm.
- **Not established** — what the measurement does *not* show. Be specific.
- **What would settle it** — the concrete next measurement.

A number without its measurement conditions is not a finding. A benchmark run
once is not a benchmark. Warm-up artifacts are a known failure mode here: the
first frames after boot are not representative of steady state, and the PSRAM
allocator's high-water mark early in a session is not its floor.

Beware the recurring pattern: **a signal that looks like the healthy one,
produced by a different mechanism.** A green test that passes because the code
path was never reached. A memory number that looks stable because the
allocator never released. Every detector needs a paired true negative — a case
that *should* trip it and does.

---

## 4. Engineering standards

### Universal

- Apache-2.0 SPDX header on **line 1** of every source file:
  `// SPDX-License-Identifier: Apache-2.0`
- **300-line ceiling** per source file. At 250 lines, plan the split. The
  ceiling is a design signal, not a formatting rule — a file that wants to be
  400 lines usually wants to be two modules.
- Typed errors via `thiserror` (or a `no_std`-compatible equivalent, see
  below). No `Box<dyn Error>`, no stringly-typed errors.
- No `unwrap()` / `expect()` / `panic!()` / indexing-panic in library code.
  Enforced by `clippy.toml` and a CI lint. Permitted only in `#[cfg(test)]`
  and in `main`-equivalent entry points where the panic is the documented
  failure mode.
- All user-visible strings go through `fl!()` (Fluent). A literal in a UI path
  is a defect. This applies to third-party apps too — the SDK exposes the same
  mechanism.
- One ADR per non-trivial decision, in `docs/adr/`, numbered `ADR-S###`.
  "Non-trivial" means: it constrains a future choice, or a reasonable engineer
  would have chosen differently.

### `no_std` specifics

- `thiserror` requires `std` in older versions; this workspace uses
  `thiserror` with `default-features = false` (2.x supports `no_std`). If that
  breaks, the fallback is `derive_more` — record the switch as an ADR, do not
  hand-roll `impl Display`.
- `alloc` is available everywhere except the bootloader. The bootloader is
  strictly allocation-free.
- No `std::time`. All time comes from `crates/kernel/time`.

### `unsafe`

`#![forbid(unsafe_code)]` at the crate root of **every** crate except the
following, which are the only crates permitted to contain `unsafe`:

| Crate | Why |
|---|---|
| `sindri-boot` | MMU/cache setup, PSRAM init, jump to image |
| `sindri-hal` | MMIO, DMA descriptors, cache maintenance |
| `sindri-lvgl-sys` | FFI to LVGL C |
| `sindri-panel` | MIPI-DSI framebuffer handoff |

In those four crates:

- Every `unsafe` block carries a `// SAFETY:` comment stating the invariant
  being upheld and who upholds it. A block without one fails review.
- `unsafe` is confined to the smallest possible scope and wrapped in a safe
  abstraction at the module boundary. Callers outside the crate never see it.
- Adding a fifth crate to this table requires an ADR.

### Testing

- Host-side unit tests for everything that can be tested off-device
  (parsers, the piece table, the UI tree differ, the ABI codec, the manifest
  validator). These run on `cargo test` with `std`.
- On-device integration tests via `probe-rs` + `defmt-test`, in `tests/device/`.
- Every host ABI function has a **conformance test** in
  `crates/abi/conformance/` that a third-party runtime or a future rewrite
  must pass. Treat this suite as the executable half of `SPEC-02`.
- Fuzz the parsers: RSS/Atom, EPUB container, the app manifest, the UI tree
  codec, and the `.wdapp` package header. These all consume hostile input.

---

## 5. Budgets are gates, not aspirations

CI fails on regression against these. Do not raise a budget to make a build
pass; raise it in a spec amendment with justification, or fix the code.

| Budget | Value | Where enforced |
|---|---|---|
| Warm app resume | ≤ 250 ms | `tests/device/resume_bench.rs` |
| Cold app launch | ≤ 600 ms | same |
| Launcher summon (from app keypress to first launcher frame) | ≤ 120 ms | same |
| System reserved PSRAM | ≤ 6.0 MiB | `xtask budget` |
| Per-app default heap | 1.0 MiB | manifest default |
| Frame budget | 16.6 ms at 60 Hz; degrade to 30 Hz before dropping input | `xtask budget` |
| Bootloader image size | ≤ 48 KiB | linker + CI check |
| Sustained input latency (keypress → glyph) | ≤ 50 ms p99 | `tests/device/input_bench.rs` |

Memory measurement rule: report **resident PSRAM high-water after a
30-second idle following a full document load**, not peak-during-load and not
first-frame. State the document used.

---

## 6. Things that are structurally forbidden

These are not style preferences. They are the properties that make the device
what it is, and they are enforced in code, not in review.

- **No sockets in the app ABI.** Apps never get a raw network handle. They get
  `feed.*`, `fetch.document`, and `sync.*`. This makes a chat client
  unimplementable rather than discouraged. Do not add a socket host function
  "temporarily."
- **No push, no server-initiated anything.** All network activity is
  scheduled and pull-shaped. The radio is power-gated between windows.
- **No unread badges, no notification API.** There is no host function to
  interrupt the user.
- **No LVGL handles across the ABI.** Apps submit a declarative tree; the host
  owns every native object. See `SPEC-02 §4`.
- **No filesystem paths across the ABI.** Apps get a private directory handle
  and documents delivered explicitly through the system picker.
- **No dynamic code loading inside an app.** No `eval`-equivalent, no nested
  module instantiation.

If a feature request requires breaking one of these, the answer is that the
feature does not belong on this device. Escalate rather than working around it.

---

## 7. The ABI is frozen once shipped

`crates/abi/` and `docs/specs/SPEC-02-host-abi.md` change together, always, in
the same commit.

- **Additive changes** bump minor: `writerdeck:app@1.1.0`.
- **Any removal or semantic change** bumps major and breaks every installed
  app. Assume you will never be allowed to do this.
- New host functions require: an ADR, a conformance test, a fuzz target if
  they parse anything, and a written answer to "what does a hostile app do
  with this?"
- Ship a deliberately small 1.0. Additive is easy; removal is impossible.

Before adding anything to the ABI, ask whether it can be a host-side *widget*
or *service* instead of a function. Widgets and services can be reimplemented
freely; functions cannot.

---

## 8. Commits and change hygiene

- Conventional commits, scoped to the crate: `feat(runtime): ...`,
  `fix(boot): ...`, `spec(abi): ...`.
- One logical change per commit. Spec amendments are their own commit.
- Every PR states: requirement IDs satisfied, budgets affected (with before/
  after numbers), ADRs added, and whether the ABI changed.
- Never commit a budget regression with a "will fix later" note. Either the
  budget moves in the spec or the code does.

---

## 9. Physical-presence checks

Some things cannot be validated in CI and must be confirmed on hardware by a
human. When an item requires one, mark it `[PRESENCE]` in the plan and do not
mark the phase complete without it. Current standing items:

- Panel tearing and refresh behaviour under sustained scroll — requires eyes
  on the device, not a frame counter.
- Sleep/wake power draw with the C6 gated — requires a meter.
- Keyboard ghosting and n-key rollover on the chosen input path.
- Thermal behaviour at sustained 400 MHz with PSRAM under load.

---

## 10. When you are unsure

Say so, in the Observed / Not established / What would settle it form. A
flagged unknown is cheap. A confident guess that turns out wrong costs a
remediation program. This project has a hard dependency on several
under-documented ESP32-P4 behaviours (see the risk register in `SPEC-00 §9`);
guessing at them is the single most likely way to lose a month.
