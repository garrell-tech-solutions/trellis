# QA Procedure: A trip survives being tidied

Covers: `features/trip_persistence.feature`, the two scenarios that changed
in `features/trip_progress.feature`, and the half of #129 no acceptance
scenario can hold — **that the rule survives a reload and a restart.**

## Interface used

The Pool screen over HTTP, `sqlite3` **read-only** for corroboration, **a
real browser** driven headless at 390×844 (the `playwright-core` and Chrome
`scripts/qa/trip_controls.cjs` already resolves), and **a real phone.**
Bind by role and accessible name, never by class
(`T-qa-binds-tolerantly-to-markup`).

**Go through the routes, never the database.** Mark done with
`POST /pool/tasks/{id}/done`, undo with `.../undone`, clear with
`POST /pool/trips/{tag}/clear`. **A fixture that writes `cleared_at` — or a
run marker, if the implementation bought one — directly proves nothing**,
because this slice is entirely about what those routes leave behind.

## The rule, in the words the screen has to obey

> **A RUN** at a tag begins when something lands there with nothing else
> waiting, and ends when **the last thing waiting there is cleared away.**
> **FORMATION:** a run of three or more things is a trip.
> **PERSISTENCE:** it stays one for as long as the run lasts.

Two consequences to hold in mind while you look at the screen, both argued in
`features/trip_persistence.feature`'s header:

- **A tag must re-earn its trip.** Three things cleared last month plus one
  captured today is **not** a trip.
- **The panel leaves on a tap, never on a tick.** Tick the last open thing in
  a trip that clearing has taken down to two and **the panel holds, reading
  "2 of 2 done"**, with `✕` still on it. It goes when you tap `✕`. **A panel
  that lingers looks like a bug if you do not know that was chosen.**

## Two scenarios changed, and they were green

`trip-progress-clearing-can-drop-a-group-05` **asserted the defect** and is
reversed, not deleted and not weakened; it is now
`trip-progress-clearing-holds-the-group-05`.
`trip-progress-clear-done-04` is the consequence: its two survivors used to
land in loose ends and now stay in the panel.

**Both were observed red against the pre-slice code before the coder
started** (`T-a-check-must-be-seen-to-fail`): `-05` failed with *"expected
trips [@homedepot], got []"* and `-04` with *"expected 2 open item(s) in the
trip"*. **Confirm the handoff commit says so**; if the reversal arrived
already green, it proves nothing.

## Procedure — the walk the slice exists for

1. Five tasks at one tag. Confirm a trip reading **5 things**.
2. Mark three done through the route. Confirm **3 of 5 done**, three struck
   in place, two open, **loose ends empty.**
3. **Clear done.** Read the screen.
4. Mark the remaining two done. Read the screen.
5. **Clear done again.** Read the screen.

### Expected Observable Outcomes
- After 3: **still a trip**, still `@homedepot`, reading **2 things**, two
  open items, nothing struck, **loose ends still empty.** This is the defect
  fixed — before this slice, two items dropped out of the panel here.
- After 4: **still a trip**, reading **2 of 2 done**, no open items, two
  struck, and **`✕ Clear done` still offered.** The complete-group control is
  gone — there is nothing left to complete.
- After 5: **the tag is gone from the screen entirely** — no empty panel, no
  heading with nothing under it, and it is not mentioned in loose ends.
- **Loose ends must be empty at every step.** A single item appearing there
  is the original defect in miniature.

## Procedure — a tag that never reached three

1. Three tasks at `@homedepot` **and two at `@supermarket`**, on one screen.
2. Mark one done at each tag. Clear done at each tag.

### Expected Observable Outcomes
- `@homedepot` **holds as a trip at two items**, reading **2 things**.
- `@supermarket`'s survivor is **a loose end carrying its tag**, and
  `@supermarket` **is not a trip.**
- **The two tags are the same size and behave differently**, and that is the
  whole rule: persistence is something a tag earns by having formed a trip,
  not something clearing confers.

## Procedure — a tag must re-earn its trip

1. Three tasks at one tag; mark **all three** done; clear done. Confirm the
   tag is gone.
2. Capture and triage **two** more at that same tag. Read the screen.
3. Capture and triage **a third.** Read the screen.

### Expected Observable Outcomes
- After 2: **two loose ends, no trip.** The three cleared ones do not count
  towards anything.
- After 3: **a trip again, reading `3 things`** — not `3 of 6 done` and not
  `6 things`. **If the label mentions the cleared three, the run boundary is
  not being applied**, and that is #129's runaway arriving through the fix
  for it.
