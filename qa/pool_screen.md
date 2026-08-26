# QA Procedure: The pool screen groups loose work by where it can be done

Covers: `features/pool_screen.feature`

## Interface used

The pool screen, reached by its **tab** from the capture screen — never by a
typed URL. The capture screen and its triage controls for setup, and
read-only `sqlite3` inspection. No project library, module, or test helper is
used.

**`T-qa-binds-tolerantly-to-markup` governs this document.** This is the
stylesheet-heaviest screen in the product, on a design system whose restyle
has already broken three sets of scripts. **Bind to ids, `data-` attributes
or classes this suite owns — never to attribute order, adjacency, or copy
quoted verbatim.** Where a procedure below quotes text, it quotes it as an
example and says so.

## The one idea this screen turns on

**A context tag becomes a trip only once three things are waiting there.**
Fewer, and those items fall into loose ends **still showing their tag**.

Everything else follows from it:

- **Trips are ranked by how many things they clear** — derived from the data,
  never maintained. **Groups have no priority and must never grow one.**
- **A trip is a unit you clear in one stop**, so the order of its items is
  noise and should never get a control.
- **A loose end is a thing you decide about**, so it should get one — **in
  its own slice, not this one.**

**Nothing on this screen reorders anything, and that is deliberate.** The
canvas draws up/down arrows on trip items, on loose ends and on quota rows.
**If you find any of them, that is a defect** — a coder reading the canvas
would add them in good faith, which is exactly why their absence is asserted
rather than assumed.

## What the canvas draws that is deliberately not built

The per-group note — *"One stop clears all 3."* — is computed in the canvas
from a hardcoded list of which tags are places and which are sittings. That
needs Trellis to know `@homedepot` is a shop, which is the managed taxonomy
`D-context-tags-are-the-taxonomy` refuses. **Its absence is intended. Do not
report it as missing.**

## By-hand walkthrough — do this once, in a real browser, on a phone

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Capture and triage as pool: three errands tagged `@homedepot`, two tagged
   `@supermarket`, one untagged.
3. Tap **Pool** in the tab bar. Confirm you never typed a URL.
4. Confirm `@homedepot` is a trip panel of three, and that **`@supermarket`
   is not** — its two items sit in loose ends, each still showing
   `@supermarket`.
5. Confirm **nothing on the screen offers to reorder anything** — no arrows
   on trip items, none on loose ends.
6. Tap back to **Capture** and confirm the tab bar marks the current screen
   both ways.

### Expected Observable Outcomes
- All six steps hold literally, per `D-visible-slices`.
- **Step 4 is the slice.** Being at Home Depot with three errands in one
  panel is the whole point; two things at the supermarket are strays that
  share a place, and the screen should say so by shape.
- **Step 5 is where the design was deliberately not followed** — worth seeing
  with your own eyes, since the canvas draws arrows on every row. Reordering
  is a slice of its own.
- **This is a phone screen.** Check it at phone width, not just narrowed
  desktop: the tab bar is sticky-bottom and the panels are the widest thing
  here.

## Setup — repeat before each procedure below

1. Start the server against a fresh database file.
2. Reach the pool screen **through the tab bar**.

## Procedure — trips, strays, and the threshold

1. Create pool tasks: three tagged `@homedepot`, two tagged `@supermarket`,
   one untagged.
2. View the pool screen.

### Expected Observable Outcomes
- **One** trip panel, `@homedepot`, reading three things.
- **`@supermarket` is not a trip.** Both its items are in loose ends and
  **both still show `@supermarket`** — losing the tag there would make the
  screen forget something the owner typed.
- The untagged item is in loose ends with no tag shown.
- The count beside the title covers **everything pooled**, trips and loose
  together — six here. It is not a count of loose ends and not a count of
  trips.
- **Add a third `@supermarket` item and re-check.** It must become a trip,
  and its three items must leave loose ends. The threshold is a boundary and
  boundaries are where this will break.

## Procedure — trip order

1. Create four pool tasks at one tag, three at another, three at a third,
   choosing tag names whose alphabetical order differs from creation order.
2. View the pool screen.

### Expected Observable Outcomes
- Trips run **most things first**; the two three-item trips are ordered
  **alphabetically** between themselves.
