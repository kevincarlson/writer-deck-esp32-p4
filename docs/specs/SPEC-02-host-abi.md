# SPEC-02 — Host ABI `writerdeck:app@1.0.0`

**Status:** Draft v0.1.0 — **NOT FROZEN**
**Depends on:** SPEC-00
**Freeze gate:** Phase 6 exit. Both the writer and the reader must be built
against this interface *before* it is declared 1.0.

---

## 0. Read this first

Once a third-party app exists in the wild, this document is frozen. Additive
changes bump minor. Removals and semantic changes bump major and break every
installed app; assume you will never be permitted to do that.

Before adding a function here, ask: **can this be a host widget or a host
service instead?** Widgets and services can be reimplemented freely. Functions
cannot. The 1.0 surface is deliberately small — the writer and the reader are
built on it precisely to prove that it is sufficient, and any function they do
not need does not ship.

The interface is described below in WIT-like notation. Whether the
implementation uses the component model or hand-written `wasmi` host functions
is an implementation choice (see §11); the *contract* is what is frozen.

---

## 1. Shape of the interface

```
writerdeck:app@1.0.0
├── interface lifecycle    (app exports)
├── interface ui           (host imports — declarative tree)
├── interface input        (host → app events)
├── interface text         (host imports — TextView command surface)
├── interface doc          (host imports — document handles)
├── interface store        (host imports — private KV + private dir)
├── interface feed         (host imports — capability: network.feed)
├── interface net          (host imports — capability: network.fetch)
├── interface sync         (host imports — capability: sync)
├── interface audio        (host imports — capability: audio)
└── interface sys          (host imports — time, log, l10n, locale)
```

Eleven interfaces, ~60 functions total. Compare with wrapping LVGL directly:
several hundred entry points and a lifetime-management problem across a trust
boundary.

---

## 2. Lifecycle (app exports)

```wit
interface lifecycle {
  record saved-state { blob: list<u8> }

  /// Cold start. No prior state.
  start: func(launch: launch-intent) -> result<_, app-error>;

  /// Warm start. `blob` is exactly what suspend returned.
  resume: func(state: saved-state, launch: option<launch-intent>)
        -> result<_, app-error>;

  /// The instance is being torn down. Return small, serialisable state.
  /// Persistent state MUST already be committed before this returns.
  suspend: func() -> saved-state;

  /// A frame is due. Build and commit the UI tree.
  frame: func(now-ms: u64) -> frame-outcome;

  /// Delivered before suspend when possible, but NOT guaranteed.
  low-memory: func();

  variant launch-intent {
    none,
    open-document(doc-handle),
    resume-last,
  }

  enum frame-outcome { idle, animating }
}
```

The Rust SDK expresses this as a trait whose `suspend` consumes the receiver:

```rust
pub trait Activity: Sized {
    type Saved: Encode + Decode;
    fn start(ctx: &mut Ctx, launch: LaunchIntent) -> Result<Self, AppError>;
    fn resume(ctx: &mut Ctx, saved: Self::Saved, launch: Option<LaunchIntent>)
        -> Result<Self, AppError>;
    fn suspend(self) -> Self::Saved;   // consumes — no live state survives
    fn frame(&mut self, ctx: &mut Ctx, now_ms: u64) -> FrameOutcome;
}
```

### R-02-01 — Saved state is small
`saved-state.blob` is capped at **16 KiB**. Exceeding it fails the suspend and
logs a defect against the app. Saved state reconstructs a *view*; it is not a
document store. Documents persist through `doc`.

### R-02-02 — Suspend must not perform I/O beyond commit
Persistent writes happen during normal operation and are flushed before
`suspend` is called. `suspend` itself has a hard 50 ms budget; overrun is
logged and the app is killed rather than waited for.

### R-02-03 — Kill after suspend is unannounced
An app that has been suspended may be discarded with no further callback. No
guarantee is made that `resume` will ever be called with a given blob.

