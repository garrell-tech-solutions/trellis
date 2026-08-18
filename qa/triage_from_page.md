# QA Procedure: Triage happens on the page, using the same validation as the API

Covers: `features/triage_from_page.feature`

## Interface used

The page at `GET /`, `curl` against whatever endpoint the page's own triage
controls submit to, and read-only `sqlite3` inspection of persisted state. No
project library, module, or test helper is used.

**What this procedure can and cannot honestly claim**, as established for
`inbox_view` (#30): `curl` verifies every server-side fact that enables the
page's behaviour, but not that a real browser's DOM actually updates without
a visible reload. The by-hand walkthrough below covers that once, literally;
the parametrized procedures after it are `curl`-only.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Open `http://localhost:8080` and capture `buy milk`.
3. Confirm the inbox row for `buy milk` offers Pool, Committed and Quota.
4. Click **Pool**. Confirm it leaves the inbox and appears in a task list on
   the same page, without the page visibly reloading.
5. Capture `call the dentist`. Click **Committed**. Confirm the form offers
   deadline type as a fixed choice between `hard` and `soft` (not a free-text
   box), and priority as a fixed choice among `P1`–`P4`.
6. Submit the committed form with one field left blank. Confirm the rejection
   names the blank field, nothing is created, and the capture is still in the
   inbox.
7. Capture something and click **Quota**. Ask for `0` sessions. Confirm it is
   refused.
8. Restart the server and reload the page. Confirm the tasks created in steps
   4 and 6 (once corrected) are still listed, and the inbox is still short.

### Expected Observable Outcomes
- All eight steps hold literally, per `D-visible-slices` — this is the demo
  the handoff brief specifies as the acceptance criterion.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the task list is empty.

## Procedure — the inbox offers all three kinds

1. Submit a capture with raw text `buy milk`.
2. `GET /` and read the rendered page.

### Expected Observable Outcomes
- The page offers a way to triage `buy milk` as pool, as committed, and as
  quota — three distinguishable controls, not one generic "triage" action.

## Procedure — pool triage through the page

1. Submit a capture with raw text `buy milk`.
2. Identify the page's pool-triage control (read the page's HTML source; do
   not assume it posts to `/captures/{id}/triage` — the acceptance criterion
   is the resulting row, not the route) and submit it with `curl`.
3. Observe the response status.
4. `GET /` and read the rendered page.

### Expected Observable Outcomes
- The triage response is not a redirect (not a `3xx` status) — the
  server-side half of "without a full page reload"; see the by-hand
  walkthrough for the browser-side half.
- `buy milk` no longer appears in the inbox.
- `buy milk` appears in the task list.

## Procedure — committed rejection through the page

1. Submit a capture with raw text `call the dentist`.
2. Submit the page's committed-triage control with `deadline` omitted.
3. Observe the response status and body.
4. Query the tasks table and count its rows.
5. Re-read the untriaged queue.

### Expected Observable Outcomes
- Triage is rejected, and the response body names `deadline` — the same
  contract `committed_triage_validation` already establishes over the API.
  This procedure exists to prove the page uses that same contract, not to
  re-verify every field; the full field-by-field matrix is
  `committed_triage_validation`'s job.
- The tasks table is still empty and the capture is still present, untriaged.

## Procedure — the committed form's closed choices

1. Submit a capture with raw text `call the dentist`.
2. `GET /` and read the committed form's markup for that capture.

### Expected Observable Outcomes
- The deadline type control offers exactly `hard` and `soft` — no other
  value, and no free-text input that could carry an arbitrary string.
- The priority control offers exactly `P1`, `P2`, `P3` and `P4`.
- The API already rejects a value outside these domains
  (`committed_field_domains`); this procedure confirms the page's own
  controls do not let a user construct a request the API would reject.

## Procedure — quota rejection through the page

1. Submit a capture with raw text `go to the gym`.
2. Submit the page's quota-triage control with `target_count` omitted.
3. Observe the response status and body.
4. Query the tasks table and count its rows.

### Expected Observable Outcomes
- Triage is rejected, and the response body names `target_count` — proving
  the page uses the same contract `quota_triage_validation` establishes over
  the API. As above, this is a wiring check, not a re-verification of every
  quota rule.
- The tasks table is still empty.

## Procedure — hostile capture text stays escaped in the task list

1. Submit a capture with raw text `<script>alert('boom')</script>`.
2. Triage it as pool through the page's control.
3. `GET /` and read the **raw HTML source** of the task list.

### Expected Observable Outcomes
- The raw response does not contain an unescaped `<script>` tag.
- The word `boom` is still present — the content survived, escaped, rather
  than being stripped. `inbox_view` (#30) proved this for the inbox; a task
  list is a second render surface with its own template, and the same defect
  could exist there independently.

## Procedure — pool is the cheapest path

1. Submit a capture with raw text `buy milk`.
2. `GET /` and read that capture's row.
3. Count what the pool control asks the user for before it can be submitted,
   and do the same for committed and for quota. Count the fields a user must
   touch — a hidden `kind` input is not one, and neither is the submit button.
4. Check whether the pool control sits inside anything that has to be opened
   first.

### Expected Observable Outcomes
- The pool control is **submittable straight from the row**. It is not inside a
  `<details>`, an accordion, a dialog, or anything else the user must open —
  committed and quota may be, and are.
- Pool asks for **strictly fewer** inputs than committed, and strictly fewer
  than quota.
- Do **not** check for a literal count. `#9`'s AC-2 was amended on 2026-08-17
  to drop it: `#47` made a life area required for all three kinds and
  `D-manual-triage-until-llm` removed the picker's silent default, so pool is
  now pick-then-submit rather than one click. The property AC-2 was always a
  proxy for is the relation, and the relation is what QA checks.
- `D-pool-is-default` is why this is worth a procedure at all: pool is the
  default kind and committed the exception, so the default path must cost
  less than the exception. **Restoring a life-area default to get back to one
  click would satisfy the count and violate the product** — if pool has become
  cheap again because the picker answers for the user, that is a failure of
  this procedure, not a pass.

## Independent of Implementation

This procedure depends only on what `GET /` renders, how the page's triage
controls respond, and the durable state left behind. It does not depend on
which templating engine is used, whether triage happens inline or via a
separate page load, or which routes the page's controls submit to.