- **This procedure passes against the pre-slice code**, and that is not a
  reason to skip it — see the breakage list, where it is the only assertion
  one specific wrong answer trips.

## Procedure — it survives a reload, and a restart

**This is why the suite exists at this tier.** Every acceptance step is a
fresh HTTP request, so the Gherkin already proves the rule is not per-render
state. It cannot see either of these.

1. Reach the state after step 3 of the first procedure — a trip of two, no
   struck items, mid-run.
2. **Hard-reload the page** in the browser. Read it.
3. **Stop the server, start it again**, load Pool. Read it.
4. Do the same at the state after step 4 — **2 of 2 done**, nothing open.

### Expected Observable Outcomes
- **Identical in all four readings.** Same trip, same label, same items.
- **A trip that dissolves when you lock your phone in the car park is this
  slice's defect one gesture further out**, and a restart is the only way to
  see run state held in process memory.
- **If the implementation bought a column for this, it is legitimate** —
  `T-ephemeral-view-state-rides-the-request` is the test, and this state
  fails it the way `cleared_at` did rather than the way `shown_kind` did.
  **The report must say whether the pull-request body argues it** and whether
  a derivation was attempted first.
- **If it did buy one, `T-migrations-append-only` means it is permanent.**
  Confirm the migration is additive and that no existing migration file
  changed.

## Procedure — the controls still agree with each other

`#125`'s complete-group button and `✕ Clear done` both act on a panel whose
lifetime this slice changed.

1. Five at one tag, three done, cleared — a trip of two, mid-run.
2. Read the complete-group control's label.
3. Tap it once. Read the screen.
4. **Untick one.** Read the screen.
5. Tap `✕`. Read the screen.
6. Tick the last item, then tap `✕` again. Read the screen.

### Expected Observable Outcomes
- Step 2: it reads **"Complete all 2"** — the open items, not the run's size
  and not the five it started as.
- Step 3: **the panel holds**, reading **2 of 2 done**, and the
  complete-group control is **gone.**
- Step 4: **the item comes back**, the label reads **1 of 2 done**, and the
  trip is still there — **the tag went from nothing-open back to one-open
  without re-earning three**, because the run never ended. **That is the edge
  neither issue mentions**; it works only because the panel stayed.
- Step 5: **the panel is still there, reading `1 things`.** The clear swept
  the one struck item and **the item you unticked is still waiting**, so the
  run has not ended. *(The label reads `1 things` rather than `1 thing`; that
  wording predates this slice and is not its to fix.)*
- Step 6: **now it is gone entirely** — the last thing waiting has been
  cleared, so the run ends.
- **Steps 5 and 6 are the rule stated twice**, and the pair is worth keeping:
  a clear that leaves something waiting is a tidy, and a clear that leaves
  nothing waiting is the end of a run.
- **`trip-controls-nothing-left-to-complete-03` asserts a trip with nothing
  open offers no complete-group control.** Confirm the two rules do not
  disagree about which state the panel is in between the completion and the
  render.

## Procedure — prove the checks can fail

**`T-a-check-must-be-seen-to-fail`.** Six breakages. **Each is paired with
the assertion it must trip, and the pairing is the point** — confirm the
named assertion is the one that goes red, not merely that something did.

1. **Restore the single predicate** — trip-ness from the uncleared count on
   every render. → `trip-progress-clearing-holds-the-group-05`, and
   `trip-persistence` 01, 02, 04 and 05, fail with *"expected trips
   [@homedepot], got []"*. **This is the defect itself.**
2. **Implement the naive derivation** — a trip if it has any open item and
   has *ever* reached three, counting cleared ones. →
   **`trip-persistence-re-earns-its-trip-03` fails and nothing else does.**
   **This breakage is that scenario's entire reason for existing**: 03 is
   green against the pre-slice code, so this is the only way to see it work.
   **Do this one.**
3. **Make the panel leave the moment nothing is open.** →
   `trip-persistence-...-01` fails at *"2 of 2 done"*. **Then check whether
   `trip-progress-fully-done-06` failed too, and report which**: if it did
   not, the implementation has two different answers for two-item and
   three-item trips, which is exactly the inconsistency the owner's decision
   rejected.
4. **Make formation count open items with no memory of the run** — an
   over-reading of *"formation requires three open items"*. →
   `trip-progress-fully-done-06` and
   `trip-controls-nothing-left-to-complete-03` fail: a fully-worked trip
   loses its panel before it is cleared.
