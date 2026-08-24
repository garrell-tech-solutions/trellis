# QA Procedure: The trip panel's own controls

Covers: `features/trip_controls.feature`, and the half of `#120` that no
acceptance scenario can hold — **expanding and collapsing**. Also carries the
assertion `pool-screen-truncation-06` gave up: **only three items are on
screen.**

## Interface used

The three screens over HTTP, read-only `sqlite3` for corroboration, **a real
browser** driven headless at 390×844 (`playwright-core` and the Chrome
`scripts/qa/phone_layout.cjs` already uses), and **a real phone**.

## Why the expand behaviour is here and not in the Gherkin

**The tier you assert in decides what the implementation must store.** #126
bought `captures.shown_kind` — a permanent, append-only column for a display
preference — and traced the cause only afterwards: the acceptance suite
speaks HTTP, so the state had to be server-rendered, so it had to be stored.
**A scenario asserting "the trip is expanded" buys that column again.**

So it is asserted here, where a browser can see a rendered page. **This slice
adds no migration.** If the implementation you are handed has one, that is
the finding — say so before anything else in the report.

`pool-screen-truncation-06` moved for the same reason and in the same
direction. It asserted `lists "3" items`, which is answered by reading the
*first* `<ul class="trip-items">` — an assertion that means something only
while the hidden items are a second list. **That second list is the defect.**
Over HTTP the trip now holds all five; "only three are on screen" is a fact
about a rendered page and lives here.

## ⚠️ The check must fail, not skip

If Chrome or `playwright-core` is missing, **this check fails loudly** — same
rule and same reason as `qa/phone_layout.md`, and it must be **gated in the
`gate` job** of `.github/workflows/ci.yml`, which already resolves a Chrome
binary into `PHONE_LAYOUT_CHROME`. Say in the report whether the `run:` line
is there.

## Procedure — the expand control, in a browser

Seed a trip of **eight** at one tag, and a second trip of **five** at
another, so independence is observable.

1. Load Pool at 390×844.
2. **Collapsed.** Exactly **three** items of the eight are visible. The other
   five are **in the document** — that is what HTTP now asserts — and **not
   on screen**: zero height, or clipped, but not painted.
3. **The control is a button.** Not a `<summary>`, not a `<details>`, and
   **no disclosure triangle** anywhere on the panel. It reads **"Show 5
   more"**.