### R-02-04 — Resume budget
Warm resume to first committed frame: ≤ 250 ms. Cold start: ≤ 600 ms.
Enforced as a CI gate for first-party apps and reported (not enforced) for
third-party apps in the SDK harness.

---

## 3. States

Three, per ADR-S005:

| State | Instance | Linear memory | Saved blob | Host-side state |
|---|---|---|---|---|
| Foreground | live | resident | — | resident |
| Suspended | destroyed | freed | held (warm slot) | layout index held in warm slot |
| Cold | none | none | on SD or absent | none |

There is no Paused. The launcher is opaque and full-screen; there is nothing
an app could usefully do while it is up.

Transitions are driven solely by the launcher (SPEC-03). An app cannot request
its own foregrounding, cannot start another app, and cannot prevent its own
suspension.

---

## 4. UI — declarative keyed tree

Per ADR-S003, apps never hold a native widget handle. Each frame the app
serialises a tree into its own linear memory and commits it. The host diffs
against the retained tree and drives LVGL natively.

```wit
interface ui {
  /// Commit a serialised node tree. `ptr`/`len` address app linear memory.
  commit: func(ptr: u32, len: u32) -> result<_, ui-error>;

  /// Query the current viewport in logical px.
  viewport: func() -> rect;

  /// Request a frame even if no input occurred (animation).
  invalidate: func();

  /// Scroll a keyed node into view. Host-driven so scrolling stays smooth
  /// when the app is slow.
  scroll-to: func(key: u32, align: scroll-align);
}
```

### Node types (1.0)

| Node | Purpose |
|---|---|
| `column` / `row` | flex containers: gap, padding, align, justify |
| `stack` | z-ordered overlay |
| `scroll` | scrollable viewport, host-driven momentum |
| `label` | single- or multi-line static text, style token |
| `text-view` | **binds a host TextView by handle** (§5) |
| `block-list` | paginated rich-text blocks (§5.3) — reader path |
| `button` | label or icon, action id |
| `list` / `list-item` | virtualised; app supplies a windowed range |
| `field` | single-line input |
| `toggle` / `slider` | settings primitives |
| `icon` | from a fixed system icon set |
| `divider` / `spacer` | |
| `sheet` | modal panel, host-managed dismissal |
| `progress` | determinate or indeterminate |
| `canvas` | immediate-mode vector sink — see R-02-08 |

No `image` node in 1.0 beyond `icon`; image display arrives with the reader's
needs in Phase 6 and is added as a minor bump if the reader actually requires
it.

### R-02-05 — Keys are mandatory and stable
Every node carries a `u32` key unique among its siblings and stable across
frames for the same logical element. Unstable keys are the app's bug and
manifest as lost focus, reset scroll, and animation restarts.

### R-02-06 — Styling is by token, not by value
Nodes reference style tokens (`text.body`, `text.heading-1`, `surface.raised`,
`accent`) from the system theme. Apps cannot set arbitrary colours, fonts, or
sizes. This is what gives third-party apps a coherent look without asking
contributors to cooperate, and it is what makes a system-wide theme or a
future e-ink variant possible.

### R-02-07 — Tree size cap
A committed tree is capped at 4096 nodes and 256 KiB encoded. Lists must be
virtualised. Exceeding the cap fails the commit with `ui-error::too-large`;
the previous frame remains on screen.

### R-02-08 — Canvas is bounded
`canvas` accepts a bounded display-list (lines, polylines, rects, arcs, text
runs, fills) capped at 8192 primitives. It exists for Mermaid diagrams and
sketching. It is not a general drawing API and gains no new primitive without
an ADR.

### Encoding

A compact little-endian binary format, not JSON. Fixed 8-byte node header
(type, key, child count, flags) plus type-specific payload. Strings are
`(offset, len)` into a per-frame string arena appended to the tree. The codec
lives in `crates/abi/` and is shared verbatim by host and SDK, so it is
host-testable and fuzzed as a unit.

---

