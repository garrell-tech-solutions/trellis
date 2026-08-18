# QA Procedure: The free time page reports when each life area is actually free

Covers: `features/free_time.feature`

## Interface used

The free time page at whatever URL the header's **Free time** link carries,
the life areas page for setup, `trellis serve --now <RFC3339>` for the DST
procedures, and read-only `sqlite3` inspection. No project library, module,
or test helper is used.

**Read the free time URL out of the header**, not from this document. The
whole point of `app-shell` is that the owner never types one, and a procedure
that types one has stopped testing that.

## What this page answers, and what it does not

**It answers "when am I actually free", never "what should go there".**
Nothing in this slice places anything. If a procedure below tempts you to
check what work landed in a free interval, stop — there is no scheduler until
M3.

**Nothing subtracts from the mask yet.** Busy, pins and buffers have no source
at M2: pins arrive at M3, calendar busy at M4, and dated exceptions in the
very next slice (#61). So free time currently equals the guardrail projected
across the range, and **that is the correct answer, not a stub.** A build that
invents a fake busy source to make this page look richer has manufactured data
the owner did not enter.

**Fourteen days is exactly two weeks**, so the window holds exactly two of
every weekday whatever day you run this. Every expected total below is a fixed
number for that reason — if a total moves depending on the day you test, the
range is not what it claims to be.

## The two DST answers, which are decisions and not accidents

- **Spring forward:** the hour that does not exist is **not** counted. A band
  covering it is shorter that day. Anything else invents an hour.
- **Fall back:** the hour that happens twice **is** counted twice. At both
  instants the owner's clock reads a time inside the band, and the day really
  does have 25 hours.

**Both are recorded in `docs/decisions.md`.** If a run disagrees with either,
that is a defect even if the number looks reasonable — the point of writing
them down is that once made, the choice is invisible in the code.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. On **Life areas**, set the timezone to your own and give **Work**
   `Mon–Fri 09:00–17:00`.
3. Click **Free time** in the header. Confirm you see Work's next fourteen
   days, each day listed, with a total of **80h** (ten weekdays × 8h).
4. Remove Work's band and re-save it ending at `16:00`. Reload Free time.
   Confirm **every Work day is an hour shorter and the total is 70h.**
5. Confirm a life area marked *never scheduled — menu only* appears with no
   intervals and a zero total, rather than being absent or erroring.

### Expected Observable Outcomes
- All five steps hold literally, per `D-visible-slices`.
- **Step 4 is the demo**: the page is not a static rendering of a guardrail,
  it is a projection that moves when the guardrail moves.
- Step 5 is the empty-answer criterion. **Absent is not the same as zero** —
  a life area with no hours still has an answer, and the answer is none.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable and the timezone reads `UTC`.

## Procedure — a guardrail projects across the range

1. Give Work a band on Mon `09:00`–`17:00`.
2. Open the free time page.
3. Count the intervals listed for Work and read its total.

### Expected Observable Outcomes
- **Two** intervals — one per Monday in fourteen days — and a total of **16h**.
- Each interval is listed, not just summarised. A total alone cannot tell the
  owner that next Thursday is empty, which is the question the page exists to
  answer.
- The intervals are **sorted and disjoint**. This is asserted exhaustively by
  the proptest over generated inputs; here it is confirmed on a real one.

## Procedure — empty is a real answer

1. Save Work as *never scheduled — menu only*.
2. Leave Home with no guardrail at all.
3. Open the free time page.

### Expected Observable Outcomes
- Both report **zero** hours and list no intervals.
- **Neither is an error, and neither is missing from the page.** A life area
  the owner deliberately walled off and one they have not finished setting up
  both have the same free time — none — and the page says so rather than
  omitting them.

## Procedure — overlapping guardrails both report their time

1. Give Work and Learning the identical band, Mon `09:00`–`17:00`.
2. Open the free time page.

### Expected Observable Outcomes
- Both report **16h**, the same hours, independently.
- **This is not double-counting and must not be "fixed".**
  `D-life-area-owns-its-time` makes overlap legal: both life areas genuinely
  offer that time and they compete when something is scheduled into it, which
  is M3's. At M2 they simply both show it.

## Procedure — the spring-forward gap

1. Start the server with `--now 2027-03-13T12:00:00-05:00`.
2. Set the timezone to `America/New_York`.
3. Give Work a band on Sun `01:00`–`04:00`.
4. Open the free time page.

### Expected Observable Outcomes
- Work reports **5h**: three hours on Sunday 21 March, and only **two** on
  Sunday 14 March, because `02:00`–`03:00` does not exist that day.
- **No panic, no negative-length interval, no phantom hour.** A crash here is
  the failure `T-jiff-epoch-millis` chose the library to prevent; a silent
  three-hour answer is worse, because it is wrong in a way nothing complains
  about.
- Repeat with the timezone left at `UTC` and confirm the total is **6h**.
  **This is the first observable consequence of the timezone setting** —
  the same guardrail, the same range, a different answer.

## Procedure — the fall-back fold

1. Start the server with `--now 2027-11-06T12:00:00-04:00`.
2. Set the timezone to `America/New_York`.
3. Give Work a band on Sun `01:00`–`04:00`.
4. Open the free time page.

### Expected Observable Outcomes
- Work reports **7h**: three hours on Sunday 14 November, and **four** on
  Sunday 7 November, because `01:00`–`02:00` happens twice and both passes are
  inside the band.
- A total of 6h means the repeated hour was dropped — the other documented
  choice, and not the one this product made.
- The two intervals covering the repeated hour may carry the **same
  wall-clock label**. That is expected and is the accepted cost of the
  decision; they are distinct instants and must be listed as two.

## Procedure — a hostile life area name stays escaped

1. Add a life area named `<script>alert('boom')</script>` and give it a band.
2. Open the free time page and read the **raw HTML source**.

### Expected Observable Outcomes
- No unescaped `<script>` tag; the word `boom` still present, escaped rather
  than stripped.
- This page is a **new render surface** for life area names. `#47` asserted
  escaping on the management list and the picker, `#59` on the guardrail row;
  each has its own template and could fail independently.

## Procedure — nothing else changed

1. Run the QA suites for `capture_endpoint`, `inbox_view`, `triage_from_page`,
   `committed_triage_validation`, `quota_triage_validation`,
   `unknown_kind_rejection`, `stats_ratio`, `life_areas`, `life_area_triage`,
   `dismiss_capture`, `guardrails`, `timezone_setting`, `task_kinds`,
   `committed_field_domains`, `migrations`, `release_binary` and
   `scheduler_core_purity`.
2. Run `app_shell` **expecting it to have changed** — see below.

### Expected Observable Outcomes
- The first seventeen pass unchanged. This slice computes and renders; it
  writes nothing and there is no migration.
- **`app_shell` is the one that legitimately changed**, and it is the rule
  working rather than drift: `T-nav-is-the-site-map` puts every route-table
  page in the header, so adding a page necessarily changes what the header
  asserts. Its link set is now **Inbox, Life areas, Free time, Stats** and it
  has a fourth per-page scenario. **If `app_shell` did *not* change, the page
  is missing from `nav::ALL`** — which compiles and ships with no link, and is
  the one step the compiler cannot force.
- `scheduler-core` purity still holds: `free_intervals` lives in the core and
  `cargo tree -p scheduler-core` must contain no `tokio` and no `sqlx`.
- `capture-endpoint-persists-quickly-01`'s 50 ms budget is load-sensitive —
  that is `#66`. Re-run quiet and say so.

## Independent of Implementation

This procedure depends only on what the free time page reports for a given
guardrail, range and timezone. It does not depend on which route serves it,
how intervals are represented, whether the projection happens in the core or
the handler, or which library resolves the transitions.
