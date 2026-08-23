# Handoff brief — `scroll`

**Date:** 2026-08-22 · **Issue:** #101 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **The first bug found by using Trellis**, and the one that most directly blocks the milestone it belongs to. Three phone-first screens shipped saying *"the layout itself is unchecked"*; this is what that produced.

---

## Demo

**This one is demonstrated on the phone, not in a terminal**, and that is the point. The live instance is on the tailnet already, and **PR #104 gives you a second port to look at a branch on** — use it.

1. Open Trellis on a phone. Go to **Capture**, the one screen whose content exceeds the viewport at 390×844.
2. **Drag up.** Today: nothing moves.
3. **The tab bar is not at the bottom of the screen.** It is at the bottom of the *document*, below the fold, so you cannot reach the other three screens without pinch-zooming out.
4. After the fix: the drag scrolls, and the tab bar stays put over it.

Steps 2 and 3 are one bug with two faces. **Fixing the scroll without fixing the tab bar is half a fix.**

## Goal and scope

**Make the phone scroll, and leave behind a check that would have caught it.**

The second half is not a nice-to-have. This bug was present in #92, #94 and #96, was flagged in all three pull-request bodies, and **no gate in this repo can see it** — every acceptance test and QA script asserts over HTTP against markup, and none can observe a computed style, a scroll height or a gesture. `T-qa-binds-tolerantly-to-markup` was recorded when a restyle broke QA three times without CI noticing; **this is the same blind spot from the other side.**

### The diagnosis is measured, not theorised — read #101 before modelling anything

```
/            body 1031px  min-height 844px      body GREW past the viewport
             main 972/972                        nothing overflows main
             document 1031/844                   the DOCUMENT scrolls
```

`body` is `min-height: 100dvh`, not `height`, so it grows to fit content; `main { flex: 1 }` grows with it; `overflow-y: auto` therefore has **nothing to overflow**. But `overflow-y` is still *computed*, so a touch drag targets `main` as its scroll container — and `overscroll-behavior: contain` stops the gesture chaining out to the document, which is the thing that can actually scroll. **Three rules, each defensible alone.**

`crates/trellis-server/static/trellis.css` — `body` at 73, `main` at 117, `header` at 125. Unchanged since measurement; confirm before you start.

### Two directions, mutually exclusive — the canvas decides, not preference

- **Make `main` the container it claims to be.** `body { height: 100dvh }` instead of `min-height`, so `main` is bounded and genuinely overflows. Keeps the sticky tab bar as drawn and keeps `overscroll-behavior: contain` meaningful.
- **Let the document scroll.** Drop `overflow-y` and `overscroll-behavior` from `main`; make the header `position: fixed` rather than `sticky`.

`D-four-screens` makes `docs/design/Trellis.dc.html` **authoritative on layout**, and it draws a fixed bottom tab bar over a scrolling body. **Read the canvas and cite it by construct.** A brief that describes the canvas without opening it was wrong in four places on #92 — do not repeat that.

### Out of scope — do not absorb

Any restyle beyond these rules, the missing done control on the canvas (**a design gap I have raised with the owner — do not invent one**), #93's Quota screen, `main`'s padding and spacing, and anything about *what* the screens show.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-four-screens` | The canvas is authoritative on layout. Four screens, a bottom tab bar. **Read it.** |
| `D-dogfood-first` | This bug exists because the owner used the product. That is the milestone working. |
| `T-qa-binds-tolerantly-to-markup` | QA binds by role and accessible name, never by class or DOM shape. **Whatever check you add inherits this** — a browser assertion pinned to a selector is the same trap one layer down. |
| `T-latency-is-a-qa-assertion` | The precedent for *"this belongs in QA, not the acceptance suite"*. **Read it before deciding where a browser check lives** — it is the closest thing to this decision the project has already made. |
| `R-multi-tenancy` | The tailnet is the entire security boundary. Nothing here adds a surface. |
| **No migration** | Nothing touches the schema. If you find yourself writing one, something has gone wrong. |

## Acceptance scenarios worth specifying

- **A screen whose content exceeds the viewport scrolls on touch at 390×844.**
- **The tab bar is at the bottom of the viewport, not the document**, on every one of the four screens, whether or not that screen overflows.
- **The three screens that fit do not gain a scrollbar** — the fix must not make short screens scroll.
- Every one of the **17** existing features passes untouched. This changes no markup and no behaviour; if a feature needed editing, the fix leaked out of the stylesheet.

