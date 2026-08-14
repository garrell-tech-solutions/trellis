# QA Procedure: The stats page reports the committed share over a rolling fourteen-day window

Covers: `features/stats_ratio.feature`

## Interface used

The page at `GET /stats`, the capture and triage controls on `GET /` (driven
with `curl`, as established for `triage_from_page`), the `trellis serve`
command line, and read-only `sqlite3` inspection of persisted state. No project
library, module, or test helper is used.

### The `--now` flag

`trellis serve` accepts `--now <RFC3339>`. The server starts believing it is
that instant and time then **advances normally** from there — it is an offset,
not a freeze. Every capture and triage performed by that process is stamped
relative to the pinned instant.

This is the only way to verify that the window is a *window*. Without it, every
task in a fresh database was triaged seconds ago and is inside any window, so
an implementation that counted every task ever written would pass every other
procedure here. It is a user-interface affordance: a flag on the command the
owner already runs, alongside `--db` and `--addr`.

`--now` applies per process, so it must be supplied on **every** start. A server
started without it uses the real clock.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Open `http://localhost:8080`. Capture and triage three things as **Pool**,
   and one as **Committed**.
3. Open `http://localhost:8080/stats`. Confirm it shows the counts — 1
   committed, 3 pool, 0 quota — and says it has not measured enough to report a
   share yet. Four tasks is below the floor.
4. Capture and triage six more as **Committed**. Reload `/stats`. Ten tasks is
   at the floor, so a share now appears: 7 of 10 committed, **70%**, and the
   page says it is over the line — in words or a marker you can read *without*
   comparing 70% to 50% yourself.
5. Restart the server and reload `/stats`. The figures are identical.

### Expected Observable Outcomes
- All five steps hold literally, per `D-visible-slices` — this is the demo the
  handoff brief specifies as the acceptance criterion.
- Step 3 into step 4 is the point of the slice: the page refuses to state a
  share it cannot support, then states one the moment it can.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. `GET /stats` and confirm it reports no tasks.

Throughout, "triage N as committed" means capture N items and triage each
through the interface the way `triage_from_page` establishes. The triage
contract itself is that procedure's job; here it is only setup.

## Procedure — the share, and the counts behind it

1. Triage 5 as committed, 4 as pool, 3 as quota.
2. `GET /stats`.

### Expected Observable Outcomes
- The page reports **5 committed, 4 pool and 3 quota**, and **12 tasks in the
  window**.
- The committed share reads **42%** (5 of 12, to the nearest whole percent).
- The counts are present alongside the percentage, not instead of it. A reader
  can see what the denominator was without inferring it — this is the whole
  reason the counts are specified, because a bare percentage whose denominator
  is ambiguous gets misread for months.

## Procedure — quota is inside the denominator

1. Triage 6 as committed and 6 as pool.
2. `GET /stats` and read the share.
3. Triage 6 more as quota.
4. `GET /stats` again.

### Expected Observable Outcomes
- After step 2: 12 tasks, share **50%**.
- After step 4: 18 tasks, share **33%** — the share *fell* because the six
  quota tasks joined the denominator.
- If the share is still 50% after step 4, quota is being excluded and the
  denominator is not what the page claims.

## Procedure — the rolling fourteen-day window

This procedure needs three server runs against **one** database. Do not use a
fresh database between them.

1. Start with `--now 2026-07-24T09:00:00Z`. Triage 3 as committed. Stop the
   server.
2. Restart the same database with `--now 2026-08-06T09:00:00Z` — thirteen days
   later. Triage 12 as pool.
3. `GET /stats`.
4. Stop. Restart the same database with `--now 2026-08-08T09:00:00Z` — fifteen
   days after the committed batch, two days after the pool batch.
5. `GET /stats`.
6. Query the tasks table and count its rows.

### Expected Observable Outcomes
- After step 3: **15 tasks in the window**, share **20%** (3 of 15). Everything
  triaged so far is inside fourteen days.
