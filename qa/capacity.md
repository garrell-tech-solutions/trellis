# QA Procedure: Capacity reports what each life area needs against what it has

Covers: `features/capacity.feature`

## Interface used

The capacity page (reached from the header's **Capacity** link), the life
areas page and free time page for setup, the inbox for triage, and read-only
`sqlite3` inspection. No project library, module, or test helper is used.

**Reach every page by its header link.** Typing a URL skips the thing
`app-shell` exists to guarantee.

## What this number is, and the direction it must never be wrong in

**Supply is free time after exceptions; demand is what committed and quota
work asks for.** The whole reason M2 exists is `#6`: a number that tells the
owner on a quiet Tuesday that Saturday will not fit.

**A capacity number that is too optimistic is worse than no number.** Every
rule below exists to keep it from drifting that way — quota counts, an
unestimated task is never treated as zero, exceptions subtract before the
comparison. If you find yourself unsure whether something should count, the
safe answer is the one that reports *less* availability.

**Pool is the exception to that instinct, and it is deliberate.** Pool
consumes nothing (`D-no-pool-on-calendar`: pool is never placed), so a
backlog of two hundred pool items must not make the fortnight look
catastrophic. **If a pool task moves the number at all, that is a defect.**

## The horizon needs no pinned clock, except for exceptions

`T-free-time-horizon-fourteen-days` guarantees exactly two of every weekday in
the window, so `Sat 09:00–11:00` is **4h** and `Mon–Fri 09:00–17:00` is **80h**
on whatever day you run this. **Only the exception procedure pins a clock**,
because an exception is a date.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Give **Fitness** a guardrail of `Sat 09:00–11:00`. Open **Free time**;
   confirm **4h**.
3. Capture something, triage it **committed** into Fitness with an estimate of
   **3h**. Open **Capacity**: *"Fitness — 3h needed, 4h available, 75%."*
4. Triage a second committed Fitness task estimated **2h**. Reload:
   *"Fitness — 5h needed, 4h available, 125%, 1h over."*
5. Widen Fitness's guardrail to `Sat 09:00–12:00`. Reload — **the warning
   clears** and the page reports 5h needed against 6h available.
6. Confirm the triage form will **not** accept a committed task with no
   estimate, and that pool and quota forms do not ask for one.

### Expected Observable Outcomes
- All six steps hold literally, per `D-visible-slices`.
- **Step 4 is the slice.** You found out on a quiet Tuesday, not at 4pm on the
  Saturday.
- Step 5 is what makes it an instrument rather than an alarm: the number
  responds to the fix, so the owner can tell whether what they did was enough.
- Step 6 is the triage-surface change this slice carries. **Only committed
  gains the field.**

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable and the capacity page is reachable from the
   header.

## Procedure — supply against demand

1. Give Fitness `Sat 09:00–11:00`.
2. Triage a committed Fitness task estimated at 180 minutes.
3. Open Capacity.

### Expected Observable Outcomes
- **3h needed, 4h available, 75%**, and **no warning**.
- The percentage is shown whether or not there is a warning. The threshold is
  **strictly above 100%**: a fortnight at 98% is reported at 98% and left to
  the owner to judge. `D-staleness-unset` says instrument first and tune at the
  first reckoning, and a margin chosen today would be chosen with no fortnight
  of real numbers behind it.
- The available figure agrees with what the **free time page** reports for
  Fitness. Two pages disagreeing about the same fortnight is the failure that
  makes both untrustworthy.

## Procedure — over-commitment is named

1. Give Fitness `Sat 09:00–11:00`.
2. Triage committed Fitness tasks estimated at 180 and 120 minutes.
3. Open Capacity.

### Expected Observable Outcomes
- **5h needed, 4h available, 125%, and 1h over** — the overage stated, not
  left to the reader to subtract.
- Widening the guardrail or removing a task clears it. **Confirm the clear**,
  not just the warning: a warning that never goes away is one the owner learns
  to ignore.

## Procedure — pool consumes nothing

1. Give Fitness `Sat 09:00–11:00`.
2. Triage **two pool** Fitness tasks.
3. Open Capacity.

### Expected Observable Outcomes
- **0h needed, 4h available.** Not "2 tasks", not an estimate, not a warning.
- Pool tasks carry no estimate at all, and the triage form must not ask for
  one. `D-no-pool-on-calendar` is why: pool is never placed, so it occupies
  nothing.

## Procedure — quota demand counts, prorated

1. Give Learning `Mon–Fri 20:00–22:00` — **20h** available.
2. Triage a quota Learning task of **3 sessions of 40 minutes per week**.
3. Open Capacity.
4. On a fresh database, repeat with **10 sessions of 45 minutes per month**.

### Expected Observable Outcomes
- Weekly: **4h needed** — the horizon is two weeks, so two periods.
- Monthly: **3.5h needed** — fourteen days of a thirty-day month is
  fourteen thirtieths of 450 minutes.
- **A month is thirty days, flatly**, not the length of whichever calendar
  month the horizon happens to straddle. The horizon can cross a month
  boundary, and "which month" has no answer when it does.
- Where proration does not divide evenly, it rounds **up** to the whole
  minute. Under-reporting demand is the direction of wrongness this number
  exists to prevent.
- **Last week's missed sessions are not in the number.** `D-quota-no-rollover`
  — a missed week is missed, and the horizon asks only what it asks.

## Procedure — a life area that opted out

1. Mark Fitness **never scheduled — menu only**.
2. Triage a committed Fitness task estimated at 180 minutes.
3. Open Capacity.

### Expected Observable Outcomes
- Fitness is reported as **never scheduled**, with no hours and **no
  warning**.
- **Not "0h available, 3h needed, 3h over".** It opted out of being
  scheduled, which is not the same as being full, and warning about it would
  train the owner to ignore the warning that matters.

## Procedure — an exception lowers what is available

1. Restart with `--now 2026-08-17T12:00:00Z`.
2. Give Fitness `Sat 09:00–11:00` and triage a committed Fitness task
   estimated at 180 minutes. Confirm **3h needed, 4h available**, no warning.
3. Mark **22 August** away for all life areas.
4. Reload Capacity.

### Expected Observable Outcomes
- **3h needed, 2h available, 1h over.** One Saturday removed halves the
  supply and tips the life area over.
- **This is the first place two of M2's mechanisms compose**, and it is worth
  running by hand as well as by script: exceptions subtract from free time,
  and capacity compares against what is left. Shipping capacity before
  exceptions would have over-reported every exception day — which is why #61
  came first.

## Procedure — archived things are excluded

1. Give Fitness `Sat 09:00–11:00` and triage a committed Fitness task.
2. Archive Fitness.
3. Open Capacity.

### Expected Observable Outcomes
- Fitness is **not reported at all**.
- **An archived *task* cannot be checked here**, and that is not an oversight:
  `tasks.archived_at` exists but nothing writes it before M8, so there is no
  user-visible act that archives a task at M2. The exclusion rule is real and
  belongs to a unit test until something can drive it.

## Procedure — a committed task with no estimate

**There is no way to create one from the page**, because triage now requires
the field, so this cannot be driven end-to-end on a fresh database. It is
recorded here because it is real in the owner's own database, where committed
tasks predate the column.

### Expected Observable Outcomes
- A `NULL` estimate means **"written before the estimate was required"**,
  never "takes no time" — the same reading `T-life-area-required-at-triage`
  gave `life_area_id`.
- It must **not** be counted as zero. That would under-report demand silently,
  in the one direction this number must never be wrong.
- It is excluded from the sum and **surfaced as a count on its life area's
  row**, so the owner can see the number is incomplete and fix it.
- **Verify by unit test, and say in the report that QA could not drive it.**

## Procedure — nothing else changed

1. Run the QA suites for `capture_endpoint`, `inbox_view`, `triage_from_page`,
   `quota_triage_validation`, `unknown_kind_rejection`, `stats_ratio`,
   `life_areas`, `life_area_triage`, `dismiss_capture`, `guardrails`,
   `timezone_setting`, `free_time`, `exceptions`, `task_kinds`,
   `committed_field_domains`, `migrations`, `release_binary` and
   `scheduler_core_purity`.
2. Run `app_shell` and `committed_triage_validation` **expecting both to have
   changed**.

### Expected Observable Outcomes
- **Expect fixture drift, and expect it to be noisy.** Every script that
  triages a valid committed task now has to send an estimate.
  `T-life-area-required-at-triage` broke five already-green scripts the same
  way. **Reproduce each failure, confirm it is the new required field, fix the
  fixture — do not assume.** A real regression hiding in predicted drift is
  exactly how one gets shipped.
- **`app_shell` legitimately changed**: this slice adds a page, so
  `T-nav-is-the-site-map` puts Capacity in the header. **If `app_shell` did
  *not* need changing, the page is missing from `nav::ALL`** and ships with no
  link.
- **`committed_triage_validation` legitimately changed**: it enumerates
  committed's required fields by name, and there is now a fourth.
- `free_time`'s totals are unchanged. Capacity reads free time; it must not
  alter it.
- `scheduler-core` purity holds: the arithmetic lives in the core, and
  `cargo tree -p scheduler-core` must contain no `tokio` and no `sqlx`.
- `capture-endpoint-persists-quickly-01`'s 50 ms budget is load-sensitive —
  that is `#66`. Re-run quiet and say so.

## Independent of Implementation

This procedure depends only on what the capacity page reports for a given set
of guardrails, exceptions, tasks and horizon. It does not depend on which
route serves it, where the estimate column lives, whether the arithmetic runs
in the core or the handler, or how demand is stored.
