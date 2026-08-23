# QA Procedure: Trellis installs to the home screen

Covers: `features/installable.feature`

## Interface used

The three screens over HTTP, the manifest and icons at whatever URLs the
pages link, read-only `sqlite3` (not needed here — this slice stores
nothing), and **a real phone, for the half that cannot be automated at all.**

## What can be checked, and what cannot

**Checkable over HTTP**, and asserted by the feature: the `<link>` on every
screen, the manifest document, its members, and whether every icon it names
actually resolves at the type it claims.

**Not checkable by anything in this project:**

- **Whether the browser offers to install.** That judgement is the browser's,
  it depends on engagement heuristics that vary by version, and no headless
  driver here can make it.
- **How an installed window behaves.** `scripts/qa/phone_layout.cjs` asserts
  the tab bar sits on the viewport's bottom edge — but it drives **a tab**.
  It cannot open a standalone window, so the safe-area behaviour that only
  appears once installed is **unverified by anything automated.**

**Say both plainly in the report.** Three phone-first screens already shipped
on a layout nobody had looked at; the correction is to name what was not
seen, not to imply the new browser check covers more than it does.

## By-hand walkthrough — this is the slice, and it cannot be demonstrated any other way

1. Open Trellis over the tailnet on the phone. Label the pull request
   `preview` and the box puts the branch on a second port within two minutes.
2. Browser menu → **Install app** / *Add to home screen*. **Confirm the
   option is offered.**
3. Confirm it lands in the launcher with **the icon**, not a screenshot of
   the page and not a generic globe.
4. Open it from the launcher. **Confirm no address bar and no browser
   chrome**, and that it has its own entry in the app switcher.
5. **Confirm the tab bar is still on the bottom edge, above the home
   indicator** — not under it, not floating above a gap. **This is where an
   installed window differs from a tab, and where this is most likely to look
   wrong.**
6. Scroll Capture. Confirm it still scrolls, in the installed window.

### Expected Observable Outcomes
- All six steps hold.
- **Step 5 is the one to look hardest at.** `viewport-fit=cover` is already
  set, so the page paints into the safe area; whether the tab bar respects it
  is the question, and no automated check in this project can answer it.
- **Step 3's icon is provisional and is meant to be.** A `T` in the product's
  own typeface on the primary green. A new palette and a dark mode are in
  flight, and a manifest's colours are literal values that cannot be CSS
  custom properties — so this will want redrawing. **Report whether it reads
  at launcher size; do not report that it is the wrong colour.**

## Procedure — the manifest is linked and served

1. Fetch each of the three screens.
2. Extract the manifest link from each; fetch what it points at.

### Expected Observable Outcomes
- **All three link a manifest**, and all three link **the same one**.
- It parses as JSON and carries `name`, `short_name`, `start_url` and
  `display: standalone`.
- **Read the URL out of the page rather than assuming it**
  (`T-qa-binds-tolerantly-to-markup`). Static assets in this project are
  explicit routes, and the path is not the contract.

## Procedure — every icon resolves

1. From the manifest, take every entry in `icons`.
2. Fetch each one. Check the status, the content type, and that the bytes are
   actually a PNG rather than an HTML error page with a 200.

### Expected Observable Outcomes
- **Every icon named is served**, as `image/png`, at 192 and 512.
- **One carries `purpose: maskable`.** Without it a launcher crops the icon
  into a shape nobody chose — the icon is inset inside the safe zone for
  exactly this reason.
- **A manifest naming an icon that 404s is the failure mode here**, and it is
  silent: the browser falls back to a screenshot and nothing errors. Check
  the bytes, not just the status.

## Procedure — no service worker

1. Fetch each screen and read the raw HTML.
2. Try the paths a service worker would conventionally live at.

### Expected Observable Outcomes
- **Nothing registers one, and none is served.**
- **This absence is deliberate and the pull request explains it.** An
  installed app that cannot reach the tailnet shows nothing; caching the
  shell would wrap an app frame around an empty screen, failing as a broken
  app rather than as a page that did not load. **If you find one, that is a
  defect — a coder reading "installable" would add it in good faith.**

## Procedure — prove the check can fail

**`T-a-check-must-be-seen-to-fail`, recorded 2026-08-23, and this slice is
the shape it was written for: a manifest that fails to parse does nothing
silently.**

1. Break the manifest — malform its JSON, or point an icon at a path that
   404s.
2. Run the suite. **Confirm it fails, and read what it says.**
3. Restore. Confirm it passes. Confirm a clean diff.

### Expected Observable Outcomes
- **It fails, and the message names what broke** — an unparseable manifest
  and a missing icon should not produce the same error.
- **Say in the report that you did this**, and which two breakages you used.
  A check nobody has seen fail is a check nobody has shown to work, and this
  project has shipped a proptest that could not fail, nine mutants surviving
  a scenario that could not fail, and a browser check that went red for a
  reason nobody predicted.

## Procedure — nothing else changed

1. Run all eighteen existing QA suites and the acceptance suite.

### Expected Observable Outcomes
- **All 18 acceptance features pass untouched.** This slice adds a document,
  some bytes and two meta tags; it changes no screen's content.
- `phone_layout` still passes — the new `<link>` and `<meta>` are in the same
  `<head>` its assertions render through.

## Independent of Implementation

This procedure depends only on what the pages link, what the manifest
declares, and whether what it names is served. It does not depend on the
manifest's URL, on how the icons are embedded in the binary, or on which
routes serve them.