- After step 5: **12 tasks in the window**, share **0%**. The three committed
  tasks are fifteen days old and have left the window; the twelve pool tasks are
  two days old and remain.
- After step 6: the tasks table still holds **15** rows. Leaving the window is
  not deletion — the page counts a subset, it does not destroy anything.
- If step 5 still reports 15 tasks or a 20% share, there is no window and the
  page is counting every task ever triaged.

## Procedure — `--now` offsets the clock, it does not stop it

1. Start with `--now 2026-07-24T09:00:00Z`.
2. Capture `first`, then capture `second`.
3. `GET /` and read the inbox order.
4. Query `created_at_ms` for both captures.

### Expected Observable Outcomes
- The two timestamps are **different**, and `second` is the later of the two.
- The inbox lists `second` before `first`, per `inbox_view`'s newest-first
  order.
- A frozen clock would stamp both identically and the inbox's ordering would
  quietly stop meaning anything. This procedure exists because this slice is
  what introduces a settable clock at all.

## Procedure — the fifty-percent line, reported distinguishably

1. Triage 5 as committed and 15 as pool.
2. `GET /stats` and read the page.
3. On a fresh database, triage 10 as committed and 10 as pool.
4. `GET /stats`.
5. On a fresh database, triage 11 as committed and 9 as pool.
6. `GET /stats`.

### Expected Observable Outcomes
- Step 2: share **25%**, reported as **under the line**.
- Step 4: share **50%**, reported as **under the line**. The alarm is at
  *more than* 50%; exactly 50% does not trip it.
- Step 6: share **55%**, reported as **over the line**.
- In every case the standing is legible **without** the reader comparing the
  percentage to 50 themselves — a word, a label, a marker, something that is not
  the number. Exact wording is the implementation's to choose; that it is
  separate from the digits is the acceptance criterion.

## Procedure — below ten tasks, and an empty database

1. On a fresh database, `GET /stats` before triaging anything.
2. Triage 4 as committed and 5 as pool — nine tasks.
3. `GET /stats`.
4. Triage one more as pool — ten tasks.
5. `GET /stats`.

### Expected Observable Outcomes
- Step 1: the page reports **0 committed, 0 pool, 0 quota**, no share, and says
  it has not measured enough yet. It does not report `0%`, and it does not
  return an error — nothing is divided by zero.
- Step 3: the page reports **4 committed, 5 pool, 0 quota** and **9 tasks in the
  window**, and still reports no share and no standing. It does not print `44%`.
- Step 5: with ten tasks the share appears — **40%**, under the line.
- The floor is exactly ten: nine tasks report no share, ten report one. The
  counts are visible at every stage, because counts are facts; the percentage
  and the standing are withheld until there is enough behind them to mean
  something.

## Procedure — the figures are computed from the rows, not held in memory

1. Triage 6 as committed and 6 as pool.
2. `GET /stats` and record every figure on the page.
3. Stop the server. Restart it against the **same** database.
4. `GET /stats`.

### Expected Observable Outcomes
- Every figure is identical to step 2 — counts, window total, share and
  standing.
- Nothing about the page depends on the process that served it having witnessed
  the triages.

## Procedure — the stats page changes nothing else

1. Capture `buy milk` and leave it untriaged. Triage 3 others as committed.
2. `GET /stats`.
3. `GET /` and read the inbox and the task list.
4. `GET /stats` a second time.
5. Query the captures and tasks tables.

### Expected Observable Outcomes
- `buy milk` is still in the inbox, untriaged, after `/stats` has been read
  twice.
- The task list still shows the three committed tasks.
- Both `/stats` reads report the same figures — reading the page is not an
  action and consumes nothing.
- Row counts in both tables are unchanged from step 1. `/stats` reads; it never
  writes.

## Independent of Implementation

This procedure depends only on what `GET /stats` renders, what `--now` does to
the clock the server stamps rows with, and the durable state left behind. It
does not depend on which SQL the page issues, whether the figures are computed
in one query or several, which template renders them, or how the standing is
worded — only that the standing is readable as something other than the
percentage itself.
