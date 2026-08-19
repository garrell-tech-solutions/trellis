# QA Procedure: The schedule places committed work into free time

Covers: `features/schedule.feature`

## Interface used

The schedule page (reached from the header's **Schedule** link), the life
areas page and inbox for setup, `trellis serve --now <RFC3339>` to pin today,
and read-only `sqlite3` inspection. No project library, module, or test
helper is used.

**Pin the clock for every procedure.** `now` is one of `schedule()`'s seven
parameters; a schedule tested against the real clock is a schedule tested
against a different question every hour.

## What this slice does and does not do

**Forward pass only, and whole tasks only.** A task that does not fit one
free interval whole is **unplaceable, never chopped**
(`D-placed-whole-or-not-at-all`). Splitting is S3, pins are S4, the backward
pass is S2. **If you find a task placed in two pieces, that is a defect at
this slice, not a feature arriving early.**

**Only committed work is scheduled.** Pool is never placed
(`D-no-pool-on-calendar`) and quota is M8's. Neither should appear on the
page at all — not as placed, and **not as unplaceable either**. They are not
inputs, so they are not part of the partition.

**The plan is a record, not a view.** It is written when you ask for it and
does not change until you ask again — see the stability procedure, which is
the one that proves it.

## The four reasons, and the order they are tried

More than one can be true of a task at once, so **the precedence is part of
the contract**:

1. `no_window` — the life area has no hours in the horizon at all
2. `deadline_unreachable` — a **hard** deadline falls before the task could
   finish even placed first
3. `capacity_exceeded` — not enough free time remains after the tasks ranked
   above it
4. `chunk_policy_unsatisfiable` — time remains, but no single interval is long
   enough to hold it whole

**A reason outside this list is a defect**, and so is the right reason
reported for the wrong precedence. Check the reason, never just that the task
was refused — a scenario that asserts only "unplaceable" passes against an
implementation that gets every reason wrong.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path> --now 2026-08-17T09:00:00Z`.
2. Give **Work** `Mon–Fri 09:00–17:00`.
3. Capture and triage a committed Work task — *"write the Q3 deck"*, 2h, due
   Friday, P2.
4. Click **Schedule**. Confirm it is placed on **Mon 17 Aug at 09:00**, inside
   Work's hours, and that the page says so in real dates and times.
5. Triage a second committed task bigger than the hours that remain. Reload —
   confirm it appears under **won't fit**, with a reason.
6. Restart the server against the same database, same `--now`. Confirm the
   plan is unchanged.

### Expected Observable Outcomes
- All six steps hold literally, per `D-visible-slices`.
- **Step 4 is the first time this product puts anything on a schedule.**
  Everything before it said when you *could* work.
- Step 5 is what makes "won't fit" a real answer rather than silence. A task
  that vanishes without a reason is worse than one that is refused.
- **Step 6 must be a genuine no-op**: the same blocks, the same times, not
  merely a plan that recomputed to the same answer. Confirm by checking the
  blocks are in the database **before** the restart.

## Setup — repeat before each procedure below

1. Start against a fresh database with `--now 2026-08-17T09:00:00Z`. **Today
   is Monday 17 August 2026, 09:00 UTC**, which is exactly when a
   `Mon–Fri 09:00–17:00` guardrail opens — so the first free interval starts
   at *now*.
2. Confirm the timezone reads `UTC` and the schedule page is reachable from
   the header.

## Procedure — a task is placed inside its own life area's hours

1. Give Work `Mon–Fri 09:00–17:00`.
2. Triage a committed Work task of 120 minutes due Friday.
3. Generate the schedule and read the page.
4. Query the block rows.

### Expected Observable Outcomes
- One block, **09:00–11:00 on Monday 17 August**, and the page shows a real
  date and time rather than an offset or a duration.
- The block lies **entirely inside** Work's guardrail. Invariant 2 asserts
  this exhaustively under proptest; here it is confirmed on a real one.
- The stored block is in state **`proposed`**. Nothing at this slice
  publishes, starts or completes anything — `T-fact-plan-line` puts those
  states in the Constraints layer, which this slice **reads and never
  writes**.

## Procedure — the tighter deadline goes first, and soft may be overrun

1. Give Work `Mon 09:00–11:00` — two hours, two Mondays.
2. Triage two committed 120-minute Work tasks, one due Monday 11:00, one due
   Friday, both **soft**.
3. Generate and read.

### Expected Observable Outcomes
- The **Monday-deadline** task takes Monday 17th; the Friday one goes to
  Monday 24th.
- The Friday one is **placed late and says so**, carrying a projected finish
  after its deadline. `T-hard-refuses-soft-slips`: soft may be overrun, and
  the overrun is reported rather than hidden.
- **Reverse the two deadlines and confirm the order reverses.** Ordering that
  happens to be right because of insertion order is the failure this catches,
  and it is invisible from a single run.

## Procedure — whole or not at all

1. Give Work `Mon 09:00–11:00`.
2. Triage a committed Work task of **180 minutes**.
3. Generate and read.

### Expected Observable Outcomes
- **Nothing is placed**, and the task is unplaceable because
  `chunk_policy_unsatisfiable`.
- **No block of 120 minutes exists anywhere**, and no pair of blocks totalling
  180. Query the table, do not just read the page — a half-placed task is the
  exact failure `D-placed-whole-or-not-at-all` forbids, and the page might
  render it as one row.
- The reason is **not** `capacity_exceeded`: four hours remain across the
  horizon, so this is not a shortage of time, it is a shortage of *contiguous*
  time. When S3 adds splitting the same task becomes placeable, and the reason
  is honest about the capability rather than about the task.

## Procedure — a hard deadline that cannot be met

1. Give Work `Mon–Fri 09:00–17:00`.
2. Triage a committed Work task of 120 minutes with a **hard** deadline of
   10:00 today — one hour away.
3. Generate and read.

### Expected Observable Outcomes
- Unplaceable, `deadline_unreachable`, and **nothing is placed**.
- **Repeat with the deadline soft.** It should be *placed*, finishing after
  its deadline, with the overrun reported. That contrast is the whole of
  `T-hard-refuses-soft-slips`, and `deadline_type` finally does something
  after carrying no behaviour since M1.
- The guardrail is **not** breached to fit it. `D-guardrails-never-yield`: a
  P1 hard deadline against full windows raises a conflict, never a breach.

## Procedure — when the hours run out

1. Give Work `Mon 09:00–11:00` — four hours across the horizon.
2. Triage **three** committed 120-minute Work tasks.
3. Generate and read.

### Expected Observable Outcomes
- **Two placed, one unplaceable because `capacity_exceeded`.**
- The one refused is the one ranked last by least slack, not an arbitrary one.
- **Every task appears exactly once**, placed or refused — invariant 5's
  partition is total. A task that appears in neither list is the failure that
  invariant exists to catch, and it is easy to miss by eye because the page
  looks fine.

## Procedure — a life area with no hours

1. Mark Fitness **never scheduled — menu only**.
2. Triage a committed Fitness task.
3. Generate and read.

### Expected Observable Outcomes
- Unplaceable, `no_window`.
- **Not `capacity_exceeded`.** Opting out is not the same as being full, the
  same distinction the capacity page draws — and the reason the owner reads
  should tell them which fix applies.

## Procedure — pool and quota are not scheduled

1. Give Work `Mon–Fri 09:00–17:00`.
2. Triage a **pool** Work task and a **quota** Work task.
3. Generate and read.

### Expected Observable Outcomes
- Nothing is placed, and **neither task appears anywhere on the page** — not
  under placed, not under won't fit.
- They are not inputs to the scheduler, so they are outside the partition
  entirely. **A pool task listed as unplaceable would be a bug that reads like
  a feature**: it looks informative and it trains the owner to expect pool
  work to be scheduled, which `D-no-pool-on-calendar` refuses.

## Procedure — the plan does not move until you ask

1. Give Work `Mon–Fri 09:00–17:00`, triage one committed task, generate.
2. Note the exact block times.
3. Triage a **second** committed task. **Reload the schedule page without
   generating.**
4. Generate. Reload.
5. Restart the server with the same `--now` and reload without generating.

### Expected Observable Outcomes
- After step 3 the plan is **unchanged** — one block, the same times, and the
  new task appears nowhere.
- After step 4 both are placed.
- After step 5 the plan is byte-identical to step 4's, read from the database
  rather than recomputed.
- **This is the procedure that proves the decision.** A plan recomputed on
  every page load would satisfy the demo and fail step 3 — and it would mean
  a block moving under the owner between one glance and the next, which is
  the opposite of something you can work from.

## Procedure — nothing to schedule

1. Give Work a guardrail and triage nothing.
2. Open the schedule page.

### Expected Observable Outcomes
- An empty-state message that is **true** — nothing is scheduled because
  nothing has been triaged — rather than a blank panel. The inbox's *"Nothing
  to triage. Add a capture above to get started"* is the precedent.
- **This is a different emptiness from "everything was refused"**, which is
  not empty at all: it is a full won't-fit list and must not show the
  empty-state message.

## Procedure — hostile text stays escaped

1. Triage a committed Work task whose text is `<script>alert('boom')</script>`.
2. Generate, then read the **raw HTML source** of the schedule page.

### Expected Observable Outcomes
- No unescaped `<script>` tag; `boom` still present, escaped rather than
  stripped.
- The schedule page is a **new render surface** for task text. Every page that
  has ever rendered it has needed its own assertion.

## Procedure — nothing else changed

1. Run every other QA suite.
2. Run `app_shell` **expecting it to have changed**.

### Expected Observable Outcomes
- All others pass unchanged. This slice adds a page and a table; it changes no
  existing behaviour.
- **`app_shell` legitimately changed** — a page was added, so
  `T-nav-is-the-site-map` puts Schedule in the header. **If it did not need
  changing, the page is missing from `nav::ALL`** and ships with no link.
- `scheduler-core` purity holds: `schedule()` lives in the core, and
  `cargo tree -p scheduler-core` must contain no `tokio` and no `sqlx`.
- `capture-endpoint-persists-quickly-01`'s 50 ms budget is load-sensitive —
  that is `#66`. Re-run quiet and say so.

## Independent of Implementation

This procedure depends only on what the schedule page reports for a given set
of tasks, guardrails, exceptions and `now`, and on what survives a restart. It
does not depend on which route serves it, how blocks are stored, whether the
forward pass sorts or searches, or where the reason enum lives.