## 5. Text — the load-bearing widget

Per ADR-S012, the piece table, the paragraph offset index, shaping, and line
layout are **host-side**. This is the largest memory consumer in the system
and it must be under the budget enforcer's control. It also keeps typing off
the fuel path: a keystroke does not require the app to run.

```wit
interface text {
  /// Create a view over an open document. Layout index is built or restored.
  create: func(doc: doc-handle, mode: view-mode) -> result<text-handle, text-error>;
  destroy: func(h: text-handle);

  /// Editing. Offsets are byte offsets into the document.
  insert: func(h: text-handle, at: u64, s: string) -> result<_, text-error>;
  delete: func(h: text-handle, range: span) -> result<_, text-error>;
  replace: func(h: text-handle, range: span, s: string) -> result<_, text-error>;

  /// Undo/redo are host-side over the append log.
  undo: func(h: text-handle) -> bool;
  redo: func(h: text-handle) -> bool;

  /// Selection and cursor.
  selection: func(h: text-handle) -> span;
  set-selection: func(h: text-handle, s: span);

  /// Read a bounded slice. Cap: 64 KiB per call.
  read: func(h: text-handle, range: span) -> result<string, text-error>;

  /// The visible byte range, updated by the host on scroll.
  visible-range: func(h: text-handle) -> span;

  /// Style runs for the visible range (ADR-S013).
  set-style-runs: func(h: text-handle, runs: list<style-run>);

  /// Structural outline the app derives; host renders navigation from it.
  set-outline: func(h: text-handle, items: list<outline-item>);

  /// Counts, computed host-side because they need the whole document.
  stats: func(h: text-handle) -> text-stats;   // words, chars, paras, est. pages

  enum view-mode { edit, read-only }
  record style-run { start: u64, len: u32, token: style-token }
  record outline-item { offset: u64, depth: u8, label: string }
}
```

### R-02-09 — Syntax classification is app-side, visible range only
On `event::visible-range-changed` the app classifies the newly visible span
and calls `set-style-runs`. Roughly 40–60 lines per call. This is what lets a
third party ship a syntax mode without an ABI change, and it bounds the fuel
cost of highlighting to the viewport.

### R-02-10 — Layout is windowed
The host computes full line layout for the viewport plus one screen of margin
in each direction. Beyond that it holds an 8-byte-per-paragraph index of byte
offset plus cached height. Height for never-laid-out paragraphs is estimated
and corrected lazily; the estimate is recorded so scrollbar jitter is a known
quantity rather than a mystery.

### R-02-11 — Block list for read-only rich content
`block-list` renders a host-laid-out sequence of blocks the app supplies:
paragraph, heading, list-item, blockquote, code, rule, and (later) image, each
with inline spans (emphasis, strong, code, link). The reader parses XHTML in
WASM and emits blocks (ADR-S014). No HTML engine exists on the device and none
will.

---

## 6. Documents

Per ADR-S019, **no path strings cross the ABI**.

```wit
interface doc {
  /// Handles arrive via launch-intent or the system picker. Never constructed.
  open: func(h: doc-handle, mode: open-mode) -> result<_, doc-error>;
  close: func(h: doc-handle);
  meta: func(h: doc-handle) -> doc-meta;      // name, ext, size, mtime

  /// Bounded read for apps that parse their own format (EPUB, .mmd).
  read-range: func(h: doc-handle, off: u64, len: u32) -> result<list<u8>, doc-error>;

  /// Whole-file write for non-TextView formats. Atomic: temp + rename.
  write-all: func(h: doc-handle, ptr: u32, len: u32) -> result<_, doc-error>;

  /// Ask the system to present a picker. Returns on a later event.
  request-pick: func(filter: list<string>, purpose: pick-purpose);

  /// Create a new document in the app's declared document scope.
  request-create: func(suggested-name: string, ext: string);

  /// Snapshot history (SPEC-04 §6). Host-owned append log.
  snapshots: func(h: doc-handle) -> list<snapshot-meta>;
  restore: func(h: doc-handle, id: snapshot-id) -> result<_, doc-error>;
}
```

