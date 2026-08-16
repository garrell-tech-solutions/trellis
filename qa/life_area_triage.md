# QA Procedure: Triage tags a task with a life area

Covers: `features/life_area_triage.feature`

## Interface used

The page at `GET /`, `curl` against whatever endpoint the page's triage
controls submit to, the page at `GET /life-areas` for setup, and read-only
`sqlite3` inspection. No project library, module, or test helper is used.

The by-hand browser walkthrough for this slice is in `qa/life_areas.md` and
covers both halves; it is not repeated here. Everything below is `curl`-only.

Throughout, "triage as pool into X" means submitting the page's own pool-triage
control with life area `X`. Read the page's HTML to find it; do not assume a
route. The triage contract for kinds and their required fields is
`committed_triage_validation`'s and `quota_triage_validation`'s job — here it
is only the life area that is under test.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the task list is empty and the five seeded life areas are offered.

## Procedure — triage offers every life area

1. Submit a capture with raw text `sketch the landing page`.
2. Add a life area named `Side project`.
3. `GET /` and read the triage controls for that capture.

### Expected Observable Outcomes
- The life area control offers exactly six: the five seeded plus
  `Side project` — a closed set of choices, not a free-text box. A free-text
  field here would let the user construct a request the boundary must then
  reject, which is the defect `triage_from_page` closed for deadline type and
  priority.

## Procedure — the tag follows the task

1. Submit captures with raw text `sketch the landing page` and `buy milk`.
2. Triage the first as pool into `Learning`.
3. Triage the second as pool into `Home`.
4. `GET /` and read the task list.
5. Query the tasks table for both rows.

### Expected Observable Outcomes
- The task list shows `sketch the landing page` tagged `Learning` and
  `buy milk` tagged `Home` — each carrying its own, not one applied to both.
- Each row references its life area **by id**, not by a copy of the name. This
  is what makes a later rename free and keeps M8's reckoning coherent across a
  quarter; a stored name string would orphan every task the first time one is
  renamed.

## Procedure — a life area is required

1. Submit a capture with raw text `buy milk`.
2. Submit the triage control as **pool** with the life area omitted entirely.
3. Observe the status and body.
4. Repeat for **committed** (with its deadline, deadline type and priority all
   present and valid) and for **quota** (with its targets and period valid).
5. Query the tasks table and count its rows.
6. Re-read the untriaged queue.

### Expected Observable Outcomes
- All three are rejected, and each rejection names the life area field —
  the same shape `committed_triage_validation` already establishes for a
  missing required field.
- Committed and quota are rejected **even though every one of their own
  required fields was valid**, which is what proves the life area is required
  in its own right rather than incidentally.
- The tasks table is still empty and the capture is still untriaged. A
  rejected triage creates nothing.

## Procedure — a life area that does not exist is refused

1. Submit a capture with raw text `buy milk`.
2. Submit the triage control as pool with life area `Gardening`.
3. Observe the status and body.
4. Query the tasks table.

### Expected Observable Outcomes
- Rejected, and the response names `Gardening` as the value that is not a life
  area — echoing what was submitted, as `unknown_kind_rejection` established
  for an unrecognised kind.
- Nothing is created, and no life area named `Gardening` is created as a side
  effect of being named. Triage consumes life areas; it does not mint them.

## Procedure — an archived life area is refused at the boundary, not merely hidden

1. Submit a capture with raw text `buy milk`.
2. Archive `Learning`.
3. Submit the triage control as pool with life area `Learning`, by hand — do
   not use the picker, which no longer offers it.
4. Observe the status and body.
5. Query the tasks table.

### Expected Observable Outcomes
- Rejected; nothing is created.
- This is the point of the procedure: the picker hiding an archived life area
  is a rendering fact, and a rendering fact is not a rule. A request composed
  by hand must still be refused, or "archived" means only "harder to pick".

## Procedure — a hostile life area name stays escaped on the task row

1. Add a life area named `<script>alert('boom')</script>`.
2. Submit a capture with raw text `buy milk` and triage it as pool into that
   life area.
3. `GET /` and read the **raw HTML source** of the task list.

### Expected Observable Outcomes
- The raw response does not contain an unescaped `<script>` tag.
- The word `boom` is still present — escaped, not stripped.
- `qa/life_areas.md` asserts this for the management list and the picker; the
  task row is a third surface with its own template and could fail
  independently.

## Procedure — the tag survives a restart

1. Triage `sketch the landing page` as pool into `Learning`.
2. `GET /` and confirm the tag renders.
3. Stop the server. Restart it against the **same** database.
4. `GET /`.

### Expected Observable Outcomes
- The task is still listed and still tagged `Learning`. The tag is a persisted
  reference, not something the serving process remembered.

## Procedure — triage otherwise behaves identically

1. Submit a capture with raw text `call the dentist`.
2. Triage as committed with `deadline` omitted but a valid life area present.
3. Observe the status and body.
4. Triage as quota with `target_count` omitted but a valid life area present.
5. Submit a capture and triage it as pool with a valid life area.
6. `GET /` and read the inbox and task list.

### Expected Observable Outcomes
- Steps 2 and 4 are rejected naming `deadline` and `target_count` respectively
  — the existing validation contract is unchanged by the arrival of a new
  required field.
- Step 5 succeeds and the capture leaves the inbox for the task list, exactly
  as `triage_from_page` specifies.
- Nothing in `capture_endpoint`, `inbox_view`, `committed_triage_validation`,
  `quota_triage_validation`, `unknown_kind_rejection` or `stats_ratio` changes
  behaviour. If any of them needed editing, something was rebuilt that should
  have been reused.

## Independent of Implementation

This procedure depends only on what `GET /` renders, how its triage controls
respond, and the durable state left behind. It does not depend on which route
the controls submit to, whether the life area is validated in `scheduler-core`
or at the store, how the reference is spelled in the request, or which template
renders the tag.
