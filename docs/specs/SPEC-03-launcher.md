# SPEC-03 — Launcher, Lifecycle Supervisor, and App Installation

**Status:** Draft v0.1.0
**Depends on:** SPEC-00, SPEC-02

---

## 1. Scope

Three things that are easy to conflate and must not be:

1. **The lifecycle supervisor** — kernel-side, native, owns app state
   transitions, warm slots, fuel, and the arena. Not an app.
2. **The launcher UI** — a WASM app like any other, but privileged: it is the
   only app permitted to request a transition.
3. **The installer** — a launcher screen plus a kernel-side verifier.

Keeping the supervisor out of WASM is what makes the always-reachable promise
enforceable: the code that summons the launcher cannot itself be starved by
the app it is rescuing you from.

---

## 2. The summon path

### R-03-01 — Summon is handled below the app
The summon gesture (a dedicated key, or a chord — resolves with OQ-1) is
intercepted by the **input service**, before event dispatch to the foreground
app. The app never sees it and cannot consume, delay, or veto it.

### R-03-02 — Summon budget
From key event to first launcher frame: ≤ 120 ms, including a cooperative
suspend of the outgoing app.

### R-03-03 — Uncooperative apps do not extend the budget
The supervisor calls `suspend` with a 50 ms budget (R-02-02). On overrun or
trap it discards the instance, synthesises an empty saved state, and
continues. The launcher appears on schedule regardless of what the app does.

Consequence worth stating plainly: an app that misbehaves loses unsaved view
state, not document state. Document state is committed continuously
(SPEC-04 §6), which is why the 50 ms cap is affordable.

---

## 3. Lifecycle supervisor

```
        launch / open-intent
Cold ─────────────────────────► Foreground
  ▲                              │  ▲
  │  evict (warm slot pressure)  │  │ resume (warm slot hit)
  │                              ▼  │
  └────────────────────────── Suspended
              discard
```

### R-03-04 — One foreground instance
Exactly one WASM instance is live at a time. The launcher counts: while it is
foreground, no app instance exists.

Note the implication for the launcher's own budget — it is instantiated on
every summon. Its cold start is on the critical path of R-03-02, so it is the
one app whose start time is measured on every single transition. Keep it
small; a launcher that grows a settings tree with a hundred nodes will show up
in that number.

### R-03-05 — Two warm slots
The supervisor retains up to two suspended apps' saved blobs **and** their
host-side layout indexes in PSRAM (ADR-S016). Eviction is least-recently-
foregrounded. A warm hit must not touch SD.

Warm slot contents per app: saved blob (≤ 16 KiB), TextView layout index
(8 bytes/paragraph), open document references, and scroll geometry. Budgeted
at ≤ 1.5 MiB per slot (SPEC-00 §6).

### R-03-06 — Arena reset is the transition
Each app allocates host-side from a dedicated arena. At suspend the arena is
**reset wholesale, not freed piecemeal** (ADR-S007). Every app switch is
therefore a guaranteed defragmentation event, which is what makes multi-day
uptime with LVGL widget churn survivable.

The paired check: a debug assertion that the arena's high-water mark returns
to its baseline after reset. If it does not, something escaped the arena and
that is a defect, not a leak to be tolerated.

### R-03-07 — Health signal clears the trial flag
On reaching Foreground with the launcher rendered and input responsive, the
kernel clears the boot trial flag (R-01-06). "Booted" is not the health
signal; "reached an interactive launcher" is. A kernel that boots and hangs
before the launcher must roll back.

### R-03-08 — Resume measurement conditions
The 250 ms warm budget is measured as: summon → select app → first committed
frame, on a device idle for 30 s, with a specified document (`tests/golden/
novel-10k.md`), median of 20 runs, first 3 discarded as warm-up. Report the
distribution, not just the median — a p99 of 800 ms with a 200 ms median is a
failure that a median hides.

---

## 4. Launcher UI

Deliberately plain. It is a switcher, not a desktop.

**Screens:**

- **Apps** — grid or list of installed apps, most-recent first. Suspended apps
  are marked; selecting one resumes rather than restarts.
