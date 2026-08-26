# QA Procedure: The phone scrolls, and the tab bar stays put

Covers: **no acceptance feature — and that is the finding, not an omission.**
See "Why there is no Gherkin for this" below.

## Interface used

A **real browser**, driven headless at a phone viewport, against a running
server. `playwright-core` driving the Chrome already installed on the box —
`/usr/bin/google-chrome`, verified working headless here. Plus the ordinary
HTTP suites, which cannot see any of this.

**This is the first browser automation ever run against Trellis**, and the
reason it exists is written into the slice: the bug it catches shipped in
#92, #94 and #96, was flagged in all three pull-request bodies, and **no gate
in this repo could see it.**

## Why there is no Gherkin for this

Every acceptance test and QA script in this project asserts over HTTP against
markup. **None can observe a computed style, a scroll height, or a gesture.**
A scenario claiming *"the screen scrolls"* would be asserting something the
runtime cannot check, which is worse than having no scenario: it would read
as covered.

So this slice adds **no feature file**. The acceptance-level guarantee is
that **all 17 existing features pass untouched** — the fix is three CSS rules
and must leak into no markup and no behaviour.

`T-latency-is-a-qa-assertion` is the precedent and it was made for this exact
shape: a property of the running system, measured where the reading means
something, rather than asserted where it cannot be.

## ⚠️ The check must fail, not skip

If the browser is missing, **this check must fail loudly.** It must never
skip, warn, or pass vacuously.

That is not fussiness. **This bug's entire history is a flag everyone read
and nobody acted on** — three pull requests said the layout was unchecked and
three merged anyway. A browser check that silently skips when Chrome is
absent reproduces exactly that failure one layer down, and it would do it
invisibly.

**Chrome on the GitHub runner is an assumption nobody has verified**, and it
cannot be verified from a worktree. Failing closed is what turns that unknown
into an answer on the first CI run instead of a check that quietly never
ran.

## What it asserts, and what it deliberately does not

Three assertions, at **390×844**:

1. **The document does not scroll.**
   `document.scrollingElement.scrollHeight <= clientHeight`.
2. **The overflowing screen's `main` does scroll.**
   `main.scrollHeight > main.clientHeight` — seed enough content to overflow
   rather than assuming which screen is long.
3. **The tab bar's bottom edge is the viewport's bottom edge**, within a
   pixel or two — `getBoundingClientRect().bottom ≈ innerHeight` — on **every
   screen**, whether or not it overflows.

**Assertion 3 is half the bug and is present even where nothing overflows.**
Fixing the scroll without it is half a fix.

**A computed-style assertion would not have caught this**, and that is worth
knowing before anyone proposes one as simpler. In the broken build
`overflow-y` computed to `auto` exactly as intended — `main` was a scroll
container with nothing to overflow, because `body` had grown past the
viewport. **The style was right and the geometry was wrong.** Assert
geometry.

**A synthesised touch gesture is closest to the real failure and is not
specified**, deliberately: it is the most brittle thing available and depends
on the driver's input emulation rather than on the page. The geometry above
is what a gesture would be *for*.

**`T-qa-binds-tolerantly-to-markup` applies to this check as much as to the
HTTP ones.** Find the tab bar by role and accessible name, never by
`header nav a` or any class. **A browser assertion pinned to a selector is
the same trap one layer down**, and a restyle has already broken this
project's QA three times.

## Which screens

**All the screens that exist**, which today is **three**: Capture (`/`), Pool
(`/pool`) and Committed (`/committed`).

`D-four-screens` names four. **Quota does not exist yet — #93 builds it —
and that slice must extend this check rather than inherit a pass.** Say so in
the report: three of four covered, by existence rather than by choice.

## By-hand walkthrough — do this once, on a real phone

The live instance is on the tailnet, and PR #104 gives a second port for
looking at a branch.

1. Open Trellis on a phone. Go to **Capture** — the one screen whose content
   exceeds 390×844 today.
2. **Drag up.** It scrolls.
3. **The tab bar stays put over the content**, at the bottom of the screen.
4. Go to **Pool** and **Committed**. Confirm the tab bar is at the bottom of
   the screen on both, and that **neither has gained a scrollbar** — the fix
   must not make short screens scroll.

### Expected Observable Outcomes
- All four steps hold.
- **Step 4's second half is the regression the fix could introduce.** Bounding
  `body` to the viewport is right; making every screen a scroll container
  with a visible bar is not.
- **This is the first time anyone has looked at Trellis on a phone as part of
  a slice.** Three screens shipped saying the layout was unchecked. Whatever
  else this procedure finds, write it down — it is the first sighting.

## Procedure — the automated check

1. Start the server against a fresh database.
2. Seed enough captures that Capture overflows 844px.
3. Drive the browser at 390×844 and run the three assertions on each of the
   four screens.

### Expected Observable Outcomes
- All three assertions hold on all four screens.
- **Quota is the fourth and it arrived after this document did** (#93), added
  to `SCREENS` in `quota-sessions`. It is the densest header the product has
  — a name, a readout, a progress track and three log controls in one row —
  so **it is the likeliest of the four to overflow 390px.** If it does,
  **that is the finding**: report it rather than widening the tolerance.
- **Deliberately break it to prove the check works**, before trusting it:
  restore `min-height` on `body` and confirm the check **fails**. A check
  that has never failed is a check nobody has shown to work — and this
  project has shipped a proptest that could not fail and nine mutants that
  survived a scenario that could not fail. **Do not skip this step.**
- If Chrome is absent, the check **fails**. Confirm that too, by moving it.

## Procedure — nothing else changed

1. Run all seventeen existing QA suites and the acceptance suite.

### Expected Observable Outcomes
- **All pass, untouched.** This slice changes three CSS rules; it changes no
  markup and no behaviour. **If a feature or a QA script needed editing, the
  fix leaked out of the stylesheet** — say which and why rather than editing
  it.

## Independent of Implementation

This procedure depends only on the geometry of the running page at a phone
viewport. It does not depend on which CSS rules produce it, on class names,
or on the DOM's shape — only that the document does not scroll, that an
overflowing screen's content region does, and that the tab bar sits on the
viewport's bottom edge.
