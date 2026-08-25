# QA Procedure: Trellis follows the device's colour scheme

Covers: `features/colour.feature` — **its three scenarios only, which are the
two meta tags and the two manifest members.** Every colour in this slice is
verified here and nowhere else. See "Why the colours are not in the Gherkin".

## Interface used

A **real browser**, driven headless at a phone viewport against a running
server — `playwright-core` driving the Chrome already on the box, the same
way `scripts/qa/phone_layout.cjs` does — **run twice per screen, once with
`prefers-color-scheme: light` and once with `dark`.** Playwright sets that
per browser context; it is the only thing in this repository that can. Plus
the ordinary HTTP suites for the manifest, and **a real phone**, because a
palette is a thing you look at.

## Why the colours are not in the Gherkin

Every acceptance test in this project asserts over HTTP against markup.
**None can observe a computed style or a composited background**, and a
contrast ratio is a fact about a rendered page, not about a document. A
scenario claiming *"the muted text is legible"* would assert something the
runtime cannot check — worse than no scenario, because it would read as
covered.

`T-latency-is-a-qa-assertion` is the precedent and `qa/phone_layout.md` is
the shape. The acceptance-level guarantee for the colours is that **all 21
existing features pass untouched**: this changes no markup and no behaviour.

## ⚠️ The check must fail, not skip

**If Chrome or `playwright-core` is missing, this check fails loudly.** It
must never skip, warn, or pass vacuously — `qa/phone_layout.md` says the same
thing for the same reason, and that reason applies harder here: **the failure
this slice fixes is one that shipped, was measured, was written down in an
issue, and stayed shipped.** A check that goes quiet when its browser is
absent reproduces that exactly, one layer down and invisibly.

**It must also be gated.** `.github/workflows/ci.yml`'s `gate` job already
resolves a Chrome binary and exports it as `PHONE_LAYOUT_CHROME`; **this
check reads the same variable so it needs no second resolution step, and gets
its own `run:` line in that job.** The browser check that came before this
one shipped working but ungated, and the wiring arrived later at the owner's
direction — say in the report whether the line is there, because a check
nothing runs and a check that cannot fail produce the same green report.

## What must be true when this is done

**Everything the product paints, in both schemes, is one of the design
system's token values for that scheme, and every piece of text clears 4.5:1
against the surface it actually sits on.** Those two sentences are the whole
specification. The procedures below are how you see them.

## The baseline — what fails today, measured against real surfaces

Recomputed from the stylesheet's and the design system's own OKLCH values and
reproducing the design system's table where the two overlap. They differ
where the **surface** differs, which is the point: **no text in this product
sits on `--color-gray-100`.** `html` paints it and `body` covers it; on a
phone it is never visible. Both issues measured against it anyway.

Light, against the surfaces that actually carry text:

| text token | app surface | secondary surface | mint panel |
|---|---|---|---|
| `gray-400` | **2.49** | **2.36** | **2.35** |
| `gray-500` | 4.63 ✓ | **4.39** | **4.37** |
| `primary-500` | **3.64** | 3.45 | — |
| `gold` | **2.17** | — | — |
| `gray-600` | 7.24 ✓ | 6.87 ✓ | 6.84 ✓ |
| `primary-800` | 10.02 ✓ | 9.50 ✓ | — |

Dark, against the same three: **one failure, and only one** — `gray-400` on
the mint panel at **4.00**. It is the struck-through text of a done item in a
trip. Everything else clears: `gray-400` 4.89 / 4.54, `gray-500` 7.63,
`primary-500` 6.19, `primary-800` 10.33, gold 7.60.

**Three things to take from that table.**

1. **`gold` as text is the worst offender in the product and neither issue
   counted it** — a past date and the `PAST` badge on Committed, at 2.17,
   below the 2.36 the slice was filed for. **The owner ruled it in scope with
   no exemptions.** Gold stays fine as a fill and as a rule, and fine as text
   in dark; it is light-mode text that has to go.
2. **`gray-500` is not a blanket problem.** It passes on the app surface and
   fails only on the secondary surface and the mint panel — three rules, not
   ten. A blanket swap would have been wrong in both directions.
3. **Dark does not pass everywhere as authored.** The design system measured
   its dark ramp against the page. Against the panel it has one failure. It
   disappears when `gray-400` stops being a text colour, so it costs nothing
   — but nothing would have caught it.

## Procedure — the automated check, in both schemes

