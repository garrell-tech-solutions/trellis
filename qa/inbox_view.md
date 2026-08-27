# QA Procedure: The capture page is a box and what is recent

Covers: `features/inbox_view.feature`

## Interface used

The page at `GET /`, plus `curl` against the quick-add submission endpoint and
read-only `sqlite3` inspection of persisted state. No project library, module,
or test helper is used.

**What this procedure can and cannot honestly claim.** This is the first UI
slice; there is no browser-automation tooling anywhere in this project's stack
(`stack.prompt` lists none, and introducing one is a stack decision, not a QA
procedure's to make unilaterally). `curl` can verify every server-side fact
that *enables* "no full page reload" — that the quick-add submission responds
with the new capture's markup rather than a redirect. It cannot verify that a
real browser actually swaps that markup into the page without a visible
reload. The first two setup steps below cover the latter by hand, once, as a
literal walkthrough of the brief's own demo; the parametrized procedures after
that are `curl`-only and do not repeat the by-hand check.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Open `http://localhost:8080` (or the configured port).
3. Confirm the empty state renders a message, not a blank page.
4. Type `buy milk` into the quick-add box and submit it.
5. Confirm it appears in the list **without the page visibly reloading**
   (watch for a browser refresh flicker or the address bar re-navigating).
6. Restart the server and reload the page.
7. Confirm `buy milk` is still listed.

### Expected Observable Outcomes
- All four demo steps from the handoff brief hold literally. If any one of
  them requires an extra step, a different URL, or manual JavaScript
  intervention, the slice is not done — this is what D-visible-slices means.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the Pool, Committed and Quota screens all list nothing. **There is no `Tasks` list on the capture page any more (#140)** — the destination screens are where a write is now visible.

## Procedure — list order

1. Submit a capture with raw text `call the dentist`.
2. Submit a capture with raw text `buy milk`.
3. `GET /` and read the rendered page.

### Expected Observable Outcomes
- Both captures appear in the list.
- `buy milk` (submitted second) appears before `call the dentist` (submitted
  first) — newest first.

## Procedure — quick-add without a full reload

1. Identify the quick-add submission's endpoint and method (read the page's
   HTML source; do not assume it is `POST /captures`).
2. Submit a capture with raw text `buy milk` through that endpoint directly
   with `curl`, sending the same headers the page's own markup indicates
   (e.g. `HX-Request: true` if the box is an HTMX form).
3. Observe the response status and body.
4. `GET /` again and read the rendered page.

### Expected Observable Outcomes
- The submission response is **not** a redirect (not a `3xx` status).
- The submission response body itself contains `buy milk` — the server sends
  back something a client can insert into the page immediately, rather than
  requiring a second round trip to see the new item. This is the server-side
  half of "without a full page reload"; see the by-hand walkthrough above for
  the browser-side half.
- A fresh `GET /` also lists `buy milk` — the quick-add path and a plain
  reload agree.

## Procedure — empty state

1. `GET /` against a database with no untriaged captures.

### Expected Observable Outcomes
- No capture rows are rendered.
- The page shows a message rather than a bare empty list or blank body.

## Procedure — a triaged row stays, and reads what it became

**This inverts what this document used to say.** It required that a triaged
capture *disappear* from the inbox — true only while a `Tasks` list existed to
show it somewhere else. #140 deletes that list, so a vanishing row would leave
the page with no feedback at all.

1. Capture `buy screws`. Triage it as **pool**, tag `@homedepot`.
2. `GET /` and read `Recent`.
3. Repeat with a pool triage carrying **no tag**, and with a **committed** one.

### Expected Observable Outcomes
- The row **is still there**, restyled, reading **`Pool · @homedepot`** — and
  **`Pool · no context`** when untagged, **`Committed · @desk`** for committed.
- **It offers no kind buttons any more.** That, not its disappearance, is how
  you can tell it was filed.
- **It appears exactly once on the page.** Count occurrences of the text in the
  raw HTML. **A second occurrence means the `Tasks` list survived** — and today
  `[quota]` really is duplicated that way, so this is a live check, not a
  hypothetical.
- **Reload. It is still there, still restyled.** Nothing here is ephemeral:
  whether a capture was triaged is already durable. **If you find a new column
  holding a "recently triaged" flag or timestamp, say that before anything else
  in the report** (`T-ephemeral-view-state-rides-the-request`; append-only
  migrations mean it can never be taken back).

## Procedure — only the three most recently triaged stay

**Three is the owner's number**, settled 2026-08-27: *"I only want the 3 most
recently triaged tasks."*

1. Capture and triage, in order: `one`, `two`, `three`, `four`.
2. `GET /`.
3. Now capture `still waiting` and leave it untriaged. `GET /` again.

### Expected Observable Outcomes
- Step 2: `Recent` shows **`four`, `three`, `two`** — and **not `one`.**
- Step 3: **`still waiting` is listed, and nothing triaged pushed it off.**
  Untriaged work can never be crowded out by confirmations of work already
  done; that is the whole reason the cap is on the triaged half rather than on
  the list.
- **Order is strict recency**, so a row you just triaged does not jump — the
  fourth-oldest triaged one drops off instead.

## Procedure — hostile capture text is escaped

1. Submit a capture with raw text `<script>alert('boom')</script>`.
2. `GET /` and read the **raw HTML source** of the response (not a rendered
   DOM view — the point is what the server sent, before any browser parses
   it).

### Expected Observable Outcomes
- The raw response does not contain an unescaped `<script>` tag — the angle
  brackets are rendered as entities (`&lt;script&gt;` or equivalent), not as a
  literal tag a browser would execute.
- The word `boom` is still present somewhere in the response — the content
  survived, escaped, rather than being silently stripped. A procedure that
  only checks for the absence of `<script>` would also pass an implementation
  that deleted the capture's text entirely, which is a different bug wearing
  the same test result.

## Procedure — prove the rewritten assertions can fail

**`T-a-check-must-be-seen-to-fail`.** This slice rewrote **30 assertions across
10 feature files**: 25 that named the `task list`, plus **5 the brief's grep
could not see** — `the inbox does not list …` / `lists no captures`, which
invert because a triaged row now stays. A rewritten assertion that has only
ever been green is not evidence. **Break each family once — one per family, not one per scenario —
and record the message.** #149 is the open example of not doing this.

| # | Break this | The named assertion that must go red |
|---|---|---|
| 1 | Let a rejected triage write its task anyway | `the committed screen lists nothing` in `committed_field_domains` — **and the capture-still-waiting line must stay green**, or the two halves are entangled |
| 2 | Let a rejected triage consume the capture but write nothing | `the capture is still waiting in the untriaged queue` — **this is the case the old wording could not see.** *"The task list is still empty"* passed here, which is the whole argument for the rewrite |
| 3 | Drop the tag when writing a pool task | `the pool screen lists "buy screws" tagged "<tag>"` in `context_tags` |
| 4 | Let dismissal create a task | `the pool screen lists nothing` in `dismiss_capture -02` |
| 5 | Restore the `Tasks` list markup | **`the capture page shows "buy screws" once`** — the count goes to two |
| 6 | Filter triaged captures back out of `Recent` | `inbox-view-triaged-row-stays-04` — the row vanishes, which is the defect this slice exists to prevent |
| 7 | Cap `Recent` at three total instead of three triaged | `inbox-view-untriaged-never-drop-06` — `still waiting` falls off, and **-05 stays green**, which is why both scenarios exist |
| 8 | Render capture text unescaped | the `<script>` half in `triage_from_page -06`; then drop the text entirely and confirm **the `boom` half** goes red. **Both halves are load-bearing and must be seen to fail separately** |

### Expected Observable Outcomes
- **Eight breakages, eight distinct messages, then restore and a clean pass
  with a clean `git status`. Say which you ran.**
- **Breakage 2 is the one to do if you do only one.** It is the entire
  justification for touching 25 assertions: the old global *"the task list is
  still empty"* passed whenever nothing was written **anywhere**, including
  when the rejection silently ate the capture. The new line names the specific
  thing.
- **Breakage 7 is the one with a trap.** If both -05 and -06 go red, your
  breakage was too broad to prove which rule holds.

## Independent of Implementation

This procedure depends only on what `GET /` renders and how the quick-add
submission responds. It does not depend on which templating engine is used,
whether HTMX or a different mechanism drives the no-reload behaviour, or
which route the quick-add box submits to — the brief is explicit that the
acceptance criterion is the resulting row, not the route.