5. **Let any tag with something cleared persist**, without requiring that it
   formed. → `trip-persistence-never-formed-stays-loose-02` fails:
   `@supermarket` appears as a trip of one.
6. **Hold the run in process memory.** → **every acceptance scenario still
   passes**, the reload still passes, and **only the restart reading fails.**
   This is the one no other tier can see, and it is the reason this document
   has a restart in it.

### Expected Observable Outcomes
- **Six breakages, six distinct failures, then restore and a clean pass with
  a clean `git status`.** Say which you ran.
- **Breakage 2 and breakage 6 are the two to do if you do only two.** They
  are the two wrong answers a reasonable coder actually reaches for, and
  neither is visible without deliberately causing it.

## By-hand walkthrough — on a real phone

**Label the pull request `preview`.**

1. Five things tagged `@homedepot`. **It is a trip.**
2. **Check three off.** The panel holds — that is #127, already true.
3. **Clear them.** **It holds**: still `@homedepot`, still a panel, two
   things left to get.
4. **Get the last two.** It reads **2 of 2 done** and it is still there.
5. **Tap `✕`.** *Now* it goes — nothing open left, nothing to stand in a shop
   for.
6. **Reload between every step.** It behaves identically.

### Expected Observable Outcomes
- All six hold. **Step 3 is the slice.**
- **Steps 4 and 5 are the owner's 2026-08-25 decision made visible**, and
  they are where the brief's own demo said the panel would go at step 4.
  **Report how the extra tap feels** — whether the lingering "2 of 2 done"
  panel reads as *finished and tidy-able* or as *stale*. That is a judgement
  only a person standing in a shop can make.
- **PR #137 re-did the palette and typeface.** The panel will not look like
  the screenshots in #122, #127 or #135 — expected, not a finding. What *is*
  a finding is a control that took a hardcoded colour rather than a token.
- **If any part of the answer is client-side**, `scripts/qa/trip_controls.cjs`
  drives real Chrome under the CI `gate` job and is the tier that can see it.
  Say whether it needed extending.

## Procedure — nothing else changed

1. Run the acceptance suite and every QA suite.
2. Run `scripts/analyzers/dry.sh` and **say what it measured.**

### Expected Observable Outcomes
- **All 24 acceptance features pass**, of which `trip_persistence.feature` is
  new and **`trip_progress.feature` is the one edited** — scenarios 04 and 05
  only. **If any other feature needed editing, the change leaked**; say which
  and why rather than editing it.
- **Every `trip_controls` scenario still holds** (#135), untouched.
- **`trip_persistence.feature` needed no new step handler** — every step in
  it already existed in `pool_screen`, `trip_progress`, `trip_controls` and
  `mark_done`. **If a step module was added, say why**; a per-feature step
  module is the single thing most likely to move the DRY number.
- **DRY has real headroom now** (`T-dry-measures-product-code`; the last two
  slices measured 1.86% and 1.81% against 3%). **Report the number and the
  formats**, not pass or fail.
- **#108 is open on `list_pool_tasks` and is not this slice's.** The query
  almost certainly changed to answer the run question. **Say whether what
  landed makes #108 easier or harder**, and flag it if grouping or
  per-group limits moved *out* of the store — that is the thing #108 exists
  to fix, not to worsen.
- **A run is identified by a tag, so it inherits that tag's canonicalisation**
  (`T-cross-capability-invariants-need-an-owner`). `pool::group` buckets by
  plain string equality, correct **only** because `capture::resolve_tag`
  folds case on the way in. **Check it rather than assuming it**: capture
  three at `@HomeDepot`, clear two, capture one more at `@HOMEDEPOT`, and
  confirm it is **one run and one panel** — not two runs that each fall
  below three. `pool-screen-case-folded-grouping-03` asserts the grouping
  half; nothing yet asserts the run half.
- **#95 is not in this slice.** `T-trips-are-derived-not-ranked` still
  forbids a reorder control on a trip and
  `pool-screen-nothing-reorders-05` still asserts that absence.

## Independent of Implementation

This procedure depends only on what the Pool screen shows as items are
checked off, cleared, completed, unchecked and captured afresh, and on what
it shows after a reload and a restart. It does not depend on how a run is
recognised, on whether a column was bought, on where the threshold is tested,
or on how the label is composed — only that a trip holds together for as long
as its run lasts, that it leaves on a tap rather than a tick, and that a tag
whose run has ended must reach three again.