### R-02-12 — Handles are unforgeable and scoped
A `doc-handle` is a host-side index with a generation counter. It is valid
only for the app instance that received it and only until suspend. After
resume the app must re-request; the saved blob stores an opaque
`doc-reference` the host can resolve back to a handle.

### R-02-13 — Archive access is mediated
An EPUB or CBZ is a container. The host exposes `container.entry-list` and
`container.read-entry` for ZIP-based formats rather than making every app
implement (and mis-implement) ZIP. Zip-bomb and traversal defences live in one
audited place.

---

## 7. Network — intent-shaped, no sockets

Per ADR-S004 this is where the distraction model survives contact with
strangers. There is no socket, no persistent connection, no server-initiated
message, and no way for an app to power the radio.

```wit
interface feed {                         // capability: network.feed
  subscribe: func(url: string, title: option<string>) -> result<feed-id, feed-error>;
  unsubscribe: func(id: feed-id);
  list: func() -> list<feed-info>;

  /// Read the local snapshot. Never triggers network.
  entries: func(id: option<feed-id>, since: option<u64>, limit: u32)
         -> list<entry-meta>;
  entry-body: func(e: entry-id) -> result<list<block>, feed-error>;
  mark-read: func(e: entry-id);

  /// Ask the net service to include this feed in the next window.
  /// Does NOT open a window; returns immediately.
  request-refresh: func(id: option<feed-id>);
  last-refresh: func() -> option<u64>;
}

interface net {                          // capability: network.fetch
  /// One-shot document fetch. Rate-limited, no streaming, no headers.
  /// Result arrives as an event carrying a doc-handle.
  fetch-document: func(url: string, accept: list<string>) -> request-id;
}

interface sync {                         // capability: sync
  push: func(h: doc-handle) -> request-id;
  pull: func() -> request-id;
  status: func() -> sync-status;
}
```

### R-02-14 — No unbounded feed surface
`feed.entries` requires a `limit` and the host caps it at 200. There is no
"all entries" call. The snapshot itself is bounded by the feed service's
retention policy.

### R-02-15 — No notification path
There is no host function that draws attention, sets a badge, plays an alert,
or wakes the screen. Refresh completion arrives as an event to a *foreground*
app only. A suspended app learns nothing until it resumes.

### R-02-16 — Fetch is rate-limited and attributable
`fetch-document` is limited per app per window and every request is logged
with the requesting app id. An app that burns its budget gets errors, not a
queue.

### R-02-17 — Parsing is host-side
RSS/Atom parsing, HTTP, and TLS are host services (ADR-S015). Hostile XML must
not be in an app's fuel budget, and there is exactly one TLS implementation to
audit and one certificate store to maintain.

---

## 8. Storage, audio, and system

```wit
interface store {                        // always granted
  get: func(key: string) -> option<list<u8>>;
  set: func(key: string, val: list<u8>) -> result<_, store-error>;
  delete: func(key: string);
  list-keys: func(prefix: string) -> list<string>;
  quota: func() -> quota-info;           // used, limit
}

interface audio {                        // capability: audio
  speak: func(text: string, voice: voice-id) -> utterance-id;  // TTS
  stop: func(u: utterance-id);
  tone: func(kind: ambient-kind, level: u8);   // procedural noise only
}

interface sys {                          // always granted
  now-ms: func() -> u64;
  monotonic-us: func() -> u64;
  locale: func() -> string;
  localise: func(key: string, args: list<tuple<string, string>>) -> string;
  log: func(level: log-level, msg: string);
  capabilities: func() -> list<string>;  // what this app was actually granted
  battery: func() -> battery-info;
}
```

### R-02-18 — Private store, quota-enforced
Default 256 KiB per app, declarable up to 4 MiB in the manifest. Keys are
namespaced to the app; there is no shared store and no cross-app read.

