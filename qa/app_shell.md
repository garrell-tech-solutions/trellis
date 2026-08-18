# QA Procedure: Every page carries the same navigation header

Covers: `features/app_shell.feature`

## Interface used

The three pages at `GET /`, `GET /life-areas` and `GET /stats`, `curl`
against whatever URLs the header's own links carry, and a real browser for
the walkthrough. No project library, module, or test helper is used, and no
database inspection — this slice writes nothing.

**Read every URL out of the header's own markup.** The point of the slice is
that the user never types one; a procedure that types one has stopped testing
the thing.

**What "from one definition" can and cannot be checked from outside.** QA can
verify that the three pages carry a byte-identical header. It cannot verify
that there is only one copy of it in the source — three copied templates
would pass every procedure below on the day they were written and drift
apart later. That structural guarantee belongs to the acceptance suite and
the architect. **What QA owns here is the observable half**, and it is worth
owning: if the three headers already differ, the shell failed before anyone
looks at the source.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Open `http://localhost:8080`. Confirm there is a header with three links:
   **Inbox**, **Life areas**, **Stats**.
3. Confirm the header shows you are on **Inbox** — and that the other two are
   not also marked.
4. Click **Life areas**. Confirm you land on the life-areas page, its content
   is what it always was, and the header now shows **Life areas** as current.
5. Click **Stats**, then **Inbox**. Confirm each lands where it says and each
   marks itself.
6. **At no point type a URL.** This step is the acceptance criterion; the
   others are how you get here.