- **Documents** — recent documents across all apps, from the host-side recents
  index. Selecting one launches the handling app with `open-document`.
- **Now** — a single screen showing: current word-count goal progress, last
  sync time, battery, storage. Read-only. No feed counts (R-02-15).
- **Settings** — theme, locale, sleep timeout, sync schedule, feed poll
  interval, per-app capabilities, storage management.
- **Install** — §5.

### R-03-09 — No badges, no counts of unread anything
The launcher displays no unread count for any feed or document. This is a
structural property, not a default that can be toggled.

### R-03-10 — Recents index is host-owned
The document recents list lives in the kernel, not in any app's store. Apps do
not read it; the launcher receives it through a privileged interface.

### R-03-11 — Launcher privilege is explicit and narrow
`writerdeck:launcher@1.0.0` is a **separate** interface, granted only to the
app whose id matches the one recorded in `sysdata` at manufacture. It adds:
`launch(app-id, intent)`, `resume(app-id)`, `kill(app-id)`, `installed-apps()`,
`recents()`, `install(doc-handle)`, `uninstall(app-id)`, `set-capability(...)`.
It is versioned separately and is not part of the frozen public ABI, so it can
evolve with the firmware.

---

## 5. Installation

### R-03-12 — Sideload from SD is always available
`.wdapp` files on SD appear on the Install screen. There is no gate that can
remove this path. A device that can only run vendor-blessed code is not the
open system in the goal (SPEC-00 §1).

### R-03-13 — Install is a kernel operation
Package parsing, signature verification, and manifest validation happen in
`crates/runtime/loader`, not in the launcher WASM. Hostile package input must
not be parsed inside a sandbox that then reports its own verdict.

### R-03-14 — Install-time disclosure
Before install the user sees: app id, version, publisher (from the signing key
or "Unsigned"), requested capabilities in plain localised language, declared
memory, and store quota. Capabilities are granted at install, wholesale. There
are no runtime permission prompts — a capability not granted is absent, and
its host functions trap (R-02-20 lets apps degrade instead).

### R-03-15 — Optional index, never required
An update index is a JSON file with app ids, versions, URLs, and hashes,
hosted in a Git repository. The device fetches it only during a normal network
window and only if the user has enabled updates. Skip the store; this is the
solo-maintainer-scale answer, and it costs no infrastructure.

### R-03-16 — Uninstall is complete and confirmed
Uninstall removes the module, assets, and private store. Documents on SD are
never touched — they belong to the user, not the app. The store deletion is
surfaced explicitly at confirmation.

---

## 6. Failure surfaces

### R-03-17 — Crash notice names the app
On trap or fuel exhaustion the launcher shows: app name, "stopped responding"
or "encountered an error", and an option to view the crash log. This is what
makes R-11 (solo-maintainer support burden) survivable — the user's bug report
goes to the right place.

### R-03-18 — Repeat-offender handling
Three traps within five launches marks the app degraded: it still launches,
but with a warning and with `fuel-per-frame` clamped to the default. The mark
clears on update or on explicit user dismissal.

### R-03-19 — The launcher itself cannot take the system down
If the launcher app traps, the supervisor restarts it from cold. Two
consecutive launcher traps enter a native minimal fallback UI (`kernel/
fallback`) that can uninstall apps and reboot. This is a small amount of
native UI code that exists solely so the system is never unrecoverable from
the front panel.

---

## 7. Testing

| Test | Where | Notes |
|---|---|---|
| Lifecycle state machine | host | exhaustive; includes kill-after-suspend |
| Summon under app infinite loop | device | R-03-02 must hold; the key test of the whole design |
| Summon under app allocation storm | device | as above |
| Warm slot eviction ordering | host | |
| Arena reset high-water | host + device | true-negative: a deliberate escape must be detected |
| Resume/launch budgets | device | per R-03-08 conditions |
| Package verification | host | fuzzed; tampered signature, truncated, oversized manifest |
| Launcher trap → fallback | device | both consecutive traps |
| `[PRESENCE]` Summon responsiveness | device | felt latency, not measured latency |
