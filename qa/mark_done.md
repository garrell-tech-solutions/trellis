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

## The model, and what is deliberately not built

**One column**, `tasks.archived_at` (`T-archived-at-only`), and **no
done-versus-killed discriminator** — the reasoning is in
`features/mark_done.feature`'s header. **If you find a second timestamp or an
outcome column, that is a defect**, not foresight.

**The canvas draws no done control** — a gap in the design rather than a
disagreement with it. The checkbox at each row's leading edge was chosen by
the owner and is not drawn anywhere. **Say so in the report**, so the design
can be corrected rather than quietly diverged from.

**It must not be an arrow.** `pool-screen-nothing-reorders-05` asserts the
absence of every reorder control; **a checkbox that renders as ▲ or ▼, or a
done action wired to an existing arrow, breaks that assertion.**

## By-hand walkthrough — do this once, in a real browser, on a phone

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Capture and triage three errands tagged `@homedepot`. Tap **Pool** and
   confirm they form a trip — three things, one stop.
3. Tick **two** of them done.
4. Reload. Confirm **`@homedepot` is still a trip**, showing its progress —
   working a trip does not dissolve it.
5. Restart the server. Confirm the two you did have not come back.
6. Confirm there is **no list of completed work** anywhere.

### Expected Observable Outcomes
- All six steps hold literally, per `D-visible-slices`.
- **⚠️ Step 4 was reversed by #122 and this document was stale until
  2026-08-26.** It used to say the trip dissolved into loose ends once two of
  three were done — true when this slice landed, **false since persistence
  began counting everything displayed** (`D-a-trip-survives-being-tidied`).
  `mark-done-trip-drops-below-three-02` was removed from the feature file for
  exactly this reason and the QA document was not updated with it. **A group
  drops to loose ends only when the owner explicitly clears its done items**,
  which is `trip-progress-clearing-can-drop-a-group-05` and belongs to
  `qa/trip_progress.md`, not here.
- **Step 6 is refused by decision, not omitted** (`D-kill-means-archive`) —
  no completed list, ever.
- **Un-do was narrowed by #122, not refused.** A completed *pool* item now
  stays on screen struck through and unchecking it puts it back;
  `qa/trip_progress.md` owns that. The rule here is the half that still
  holds: **nothing brings back what has left the screen.**
- **Check the checkbox is a real tap target at phone width**, and check the
  Committed row hardest: leading-edge checkbox, 66px date, text, context tag
  is four columns, and **a long item beside `BY THU 17:00` and a tag** is the
  tightest thing on the phone. Nobody has seen it — if it wraps or clips, say
  so; this is a first sighting rather than a regression.

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

## Procedure — a trip you are working through

1. Create three pool tasks at one tag. Confirm they form a trip.
2. Mark **two** done.
3. Reload.

### Expected Observable Outcomes
- **Still a trip**, with the two done items shown struck through and the
  third outstanding. **Working a trip never dissolves it**
  (`D-a-trip-survives-being-tidied`).
- **⚠️ This procedure asserted the opposite until 2026-08-26** — that the trip
  fell apart into loose ends. **#122 reversed it and this document was not
  updated**, so QA running it would have reported a defect that is now the
  correct behaviour. Recorded rather than quietly rewritten.
- Mark the third done. **The trip is still there**, all three struck through.
- **Clearing the done items is the only thing that drops the group**, and that
  is `qa/trip_progress.md`'s to assert, not this document's.

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

## Procedure — nothing lists completed work

1. Mark a task done.
2. Read the raw HTML of both screens.

### Expected Observable Outcomes
- **No completed list and no link to one** — not hidden, not disabled.
  Absent. (Unchecking a *visible* struck pool item is a different thing and is
  `qa/trip_progress.md`'s.)
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
