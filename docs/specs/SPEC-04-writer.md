# SPEC-04 — Writer

**Status:** Draft v0.1.0
**Depends on:** SPEC-02, SPEC-03
**App id:** `codes.appthere.writer`

---

## 1. Scope

Editing of Markdown and Fountain, with the production tooling that makes a
writer deck worth more than a text box. Mermaid and Marp are in scope but
**scoped down deliberately** (§7, §8) — they are the two items most likely to
consume a schedule, and ADR-S017/S018 exist to keep them from doing so.

The writer is also the primary consumer of the draft ABI. Its job during
Phase 5 is to find out what `SPEC-02` got wrong while the interface can still
change (R-09).

---

## 2. Division of labour

| Concern | Where | Why |
|---|---|---|
| Piece table, undo log | host | Largest allocation; must be budget-controlled (ADR-S012) |
| Line layout, shaping, glyph cache | host | Expensive; shared across apps |
| Word/char/paragraph counts | host | Needs the whole document |
| Markdown/Fountain classification | **app** | Third parties can add modes (ADR-S013) |
| Outline extraction | **app** | Format-specific |
| Fountain production reports | **app** | Pure computation over the app's own parse |
| Export/PDF | host service | One font subsetter, one PDF writer |

The app's per-frame work is: classify the visible span, emit style runs,
commit a UI tree of roughly 30–80 nodes. That is what the fuel budget is sized
for, and it is why an interpreter is viable.

---

## 3. Editing surface

### R-04-01 — Typing does not require the app to run
A keystroke in a `text-view` is handled by the host TextView: insert, re-layout
the affected lines, redraw. The app is notified after the fact via
`event::text-changed` with the changed span. If the app is slow, typing is
still smooth. This is the single most important performance property of the
device and it falls directly out of ADR-S012.

### R-04-02 — Style runs on visible-range change
The app receives `event::visible-range-changed` (also fired after
`text-changed` when classification could be affected) and responds with
`set-style-runs` over that span plus a small margin. Target: under 2 ms of
app time for a 60-line span.

Both parsers must therefore be **incremental from a line boundary** and must
not require whole-document context. Markdown mostly permits this; the
exceptions are fenced code blocks, link reference definitions, and setext
headings. The app maintains a per-line "block context" cache (in a fenced
block, in a list at depth N) so classification can start at any cached line.

### R-04-03 — No live preview mode in 1.0
Syntax-highlighted source is the editing view. A side-by-side rendered preview
doubles layout cost and, on a 1024x600 panel, halves the writing column. A
full-screen render preview toggle is in scope; a split live preview is not.

### R-04-04 — Focus modes
Typewriter scrolling (caret held at a fixed vertical position) and paragraph
focus (dimming everything but the current paragraph) are host TextView
features, requested by the app through view flags. They are the features most
directly aligned with the device's purpose and they are cheap.

---

## 4. Markdown

CommonMark-subset classification, plus the extensions the rest of the system
needs: front matter (TOML/YAML), tables, footnotes, and fenced blocks tagged
`mermaid`.

### R-04-05 — Outline from headings
ATX and setext headings become `outline-item`s, pushed via `set-outline`. Host
renders the navigation drawer from it; the app does not draw an outline UI.

### R-04-06 — Export targets
Markdown → PDF via the host export service. HTML export writes a file to SD.
There is no export requiring network.

---

## 5. Fountain

This is the differentiator over a plain writer deck, and most of it is nearly
free once the AST exists.

### R-04-07 — Full Fountain element classification
Scene heading, action, character, dialogue, parenthetical, transition, shot,
lyric, centered text, dual dialogue, notes, boneyard, sections, synopses,
title page. Classification drives both styling and the reports.

### R-04-08 — Screenplay layout mode
A TextView view flag selects fixed-pitch screenplay geometry (industry margin
conventions, 55-line page). Page estimation and page breaks are host-side
because they need the whole document; the app supplies the element type per
line and the host applies the geometry.

### R-04-09 — Production reports
Derived from the AST, rendered as `list` nodes:

- Scene list with page number, length, and INT/EXT + day/night breakdown.
- Character report: scene count, line count, first and last appearance.
- Location report.
- Page-count estimate with a stated convention (1 page ≈ 1 minute).

### R-04-10 — Corkboard
Scenes as index cards in a virtualised grid: heading, synopsis (from `=`
synopsis lines), page number. Reordering moves the underlying text via
`text.replace` over whole scene spans, and every reorder is a snapshot point.

Reordering is the one destructive operation in the writer. It gets an explicit
undo entry and a snapshot, and the implementation must be tested against
scenes containing boneyard sections and notes, which is where span arithmetic
goes wrong.

---

## 6. Snapshots

Not git. A host-owned chunked, content-addressed append log on SD (`doc.
snapshots` / `doc.restore`).