- **Creation order must not decide it.** Choose the fixture so that creation
  order, alphabetical order and size order all disagree — otherwise a build
  that sorts by the wrong one passes.

## Procedure — one tag however it is spelled

1. Create three pool tasks tagged `@HomeDepot`, `@homedepot` and
   `@HOMEDEPOT`.
2. View the pool screen.

### Expected Observable Outcomes
- **One** trip of three, labelled with the spelling first used.
- **This screen is where #82's decision earns itself.** Three separate
  one-item groups would each fall below the threshold, so all three errands
  would land in loose ends and the trip would never appear — the owner drives
  to Home Depot twice, or not at all.

## Procedure — only pool work appears

1. Create one pool, one committed and one quota task, all tagged the same.
2. View the pool screen.

### Expected Observable Outcomes
- Neither the committed nor the quota task appears — **not in a trip, not in
  loose ends, not in the count.**
- `D-no-pool-on-calendar` makes this the *only* place pool work is offered;
  it does not make it a place other work is offered too.

## Procedure — order, and the absence of controls

1. Create three pool tasks at one tag and two untagged ones.
2. View the pool screen.
3. Read the markup of the trip panel and of the loose-end rows.

### Expected Observable Outcomes
- A trip's items are **newest first**; loose ends are **newest first**.
- **No up or down control exists anywhere on the screen** — not inside a trip
  panel, not on a loose end, not hidden, not disabled, not present-but-inert.
  Absent.
- **No endpoint reorders anything.** If a plausible reorder route exists, try
  it by hand; it should refuse or not exist.
- **This is the assertion most likely to be undone by accident**, because the
  canvas draws the arrows and a good-faith reading of the design puts them
  back. Reordering has its own slice; when it lands it belongs to loose ends
  alone, and never to trips or groups.

## Procedure — a long trip

1. Create five pool tasks at one tag.
2. View the pool screen; use the control that offers the rest; use it again.

### Expected Observable Outcomes
- **Three** items shown, and a control offering the remaining two.
- Using it reveals all five; using it again returns to three.
- The trip's count reads **five** throughout — the count is what is waiting,
  not what is displayed.

## Procedure — nothing pooled

1. View the pool screen against an empty database.

### Expected Observable Outcomes
- An empty-state message and **a way back to Capture**.
- The count beside the title reads as empty rather than as a zero — the
  canvas distinguishes them.
- **A capture that exists but is untriaged does not count.** Only pool
  *tasks* appear here.

## Procedure — the tab bar

1. View the capture screen, then the pool screen.

### Expected Observable Outcomes
- **Exactly four tabs**, Capture, Pool, Committed and Quota. **Updated by
  #93**, which read *"exactly three... not four: Quota arrives with its own
  slice, and a dead link is worse than no link."* That slice has arrived, so
  the link is no longer dead and the count is four. The rule the old wording
  was protecting is unchanged and still worth checking: **no tab may point at
  a route the app cannot serve** — follow all four from all four screens.
- Each screen marks itself current, and only itself.
- The marking is in the markup, not by colour alone.
- **`one_screen`'s route list is the other half of this**, and it changed in
  this slice: `/pool` now answers 200. Confirm the removed five still 404.

## Procedure — hostile text stays escaped

1. Create three pool tasks tagged `<script>alert('boom')</script>`.
2. Read the **raw HTML** of the pool screen.

### Expected Observable Outcomes
- No unescaped `<script>`; `boom` still present, escaped rather than
  stripped.
- **Check the trip heading and the loose-end tag label separately.** A tag
  renders in both places, and this screen is the first to render one as a
  panel heading.

## Procedure — nothing else changed

1. Run all fourteen existing QA suites.

### Expected Observable Outcomes
- All pass. This slice adds a screen and a route; it changes no capture or
  triage behaviour.
- **`one_screen` legitimately changed** — a route was added.
- **Believe the re-run, not the expectation.** A restyle has broken QA
  scripts three times in this project without CI noticing, and this slice
  edits the stylesheet and the header.

## Independent of Implementation

This procedure depends only on what the pool screen renders for a given set
of tasks and tags, and what survives a reload and a restart. It does not
depend on which route serves it, how the grouping is queried, how the manual
order is stored, or how the tab bar is templated.