1. Start the server against a fresh database.
2. **Seed so that every component actually appears.** An element that does
   not render cannot be measured, and a check that silently measures nothing
   is the failure mode this project has now met five times. At minimum:
   several captures (rows, meta lines, the quick-add box and both its
   placeholders); one capture with a committed panel open (the chosen kind
   chip, the open commitment summary, the field panel and its labels); one
   triage rejection (the warning panel); a context tag with **at least three
   pool tasks, one of them marked done** (a trip panel, its progress label,
   its clear control and a struck item); a loose end (its tag); and **a
   committed task with a past date** (the past date cell and the `PAST`
   badge). **Report the seed inventory** — say which components you got on
   screen, not just that the check passed.
3. For each screen — Capture `/`, Pool `/pool`, Committed `/committed` — and
   for each scheme — `light`, `dark` — at 390×844, assert:

   **A. Every text-bearing element clears 4.5:1.** For each element with a
   non-empty text node of its own, take its computed `color` and the
   **composited background** — walk up ancestors to the first
   non-transparent `background-color` — and compute the WCAG 2.x ratio.
   **4.5:1 is the bar for all of it**; nothing here is large text. Report
   every failure with the element, the scheme, both colours and the ratio.
   **No exemption list, by the owner's decision.**

   **B. Nothing paints literal white in dark.** No element's computed
   `color` or `background-color` is `rgb(255, 255, 255)` when the scheme is
   dark. **This is the eleven `#fff` literals, caught in one assertion** —
   no token redefinition reaches a literal, so any one that survives leaves
   dark mode cosmetic in that spot.

   **C. The palette is closed.** Every computed `color` and non-transparent
   `background-color` on the page resolves to **one of the design system's
   token values for the scheme being rendered**. Build the expected set by
   reading the custom properties off `:root` at runtime rather than hardcoding
   them — the tokens are the contract, their hex is not. This is what catches
   a leftover green, an invented near-miss grey, and anything the coder
   reached for that the palette does not contain.

   **D. The two schemes genuinely differ.** For the same screen, the app
   surface, the body text colour and the tab bar's background all differ
   between light and dark. **A page that satisfies A–C in both schemes while
   painting identically in both has a dark block nothing reaches**, and this
   is the assertion that says so.

   **E. Nothing moved.** `phone_layout.cjs`'s three geometry assertions still
   hold — **and in dark as well as light.** This slice changes no layout.

**Bind tolerantly** (`T-qa-binds-tolerantly-to-markup`). Walk the DOM and
measure what is there; never assert *"`.row-meta` is `gray-600`"*. The
property is the ratio, not the rule that produced it. A restyle is not a
behaviour change and must not break this check.

### Expected Observable Outcomes

- **All five assertions hold, on all three screens, in both schemes.**
- **The failures listed in the baseline table are gone**, and the report says
  so token by token — including **both gold text rules**, which are the two
  nobody had counted.
- **Say what surface each was measured against.** That is the whole reason
  the numbers in this document differ from the ones in the issues.

## Procedure — prove the check can fail

**`T-a-check-must-be-seen-to-fail`. Four breakages, one per assertion, and
they must produce four different messages** — a check that says the same
thing however you break it has one assertion wearing four names.

1. **A**: put `--color-gray-400` back on one text rule. Confirm a contrast
   failure naming that element, the scheme and the ratio.
2. **B**: restore one `#fff` background. Confirm a literal-white failure in
   dark, and confirm it stays quiet in light.
3. **C**: set one token to the old green `oklch(0.61 0.12 142.9)` **at a use
   site rather than in `:root`**, so it is off-palette rather than a
   redefinition. Confirm a closed-palette failure.
4. **D**: restore `<meta name="color-scheme" content="light">`. Confirm the
   schemes-differ assertion fails — **this is the one the whole slice turns
   on, and it is the one most likely to be silently satisfied.**
5. Move the Chrome binary. Confirm the check **fails** rather than skips.
6. Restore everything. Confirm a clean pass and a clean `git status`.

### Expected Observable Outcomes

- **Five distinct failures with five distinct messages**, then a clean pass.
- **Say in the report which five you used and what each said.** A check
  nobody has seen fail is a check nobody has shown to work, and this project
  has shipped a proptest that could not fail, nine mutants surviving a
  scenario that could not fail, and a browser check that went red for a
  reason nobody predicted.

## Procedure — the metadata, over HTTP

1. Fetch each screen; read `<meta name="color-scheme">` and every
   `<meta name="theme-color">`.
