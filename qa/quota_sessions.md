# QA Procedure: Logging hours against a quota

Covers: `features/quota_sessions.feature`, and the two halves no acceptance
scenario can hold — **the expand and `Other…` disclosures**, and **a real week
boundary.**

**What is not acceptable is a half-built session surface**: a `+30m` that
writes nothing, a `This week` that cannot delete, or a Monday reset that is a
`TODO`. **If you find one, that is the finding**, and it outranks everything
else in this document. **Nor is a green reached by weakening, deleting or
parking the feature file** — the only acceptable green is nine scenarios
passing against a real implementation.

## Interface used

The Quota screen over HTTP, **the `--now` flag** to start a server believing it
is another instant, `sqlite3` **read-only**, **a real browser** at 390×844, and
**a real phone.**

`--now <RFC3339>` is a **user-interface affordance**, not a test hook: the
server starts believing it is that instant and **time advances normally from
there** (`platform/clock.rs`). It is how a week boundary is reachable without
waiting until Monday.

**Log every session through the controls.** A row inserted with `sqlite3`
proves nothing about `+30m`.

## The two rules this suite exists to check

`D-logging-is-retrospective-and-separate` — the day picker offers only days
that have already happened. `D-quota-no-rollover` — counters reset Monday and
shortfalls never carry forward. Both, and the consequence the owner was shown
and kept (you cannot log Sunday evening's practice on Monday morning), are
argued in `features/quota_sessions.feature`'s header. **Look for that
consequence in real use and report how it feels** — it is the most likely thing
here to be found wrong within an hour of merging, the way #129 was.

**"Monday" needs a timezone**, and the path is `settings::current_timezone` +
`scheduler_core::timezone::resolve`, exactly as `committed/body.rs` does it
(`T-timezone-is-a-setting`). **Check it took that path and did not reach for
UTC.** ⚠️ **#118** is open — `/timezone` is `POST`-only and reachable from no
page, the live database reads `America/New_York` but the schema default is
`UTC` — so **this slice makes a second capability depend on a setting nobody
can edit.** Fixing #118 is not in scope; **saying so in the report is.**

## Procedure — the logging loop

1. Seed `Piano`, 4 hours. Server believing it is **Tuesday**.
2. Tap **`+1h`**. Read the row.
3. Tap **`+30m`**. Read the row.
4. Open **`Other…`**, pick **Monday**, enter **20** minutes, **Add**.
5. Expand the row. Read `This week`.

### Expected Observable Outcomes
- Step 2: **`1h / 4h`**, note **`3h left this week · 25%`**, and the bar
  visibly moved.
- Step 3: **`1h 30m / 4h`**.
- Step 4: **`1h 50m / 4h`**, and the `Other…` panel closed itself.
- Step 5: three sessions, **ordered Monday first** — Mon 20m, then Tue 60m and
  Tue 30m — summarising **`3 sessions · 1h 50m`**.
- **Corroborate in `sqlite3`**: three rows, each carrying enough to know which
  day of which week it belongs to. **A row storing only `"Mon"` cannot survive
  a week boundary** — if that is what you find, go straight to the Monday
  procedure, because it will fail there.

## Procedure — the day picker shrinks to the week so far

1. Start servers believing it is **Monday**, **Tuesday**, **Friday** and
   **Sunday** in turn, each with the same seeded quota.
2. Open `Other…` on each and read the day list.

### Expected Observable Outcomes
- **Mon** → `Mon`. **Tue** → `Mon, Tue`. **Fri** → `Mon…Fri`. **Sun** → all
  seven.
- **Today is the default selection** every time.
- **There is no way to reach a future day** — not by the picker, and **not by
  posting one directly.** Try it: submit a session for Saturday from a server
  that believes it is Tuesday, and confirm it is refused rather than merely
  un-offered. **A guard that only exists in the dropdown is not a guard.**

## Procedure — correcting and deleting

1. Seed Tuesday. Log **25m on Monday** and **35m on Tuesday**.
2. Change the Monday session to **Tuesday**. Read the total.
3. Change it to **45** minutes. Read the total.
4. **Delete** it. Read the total and the list.
5. Delete the last one. Read the row.

### Expected Observable Outcomes
- The readout tracks every change: `1h / 4h` → `1h / 4h` (day only) →
  `1h 20m / 4h` → `35m / 4h` → `0m / 4h`.
- Step 5 leaves **`No sessions yet this week. Log one above when you have done
  it.`** and the summary **`nothing logged`** — and **the quota itself is still
  there.** Deleting every session must not delete the quota.
- **All three routes go through one front door**
  (`T-one-front-door-per-capability`). Proxy for it: log 20 sessions and delete
  them one at a time, and confirm nothing degrades in a way that suggests a
  second write path or a per-row round trip that should have been one statement
  (`T-set-operations-execute-in-the-store`).

## Procedure — Monday starts again at zero

**The half no acceptance scenario can hold in a real deployment.**

1. Server believing it is **Tuesday**. Log 25m Monday, 35m Tuesday. Confirm
   **`1h / 4h`**.
2. **Stop the server. Start it again with `--now` set to the following
   Monday**, same database file.
3. Read the Quota screen.
4. Log **30m** for that Monday. Read the row.

### Expected Observable Outcomes
- Step 3: **`0m / 4h`**, the note back to `4h left this week · 0%`, and `This
  week` saying there is nothing logged. **The quota, its name and its target
  survive; only the week resets.**
- **Last week's sessions are still in the database.** Confirm in `sqlite3` —
  `D-quota-no-rollover` says the counter does not carry, **not** that the
  history is destroyed. **If the reset deleted rows, that is a finding**:
  `D-kill-means-archive`'s concern on a new surface, and it makes the reset
  irreversible.
- Step 4: **`30m / 4h`** — the new week counts normally.
- **Then try to log Sunday's session on that Monday.** Confirm the picker does
  not offer last week and the total does not move. **Report how this felt.**

## Procedure — the disclosures, in a browser

**Which row is expanded, and whether its `Other…` panel is open, are exactly
the state that must not buy a column**
(`T-ephemeral-view-state-rides-the-request`); `scripts/qa/trip_controls.cjs`
step 10 is the pattern, CI-gated under a no-skip contract, and anything added
here must be too.

1. Seed three quotas, each with sessions. Load Quota at 390×844.
2. **Expand one.** The others stay collapsed.
3. **Open `Other…` on a different row.** Confirm what happens to the first.
4. **With a row expanded, log a session in it.** ⚠️ **The row must still be
   expanded afterwards, and the session you just logged must be on screen.**
5. **Reload.**
6. **Check the schema.**

### Expected Observable Outcomes
- **Step 4 is the one this slice is most likely to get wrong.** If the fragment
  swap collapses the row, the panel closes under your thumb and takes the
  session you just logged off the screen with it — **the failure
  `D-a-trip-survives-being-worked` was written against**, arriving on a third
  screen through a third door.
- Step 5: a fresh load starting collapsed is **correct and expected**, the same
  as the pool's expand state.
- Step 6: **no column holds which row is expanded, or whether `Other…` is
  open.** If one does, **that is the finding and it goes first in the report** —
  `T-migrations-append-only` means it can never be taken back.
- **A logged session is the opposite and earns its table**
  (`D-a-trip-survives-being-tidied`: storage permitted is not storage
  required). Report which way each question went and whether the pull-request
  body argues it.

## Procedure — prove the checks can fail

**`T-a-check-must-be-seen-to-fail`.** Each breakage is paired with the
assertion it must trip; confirm the named one goes red.

1. **Store a session as a bare weekday with no week.** → the **Monday
   procedure** fails: last week's hours reappear in the new week. **Every
   acceptance scenario in `-01` to `-07` still passes**, because none of them
   crosses a boundary. **This is the breakage that justifies `--now` being in
   this document.**
2. **Offer all seven days.** → `quota-sessions-only-days-that-have-happened-03`
   fails on the Monday, Tuesday and Friday rows, and the Sunday row still
   passes. **Read that**: the Sunday row passing is why it is in the table.
3. **Accept a future day posted directly while still hiding it in the
   picker.** → **no acceptance scenario fails**; only the direct-post step in
   the picker procedure does. **This one is invisible to the Gherkin.**
4. **Reset by deleting last week's rows.** → the acceptance suite passes
   entirely; only the `sqlite3` corroboration in the Monday procedure fails.
5. **Make the fragment swap collapse the expanded row.** → nothing outside the
   browser check moves.
6. **Accept a zero-minute session.** →
   `quota-sessions-a-session-must-be-positive-09` fails.
7. **Move the Chrome binary.** → the browser check **fails** rather than skips,
   same contract as `qa/trip_controls.md`.

### Expected Observable Outcomes
- **Seven breakages, seven messages, then restore and a clean pass with a clean
  `git status`.** Say which you ran.
- **Breakages 1, 3 and 4 are the ones to do if you do only three.** All three
  leave the entire acceptance suite green, which is precisely why this document
  exists.
- **⚠️ These nine scenarios were written before an implementation existed** and
  have only ever been red on "no route yet", which is not the same thing
  (`T-a-check-must-be-seen-to-fail`). **Going red-to-green is not evidence.**
  Every breakage above must be run once the feature works.
- **⚠️ Do not read the mutation manifest as evidence either.** **#145**: a
  scenario with a surviving mutant is dropped from the manifest rather than
  recorded, so every checked-in manifest reads 100% killed whether or not it
  is. **Report what the tool printed on the run**, not what the file says
  afterwards — and do not hand-edit it.

## By-hand walkthrough — on a real phone

**Label the pull request `preview`.**

1. **Play piano for an hour. Tap `+1h`.** The bar moves; the readout says one
   hour against four.
2. **Tap `+30m`** after a short session.
3. **Use `Other`** to log the 20 minutes you did on Sunday — *or find that you
   cannot, because Sunday has not happened yet this week.* **Say which, and
   whether the picker made that obvious or just felt broken.**
4. **Expand the quota.** `This week` lists what you logged. **Fix the day you
   got wrong. Delete the one you did not actually do.**
5. **Reload after every step.** Everything sticks.
6. **Come back on Monday** — or start the server believing it is Monday. **It
   reads zero against four again.**

### Expected Observable Outcomes
- All six hold. **That loop — define, log, correct — is the slice. If any step
  needs a `curl`, it is not done.**
- **Step 3 is worth reporting even if it passes.** The brief's demo said "log
  the 20 minutes you did on Sunday", and under a Monday-to-Sunday week that day
  is usually still ahead of you. **The demo and the retrospective rule pull in
  opposite directions**, and which one feels right with a phone in your hand is
  a judgement only a person can make.
- **#137 re-did the palette and typeface** — this screen has never been seen in
  the new one. Report how the 6px progress track and the tabular readout look
  before anything about behaviour.

## Procedure — nothing else changed

1. Run the acceptance suite and every QA suite.
2. Run `scripts/analyzers/dry.sh` and **say what it measured.**

### Expected Observable Outcomes
- **All 26 acceptance features pass, `quota_sessions` included.** `trunk` has
  been red on exactly this one feature and green on the other 25 since
  `bf8c7f9`; **if a 27th thing is failing, it is new and it is yours** — do not
  fold it into the known red.
- **The language-mutation run returns with this slice**, having been
  unavailable for a whole slice. **Expect it to have something to say**, and
  report it rather than the manifest (**#145**).
- **`/quota` joins `colour.cjs` and `phone_layout.cjs`'s `SCREENS` lists.**
  Both had a hardcoded three, so the fourth screen has never been measured for
  palette, dark mode, contrast, tap targets or overflow. **If adding it turns
  either gate red, that is the point** — report what it found rather than
  adjusting the screen to suit the check. **The quota row is the densest header
  the product has**, so phone layout is the likelier of the two to bite.
- **Confirm `/quota` links the same manifest the other three do.** A screen
  that does not is a real defect for an installable app and nothing currently
  checks it. Report it; do not fix it here.
- **DRY: report the number and the formats.** #144 measured 2.05% against a 3%
  product-code threshold. **A second step module for the same screen is the
  thing most likely to cross it** — say whether a shared family was extracted
  early or bolted on at the end.
- **#118 is not fixed here and is now load-bearing twice**, so a fresh install
  gets a Monday that is not the owner's Monday. **Say so in the report.**

## Independent of Implementation

This procedure depends only on what the Quota screen shows as sessions are
logged, corrected and deleted, on what it offers in its day picker, and on what
it shows after a reload and across a week boundary. It does not depend on how a
session is stored, how the week is computed, what the routes are called, or how
the disclosures are held — only that logging is one tap from the list, that you
cannot record time you have not done, that corrections and deletions stick, and
that Monday starts again at zero without destroying what came before.
