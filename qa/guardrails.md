# QA Procedure: Each life area carries its own weekly guardrail

Covers: `features/guardrails.feature`

## Interface used

The life areas page at `GET /life-areas`, `curl` against whatever endpoints
its own guardrail controls submit to, and read-only `sqlite3` inspection of
persisted state. No project library, module, or test helper is used.

**Read every endpoint out of the page's markup.** As with every page in this
product, the routes are not part of the contract this procedure tests.

## What this slice does and does not mean

**A guardrail says where a life area's work may be scheduled. It says nothing
about what kinds of task the life area holds.** Work has a guardrail *and*
holds pool tasks — pool is the default kind (`D-pool-is-default`) and a life
area is required at triage for all three kinds. The guardrail is the wall its
work is scheduled inside, pool work included; that is the reservation half of
`D-life-area-owns-its-time`, and the reason *"a Fitness guardrail nothing is
scheduled into is a wall protecting an empty room"*.

**"Pool-only" is a property of the life area, not a restriction on kinds.** It
means the life area has no hours at all, so its work is never placed and only
ever offered by the menu. **The page does not use the phrase "pool-only"** --
it reads **"never scheduled - menu only"**, precisely because "pool-only"
reads as "only holds pool tasks" and misled a reader the first time it was
said aloud. `pool-only` remains the concept's name in `docs/decisions.md`;
the two are the same thing. **If any procedure below tempts you to check that a
never-scheduled life area rejects committed tasks, stop — that is not what it
means, and nothing in this slice changes triage.**

**Nothing here converts a band to an instant.** Bands are civil wall-clock;
projecting them onto a calendar is `#60`. So no procedure below asserts what
`09:00` means as a moment in time, and none should.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Click **Life areas** in the header. Confirm each of the five seeded life
   areas shows **"no guardrail"**.
3. On **Work**, tick Mon–Fri, enter `09:00` and `17:00`, and Save. Confirm the
   band appears on Work's row, listed as you authored it.
4. On **Fitness**, save `Mon/Wed/Fri 06:00–07:00`, then save `Sat 09:00–11:00`.
   Confirm **both** bands are listed — one life area, two bands.
5. Add a life area **Side project** and tick **"never scheduled - menu only"**,
   then Save. Confirm it is accepted with no hours at all, and that its row
   says so rather than saying "no guardrail" -- *deliberately has no hours*
   and *not finished being set up* must not look the same.
6. On **Home**, Save with neither days-and-times nor the never-scheduled
   mark.
   Confirm it is **refused**, the message lands on Home's own row, and the
   message names both of the things that would satisfy it.
7. Restart the server and reload. Confirm everything from steps 3–5 is still
   there.

### Expected Observable Outcomes
- All seven steps hold literally, per `D-visible-slices`.
- **Step 6 is the one worth the trip.** `T-life-areas-are-data` has said since
  2026-08-14 that a life area is well-formed only once it has a guardrail or
  is explicitly marked pool-only, *"enforced at M2, when guardrails exist"*.
  This is the moment that stops being a promise.
- Step 4 is the model itself: **multiple bands per life area, multiple days
  per band**. A page that accepts only one band per life area has implemented
  something else.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable and the five seeded life areas are listed.

## Procedure — a fresh database has no guardrails

1. `GET /life-areas` and read each life area's row.

### Expected Observable Outcomes
- All five show **"no guardrail"** — not a blank, not a zero, not an empty
  table. The seeded life areas were created before guardrails existed and
  none has one.
- **This state is legal.** A life area with no guardrail is *unfinished*, not
  invalid; nothing refuses it until the owner saves it. That is why the
  refusal in the next-but-one procedure attaches to a Save and not to the
  mere existence of a bare life area.

## Procedure — a band is stored as authored and read back the same way

1. Save a band on Work for Mon–Fri, `09:00` to `17:00`.
2. `GET /life-areas` and read Work's row.
3. Query the database for what was written.

### Expected Observable Outcomes
- The page lists the band naming the same days and the same times that were
  submitted. **What you typed is what you see.**
- The stored times are **civil wall-clock** — `09:00` as a time of day, not an
  epoch instant and not a UTC offset baked in at save time. Read the column
  types off the schema (`PRAGMA table_info`); do not assume names.
- This is the check `T-jiff-epoch-millis` asks for and the one `#60` depends
  on. **A band stored as an instant looks identical on the page today and
  leaves the next slice with nothing to fix** — it would already have silently
  chosen a timezone, on the day the product had none.

## Procedure — saving with neither a band nor the never-scheduled mark is refused

1. Save Work with no days, no times, and the never-scheduled mark unticked.
2. Observe the status and the response body.
3. `GET /life-areas` and read Work's row.

### Expected Observable Outcomes
- Rejected with **`422`**, and the body is the re-rendered life-areas fragment
  (`T-422-is-product-wide`).
- The message lands **on Work's own row**, not above the list and not on
  another row.
- The message names **both** ways to satisfy it — a guardrail band, or the
  never-scheduled mark. Naming only one tells the owner half the rule.
