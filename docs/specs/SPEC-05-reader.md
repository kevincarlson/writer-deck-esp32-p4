# SPEC-05 — Reader (RSS + EPUB) and the Network Services

**Status:** Draft v0.1.0
**Depends on:** SPEC-02, SPEC-03
**App id:** `codes.appthere.reader`

---

## 1. Scope

Two host services and one app:

- **net service** (kernel) — C6 link, power windows, HTTP, TLS.
- **feed service** (kernel) — subscription store, scheduled fetch, RSS/Atom
  parse, bounded snapshot store.
- **reader app** (WASM) — presentation of the feed snapshot, and EPUB reading.

The services are kernel-side because parsing hostile XML must not sit in an
app's fuel budget, because there must be exactly one TLS implementation to
audit, and because the *shape* of the feed store is what makes feeds
non-infinite (ADR-S015).

---

## 2. Net service

### R-05-01 — The radio is off by default
The C6 rail is powered down between windows (SPEC-00 §7). No host function
powers it on directly. Apps request work; the service decides when to open a
window and coalesces pending requests into it.

### R-05-02 — Windows are scheduled or user-initiated
Triggers: an explicit user sync/fetch action, or the scheduled feed poll.
Default poll is twice daily. The **minimum configurable interval is 1 hour**
and that floor is a structural decision, not a default — a five-minute poll is
a notification system with extra steps.

### R-05-03 — Windows are bounded
A window has a maximum duration and a maximum byte budget. Exhaustion closes
the window with partial results; unfinished work is retried in the next
window, not extended into the current one.

### R-05-04 — One TLS path
`embedded-tls` or a `no_std` `rustls` configuration, decided by Spike N1 on
measured footprint. A pinned root store shipped with firmware and updated with
firmware. Certificate validation is never skippable, in any build.

### R-05-05 — The C6 link is treated as untrusted
The host validates all frames from the C6 and never treats it as a source of
authority. If ESP-HOSTED from bare-metal Rust proves impractical (R-06), the
fallback is a minimal custom protocol we define — in which case that protocol
must be specified before it is implemented, not discovered by writing it.

---

## 3. Feed service

### R-05-06 — Bounded snapshot, not a stream
Retention: the most recent N entries per feed (default 50, max 200) and M days
(default 30). Beyond that, entries are pruned. There is no archive and no
"load older" path. **This is the property that makes RSS admissible on this
device** — a feed reader with unbounded history is an infinite scroll.

### R-05-07 — Parsing is defensive and host-side
RSS 2.0, RSS 1.0/RDF, Atom, and JSON Feed. The parser is fuzzed, has hard
caps on element depth, entity expansion, and total document size, and rejects
rather than repairs malformed input. Feed XML is hostile input by default.

### R-05-08 — Content is normalised to blocks at fetch time
Entry bodies are converted from HTML to the block list (R-02-11) **during the
fetch window**, and the block form is what is stored. The reader app never
sees HTML for feed content. Consequences: parsing cost is paid once, the app's
fuel budget is untouched, and the stored form is small.

### R-05-09 — No image fetching by default
Feed entry images are not fetched. An opt-in setting permits fetching images
within the same window, decoded via the hardware JPEG codec, capped per entry
and per window. Default off, because it is the largest byte-budget consumer.

### R-05-10 — Read state is local and quiet
Read/unread is tracked for ordering only. There is no unread count exposed to
the launcher (R-03-09) or to any app.

---

## 4. Reader app — feed reading

