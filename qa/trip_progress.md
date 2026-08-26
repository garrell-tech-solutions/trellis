# QA Procedure: A trip survives being worked

Covers: `features/trip_progress.feature`

## Interface used

The Pool screen and its checkboxes, over HTTP, plus `sqlite3` **read-only**.
Bind by role and accessible name, never by class
(`T-qa-binds-tolerantly-to-markup`).

**Mark every task done through `POST /pool/tasks/{id}/done`.** A fixture that
writes `archived_at` directly proves nothing here — **this slice is entirely
about what that route leaves behind**, so the shortcut is both more tempting
and more wrong than usual (`T-cross-capability-invariants-need-an-owner`).

## Two rules that are not the same rule

- **Formation** needs **three open items.** Completed ones never conjure a
  trip.
- **Persistence** counts **everything displayed.** Working a trip never
  dissolves it.

**If those are ever collapsed into one predicate, one of the two behaviours
is wrong.** #129 replaced "displayed" — clearing is precisely what stops
something being displayed — with a rule stated over the *run*, written out in
`qa/trip_persistence.md`. Everything below still holds; only the clearing
procedure moved.

## Nothing clears itself

**There is no time window and no automatic sweep.** A struck item is a record
of what you did until you tap the clear control. **If you find items
disappearing on their own — overnight, on reload, on revisiting the screen —
that is a defect**, and specifically the option the owner rejected.

**The control is a bare `✕` immediately after the progress label**, with an
accessible name of *Clear done*, appearing only once something is struck.
**Find it by its accessible name, never by its glyph or position**
(`T-qa-binds-tolerantly-to-markup`). **Its position is doing the work**:
there is no conventional icon for "clear completed", and it reads correctly
only because it sits directly after the count of done things. **The
misreading to watch for is "close this group"** — if that is what it looks
like on the phone, say so, because the fix is placement rather than a
different glyph.

## What can and cannot be undone

- **Unchecking a struck item puts it back.** It is the direct inverse of the
  tap that struck it.
- **Clearing is final.** Nothing brings back a cleared item, and there is no
  list of completed work anywhere (`D-kill-means-archive`).

**#103's no-undo assertion was narrowed by this slice, not dropped** —
`mark_done.feature` changed with it. The rule is now: **you can uncheck what
you can see, and nothing brings back what has cleared.** Check both halves.

## By-hand walkthrough — on a phone

1. Capture five things tagged `@homedepot`. Confirm a trip.
2. Check them off **one at a time, as if in the shop.** Confirm after each
   that **the panel holds** and the others stay in place.
3. At three, confirm the label reads **3 of 5 done** and the three you did are
   struck through, in position.
4. **Uncheck one.** Confirm it returns to open and the label reads 2 of 5.
5. Tap the **✕** after the progress label. Confirm only the struck ones go —
   and note whether, before tapping, you read it as *clear those three* or as
   *close this group*.
6. Confirm nothing disappears when you leave the screen and come back.

### Expected Observable Outcomes
- All six steps hold.
- **Step 2 is the defect.** Before this, the third check dissolved the panel.
- **Step 6 is the owner's decision made visible.** Glancing at Pool must not
  silently erase shopping progress.

## Procedure — struck in place, and the label

1. Five tasks at one tag; mark three done through the route.
2. Read the Pool screen.

### Expected Observable Outcomes
- **3 struck, 2 open, in their original positions** — not reordered, not
  moved to the bottom.
- The label reads **3 of 5 done**, not `5 things` and not `2 things`.
- The two open items are still in the same trip, **not in loose ends**.

## Procedure — clearing, and what it does not take with it

**Changed by #129 (`qa/trip_persistence.md`): `D-a-trip-survives-being-worked`
was extended on 2026-08-24 so that clearing tidies the panel rather than
dissolving it.** An earlier version of this document asserted the opposite
for case 2 below; both cases now hold.

1. Five tasks at one tag, three done. Clear done. Read the screen.
2. On a fresh database: **three** tasks at one tag, **one** done. Confirm a
   trip. Clear done. Read the screen.

### Expected Observable Outcomes
- After 1: two items remain, none struck, **still a trip**, reading
  `2 things`, and **loose ends stay empty.**
- After 2: **the same** — still a trip at two items, both keeping their tag,
  loose ends empty. It formed a trip, so it stays one for the rest of the
  run.
- **That second case is the interesting one** and the one a five-item fixture
  cannot reach: it is where formation and persistence visibly differ.
  `trip-progress-clearing-holds-the-group-05` is the scenario, and it was
  reversed in the open rather than edited quietly.

## Procedure — a group with nothing open left

1. Three tasks at one tag; mark **all three** done.
2. Read the screen. Then clear done.

### Expected Observable Outcomes
- Before clearing: the panel is still there, reading **3 of 3 done**, with no
  open items.
- After clearing: **the group is gone entirely** — no empty panel, no heading
  with nothing under it.
- **#125's complete-group button must not appear or must not act in this
  state** — there is nothing left to complete. **That landed in PR #135 and
  `trip-controls-nothing-left-to-complete-03` now asserts it**, so this is a
  cross-check rather than a note forward.
- **#129 extended what "gone entirely" means** below three items: a trip that
  clearing has taken down to two keeps its panel through the last check-off
  and leaves when you tap `✕`, exactly as this three-item case does. See
  `qa/trip_persistence.md`.

## Procedure — loose ends are unchanged

1. Two untagged pool tasks. Mark one done. Read the screen.

### Expected Observable Outcomes
- **It leaves the screen at once**, as it did before this slice.
- **This is deliberate, not an oversight.** The defect being fixed is a panel
  dissolving mid-use, and a loose end has no panel to dissolve. Extending
  strike-through there would need a *clear done* control for a section with
  no label to hang it on.

## Procedure — prove the check can fail

`T-a-check-must-be-seen-to-fail`.

1. Restore the original predicate — filter completed tasks out of the query
   again.
2. Run the suite. **Confirm it fails, and that the message is about the panel
   dissolving** rather than a generic count mismatch.
3. Restore; confirm a clean diff and a clean pass.
4. **Say in the report that you did this.**

## Procedure — nothing else changed

1. Run every existing QA suite and the acceptance suite.

### Expected Observable Outcomes
- **`mark_done` legitimately changed** — its no-undo scenario narrowed with
  this slice. If it did **not** need changing, the unchecking behaviour is
  missing.
- `pool_screen`'s threshold and ordering procedures still pass untouched:
  **order stays derived**, most items first, alphabetical tiebreak, and
  `pool-screen-nothing-reorders-05` still holds.
- **Reorder arrows are approved — for loose ends** (#95,
  `D-menu-is-a-worklist`); an earlier version of this document said they were
  not approved at all, which was overbroad. What must not appear is a reorder
  control **on a trip**, which is what `pool-screen-nothing-reorders-05`
  asserts.
- **#108 is open on this exact query and is not this slice's** — if the
  `WHERE` moved out of the store to make this work, say so; that is the
  thing #108 exists to prevent.

## Independent of Implementation

This procedure depends only on what the Pool screen shows before and after
items are checked off, unchecked and cleared. It does not depend on how the
query filters, where the threshold is tested, or how the label is composed.
