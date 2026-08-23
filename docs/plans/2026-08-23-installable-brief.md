# Handoff brief — `installable`

**Date:** 2026-08-23 · **Issue:** #112 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **The owner asked how hard a native Android app would be.** This is the answer that costs a slice instead of a quarter — and it is the first slice whose whole subject is how the product *feels* to open.

---

## Demo

**On the phone, and this one cannot be demonstrated any other way.**

1. Open Trellis over the tailnet. Use the browser menu: **Install app** / *Add to home screen*.
2. It appears in the launcher with **an icon somebody chose**, not a screenshot of the page.
3. Open it from there. **No address bar, no browser chrome** — its own window, its own entry in the app switcher.
4. The tab bar still sits on the bottom edge, and the safe area is still respected.

## Goal and scope

**Make Trellis installable.** A web app manifest, icons, and the two meta tags that make an installed window behave.

`base.html` has **no `<link rel="manifest">` and no `theme-color`** today.

### Why this and not the Android build the owner asked about

Assessed 2026-08-23 and recorded on #112: **a packaged TWA is still Chrome in a box with zero native components.** A native or React Native client needs a JSON API for every screen — **there is none**; all five `GET` routes return HTML and the only JSON in the tree is one endpoint in `triage/http.rs`. React Native would also abandon `D-four-screens`: the canvas is HTML and CSS and React Native consumes neither, so the design source and the implementation would diverge — **the exact problem that decision exists to end.**

An installable web app delivers the home-screen icon, the standalone window and the app-switcher entry, with **no new toolchain** in a repo that deliberately has no Node build step.

### Out of scope — do not absorb

**A service worker and any offline behaviour** (see below), an APK or TWA, push notifications, and **any visual design system**.

## Offline is deliberately excluded, and the pull request must say so

**An installed app that cannot reach the tailnet shows nothing** — and it fails *looking like a broken app* rather than a page that did not load. A service worker could cache the shell, but **the data is entirely server-side**, so caching the shell yields an app frame around an empty screen. That is worse than the browser's own error message, not better.

**Do not add one.** If offline turns out to matter it is a sync problem and a separate product decision, and the fortnight of real use is what should decide it (`D-dogfood-first`). **State this in the pull request** rather than leaving a future reader to wonder whether it was forgotten.

## Colours are provisional, and that is deliberate

A manifest's `theme_color` and `background_color` are **literal values — they cannot be CSS custom properties.** A new Trellis design system with a blue palette and a dark mode is being built in parallel, and `base.html:6` currently hard-codes `<meta name="color-scheme" content="light">`.

**Take today's values from `trellis.css` and mark them provisional in the pull-request body. Do not try to anticipate the new palette** — reworking two hex values later is cheaper than guessing now.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-four-screens` | Four screens in a bottom tab bar; the canvas is authoritative on layout. **It draws no app icon** — that is a gap to flag, and it is the fourth. |
| `R-multi-tenancy` | **Deferred, not refused.** The tailnet is the entire security boundary and this changes nothing about that: an installed PWA is still served from, and reachable only through, the tailnet. |
| `D-dogfood-first` | Daily use by 2026-09-03. This is about the friction of opening the thing. |
| `T-nav-is-the-site-map` | Every page in the route table gets a header link. **A manifest is not a page** — say so explicitly, because the rule is otherwise unconditional. |
| `T-a-check-must-be-seen-to-fail` | **Recorded 2026-08-23.** Whatever check you add here, break it, watch it fail, restore it, and say so. A manifest that fails to parse does nothing *silently* — this is exactly the shape. |

## Acceptance scenarios worth specifying

- The manifest is served, parses, and carries the members an installable app requires.
- **Icons exist at the sizes Android uses, including a maskable one.** Without `purpose: maskable` the launcher crops the icon into a shape nobody chose.
- **The installed window respects the safe area.** `viewport-fit=cover` is already set (`base.html:5`) and **the tab bar sits on the bottom edge** — that is where an installed window differs from a browser tab, and where this is most likely to look wrong.
- Each of the four screens still renders exactly as it does today.
- **All 18 acceptance features pass untouched.**

## Known repo gotchas

1. **`trunk` is green at `ed93998`** (the #117 merge). Cut from `origin/trunk`, not the local `trunk`, which is routinely stale.
2. **Expect 18 features.** Analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`.
3. **Static assets are explicit routes, not a directory.** `platform/app.rs` names `/static/htmx.min.js` and `/static/trellis.css` one by one, embedded with `include_str!`. There is no `tower-http` file server and **`platform/boundary.rs` will notice if you add one.** **Icons are binary, so `include_str!` will not do** — say what you used and why.
4. **Update `scripts/ci/complexity-baseline.json` as a step.** #117 was the first slice in four to arrive with it already correct; keep that.
5. **`scripts/qa/phone_layout.cjs` is gated in CI** and drives a real Chrome at 390×844. **A manifest that fails to parse is invisible** — decide whether installability belongs there, and if you assert it, prove the assertion can fail.
6. **`cargo test --workspace` skips the `#[ignore]`d property tests** — CI now runs them via `--include-ignored` as of #113, but your local run does not unless you pass it.
7. **The company standard binds you** — `swarmforge/constitution/articles/engineering.prompt` carries it, `T-set-operations-execute-in-the-store` is its Trellis row.
8. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
9. **Open the pull request when QA is done.** **Label it `preview`** and the box will put it on the phone within two minutes (#114, merged) — this is the first slice where the owner genuinely cannot review it any other way.

## Dependencies and sequencing

- **Nothing blocks this.** `trunk` is green, the pipeline is empty, no pull request is open.
- **#111** (undo) and **#93** (Quota) are queued behind.
- **#118** (the timezone setting has no surface) is unrelated but touches `base.html`'s neighbourhood; it is not queued yet.

## Source

- Issue **#112** — the assessment of Android options and why this is the one
- `crates/trellis-server/templates/base.html:5,6` · `crates/trellis-server/src/platform/app.rs`
- `docs/decisions.md` — `D-four-screens`, `D-dogfood-first`, `R-multi-tenancy`, `T-a-check-must-be-seen-to-fail`