2. Fetch the manifest; read `theme_color` and `background_color`.
3. **Then cross-check in the browser**: the light theme colour equals the
   colour the page's own surface paints in light, and the dark one equals it
   in dark.

### Expected Observable Outcomes

- `color-scheme` names **both** schemes on all three screens.
- Two theme colours: one unmediated (`#f9fafb`) and one under
  `media="(prefers-color-scheme: dark)"` (`#0b0f14`). **The unmediated one is
  the light one and stays first**, because `installable-theme-colour-05`
  matches a page's theme colour against the manifest's and must keep passing
  untouched.
- The manifest carries `#f9fafb` for **both** members. That is
  `--color-gray-50` in light — the app's own surface. **They are the same
  value on purpose**: both answer *what does Trellis look like before it has
  painted anything*, and Trellis has one surface. `background_color` already
  was this; `theme_color` stops being a brand block above a white app.
- **A manifest has one of each and cannot express a custom property**, which
  is why the per-scheme half is a media-scoped meta rather than a second
  manifest.
- **These literals go stale silently.** Step 3 is what makes them a
  contract instead of two numbers someone typed once — do not skip it.

## By-hand walkthrough — on a real phone, and this is the slice

**Label the pull request `preview`** and the box puts the branch on a second
port within two minutes. **This slice cannot be reviewed any other way.**

1. Open Trellis over the tailnet. **The tab labels are legible** — 9.5px on
   every screen, and today the least legible thing in the product.
2. **It is blue, not green.**
3. **The Save capture button is still gold**, and still the only warm thing
   on the screen. If it has become another blue rectangle, the most-used
   control has been folded into the background.
4. **Go to Pool. The trip panels are cream, not cyan.** This is expected and
   is the one visible change nobody asked for: the design system moved mint
   into gold's hue family reasoning that it is the warning panel, and in the
   shipped product `.trip` uses `.panel` too. **Report how it reads** — a
   screen of trips now shares its surface with the validation-error panel, as
   it already did in cyan.
5. **Go to Committed with something in the past.** The past marker still
   reads as past. It is no longer gold text — that was 2.17:1 — so **report
   whether whatever replaced it still says "this one has gone by" at a
   glance.** This is the judgement the automated check cannot make.
6. **Turn the phone to dark mode.** Trellis follows, immediately, with no
   reload and no toggle anywhere.
7. In dark: **nothing is a white slab.** Look at the quick-add box, the field
   panels, the inputs inside them and the chips — **each should still be
   distinguishable from the surface behind it.** The automated check proves
   nothing paints white; **only you can say whether the three surface levels
   still tell each other apart**, and in dark they invert — a panel that
   recedes by going darker in light recedes by going lighter in dark.
8. **Turn it back.** Light mode looks like it did this morning. The app
   surface moved from `#fff` to `--color-gray-50`, one rung, and **if you can
   see that, say so.**
9. **Nothing has moved.** No layout change, no new control, no reflow.

### Expected Observable Outcomes

- All nine hold.
- **Steps 4, 5 and 7 are the ones to look hardest at** — each is a place
  where the check can be fully green and the screen can still be wrong.
- **Step 6 is the demo.** If it needs a reload, `prefers-color-scheme` is not
  what is driving it.

## Procedure — nothing else changed

1. Run the acceptance suite and all 22 existing QA suites, plus this one.
2. Run `scripts/analyzers/dry.sh` and **say what it measured.**

### Expected Observable Outcomes

- **All 21 acceptance features pass untouched**, and `colour.feature` is the
  22nd. **This slice changes two meta tags, two manifest members and a stylesheet.
  If a feature or a QA script needed editing, colour leaked into structure**
  — say which and why rather than editing it.
- `phone_layout` still passes, in the light scheme it has always run in.
- **DRY headroom is zero** — #127 landed at exactly 3.00%, and #130 is in
  flight scoping the gate to product code. `dry.sh` measures `rust,bash`
  only, so **the new `.cjs` is not measured and the new `.sh` wrapper is**:
  keep the wrapper thin and put anything shared in `scripts/qa/lib.sh`.
  **Report the number and the formats it was taken over**, not just pass or
  fail.

## Independent of Implementation

This procedure depends only on what a browser renders at a phone viewport
under each colour scheme, and on what the pages and the manifest declare over
HTTP. It does not depend on which CSS rules produce a colour, on class names,
on the DOM's shape, on how the dark block is authored, or on which token the
coder chose for any particular element — only that every painted colour is in
the palette, that every piece of text clears 4.5:1 against what is actually
behind it, and that both schemes are reachable and different.