## The open question that is actually the slice

**Where does a check that can see this live, and does CI run it?**

This is the decision with the longest reach and I am not making it for you. What is true:

- **`playwright-core` driving the installed Chrome works on this box** — it produced the numbers in #101, and it is the first browser automation ever run against Trellis. Chrome is at `/usr/bin/google-chrome` (151.0.7922.137), Node is v22.14.0.
- **A Node dev dependency is not new.** The product has no Node build step and must not gain one, but `scripts/analyzers/dry.sh` shells out to `jscpd`, and CI's quality job already runs `npm install -g jscpd` (`ci.yml:436`). **A browser check is the same shape as an existing analyzer, not a new class of thing** — which is an argument for it, and worth saying out loud rather than treating it as a novelty.
- **Chrome on the GitHub runner is an assumption I have not verified.** Do not take it from me; check it. If CI cannot run the check, a local-only QA procedure is a legitimate answer — `T-latency-is-a-qa-assertion` is the precedent — but then **say plainly that it is not gated**, because a check nobody runs is worse than no check, and this bug's whole history is a flag everyone read and nobody acted on.

Settle it and **answer it in the pull-request body, in a table, with reasoning**. Two further parts to answer with it: **what exactly does it assert** (computed style, `scrollHeight` versus `clientHeight`, or a synthesised gesture — the last is closest to the real failure and the most brittle), and **which screens does it run against** (the overflowing one only, or all four — the tab-bar half of the bug is present even on screens that fit).

## Known repo gotchas

1. **`trunk` is green at `373ba4e`** (the #103 merge). Confirm CI on your base commit before starting. Cut from `origin/trunk`, not the local `trunk`, which is routinely stale.
2. **Expect 17 features.** Run the analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh` — the first two report nonsense without the generated acceptance entrypoints.
3. **Update `scripts/ci/complexity-baseline.json` as a step, not as an afterthought.** The gate has arrived failing on each of the last two slices. It pins the accepted over-threshold set with exact scores and fails when the set grows, when a score moves *in either direction*, or when a row goes stale — **so a stale row fails a slice that changed nothing.** Note that the two stale entries last time wanted *different* answers: one was a genuine new dispatch arm to record, the other was fixed by **extracting** the code rather than baselining it. Regenerating blindly is the wrong move; `T-complexity-8`'s threshold is in the constitution and cannot be raised.
4. **`cargo test --workspace` compiles zero acceptance tests on a fresh checkout** — entrypoints are gitignored. Run `scripts/acceptance/run.sh` yourself.
5. **PR #104 is open and touches `ops/` and `.github/workflows/ci.yml`.** If your check needs a CI step, you and it are editing the same file — **say so rather than resolving it silently.** It is also what lets you look at your own branch on a phone.
6. **`features/mark_done.feature:50` names `BY THU 17:00`, a string Trellis never renders** — a `by` renders `BY THU` only, deliberately, per `T-commitment-is-chosen-not-derived`. **My error, propagated from #102.** Fix the comment in passing if you touch the file; do not build anything on it.
7. **A scenario asserting invariance under exactly the transformation the mutator applies is untestable by that mutator and looks fully covered.** If a scenario here is about sameness across the four screens, write literal scenarios per screen — `app_shell` set that precedent.
8. **Do not add a visual design system.** Three rules are in scope.
9. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
10. **Open the pull request when QA is done**, before taking another brief.

## Dependencies and sequencing

- **Nothing blocks this.** `trunk` is green, the pipeline is empty, #103 merged.
- **#104 is open** and may land under you; it adds no behaviour and touches no crate.
- **#93 (Quota) is the last of the four screens.** It should land *after* this, or it arrives unverified exactly as the other three did.

## Source

- Issue **#101** — the measurements, the three-part cause, and the two directions
- `crates/trellis-server/static/trellis.css` — `body` 73, `main` 117, `header` 125
- `docs/design/Trellis.dc.html` — **authoritative on layout; open it**
- `docs/decisions.md` — `D-four-screens`, `D-dogfood-first`, `T-qa-binds-tolerantly-to-markup`, `T-latency-is-a-qa-assertion`
- **#92 / #94 / #96** — each shipped saying the layout was unchecked, and each shipped this bug