7. On the inbox, confirm each navigation was a **full page load** (the tab
   spinner turns, the browser's back button walks the three pages).
8. Confirm the page still does everything it did: quick-add a capture, triage
   it as pool, dismiss another. Confirm the lists still swap in place without
   a reload, and that the header does **not** flicker, duplicate, or vanish
   when they do.

### Expected Observable Outcomes
- All eight steps hold literally, per `D-visible-slices` — steps 1–6 are the
  demo the handoff brief specifies as the acceptance criterion.
- Step 7 is open question 4 made visible: the links are plain `href`s, not
  `hx-boost`ed. If navigation happens without a full load, the decision was
  reversed silently and the interaction with the 422 override in step 8's
  territory is live.
- Step 8 is the whole risk of the slice in one step. A shared frame that
  leaks into a swapped fragment shows up here and nowhere else.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.

## Procedure — the same header on all three pages

1. `GET /`, `GET /life-areas` and `GET /stats`.
2. Extract the header region from each.
3. Compare the three, ignoring only the current-page marking.

### Expected Observable Outcomes
- All three carry a header offering exactly **Inbox**, **Life areas** and
  **Stats** — the same three, in the same order, with the same labels and the
  same targets.
- Apart from which entry is marked current, the three are **identical**. Any
  difference at all is the failure this procedure exists to catch: a header
  that was copied rather than shared has already started to drift.
- **Exactly three links.** Not a guardrails link — `#59` adds that page and
  its link together, and a link to a page that does not exist is worse than
  no link.

## Procedure — the current page is marked, and only it

1. `GET /` and read the header.
2. Repeat for `GET /life-areas` and `GET /stats`.

### Expected Observable Outcomes
- Each page marks **its own** entry as current: the inbox marks Inbox, the
  life-areas page marks Life areas, the stats page marks Stats.
- Exactly **one** entry is marked on each page. Two marked entries and zero
  marked entries are both failures, and both are more likely than they sound:
  the indicator is computed per page, so the natural bug is a page that
  reports the wrong identity or none.
- The marking is carried in the markup, not by styling alone. A marker that
  exists only as a colour cannot be verified without eyes and does not survive
  a reader who cannot see it.

## Procedure — every link goes where it says

1. `GET /` and read the three links' targets out of the header.
2. Request each target.
3. On each response, read the header again.

### Expected Observable Outcomes
- The link labelled **Inbox** reaches a page marking Inbox current; the same
  for Life areas and Stats.
- This is the pairing that catches a crossed link. Each page marking itself
  correctly (the procedure above) and each link reaching a page that marks the
  label you clicked are two different facts, and a nav wired one entry off
  passes the first and fails this one.
- Each response is a full page — a header and the page's own content — not a
  fragment.

## Procedure — the 422 swap handling is on every page now, and unchanged where it was

1. `GET /` and confirm the page declares htmx's 422 swap handling.
2. Repeat for `GET /life-areas` and `GET /stats`.
3. On the inbox, submit a triage with no life area chosen. Confirm the
   rejection still comes back `422` and its body still swaps onto the row.
4. On the life-areas page, submit a duplicate life area name. Confirm the same.

### Expected Observable Outcomes
- All three pages declare it, including **stats**, which carries no form and
  previously loaded no htmx at all.
- Steps 3 and 4 behave exactly as `qa/life_area_triage.md` and
  `qa/life_areas.md` already specify. **Hoisting the override changed where it
  is written, not what it does.**
- **Stats gaining it has no behavioural consequence today, and that is the
  point of checking it.** `T-forms-swap-one-fragment`'s cost paragraph warns
  that the override makes 422 swappable for every htmx request on the page,
  present and future. Moving it to the shared shell extends that obligation to
  every page Trellis will ever have: **422 means exactly "validation
  rejection, body is the re-rendered fragment", product-wide.** An endpoint
  that cannot honour that must not return 422. Asserting it here is what makes
  the obligation visible to whoever adds page five.

## Procedure — the links are ordinary links

1. `GET /` and read the header's markup.

### Expected Observable Outcomes
- Each entry is a plain anchor with a real target. **No `hx-boost`, no
  `hx-get`, no attribute that turns navigation into an htmx request.**
- This is open question 4, and it is not a style preference. With the 422
  override now global, a boosted navigation to an endpoint answering 422 would
  have that body swapped into the DOM. Plain links keep navigation and
  fragment-swapping as two separate mechanisms, which is what makes the
  override's blast radius knowable.

## Procedure — a fragment swap leaves the header alone

1. Submit a capture with raw text `buy milk`.
2. Triage it as pool through the page's own control.
3. Read the **raw body of that response**.
4. `GET /` afterwards and read the whole page.

### Expected Observable Outcomes
- The triage response is the `#lists` fragment and **carries no header**. A
  fragment that carries the frame produces a nested header the moment it is
  swapped in — visible instantly in a browser, invisible to every test that
  only checks the lists.
- The page afterwards carries **exactly one** header.
- Repeat with a dismissal and with a quick-add; all three swap the same
  fragment family and all three could differ.

## Procedure — the header renders no user data

1. Submit a capture with raw text `<script>alert('boom')</script>`.
2. Add a life area with the same name.
3. `GET /`, `GET /life-areas` and `GET /stats`, and read the **header region**
   of each raw response.

### Expected Observable Outcomes
- No header contains an unescaped `<script>` tag, and none contains the word
  `boom` — **the header renders no user-supplied text at all.**
- This is a negative assertion on purpose. The header has no user data today,
  so the check is that it stays that way; the day something user-supplied
  reaches it, this procedure fails and the escaping question gets asked
  deliberately rather than discovered.

## Procedure — nothing else changed

1. Run the existing QA suites for `capture_endpoint`, `inbox_view`,
   `triage_from_page`, `committed_triage_validation`,
   `quota_triage_validation`, `unknown_kind_rejection`, `stats_ratio`,
   `life_areas`, `life_area_triage`, `dismiss_capture`, `task_kinds`,
   `committed_field_domains`, `migrations`, `release_binary` and
   `scheduler_core_purity`.

### Expected Observable Outcomes
- All fifteen pass unchanged. This slice adds a frame; it changes no page's
  content and no endpoint's behaviour. **If one needed editing, the frame
  leaked into the content.**
- `capture-endpoint-persists-quickly-01` asserts a 50 ms wall-clock budget and
  is load-sensitive — that is `#66`, not a regression. Confirm by re-running
  it on a quiet machine and say so rather than chasing it.

## Independent of Implementation

This procedure depends only on what the three pages render and where their
links lead. It does not depend on whether the shell is template inheritance
or an include, how a page declares which entry is current, which module the
header's markup lives in, or what the nav is styled with — there being, per
the brief, deliberately no styling to depend on.
