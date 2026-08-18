# Handoff brief — `app-shell`

**Date:** 2026-08-17 · **Issue:** #58 · **Milestone:** none (sequenced ahead of M2) · **Route:** pipeline

> **M1 closed today** (PR #64). This is the first slice after it, and it is deliberately not an M2 slice: M2 adds at least four more pages, and this is what makes them reachable as they land.

---

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

Open `http://localhost:8080`:

1. There is a **header**.
2. Click **Life areas**. You are on the life-areas page, and the header shows which page you are on.
3. Click **Stats**, then **Inbox**. Back where you started.
4. **At no point did you type a URL.**

That is the whole demo, and it is worth the trip: the product has three pages and, until this slice, no way to move between them.

## Goal and scope

**Give Trellis a navigation header, from one definition, and with it the product's first shared layout.**

### Why this is a slice and not a chore

`D-visible-slices` judges a slice by whether the owner can run it and watch the new behaviour happen. Each of the three page slices passed that test **individually** — and the product got *less* navigable with every page added, because no slice was responsible for the whole. The third page made it worse than the second. This is the first slice whose subject is the product rather than a feature of it.

It is also the **first shared-layout decision**, and this project has a specific bad record with those: `T-forms-swap-one-fragment` had to be recorded retroactively because `triage-from-page` made exactly this class of call — the first error-display pattern — and answered it in a template and a commit message. Every page Trellis will ever have inherits what you choose here.

### The three pages

```
GET /             the inbox      (inbox.html)
GET /stats        the ratio      (stats.html)
GET /life-areas   management     (life_areas.html)
```

### One duplication to absorb, and it is the strongest argument for the base template

`T-forms-swap-one-fragment` records the htmx 422 override as a one-line global configuration living in `inbox.html`, and flags its cost explicitly: *"422 is a swappable status for every htmx request on it, present and future."*

**It is now in two templates.** `life_areas.html:8` and `inbox.html:11` both carry:

```js
htmx.config.responseHandling.unshift({code: "422", swap: true});
```

The third page to grow a form copies it again, and the first one to forget it breaks its own error handling in a way no test currently catches. **A base template is where this belongs** — but hoisting it makes the override genuinely global to the whole product, including `stats.html`, which has no forms and never asked for it. That is a decision, not a move. See open question 3.

### Out of scope — do not absorb

**Any visual design system.** There is still no visual direction in this repo and inventing one inside this slice would bury an unreviewable decision. This is a row of links. **Plain and ugly remains correct.**

Also out: a guardrails link (that page does not exist — #59 adds it *and* its link), breadcrumbs, a footer, a mobile menu, active-state animation, restyling any page's content, and touching what any page does.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `T-forms-swap-one-fragment` | **Read the cost paragraph before touching the 422 config.** The guard it names is that *422 means exactly "validation rejection, body is the re-rendered fragment" everywhere in this product*. Widening its scope widens that obligation. |
| `T-templates-take-view-models` | Templates render `http::view` models, never `store` row types. Whatever tells a template which nav entry is current is a **view** concern, not a handler passing a magic string. |
| `T-package-by-business-domain` | The tree names capabilities. **A shared shell is not one** — `platform/` exists and is *"named to say 'not a capability' out loud."* It already holds the composition root, the clock, the vendored assets and the shared response mapping. |
| `T-one-front-door-per-capability` | A capability read by others exposes one function, in its `mod.rs`. **Newly enforced** — see gotcha 3. |
| `T-inbox-owns-membership` | `inbox::lists` is now **private**. If you need the `#lists` fragment, it is `inbox::render_lists`. |
| `D-visible-slices` | Every slice ends in something the owner can run and see. |
| **No visual design system** | Plain and ugly remains correct. |

## Acceptance scenarios worth specifying

Yours to structure. These are the behaviours that matter.

- Every `GET` page carries the same header, **from one definition** — a fourth page added later must not require copying a nav.
- The header links all three pages, and each link goes where it says.
- **The current page is indicated, and the indicator is correct on each of the three.** The most likely bug is a page that thinks it is a different one.
- **Each page's own content is unchanged.** This adds a frame; it restyles nothing.
- **Every existing acceptance feature passes untouched.** Fifteen of them. Capture, triage, rejection, dismissal, the inbox, life areas, `/stats` — every fragment swap still targets what it targeted. If one needed changing, the frame leaked into the content.
- The 422 swap behaviour is unchanged **on the pages that had it**, and whatever you decide for `stats.html` is asserted rather than incidental.
- Hostile text stays escaped. The header renders no user data today; assert that it stays that way.

## Known repo gotchas

1. **`trunk` is green at `5a3247e`** (the #64 merge). Confirm CI on your base commit before starting.
2. **Expect 15 features** before your change — `dismiss_capture` landed today. Run the analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`. The first two report nonsense without the generated acceptance entrypoints.
3. **`platform/boundary.rs` has a fourth check as of today:** no capability names another capability's `store` in production. It also still enforces that no persistence module names `axum` or `StatusCode`, that nothing outside a `store.rs` writes production SQL, and that no top-level directory carries a technical-role name. **It walks `src/`, so anything you add is covered the moment it exists.**
4. **`inbox::lists` went private today** and `inbox::store`'s membership queries are `pub(super)`. Reaching past the inbox's front door does not compile.
5. **A new step module means a new dispatch row, and there is a CI gate on it.** `scripts/ci/complexity-baseline.json` pins the accepted over-threshold set with **exact** scores and fails when the set grows, when a score moves in either direction, or when a row goes stale. **Keep every dispatch arm a one-line delegation** — put logic in the handler. Do not flatten any dispatcher into a `(Regex, handler)` table; that was settled 2026-08-12 and the threshold is in the constitution.
6. **`cargo test --workspace` compiles zero acceptance tests on a fresh checkout** — entrypoints are gitignored. CI gates the real suite; locally run `scripts/acceptance/run.sh` yourself.
7. **DRY is at 2.82% against a threshold of 3** — its highest since #50 made the gate measure code rather than prose. A base template that removes the duplicated 422 line helps; templates copied per page would not. Watch the number.
8. **#66 is open and not yours:** `capture-endpoint-persists-quickly-01` asserts a 50 ms wall-clock budget and is load-sensitive. If it fails under load during your run, that is #66 and not a regression — confirm by re-running quiet, and say so rather than chasing it.
9. **You need no migration.** `0005` is taken; nothing here touches the schema. If you find yourself writing one, something has gone wrong.
10. **Don't copy `task_kinds.feature`'s step style** — its regexes match the placeholder `"<(\w+)>"`, so one Examples cell feeds both request and assertion and the scenario cannot fail under mutation. `committed_triage_validation` does it correctly.
11. **A scenario asserting invariance under exactly the transformation the mutator applies is untestable by that mutator and looks fully covered.** Found on `life-areas-duplicate-03`. If a scenario here is about sameness across pages, consider literal scenarios per page.
12. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
13. **Open the pull request when QA is done**, before taking another brief.

## Open questions for you

Settle these and **say so where the decisions log can pick it up** — PR #64 answered its four in a table in the pull-request body, which worked well and is the pattern to copy.

1. **Askama template inheritance (`{% extends %}`) or an included partial?** Inheritance makes "every page carries the header" structural rather than remembered, which matters because the failure mode is a page added later that forgets. Say which and why; every future page inherits it.
2. **How does a page declare which nav entry is current?** A field on every template struct is obvious and means touching all three. `T-templates-take-view-models` says this is a view concern. **This is the part most likely to be got wrong in a way that only shows up on page five** — when adding a page means editing the nav, the shell has failed at its one job.
3. **Does the 422 override move to the base template?** It is currently duplicated in `inbox.html` and `life_areas.html`. Hoisting it deduplicates a global config and extends it to `stats.html`, which has no forms. Leaving it per-page keeps the blast radius small and keeps the duplication. **`T-forms-swap-one-fragment`'s guard — 422 means exactly one thing product-wide — argues for hoisting**, since a rule with two copies has two chances to drift. Decide it, don't inherit it.
4. **Plain `href`, or `hx-boost`?** Plain is conservative and almost certainly right. `hx-boost` turns every navigation into an htmx request, which **interacts directly with question 3**: an endpoint returning 422 during a boosted navigation would have its body swapped into the DOM. Record the reason either way.
5. **Does `/stats` belong in the nav at all?** It is a read-only instrument (R2, #20), not a place you work. **#62 will ask the same question about a capacity page**, and the two answers should agree — so whatever you decide, state the rule rather than the instance.

## Dependencies and sequencing

- **Nothing blocks this.** #48 merged, `trunk` is green, the pipeline is empty.
- **Sequenced ahead of M2** (#10): #59–#62 add four more pages, and each arrives reachable if this lands first.
- **#59 adds the guardrails link** when that page exists — do not add a dead one here.
- **#63 and #65 are ops PRs** that may land independently; #63 touches module visibility and #65 would move templates. Neither is queued against this, but if #65 ever runs, this slice's template layout is what it moves.

## Source

- Issue **#58** — acceptance criteria and the demo
- `docs/decisions.md` — `T-forms-swap-one-fragment` above all, plus `T-templates-take-view-models`, `T-package-by-business-domain`, `D-visible-slices`
- `docs/design/architecture.md` — the module boundary, the front doors, and the route table
- **#10** — M2, which this unblocks the usability of
- **#66** — the flake that is not yours