- Work still shows "no guardrail". A rejected save writes nothing.

## Procedure — a life area's own bands may not overlap

1. Save a band on Work for Mon `09:00`–`12:00`.
2. Save a second band on Work for Mon `11:00`–`17:00`.
3. Observe the status, and count Work's bands.
4. Save a third band on Work for Mon `12:00`–`17:00`.
5. Count Work's bands again.

### Expected Observable Outcomes
- Step 2 is **rejected**, and the message says the band overlaps one the life
  area already has. Work still has exactly one band.
- Step 4 is **accepted** — bands that touch do not overlap. Work now has two.
- **The overlap is rejected, not merged.** Merging would silently rewrite what
  the owner typed into something they did not, in a product whose entire value
  is that the user trusts what it shows. A rejection names the conflict and
  leaves the fix to the person who knows which band they meant.
- **This is within one life area only.** Two different life areas claiming the
  same hours is explicitly legal — see the next procedure.

## Procedure — two life areas may claim the same hours

1. Save a band on Work for Mon `09:00`–`17:00`.
2. Save the identical band on Learning.
3. `GET /life-areas` and read both rows.

### Expected Observable Outcomes
- Both are accepted and both are listed. **Overlap between life areas is not
  an error**, and a build that rejects it has implemented the strict partition
  `D-life-area-owns-its-time` explicitly rejected.
- Nothing here resolves the competition between them. Which life area wins a
  contested hour is the scheduler's, at M3; this slice only records the claim.

## Procedure — a never-scheduled life area is well-formed with no hours

1. Save Side project with the "never scheduled - menu only" mark ticked.
2. `GET /life-areas` and read its row.
3. Query the database.

### Expected Observable Outcomes
- Accepted, its row reads "never scheduled - menu only", and it carries
  **zero** bands.
- The mark is stored **explicitly**, as its own signal — not
  inferred from having no bands. The two states are different: *deliberately
  has no hours* and *not finished being set up*, and the refusal procedure
  above only exists because the product can tell them apart.

## Procedure — a band can be removed

1. Save a band on Work for Mon `09:00`–`17:00`.
2. Remove it from Work's row.
3. `GET /life-areas` and read Work's row.

### Expected Observable Outcomes
- The band is gone and Work shows "no guardrail" again.
- **Removing the last band is allowed**, and returns the life area to the same
  unfinished-but-legal state the seeded five start in. Refusing it would trap
  the owner: to replace a wrong band they would have to add the right one
  first, inside a rule that forbids overlaps.

## Procedure — guardrails survive a restart, and archiving keeps them

1. Save bands on Work and on Fitness, and save Side project as never
   scheduled.
2. Archive **Fitness**.
3. Stop the server. Restart it against the **same** database.
4. `GET /life-areas`.
5. Query the database for every guardrail band and its life area.

### Expected Observable Outcomes
- Work's bands and Side project's never-scheduled mark are still listed after
  the restart.
- **Fitness's bands are still in the database**, even though Fitness is
  archived and therefore appears nowhere on the page. `D-kill-means-archive`
  keeps the row, and that has to mean the row and what hangs off it.
- **This is the one acceptance criterion the page cannot show**, because
  `list_active` filters archived life areas out of the management list and
  there is no un-archive. The database is the only witness, which is why this
  is a QA procedure and not an acceptance scenario.

## Procedure — hostile text stays escaped

1. Submit a guardrail save whose time field carries
   `<script>alert('boom')</script>`.
2. Read the **raw HTML source** of the response.

### Expected Observable Outcomes
- Rejected, and the raw response contains no unescaped `<script>` tag.
- If the message echoes what was submitted, the word `boom` survives escaped
  rather than stripped.
- The day fields are a closed set of seven checkboxes and cannot carry text at
  all, which is the point of choosing them over a parsed free-text band —
  the same reasoning `triage_from_page` used for deadline type and priority.

## Procedure — nothing else changed

1. Run the existing QA suites for `capture_endpoint`, `inbox_view`,
   `triage_from_page`, `committed_triage_validation`,
   `quota_triage_validation`, `unknown_kind_rejection`, `stats_ratio`,
   `life_areas`, `life_area_triage`, `dismiss_capture`, `app_shell`,
   `task_kinds`, `committed_field_domains`, `migrations`, `release_binary`
   and `scheduler_core_purity`.

### Expected Observable Outcomes
- All sixteen pass unchanged. In particular **triage is untouched**: a life
  area with no guardrail still accepts tasks of every kind, exactly as before.
  Nothing in this slice makes well-formedness a triage-time rule, and a build
  that does will fail `life_area_triage` on every seeded life area.
- `capture-endpoint-persists-quickly-01`'s 50 ms budget is load-sensitive —
  that is `#66`, not a regression. Re-run quiet and say so.

## Independent of Implementation

This procedure depends only on what the life areas page renders, what its
guardrail controls do when submitted, and the durable state left behind. It
does not depend on which routes those controls submit to, whether a band is
one row or several, how the days of a band are encoded, or whether
well-formedness is decided in `scheduler-core` or at the boundary.
