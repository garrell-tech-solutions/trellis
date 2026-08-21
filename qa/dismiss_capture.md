# QA Procedure: A capture can be dismissed, and its row is kept

Covers: `features/dismiss_capture.feature`

## Interface used

The page at `GET /`, `curl` against whatever endpoint the page's own dismiss
control submits to, and read-only `sqlite3` inspection of persisted state. No
project library, module, or test helper is used.

Throughout, "dismiss X" means submitting the page's own dismiss control for
that capture's row. **Read the endpoint out of the page's markup; do not
assume a route.** The dismiss affordance is page-only — there is no JSON
dismissal API to fall back on, so if the control cannot be found in the
markup, that is a failure of this procedure, not a reason to compose a
request by hand.

**What this procedure can and cannot honestly claim**, as established for
`inbox_view` (#30) and `triage_from_page` (#33): `curl` verifies every
server-side fact, but not that a real browser's DOM updates without a visible
reload. The by-hand walkthrough below covers that once; every procedure after
it is `curl`-only.

**The row-count guarantee is deliberately invisible on every page.**
`D-kill-means-archive` keeps the row and builds no browsable archive, so a
dismissed capture appears on no surface at all. Read-only `sqlite3` is
therefore the only way to observe it, and it is the durable state itself, not
a project API.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Open `http://localhost:8080` and capture `asdfgh`.
3. Confirm its inbox row offers a dismiss action alongside Pool, Committed and
   Quota.
4. Click it. Confirm `asdfgh` leaves the inbox without the page visibly
   reloading, **no confirmation dialog appears** (`D-three-strike`: friction is
   a feature in exactly one place in this system, and this is not it), and no
   task appears in the task list.
5. Confirm the inbox now shows its **ordinary** empty state, not a
   dismissal-specific variant. As of PR #87 that reads `Nothing to triage.
   Start with the thing you keep half-remembering.` — **read the current
   wording out of the page rather than trusting this line**; what is under
   test is that dismissing everything produces the same message as never
   having captured anything.
6. Capture `buy milk`, pick a life area, click **Pool**.
7. Restart the server and reload. Confirm the inbox is empty, `buy milk` is
   still in the task list, and `asdfgh` has not come back.
8. Confirm there is **no link, tab, or list anywhere on the page** that shows
   dismissed captures, and no way to un-dismiss one.

### Expected Observable Outcomes
- All eight steps hold literally, per `D-visible-slices` — steps 1–7 are the
  demo the handoff brief specifies as the acceptance criterion.
- Step 8 is the negative half of `D-kill-means-archive`, and it is a real
  check rather than a formality: *"the moment an archive is browsable it
  becomes a place to hide from decisions."* A dismissed-captures view is the
  most natural thing in the world to add while building this, which is
  exactly why QA looks for it.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the task list is empty and the inbox is empty.

## Procedure — every untriaged capture offers a dismiss action

1. Submit captures with raw text `asdfgh` and `buy milk`.
2. `GET /` and read both inbox rows.

### Expected Observable Outcomes
- **Both** rows carry a dismiss control, not just the first — the affordance
  belongs to the row, per `T-forms-swap-one-fragment`, so a control rendered
  once outside the loop would pass a one-capture check and fail here.
- The control submits to the server. A control that only hides the row in the
  browser would satisfy the eye and lose the capture on reload.

## Procedure — dismissing removes it from the inbox and creates nothing

1. Submit a capture with raw text `asdfgh`.
2. Dismiss it.
3. Observe the response status and headers.
4. `GET /` and read the inbox and the task list.
5. Query the tasks table and count its rows.

### Expected Observable Outcomes
- The response does not redirect the browser (no `3xx`, no `Location`), and
  carries the re-rendered `#lists` fragment — the same swap every other action
  on this page uses.
- `asdfgh` is gone from the inbox.
- The tasks table is **empty**. Dismissal is not a quiet triage; nothing is
  created.

## Procedure — the row is kept, whichever way the capture leaves

1. Submit captures with raw text `asdfgh` and `buy milk`.
2. Triage `buy milk` as pool into a valid life area.
3. Dismiss `asdfgh`.
4. `GET /` and read the inbox.
5. Query the captures table and count **all** its rows.
6. Query the tasks table and count its rows.

### Expected Observable Outcomes
- The inbox lists nothing; both captures have left it by different doors.
- The captures table still holds **two** rows. This is `#9` AC-4, and it is
  the whole point of the slice: the row feeds M8's reckoning ("47 archived
  this quarter, 31 Learning" is real signal), which a deleted row cannot.
- The tasks table holds **one** row — the triaged one only.
- Which door each capture left by is **derivable**: both carry the stamp that
  records leaving, and the triaged one — and only the triaged one — is
  referenced by a `tasks` row. Read the stamp column's name off the schema
  (`PRAGMA table_info(captures)`); do not assume it. As shipped that is one
  column, `left_inbox_at`, and the `tasks` reference is the whole of the
  discriminator; the procedure is written to hold either way.

## Procedure — a capture leaves the inbox exactly once

1. Submit a capture with raw text `asdfgh` and triage it as pool into a valid
   life area.
2. Dismiss it. Observe the status.
3. Submit a capture with raw text `buy milk` and dismiss it.
4. Triage it as pool into a valid life area. Observe the status and body.
5. Dismiss it a second time. Observe the status.
6. Query the captures and tasks tables.

### Expected Observable Outcomes
- Steps 2, 4 and 5 are all rejected with `422` — the status this product
  reserves for "validation rejection, body is the re-rendered fragment"
  (`T-forms-swap-one-fragment`). A dismissal arriving from a stale page gets
  back the current lists, so the stale page corrects itself on the swap.
- Step 4's JSON rejection names the reason: `not_in_inbox`, echoing the
  capture id — the same "report what was submitted" shape
  `unknown_life_area` established for a value that does not resolve.
- The captures table holds exactly two rows and the tasks table exactly one.
  **No rejected second exit created, deleted, or re-stamped anything.**
- This is what makes the impossible state impossible in practice rather than
  on paper. `T-archived-at-only` warns that two fields for one state give
  every path two chances to set one and forget the other — and the shipped
  schema answers it by having only one field: a capture cannot be stamped
  both triaged and dismissed when there is a single stamp for leaving. What
  this procedure exercises is the remaining question that no schema can
  answer — that the **first** exit wins and every later one is refused,
  rather than the second quietly overwriting the first.

## Procedure — a dismissed capture stays gone across a restart

1. Submit captures with raw text `asdfgh` and `buy milk`.
2. Dismiss `asdfgh`; triage `buy milk` as pool into a valid life area.
3. `GET /` and confirm the inbox is empty.
4. Stop the server. Restart it against the **same** database.
5. `GET /` and read the inbox and the task list.
6. Query the captures table and count its rows.

### Expected Observable Outcomes
- The inbox is still empty. `asdfgh` has **not** come back — dismissal is a
  persisted stamp, not something the serving process remembered.
- `buy milk` is still in the task list.
- The captures table still holds two rows after the restart.

## Procedure — the inbox says the same thing when everything is dismissed

1. Submit a capture with raw text `asdfgh`.
2. Dismiss it.
3. `GET /` and read the inbox region.

### Expected Observable Outcomes
- The inbox shows **the existing empty-state message, unchanged** — the same
  one an inbox that was never filled shows. As of PR #87 that is `Nothing to
  triage. Start with the thing you keep half-remembering.`, and it has
  changed once already, which is why what this asserts is **sameness with the
  ordinary empty state** rather than a literal string. It is still true, and a
  dismissal-specific variant ("nothing left, you dismissed it all") would be
  the first step toward the browsable archive `D-kill-means-archive` refuses:
  it invites the user to reflect on what they threw away.

## Procedure — hostile capture text stays escaped in what a dismissal returns

1. Submit a capture with raw text `asdfgh`, then one with raw text
   `<script>alert('boom')</script>`.
2. Dismiss `asdfgh`.
3. Read the **raw HTML source of the dismissal response**, not a re-fetched
   page.

### Expected Observable Outcomes
- The response does not contain an unescaped `<script>` tag.
- The word `boom` is still present — escaped, not stripped.
- The dismiss control adds no new render surface of its own, but it adds a new
  **response path** that returns the shared fragment, and htmx swaps that
  response straight into the DOM. `#30`, `#33` and `#47` each re-asserted
  escaping on their own new surface; this asserts it on the new path.

## Procedure — nothing else changed

1. Run the existing QA suites for `capture_endpoint`, `inbox_view`,
   `triage_from_page`, `committed_triage_validation`,
   `quota_triage_validation`, `unknown_kind_rejection`, `stats_ratio`,
   `life_areas` and `life_area_triage`.

### Expected Observable Outcomes
- All pass unchanged. If one needed editing, something was rebuilt that should
  have been reused — with the single expected exception noted in
  `qa/life_area_triage.md`, where the picker's new unselected placeholder is
  fixture drift rather than regression.
- `/stats` in particular reports the same ratio it would have before: it counts
  tasks, and a dismissal creates none. A dismissed capture must not enter the
  denominator by any route.

## Independent of Implementation

This procedure depends only on what `GET /` renders, what the dismiss control
does when submitted, and the durable state left behind. It does not depend on
which route the control submits to, what the dismissal column is called, how
the exclusivity is enforced, or whether the check lives in `scheduler-core` or
at the store.