4. **Record network activity, then tap it.** *(Playwright's `request` event.)*
5. **Expanded.** All eight are on screen. **The label now reads "Show
   fewer"** — the canvas's own wording (`Trellis.dc.html:850`).
6. **It is one list.** The fourth item's box sits directly below the third's,
   with **the same vertical spacing as third-below-second**, and the eight
   share one left edge. **This is the assertion that fails if the revealed
   items are still a nested list with its own margin.**
7. **No request was made.** The toggle is client state
   (`Trellis.dc.html:851`, keyed by group). **Zero requests between step 4
   and step 5.**
8. **Tap again.** Back to three, label back to "Show 5 more".
9. **Independence.** Expand the eight; the five-item trip stays collapsed.
10. **Expanded survives being worked.** With the eight expanded, **tick the
    sixth item.** The trip is **still expanded**, all eight still on screen,
    and the item you ticked is **struck in place**, where you can see it.

### Expected Observable Outcomes

- All ten hold.
- **Step 10 is the one this slice is most likely to get wrong, and it is not
  in either issue.** Every checkbox in the panel swaps `#pool-body`
  `outerHTML`, so a client-only toggle is destroyed by the act of working the
  trip: the panel collapses under your thumb and takes the item you just
  struck off the screen with it. **That is exactly the failure
  `D-a-trip-survives-being-worked` was written against**, arriving through a
  different door.
- **Step 7 is the other one.** An htmx round trip to expand a list is a real
  cost on a phone over a tailnet, and it drags the state back to the server —
  which is the road that ends in a column. **If you observe a request, that
  is a finding even if everything looks right.**
- **Step 6 cannot be checked over HTTP and is half of #120.** The old control
  had three faults and this is the third: the revealed items were a second
  list with its own spacing, and nothing reconciled them.

## Procedure — the complete-group control

1. Seed a trip of **eight**, collapsed, nothing done.
2. Read the control's label. **It names the count: "Complete all 8"** — the
   number of *open* items, and it includes the five you cannot see.
3. **Measure its placement.** It is **not adjacent to `✕ Clear done`**:
   either on a different row, or with **at least one tap target (44px,
   `--tap`) of clear space between their boxes.** And **no part of it sits in
   the horizontal band the item checkboxes occupy.**
4. **Tap it once.** All eight strike through in place. The label reads **"8
   of 8 done"**, `✕ Clear done` is now offered, and **the complete-group
   control is gone** — there is nothing left to complete.
5. **Untick one.** It comes back. The label reads "7 of 8 done" and the
   complete-group control returns reading **"Complete all 1"**.
6. Seed a fresh trip of eight, mark **five** done individually, then read the
   control. It reads **"Complete all 3"** — the open ones, not the group's
   size.
7. **Go through the route, never the database.** A fixture that archives rows
   directly proves nothing about the control.
8. **Corroborate in `sqlite3`**: the eight rows carry `archived_at`, and
   nothing outside the group does.

### Expected Observable Outcomes

- All eight hold.
- **Step 2 is the whole safeguard.** `D-bulk-completion-is-explicit` exists
  to prevent *"losing six items whose only shared property is that they
  mention `@homedepot`"* — and the specific way that happens here is
  **looking at three things and completing eight**. The count in the label is
  what makes that impossible to do unknowingly, and it is why this control
  cannot be a glyph: **an icon cannot say a number.**
- **Step 4's disappearance is `D-bulk-completion-is-explicit`'s "and nothing
  else"**, and it is also #103's half-pass trap wearing a new label: a panel
  offering to complete a group with nothing open in it is a control that
  reads correctly until you tap it.
- **Step 3 is a hazard, not a nicety.** `✕ Clear done` permanently clears
  what the complete-group control just struck, and the slip is likeliest in
  the second immediately after using it — thumb still on the header, eight
  rows having just changed under it.
- **What reverses this is unchecking, item by item** — the pool's undo, per
  `D-a-trip-survives-being-worked`. **Undoing eight by unchecking eight is
  tedious and it is not undo**, and this slice does not build one: nothing
  was lost, the panel shows precisely what happened, and #111 is narrowed to
  Committed for exactly this reason. **Say so in the report** rather than
  leaving a reader to wonder whether it was considered.

## Procedure — one statement, not a loop

`T-set-operations-execute-in-the-store`. On a quiet machine
(`T-latency-is-a-qa-assertion`'s own condition):

1. Seed a trip of **20**. Complete the group. Time the request.
2. Seed a trip of **1** — below the trip threshold, so do this by completing
   a single item. Time that request.

### Expected Observable Outcomes

- **Completing twenty is one round trip**, and it does not cost anything like
  twenty times completing one. **This is a proxy and it is worth saying so**:
  it cannot see a loop in a handler directly, but a loop of twenty statements
  through `sqlx` on a quiet machine is visible in wall clock, and this is the
  only view QA has of it.
- **It goes through `mark_done`'s front door.** That capability is the only
  one declaring `mod store;` privately
  (`T-cross-capability-invariants-need-an-owner`); if a second write path
  appeared, this is the slice it appeared in.

## Procedure — prove the checks can fail

**`T-a-check-must-be-seen-to-fail`.** Five breakages, five distinct messages:

1. Put the old `<details>`/`<summary>` back. Confirm the collapsed-count,
   the button, the label-flips and the one-list assertions fail — and **read
   them**: four assertions going red on one breakage is right here, because
   one structure carried all four faults.
2. Make the toggle an `hx-get`. Confirm the **no-request** assertion fails
   while everything else stays green.
3. Make the complete-group control label itself "Complete group" with no
   count. Confirm the label assertion fails.
4. Have the group completion skip the hidden items. Confirm
   `trip-controls-completes-the-hidden-too-04` fails in the acceptance suite
   — **this one is Gherkin, and it should be**, because it is a state change.
5. Move `✕ Clear done` up against the complete-group control. Confirm the
   spacing assertion fails.
6. Move the Chrome binary. Confirm the check **fails** rather than skips.
7. Restore. Clean pass, clean `git status`.

### Expected Observable Outcomes

- **Six failures, six messages, then a clean pass. Say which you used.**
- **Breakage 2 is the important one** — it is the difference between a design
  decision and an accident, and it is the one a reasonable coder is most
  likely to undo later for a plausible reason.

## By-hand walkthrough — on a real phone

**Label the pull request `preview`.**

1. **A trip of eight.** It shows three and a control saying five more are
   hidden.
2. **Expand it.** The list **continues** — one list, not two — and the
   control now reads as something you can collapse. **It should feel
   instant**; if it does not, it went to the server.
3. **Tick something in the middle of the expanded list.** It strikes in
   place and **the panel does not collapse.**
4. **Complete the group.** One tap. Read the button first: **it should have
   told you how many.**
5. **Collapse, expand, complete, untick, complete again.** **Nothing about
   the panel surprises you** — this is the acceptance criterion the brief
   wrote and it is a judgement only a person can make.
6. Look at the header with all four things on it — the tag, the count, `✕`,
   and the complete-group button — **on 390px.** **Report whether it is
   crowded**, and whether you could hit `✕` while reaching for the other one.

### Expected Observable Outcomes

- All six hold.
- **Step 6 is the one to report even if it passes.** Three slices have now
  added something to this header without any of them seeing the others'
  work, which is why these two were briefed as one slice — **and this is the
  first time anyone looks at the result.**
- **`colour` is in the pipeline ahead of this** and restyles the same
  stylesheet, including `.clear-done` and `.trip-more`. **The new controls
  must take palette tokens, not `#fff` and not the old green**, and the trip
  panel is cream rather than cyan by the time this lands. If you are looking
  at a preview built before `colour` merged, say so.

## Procedure — nothing else changed

1. Run the acceptance suite and every QA suite.
2. Run `scripts/analyzers/dry.sh` and **say what it measured.**

### Expected Observable Outcomes

- **All 22 acceptance features pass**, of which `trip_controls.feature` is
  new and **`pool_screen.feature` is the one edited** — scenario 06 only, and
  its reasoning is in that file's header. **If any other feature needed
  editing, the change leaked**; say which and why rather than editing it.
- **Every `trip_progress` scenario still holds** (#127). Its counts already
  scan the whole panel rather than one list, so one list changes nothing for
  them — **confirm that rather than assuming it.**
- `phone_layout` still passes: the panel is taller when expanded, and the
  document must still not scroll.
- **DRY headroom is zero** — #127 landed at exactly 3.00% and #130 is in
  flight scoping the gate to product code. `dry.sh` measures `rust,bash`
  only. **Report the number and the formats**, not just pass or fail.
- **#129 is open on this same panel** and changes when a group stops being a
  trip. **This slice lands first**; #129 inherits both controls.

## Independent of Implementation

This procedure depends only on what a browser renders at a phone viewport,
what requests it makes, and what the routes do to the stored rows. It does
not depend on how the expand state is held, on which element carries it, on
class names, or on where in the header the complete-group control is placed —
only that expanding costs nothing and survives being worked, that the control
names the count of what it will complete, that completing a group reaches the
items you cannot see and nothing outside it, and that unchecking puts an item
back.