### R-04-11 — Snapshot triggers
On: 5 minutes of elapsed editing with changes pending, app suspend, document
close, corkboard reorder, and explicit user request. Not on every keystroke.

### R-04-12 — Paragraph-level restore
The restore UI presents a diff at paragraph granularity and permits restoring
a single paragraph, not only a whole-document rollback. This is the feature
writers actually miss, and it is why the log is chunked by paragraph rather
than storing whole-file copies.

### R-04-13 — Retention is bounded and stated
Hourly for 24 h, daily for 30 days, then monthly, capped at a per-document
size budget. When the cap forces pruning, the UI says so rather than silently
losing history.

---

## 7. Mermaid — scoped down

Per ADR-S018. Mermaid's flowchart quality comes from dagre's layered
(Sugiyama) layout. There is no path to running the real library, and
reimplementing layered layout well — ranking, cycle breaking, ordering to
minimise crossings, coordinate assignment, spline routing — is a project in
its own right, not a feature.

### R-04-14 — Phase 1 diagram types
Sequence, state, Gantt, pie, and mindmap. All have near-trivial layout:
sequence is columns plus vertical ordering, Gantt is a time axis, state and
mindmap are trees. These render to `canvas` display lists.

### R-04-15 — Flowchart and class diagrams are behind a spike
Spike M1 evaluates: a reduced layered layout in Rust, versus rendering
flowcharts only on export via a desktop-side tool, versus not supporting them.
Do not start M1 inside a milestone that promises anything else (R-10).

### R-04-16 — Compatibility is stated honestly
The UI and documentation say "Mermaid sequence, state, Gantt, pie, and mindmap
diagrams." It does not say "Mermaid support." Unsupported diagram types render
a clear placeholder naming the type, not a broken diagram.

---

## 8. Marp — directive-compatible, not CSS-compatible

Per ADR-S017. Marp themes are CSS. There is no CSS engine on this device and
there will not be one.

### R-04-17 — Directives, not themes
Supported: slide separators, `paginate`, `header`, `footer`, `class`,
`backgroundImage` (solid/gradient only in 1.0), per-slide directives, and
speaker notes. A fixed set of native layout templates (title, title+content,
two-column, quote, image+caption, section break) is selected by the `class`
directive.

### R-04-18 — Custom CSS is reported, not ignored
A deck containing a custom theme or inline CSS renders with the nearest native
template and shows a clear notice that theming was not applied. Silently
dropping it is the "signal that looks healthy but isn't" failure — the deck
looks fine on-device and wrong everywhere else.

### R-04-19 — Export via the host PDF service
Slides render to the host PDF writer with one or two subset fonts. A general
PDF stack is out of scope; a minimal writer that emits text, vectors, and
JPEG-passthrough images is sufficient and is the same code the Markdown export
path uses.

---

## 9. Writing support features

| Feature | Implementation | Budget |
|---|---|---|
| Spell check | Host mmap'd DAWG in the `dict` partition; app requests check for the visible span, receives misspelled offsets | ~0 heap; DAWG resident in flash |
| Thesaurus / dictionary | Host lookup against the same partition | ~0 heap |
| Word-count goals, sprints, streaks | App-side, persisted to `store` | trivial |
| Bibliography (CSL-JSON / BibTeX) | App-side parse, autocomplete over a document-adjacent `.bib`/`.json` | ≤ 200 KiB working |
| TTS proofreading | `audio.speak` over the selection or current paragraph | see SPEC-00 |
| Todo (todo.txt) | Separate small app, project-scoped, **no dates** | — |

### R-04-20 — Spell check uses a DAWG, not Hunspell
Hunspell's hash-table structure and affix-expansion behaviour are the wrong
shape for a memory-constrained mmap-style lookup. An affix-compressed DAWG
gives near-zero heap and a flash-resident structure. Budget ~600–900 KiB of
flash for English; the 8 MiB `dict` partition allows several languages.

### R-04-21 — Todo has no dates and no reminders
Deliberate. A due date implies a notification and a notification is the thing
the device does not have (R-02-15). Project-scoped ordering only.

---

## 10. Testing

| Test | Where | Notes |
|---|---|---|
| Markdown classifier | host | CommonMark spec corpus, subset-scoped; fuzzed |
| Fountain classifier | host | golden corpus of real screenplays; fuzzed |
| Incremental classification | host | **must produce identical output to a from-scratch pass** — this is the true-negative for the block-context cache |
| Span arithmetic on reorder | host | scenes with boneyard, notes, dual dialogue |
| Snapshot restore | host + device | including restore after prune |
| Style-run latency | device | ≤ 2 ms for 60 lines, p99 |
| Typing latency | device | ≤ 50 ms p99 keypress → glyph, 10k-paragraph document |
| Memory high-water | device | per CLAUDE.md §5 measurement rule |
