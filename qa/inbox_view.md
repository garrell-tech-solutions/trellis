# QA Procedure: The inbox renders the untriaged capture queue

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
  intervention, the slice is not done — this is what D14 means.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the task list is empty.

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

## Procedure — triaged captures are excluded

1. Submit a capture with raw text `call the dentist`.
2. Triage it as a pool task.
3. `GET /` and read the rendered page.

### Expected Observable Outcomes
- The capture does not appear in the inbox. It has already been triaged
  (`captures.triaged_at` is set); confirm this directly against the database
  as well as by reading the page.

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

## Independent of Implementation

This procedure depends only on what `GET /` renders and how the quick-add
submission responds. It does not depend on which templating engine is used,
whether HTMX or a different mechanism drives the no-reload behaviour, or
which route the quick-add box submits to — the brief is explicit that the
acceptance criterion is the resulting row, not the route.
