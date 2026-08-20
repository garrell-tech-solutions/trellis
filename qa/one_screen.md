# QA Procedure: Trellis is one screen until the Menu returns

Covers: `features/one_screen.feature`

## Interface used

`curl` against the removed paths, the inbox at `GET /`, and read-only
`sqlite3` inspection. No project library, module, or test helper is used.

## What this slice removed, and what it did not

**Removed:** the `/stats`, `/life-areas`, `/free-time`, `/capacity` and
`/schedule` routes, the six server modules behind them, five core modules,
and nine acceptance features with their QA suites and step modules.

**Not removed, and each for a stated reason:**

- **Every table.** `tasks.life_area_id` carries a foreign key into
  `life_areas`, so dropping it means rebuilding `tasks` under
  `T-migrations-append-only`'s full table-rebuild pattern. Unused tables cost
  SQLite nothing, and leaving them keeps the owner's data as a second safety
  net beside git. **If you find a migration `0010` in this slice, that is a
  defect** — it deletes code and adds none.
- **`settings`.** Its only reader (`free_time`) is gone, so it now stores a
  value nothing reads. Kept deliberately: #85 needs it, and the timezone is
  the owner's data, not scaffolding.
- **`scheduler_core::schedule` and `::interval`.** M3 is paused, not
  cancelled, and #75's properties still hold.

## The timezone is a one-way door until #85

It lives on the life-areas page, which is gone. It is stored as
`America/New_York` and **persists** — nothing is lost — but **there is no
longer any way to change it from the running app.** `D-menu-is-a-worklist`
puts controls inline, so the Menu owns it from #85. **Do not report this as a
defect, and do not expect a settings page**; it is a named consequence.
Confirm the stored value is still there after this slice, which is the part
that matters.

## R2's counter goes dark, and nothing is lost

`#20` calls the committed:pool ratio *"the highest-leverage counter in the
product"*, and removing `/stats` stops it being computed. **It was always
computed on demand from `tasks` rows, which stay.** No history was thrown
away and nothing needs migrating; the moment a surface wants the number
again, it is recomputable from the same rows. Confirm the task rows survive.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Open `http://localhost:8080`. Confirm the capture box and the inbox are
   there and **there is no header** — no nav, no links to anywhere.
3. Quick-add a capture. Triage it. Dismiss another. Confirm all three still
   work exactly as before.
4. Type `http://localhost:8080/stats` by hand. Confirm a 404.
5. Repeat for `/life-areas`, `/free-time`, `/capacity`, `/schedule`.

### Expected Observable Outcomes
- All five steps hold literally.
- **Step 3 is the point of the slice and the thing most at risk.** A
  demolition that breaks capture has failed however clean the deletion was.
- Step 2's absence of a header is a real check, not a formality: a header
  rendering an empty list is a different outcome from no header, and only one
  of them is correct.

## Procedure — every removed path is gone

1. Request each of `/stats`, `/life-areas`, `/free-time`, `/capacity` and
   `/schedule`.

### Expected Observable Outcomes
- **404 on all five.** Not 200 with an empty page, not a redirect to `/`, not
  500.
- A redirect would be the failure worth naming: it looks tidy and it means the
  route still exists. The brief's word is *gone*.

## Procedure — no header renders

1. `GET /` and read the raw HTML.

### Expected Observable Outcomes
- **No `<header>` and no `<nav>`**, and no anchor pointing at another page of
  this product.
- `T-nav-is-the-site-map` is not superseded by this — with one route the site
  map is empty, and an empty site map renders nothing. The rule did the work.

## Procedure — capture and triage are untouched

1. Run the QA suites for `capture_endpoint`, `inbox_view`, `triage_from_page`,
   `committed_triage_validation`, `quota_triage_validation`,
   `committed_field_domains`, `unknown_kind_rejection`, `task_kinds`,
   `dismiss_capture`, `migrations`, `release_binary` and
   `scheduler_core_purity`.

### Expected Observable Outcomes
- **All pass.** This slice deletes; it changes no surviving behaviour.
- **Expect fixture drift wherever a script supplied a life area at triage** —
  the field is optional as of #82 and its picker is gone as of this slice.
  Reproduce each failure and confirm it is drift before fixing it.
- **PR #87 landed a 636-line stylesheet and restyled three templates without
  touching a single feature, QA script or step module.** CI stayed green, so
  the selectors these scripts key off survived — but this slice deletes a lot
  of what remains around them. **Re-run everything and believe the result
  rather than the expectation.**

## Procedure — the capture latency budget, now measured here

1. Start the server against a fresh database **on a quiet machine** —
   confirm load average is low before starting, and say what it was.
2. Send ten capture requests and record each response time.
3. Read the persisted rows.

### Expected Observable Outcomes
- Every response is **201**, every row is persisted, and the responses arrive
  **within 50 ms**.
- **This assertion moved here from the acceptance and unit suites in this
  slice** (`T-latency-is-a-qa-assertion`). It is a property of the deployed
  system on a machine that is not fighting itself, and that is the only place
  the reading means anything.
- **If the machine is busy, the number is not evidence — say so and re-run.**
  Reporting a failure measured under load is how this assertion cost four
  re-runs in six slices.
- The harm that decided it: under mutation-run contention this request took
  **1.885 s**, which failed `cargo-mutants`' unmutated **baseline**, so it
  refused to test a single mutant and the whole `trellis-server` crate had
  **zero mutation coverage** with no error connecting cause to effect.

## Independent of Implementation

This procedure depends only on what the surviving routes serve, what the
removed ones return, and what is left in the database. It does not depend on
how routing is wired, which modules were deleted, or whether the nav module
survived.
