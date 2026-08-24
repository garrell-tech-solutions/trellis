# Handoff brief — `colour`

**Date:** 2026-08-24 · **Issues:** #124 **and** #128 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **Two issues, one slice, deliberately.** #124 fixes *which* token is used; #128 changes *what the tokens are worth* and adds dark mode. **They touch the same declarations in the same file**, and #128 has to re-verify contrast anyway — **the new palette carries the identical failures**, because it preserved lightness and chroma on purpose. Splitting them means editing the same lines twice and measuring contrast twice.

---

## Demo

**On the phone.** Label the pull request `preview`.

1. **The muted text is legible.** Today the 10.5px tab labels sit at **2.36:1** — the least legible thing in the product, on every screen.
2. **It is blue, not green.**
3. **Turn the phone to dark mode.** Trellis follows. Today it has no dark handling at all.
4. Nothing has moved. **This changes no layout and no behaviour.**

## Part one — contrast (#124)

Computed from `trellis.css`'s own OKLCH values, OKLCH → OKLab → linear sRGB → WCAG relative luminance, and reproduced independently of the report that raised them:

| foreground | on `gray-100` | on `gray-50` | |
|---|---|---|---|
| `--color-gray-400` | **2.36** | **2.49** | fail |
| `--color-gray-500` | **4.39** | 4.63 | **fails on the page, passes on a card** |
| `--color-primary-500` | **3.28** | **3.46** | fail |
| `--color-gray-600` | 6.87 | 7.24 | passes — the obvious substitution |
| `--color-primary-800` | 9.02 | 9.50 | passes |

**AA needs 4.5:1 for body text.** `gray-500` is the sly one: **legal on a card and illegal on the page**, which is why nobody caught it by looking.

**Revaluing cannot fix `gray-400`.** Reaching 4.5 needs lightness ≈ 0.545, **which is `gray-500`'s value** — the two steps would merge and the ramp would lose a rung. **So the fix is which token is used, not what it is worth.**

There are **21 text usages** of the three: `gray-400` ×7, `gray-500` ×10, `primary-500` ×4, at lines 104, 167, 217, 276, 304, 311, 360, 371, 453, 522, 557, 572, 614, 679, 719, 725, 731, 798, 843, 849, 889.

**Judgement, not a blind replace.** Check each — some may not be text, and **say what surface each was measured against**, because a blanket swap loses the card-versus-page distinction and a blanket keep leaves the failures.

## Part two — the palette (#128)

**`Trellis Design System`**, Claude Design project `de98a7ad-1291-4326-b43a-5c9af1f94798`, holds **all 24 token names the product already uses** — verified present in both the light and dark blocks, nothing renamed — plus `--color-on-primary` and `--color-on-gold`.

- **Hue 255 for the primary ramp.** The neutrals already span 247.8 → 264.7, so 255 is that band's midpoint: **the accent is the same cool family as the greys**, while staying clear of 264 where it reads indigo. **Lightness and chroma are byte-identical on every step; only hue moved.**
- **Do not assume contrast survived the rotation.** *"Preserved by construction"* is **false** — WCAG weights green luminance at 0.7152 and blue at 0.0722, so blue at green's OKLCH lightness is darker in Y. Every primary pair moved slightly **up** (`primary-800` 9.92 → 10.46) — safe direction, but **a shift. Re-measure.**
- **Gold survives untouched.** The only warm colour, and it is the **Save capture** button. Fold it in and the most-used control becomes another blue rectangle. Near-complementary to 255, so it can never read as primary-toned.
- **Mint folds into gold's hue family**, because both usages are gated on `hasWarning` and carry `warningText` — **it is the warning panel, not a trip panel**, and gold is already its top rule. Forced anyway: mint's lightness (0.9656) equals `gray-100`'s (0.967), so rotated to blue the panel would vanish against the page.

## Part three — dark mode (#128)

**Nothing in the product has any dark handling.** The design system authors it as **a redefinition of the same token names**, so consuming code needs no new names and no conditionals.

**The dark ramp's middle is deliberately empty**: 50–300 compress into L 0.165–0.330 (surfaces, borders); **nothing between 0.330 and 0.600**; 400–950 expand into 0.600–0.988 (ink). A naive inversion puts `gray-400` at 0.293 and `gray-500` at 0.449 — the two tokens carrying muted and secondary text, 38 uses between them — squarely in the mud. **Nothing here needs a mid-grey.**

**Dark passes contrast everywhere as authored** — `gray-400` 4.54, `gray-500` 7.09, `primary-500` 5.75, gold 7.07 — **but that is the design system's measurement, not yours.** Confirm against what you ship.

## Three literals that cannot be tokens

- **`base.html:6`** is `<meta name="color-scheme" content="light">`. **Dark mode does not work while that is asserted.**
- **`trellis.css` hardcodes `#fff` eleven times.** **No token redefinition reaches those, so dark mode is cosmetic until they are tokenised.** `--color-on-primary` exists for this. **Check each** — some are backgrounds, some are text on a filled control, and they do not take the same token.
- **`manifest.webmanifest`** carries `"theme_color": "#0f4e0f"` and `"background_color": "#f9fafb"` — **literals a manifest cannot express as custom properties.** #121 shipped them marked provisional; this is when they stop being. **A manifest has one of each — decide what they are when the device is dark, and say why.**

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `T-canvas-is-authoritative-where-it-speaks` | **Amended 2026-08-24.** The canvas leads a slice being specified against it; the implementation is the record once shipped. **The canvas still links the GTS design system and will not change colour with this — expected drift, not a defect.** The pipeline does not edit the canvas. |
| `D-four-screens` | Authority bounded to the slice. **No layout changes here.** |
| `T-dry-measures-product-code` | **Settled 2026-08-24** — the gate measures product code only. **#130 is in flight implementing it**; until it lands the old measurement applies and headroom is zero. |
| `T-a-check-must-be-seen-to-fail` | Whatever you assert, break it and watch it fail. |

## Known repo gotchas

1. **`trunk` is green at `b3d26b4`.** Cut from `origin/trunk`, not the local `trunk`.
2. **Expect 21 features.** Analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`.
3. **`scripts/qa/phone_layout.cjs` is gated in CI and drives real Chrome.** **It can set `prefers-color-scheme`** — dark mode is otherwise unverifiable by anything in this repo, and **contrast is computable from a rendered page.** Strongly consider making this a check rather than a one-time fix: **a one-time fix with nothing guarding it is exactly how this arrived.** If you add one, prove it can fail.
4. **DRY headroom is zero until #130 lands** — #127 finished at exactly 3.00%. This is a CSS slice and should not add to it; **say what it measured.**
5. **All 21 acceptance features pass untouched.** This changes no markup and no behaviour. If one needed editing, colour leaked into structure.
6. **`cargo test --workspace` skips the `#[ignore]`d property tests locally** — CI runs them via `--include-ignored` since #113.
7. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
8. **Open the pull request when QA is done, and label it `preview`.** This slice cannot be reviewed any other way.

## Out of scope

Any layout change, new components, typography, spacing, **a manual theme toggle** (`prefers-color-scheme` follows the device; a switch is a settings surface that does not exist — **#118**), and the canvas.

## Source

- Issues **#124** and **#128**
- `crates/trellis-server/static/trellis.css` · `templates/base.html:6` · `static/manifest.webmanifest`
- `Trellis Design System` — Claude Design project `de98a7ad-1291-4326-b43a-5c9af1f94798`
- `docs/decisions.md` — `D-four-screens`, `T-canvas-is-authoritative-where-it-speaks`, `T-a-check-must-be-seen-to-fail`, `T-dry-measures-product-code`