### R-05-11 — Two views only
**Feed list** (subscriptions with last-updated time) and **entry list**
(virtualised, newest first, capped by R-02-14's 200-entry limit). Selecting an
entry opens the reading view.

### R-05-12 — Reading view is a block-list
The entry's stored block list renders through the host `block-list` node with
the same layout engine as the writer's TextView in read-only mode. Pagination,
not scrolling, is the default for body text — it is better on a fixed-size
panel and it removes the momentum-scroll feel that makes browsing feel like
browsing.

### R-05-13 — Save to document
An entry can be written to SD as Markdown via `doc.request-create`. This is
the bridge into the writer, and it is the only place feed content leaves the
feed store.

### R-05-14 — Refresh is a request, not an action
The refresh control calls `feed.request-refresh` and immediately shows "queued
for next window" with the scheduled time. It never blocks and never implies
that content is arriving now. The UI must not present a spinner that suggests
a live fetch, because that trains exactly the behaviour the device avoids.

---

## 5. Reader app — EPUB

### R-05-15 — Read-only, no round-trip obligation
The reader never writes an EPUB. Byte-lossless-save concerns do not apply
here. Annotations and bookmarks live in the app's private `store`, keyed by a
stable identifier derived from the book (§5.4), never written into the file.

### R-05-16 — Container access is mediated
ZIP entry enumeration and reads go through `doc.container.*` (R-02-13). The
app does not implement ZIP. Zip-bomb, path-traversal, and duplicate-entry
defences live in one audited host implementation shared with CBZ.

### R-05-17 — XHTML → blocks, app-side
Per ADR-S014 the app parses spine documents into the block list. Supported
inline: emphasis, strong, code, links, superscript (footnotes). Supported
blocks: paragraph, heading 1–6, list, blockquote, code, rule, image,
horizontal rule. Everything else degrades to paragraph.

CSS is not interpreted. Class-based styling is ignored; semantic HTML drives
appearance. Say this in the documentation rather than letting users discover
it as a rendering bug.

### R-05-18 — Chapter-at-a-time, paginated
One spine item is parsed and laid out at a time, paginated to the panel. The
layout index is retained per chapter in the warm slot so returning to a
chapter does not re-parse. Reading position is a `(spine-index, block-index,
offset)` triple, which survives re-layout after a font-size change — a page
number does not.

### R-05-19 — Book identity is content-derived
The bookmark key is a hash of the OPF identifier plus title and creator. A
file renamed or re-copied to a different SD card keeps its position. Record
the fallback ordering explicitly for books with no usable identifier.

### R-05-20 — Vertical writing modes are refused, not attempted
`writing-mode: vertical-rl` and CJK vertical layout are not supported in 1.0.
A book declaring them shows an explicit notice. Attempting partial support
here produces silent content loss, which is the worst available outcome — it
looks like a rendered page and is not one.

### R-05-21 — Image handling
Images are decoded via the hardware JPEG codec where possible, downscaled to
panel width by the PPA, and cached per chapter with a hard cap. PNG decode is
software and is size-capped more aggressively. SVG is not supported in 1.0.

---

## 6. What is deliberately absent

Stated so it is not re-litigated:

- No browser, no in-app link following to arbitrary URLs. A link in an entry
  or book offers "save the page as a document for later" via
  `net.fetch-document`, which lands as a file, not a page view.
- No sharing, no comments, no accounts, no sync of read state to a service.
- No recommendations, no "related entries", no algorithmic ordering.
  Chronological only.
- No podcast or audio-feed enclosure playback (R-02-19 forbids it anyway).

---

## 7. Testing

| Test | Where | Notes |
|---|---|---|
| RSS/Atom/JSON Feed parsers | host | golden corpus of real feeds; fuzzed; hostile cases (billion laughs, deep nesting, bad encodings) |
| HTML → block conversion | host | golden corpus; must be deterministic |
| ZIP container defences | host | zip bomb, traversal, duplicate entries, truncated central directory |
| XHTML → block conversion | host | EPUB 2 and 3 corpora |
| Reading position stability | host | position survives font-size change and re-layout |
| Retention pruning | host | including the true-negative: a case that should prune and does |
| Window budget enforcement | device | oversized feed must not extend the window |
| Radio off between windows | device | `[PRESENCE]` — measured with a meter, not a log line |
| Cold open of a 2 MB EPUB | device | ≤ 600 ms to first page |
