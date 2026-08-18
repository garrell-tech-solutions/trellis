# QA Procedure: A dated exception is how the owner says a week is not normal

Covers: `features/exceptions.feature`

## Interface used

The free time page (reached from the header's **Free time** link), its own
exception controls, the life areas page for guardrail setup,
`trellis serve --now <RFC3339>` to pin today, and read-only `sqlite3`
inspection. No project library, module, or test helper is used.

**Read every endpoint out of the page's markup**, and reach the page by its
header link rather than typing a URL.

**Pin the clock for every procedure below.** An exception on "24 August"
means nothing without a fixed today, and a suite that passes in one week and
fails the next has tested the calendar rather than the product.

## What an exception is, and what it is not

**An exception only ever removes hours.** There is no such thing as an
exception that adds availability. *"Working this Saturday"* is not an
exception — it is a different guardrail, and the owner changes the guardrail
to say it.

Two reasons, and the second outlives this slice:

- `D-guardrails-never-yield` says a guardrail is never breached, *"an
  override that exists will get used, and then the walls are decorative."* A
  wall that can only ever shrink for a day cannot be argued into yielding.
- M3's pins and M4's calendar busy are both **subtractive**, and this slice is
  the first subtrahend, so its shape is the one they inherit. An additive
  exception would make this a per-day override of the mask instead — a
  different mechanism, which neither of them wants.

**If a control anywhere lets an exception grant hours the guardrail does not
already have, that is a defect, not a feature.**

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file with
   `--now 2026-08-17T12:00:00Z`. **Today is Monday 17 August 2026.**
2. Confirm the server is reachable.
3. On **Life areas**, give **Work** `Mon–Fri 09:00–17:00`.
4. Open **Free time** and confirm Work reports **80h** before going further.
   The fourteen-day horizon covers 17–30 August and holds exactly ten
   weekdays. **If this number is not 80, stop** — every expectation below is
   derived from it and a different baseline makes them all meaningless.

## By-hand walkthrough — do this once, in a real browser

1. Start as above and confirm Work shows 80h.
2. Mark **24–28 August** away for all life areas.
3. Reload Free time. Confirm those five days are **gone** from Work's
   intervals and the total is **40h**.
4. Restart the server against the same database, with the same `--now`.
   Confirm it is still 40h.
5. Remove the exception. Confirm Work is back to 80h and the exception is
   **gone from the list**, not merely struck through or greyed.

### Expected Observable Outcomes
- All five steps hold literally, per `D-visible-slices`.
- **Step 3 is the point of the slice.** It is the first time anything in this
  product removes availability, and the page shows it happening.
- Step 5 is the `D-kill-means-archive` answer, and it is a **departure worth
  being deliberate about**: an exception is *removed*, not archived. That
  decision keeps rows for **work items** — a capture, a task — because the row
  feeds M8's reckoning. An exception is **configuration**, like a guardrail
  band, which `#59` already removes outright. Archiving configuration would
  accumulate rows no surface reads and no reckoning counts.

## Procedure — an exception removes those dates

1. Mark 24–28 August away for all life areas.
2. Reload Free time and read Work's total and intervals.
3. Repeat, on fresh databases, for each of: 24 August alone; 22–23 August;
   10–20 September.

### Expected Observable Outcomes
- 24–28 August → **40h**. Five weekdays removed.
- 24 August alone → **72h**. One weekday removed.
- 22–23 August → **80h**, unchanged. That weekend carries no Work hours, so
  removing it removes nothing. **An exception on days a guardrail does not
  cover is not an error and is not a no-op to be rejected** — it is simply a
  statement about days that happened to be empty.
- 10–20 September → **80h**, unchanged: entirely outside the fourteen-day
  horizon. It is still stored and still listed.
- The intervals for the removed days are **absent**, not zero-length. A
  zero-length interval is something a scheduler could try to place work into.

## Procedure — scope is explicit, and visible

1. Give **Fitness** `Mon–Fri 06:00–07:00`.
2. Mark 24–28 August away **for Fitness only**.
3. Reload Free time.
4. Mark 1–2 September away **for all life areas**.
5. Read the exceptions list.

### Expected Observable Outcomes
- Work is still **80h**; Fitness drops to **5h**. One life area's exception
  does not touch another's.
- The list says, **for each exception, which life areas it covers** — the
  words "All life areas", or the life area's own name. **Scope shown, never
  implied.** An exceptions list that does not say what an entry applies to is
  one the owner has to guess at, and the guess they will make is "everything".
- A global exception and a per-life-area one are visibly different rows.

## Procedure — a past exception stops mattering by itself

1. Mark 10–14 August away for all life areas — entirely before today.
2. Reload Free time and read Work's total.
3. Read the exceptions list.

### Expected Observable Outcomes
- **80h**, unchanged: those dates are behind the horizon.
- The exception is **still listed and still stored**. It stopped mattering
  because the window moved past it, **not because anything cleaned it up**.
  There is no expiry job, no purge, and no state to get wrong — the horizon
  does the work.

## Procedure — overlapping exceptions subtract their union

1. Mark 24–28 August away for all life areas.
2. Mark 26–29 August away for all life areas.
3. Reload Free time.

### Expected Observable Outcomes
- **40h** — the same as one exception alone, because the second overlaps the
  first and adds only 29 August, a Saturday Work does not cover.
- **Not 16h**, which is what subtracting each exception's hours separately
  would give. Double subtraction is the failure mode here, and it is
  invisible without a case that overlaps deliberately.
- No negative-length interval appears anywhere. The proptest asserts this
  exhaustively over generated inputs; this confirms it on a real overlap.

## Procedure — an exception across a DST transition

1. Restart with `--now 2027-03-13T12:00:00-05:00`, timezone
   `America/New_York`, and give **Learning** `Sun 01:00–04:00`.
2. Confirm Learning reports **5h** before any exception — three hours on
   Sunday 21 March, two on Sunday 14 March, which loses the hour that does
   not exist.
3. Mark **21 March** away. Reload.
4. Repeat the whole procedure at `--now 2027-11-06T12:00:00-04:00`, marking
   **14 November** away.

### Expected Observable Outcomes
- Spring: **2h** remain — the transition Sunday, still two hours long.
- Autumn: **4h** remain — the transition Sunday, still four hours, because
  `01:00–02:00` happens twice and `T-fold-counts-both-passes` counts both.
- **These procedures remove the *ordinary* Sunday on purpose.** Removing the
  transition day would leave a plain three-hour Sunday and prove nothing; what
  is under test is that subtraction leaves the odd-length day **intact and
  still odd**. A result of 3h in either direction means the transition was
  flattened somewhere in the subtraction.

## Procedure — a life area given hours and then marked never scheduled

1. Confirm Work has its `Mon–Fri 09:00–17:00` band and reports 80h.
2. On Life areas, mark Work **never scheduled — menu only**.
3. Reload Free time.
4. Query the database for Work's bands.

### Expected Observable Outcomes
- Work reports **0h**.
- **The bands are very likely still stored, and that is fine** — what matters
  is that nothing offers them. The rule belongs to the front door every reader
  passes through, not to each reader.
- **This procedure exists because of a real defect**, found in `#60` after its
  own scenarios passed: the guardrail front door handed out bands regardless
  of the mark, and `/free-time` reported **32h for a life area the owner had
  marked never scheduled**. The scenario that should have caught it marked a
  life area never-scheduled *from a clean state*, so it read 0h either way.
  **The order is the test.** Do not simplify this procedure by marking a bare
  life area — that is the version that already passed while the bug was live.

## Procedure — a backwards range is refused

1. Mark 28 August to 24 August away for all life areas.
2. Observe the status and body.
3. Read the exceptions list and Work's total.

### Expected Observable Outcomes
- Rejected with **`422`** carrying the re-rendered fragment
  (`T-422-is-product-wide`), and the message says the last day precedes the
  first — **not a generic "invalid dates"**. A rejection that does not say
  what was wrong is a failure even with the right status code.
- Nothing is stored: the list is empty and Work is still 80h.

## Procedure — hostile text in a label stays escaped

1. Mark 24–28 August away with the label `<script>alert('boom')</script>`.
2. Read the **raw HTML source** of the exceptions list.

### Expected Observable Outcomes
- No unescaped `<script>` tag; the word `boom` still present, escaped rather
  than stripped.
- The label is the **only free text** an exception carries — dates are dates
  and scope is a closed choice — so it is the only place this can go wrong,
  which is exactly why it gets its own procedure.

## Procedure — nothing else changed

1. Run the QA suites for `capture_endpoint`, `inbox_view`, `triage_from_page`,
   `committed_triage_validation`, `quota_triage_validation`,
   `unknown_kind_rejection`, `stats_ratio`, `life_areas`, `life_area_triage`,
   `dismiss_capture`, `app_shell`, `guardrails`, `timezone_setting`,
   `free_time`, `task_kinds`, `committed_field_domains`, `migrations`,
   `release_binary` and `scheduler_core_purity`.

### Expected Observable Outcomes
- **All nineteen pass unchanged.** This slice adds no page — the exception
  controls live on the free time page, the only page whose numbers they
  change — so `nav::ALL` is untouched and `app_shell` should **not** need
  editing. **If `app_shell` did change, a page was added; if a page was added
  and `app_shell` did not change, it is missing from `nav::ALL`** and ships
  with no link.
- `free_time`'s own totals are unchanged in the absence of any exception. A
  slice that subtracts must subtract **nothing** when there is nothing to
  subtract.
- `scheduler-core` purity holds: subtraction lives in the core, and
  `cargo tree -p scheduler-core` must contain no `tokio` and no `sqlx`.
- `capture-endpoint-persists-quickly-01`'s 50 ms budget is load-sensitive —
  that is `#66`. Re-run quiet and say so rather than chasing it.

## Independent of Implementation

This procedure depends only on what the free time page reports for a given
guardrail, exception set, horizon and timezone, and on what survives a
restart. It does not depend on which route the controls submit to, whether an
exception is stored as dates or instants, whether scope is a nullable column
or something else, or where the subtraction happens.