### R-02-19 — No arbitrary audio playback
`audio` exposes TTS and procedural ambient tones. There is no sample playback
and no audio file decode. A music player is not implementable, which is
intentional.

### R-02-20 — Capability introspection is honest
`sys.capabilities` returns what was granted, not what was requested, so a
well-written app can degrade gracefully rather than trapping.

---

## 9. Packaging, signing, and manifest

An app ships as a `.wdapp`: a header, a CBOR manifest, the WASM module,
optional assets and Fluent bundles, and an Ed25519 signature over the whole
(ADR-S011).

```toml
[app]
id = "codes.kevincarlson.writer"      # reverse-DNS, immutable
name = "Writer"                        # localisable via bundles
version = "1.0.0"
abi = "^1.0"                           # semver range; major mismatch refuses

[budget]
memory-pages = 16                      # 64 KiB pages -> 1.0 MiB
fuel-per-frame = 5_000_000
store-quota-kib = 256

[capabilities]
grants = ["documents.md", "documents.fountain", "audio"]

[documents]
handles = ["md", "markdown", "fountain"]
```

### R-02-21 — Budgets are refused at load, not enforced at crash
The loader refuses to instantiate an app declaring more memory than the free
budget allows, and reports the shortfall. This turns an OOM crash into a
comprehensible install-time message.

### R-02-22 — Unsigned apps get no network
An unsigned `.wdapp` installs from SD after an explicit warning and is denied
`network.*` and `sync` unconditionally. The sideload path stays open; the
network path does not.

### R-02-23 — Signature covers everything
Header, manifest, module, and assets. A verified app whose assets were swapped
is a failed verification, not a warning.

---

## 10. Fuel, memory, and containment

### R-02-24 — Fuel is refilled per frame
The app receives `fuel-per-frame` (manifest, capped at 20 M) at each frame
boundary. Exhaustion traps. The host then: logs, suspends the app with a
synthetic empty saved state, and returns to the launcher with a "stopped
responding" notice attributable to the app by id.

This is the mechanism that keeps the launcher-always-reachable promise
against a third-party infinite loop. **Build it in Phase 3, not later** — it
is the difference between a buggy app and a bricked session.

### R-02-25 — Linear memory is hard-capped
`memory.grow` beyond the manifest cap fails inside the sandbox. The app OOMs
in its own address space where it can do no harm.

### R-02-26 — Traps are contained and attributable
A trap destroys the instance and returns to the launcher. It never takes down
the kernel. A per-app crash log with the trap reason and last committed frame
key is written to `applocal`, so a bug report names the right app (R-11).

### R-02-27 — No dynamic code loading
No nested instantiation, no `eval`-equivalent, no host function that accepts
WASM bytes.

---

## 11. Versioning and implementation notes

- Interface version is `writerdeck:app@MAJOR.MINOR.PATCH`. The loader refuses
  a major mismatch and warns on an unknown minor (forward compatibility is not
  promised; an app requesting `^1.3` on a `1.2` host is refused).
- Evaluate WIT and the component model even if 1.0 ships hand-written `wasmi`
  host functions. The tooling around versioned interfaces is materially better
  than hand-rolled `extern "C"`, and the SDK can generate bindings either way.
  Record the decision as an ADR before Phase 3 closes.
- `crates/abi/` is shared verbatim between host and SDK. Divergence between
  them is the failure mode that produces "works in the emulator, traps on
  device."

### R-02-28 — Conformance suite is the executable spec
Every function has a test in `crates/abi/conformance/`, runnable against the
device and against the host-side simulator. A change that does not update the
conformance suite is incomplete.

### R-02-29 — Host-side simulator is a first-class deliverable
Third parties cannot be expected to own hardware. `sdk/` ships a desktop
simulator running the same ABI implementation against a windowed LVGL, so an
app can be developed and conformance-checked without a device. This is also
how first-party apps get fast iteration and how the ABI gets host-side tests.
