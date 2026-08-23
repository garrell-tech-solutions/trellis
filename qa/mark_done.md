# QA Procedure: A task you have done leaves the screen it lives on

Covers: `features/mark_done.feature`

## Interface used

The Pool and Committed screens reached by their **tabs**, the capture screen
and its triage controls for setup, and read-only `sqlite3` inspection. No
project library, module, or test helper is used.

**`T-cross-capability-invariants-need-an-owner` bites here, and it is the
easiest mistake to make in this whole document.** **Never mark a task done by
writing the column directly.** A fixture that sets `archived_at` in SQL skips
the production route the screens depend on, and every procedure below would
then pass against an implementation whose control does nothing. **Use the
checkbox, through the page.**

## Why this slice exists

**The capture side has an exit and the task side did not.** #48 gave the
inbox a "no"; once a capture became a task it was permanent.

That is worse than merely missing, because of `T-trips-are-derived-not-ranked`:
**completed items keep counting toward the trip threshold**, so the grouping
that makes Pool worth opening is the first thing to rot. A Pool screen that
only grows is worse after a week than no Pool screen.

## The model, and what is deliberately not built

**One column.** Marking done writes `tasks.archived_at`, which has existed
since `0002` and which nothing has ever written. `T-archived-at-only` stays
literally true — it remains the single archive signal.

**No done-versus-killed discriminator, on purpose.** `T-capture-leaves-inbox-once`
managed with one column because the exit was *derivable* — a triaged capture
has a `tasks` row referencing it and a dismissed one does not. **Here there
is no such row**, so the distinction would have to be *stored* — and **nothing
in Trellis can kill a task today**, so the column could only ever hold one
value. That is the speculative schema this project refuses. When a kill
control arrives it adds the discriminator and backfills every existing row as
`done`, which is provably correct because nothing else could have set it.

**If you find a second timestamp or an outcome column, that is a defect**, not
foresight.

## The canvas does not draw this control

Its complete set of `aria-label`s is `Raise priority` ×3, `Lower priority`
×3, `Minutes`, `Day`, `Save capture`, `Hours a week`, `Delete session`.
**There is no done control anywhere in the design.**

`D-four-screens` makes the canvas authoritative on layout, so **this is a gap
in the design rather than a disagreement with it.** The checkbox at each
row's leading edge was chosen by the owner and is **not drawn**. Say so in
the report, so the design can be corrected rather than quietly diverged from.

**It must not be an arrow.** `pool-screen-nothing-reorders-05` asserts the
absence of every reorder control and its document calls finding one a defect;
#95 owns those. **A checkbox that renders as ▲ or ▼, or a done action wired
to an existing arrow, breaks that assertion.**

## By-hand walkthrough — do this once, in a real browser, on a phone

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Capture and triage three errands tagged `@homedepot`. Tap **Pool** and
   confirm they form a trip — three things, one stop.
3. Tick **two** of them done.
4. Reload. Confirm **`@homedepot` is no longer a trip**: one item left, below
   the threshold, sitting in loose ends **still carrying its tag**.
5. Restart the server. Confirm the two you did have not come back.
6. Confirm there is **no un-do**, and **no list of completed work** anywhere.

### Expected Observable Outcomes
- All six steps hold literally, per `D-visible-slices`.
- **Step 4 is the point of the slice.** Without it the screen keeps sending
  you on a trip you already made.
- **Step 6 is refused by decision, not omitted.** `D-inaction-archives`:
  survival requires a deliberate act, so there is no un-do.
  `D-kill-means-archive`: *"the moment an archive is browsable it becomes a
  place to hide from decisions"* — so no completed list, ever.
- **Check the checkbox is a real tap target at phone width**, and check the
  Committed row hardest. The checkbox goes at the **leading edge on both
  screens** — so a Committed row is four columns wide: checkbox, 66px date,
  text, context tag. **The worst case is a long item beside `BY THU 17:00`
  and a tag**, and it is the tightest thing on the phone. Nobody has seen it.
  If it wraps or clips, say so — the layout is a gap the design never filled,
  so this is the first sighting rather than a regression.

## Setup — repeat below

1. Start against a fresh database.
2. Reach each screen **through its tab**.

## Procedure — a pool task leaves the screen

1. Create two pool tasks, one tagged and one not.
2. Mark the tagged one done **through the checkbox**.
3. Reload the Pool screen. Query the tasks table.

### Expected Observable Outcomes
- It is gone from the screen; the other remains.
- **The row is still in the database** with its `archived_at` set —
  `D-kill-means-archive` keeps the row. A deleted row is a defect.
- Its context tag is **unchanged**. The tag lives on the capture and nothing
  removes it, which is what lets a later reckoning ask what was done at
  `@homedepot`.

## Procedure — the trip that falls apart

1. Create three pool tasks at one tag. Confirm they form a trip.
2. Mark **two** done.
3. Reload.

### Expected Observable Outcomes
- **No trip.** The survivor is in loose ends, **still showing its tag**.
- **This is the case the slice exists for**, and the one a naive
  implementation passes halfway: hiding done items from the list while still
  counting them toward the threshold leaves a trip panel of one, which looks
  fine until you read the number.
- Mark the third done. The screen shows its empty state.

## Procedure — a committed task leaves the screen

1. Create two committed items, one an `at` and one a `by`.
2. Mark one done. Reload.

### Expected Observable Outcomes
- It is gone; the other remains, still rendering its own date cell correctly.
- **A past-dated item can be marked done too** — that is the whole reason the
  Committed screen keeps showing them.

## Procedure — the counts

1. Create two pool tasks and one committed.
2. Mark one of each done.
3. Read the count beside each screen's title.

### Expected Observable Outcomes
- Pool reads `1 waiting`; Committed reads `nothing dated`.
- **A done task is in no count any screen shows.** A count that still
  includes it is the same defect as the trip threshold one, in a place that
  is easier to miss because the list looks right.

## Procedure — quota work is untouched

1. Create a quota task.
2. Look for a done control on it, wherever quota work appears.

### Expected Observable Outcomes
- **There is none, and that is correct.** A quota recurs — it is never
  "done" — so completing it means logging a session, which is #93's.
  `D-logging-is-retrospective-and-separate`: completing **may offer** to log
  time and never does it silently.
- **A done control on a quota item is a defect**, and it would also prejudge
  the slice that follows this one.

## Procedure — nothing offers to undo it

1. Mark a task done.
2. Read the raw HTML of both screens.

### Expected Observable Outcomes
- **No un-do control, no completed list, no link to either** — not hidden,
  not disabled. Absent.
- **No reorder arrow appeared anywhere** while adding this control.
- Try any plausible un-archive route by hand. It should refuse or not exist.

## Procedure — hostile text stays escaped

1. Create two pool tasks, one whose text is `<script>alert('boom')</script>`.
2. Mark the *other* one done and read the **raw body of that response**.

### Expected Observable Outcomes
- No unescaped `<script>`; `boom` still present, escaped rather than
  stripped.
- The response is the re-rendered fragment, so it carries the surviving rows
  — **the hostile one is in the response even though it was not the row you
  acted on.**

## Procedure — nothing else changed

1. Run every existing QA suite.

### Expected Observable Outcomes
- All pass, and **`pool_screen`'s no-reorder procedure in particular**.
- **Believe the re-run.** A restyle has broken QA scripts three times in this
  project without CI noticing, and this slice adds a control to two styled
  screens.

## Independent of Implementation

This procedure depends only on what the two screens render before and after a
task is marked done, and what survives a restart. It does not depend on which
route the control posts to, which column records it, or how the fragment is
templated.
