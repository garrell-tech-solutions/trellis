# Handoff brief — `pool-screen`

**Date:** 2026-08-21 · **Issue:** #92 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **The design is checked in.** `docs/design/Trellis.dc.html`, four screens, with `docs/design/README.md` on how to read it. **Read the canvas before specifying** — this brief describes what it draws, and where the two disagree the canvas wins on layout.

---

**Dogfood: the first Menu tab, and the first screen the design draws that has no backend.** `D-four-screens` · `docs/design/Trellis.dc.html`.

**Read the canvas.** It is checked in now — the Pool screen is the `isPool` guard. Everything below is what it draws; where this brief and the canvas disagree, **the canvas wins on layout** and you should say so.

## Goal

**Pool tasks grouped by context tag, so being at Home Depot is actionable.**

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

On a phone: capture and triage three errands tagged `@homedepot`, two `@supermarket`, one untagged. Tap **Pool**.

```
POOL                                    6 loose

WORTH A TRIP
Grouped by where you would do it.

  @homedepot          3
    buy screws · return the drill · pick up trim
  @supermarket        2
    milk · coffee

  (loose)             1
    fix the door latch
```

## What the canvas specifies

- **Title `Pool`** with `poolMeta` to its right.
- **`Worth a trip`** — a `SectionHeading`, subtitled *"Grouped by where you would do it."*
- **A loose section** under `looseLabel` for tasks with no context tag.
- **Empty state:** *"Nothing in the pool. Whatever you sort here gets grouped by context."* with **`Go to Capture →`**.

**`hint-placeholder-*` attributes in the canvas are sample data, not specification.** They exist so it renders.

## The tab bar has no renderer — you rebuild it

#88 deleted `platform::nav` along with the six pages, and #87's stylesheet still carries the tab bar it was styled for: `header nav a`, sticky bottom, `order: 2`, equal-width, uppercase 9.5px, current marked by `aria-current="page"`.

**This slice restores the header with two live tabs — Capture and Pool.** `T-nav-is-the-site-map` needs no amendment: the header is the route table, and there will be two routes. **Quota and Committed arrive with their own slices, not as dead links.**

## Scope

**In:** a `/pool` route and screen; grouping by context tag; the loose group; the empty state; the tab bar rebuilt with Capture and Pool.

**Out:** Quota and Committed tabs, at/by, quota reshaping, hour logging, manual priority ordering *(the canvas draws no reorder control — if you find one, say so)*, and **anything the canvas does not draw.**

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-four-screens` | Four screens in a bottom tab bar. The canvas is authoritative on **layout**; `docs/decisions.md` on **behaviour**. **Raise disagreements rather than resolving them quietly.** |
| `D-menu-is-a-worklist` | Container superseded, behaviours intact: **no solver, no backward pass, no splitting, no pins**, and **no settings page — controls inline.** |
| `D-context-tags-are-the-taxonomy` | Context tags are the only taxonomy. **This slice is the first consumer of the grouping key #82 landed.** |
| `T-qa-binds-tolerantly-to-markup` | **New, and this slice is a stylesheet-heavy screen.** Bind QA to ids, `data-` attributes or classes it owns — never attribute order, adjacency, or copy quoted verbatim. Three sets of scripts have already broken on this. |
| `T-latency-is-a-qa-assertion` | Wall-clock budgets live in QA, against a real server on a quiet machine. |
| `D-no-pool-on-calendar` | Pool is never placed. This screen is the *only* place pool work is offered. |
| `T-templates-take-view-models` · `T-forms-swap-one-fragment` · `T-422-is-product-wide` · `T-capability-owns-its-queries` · `T-one-front-door-per-capability` | Unchanged. |

## Acceptance scenarios worth specifying

- Pool tasks appear **grouped by context tag**, each group counted.
- **Untagged pool tasks appear in the loose group** — they must not vanish.
- **Case-folded grouping holds**: `@HomeDepot` and `@homedepot` are one group (#82 settled this, and **this screen is where the failure it prevents would have been visible** — two Home Depot lists sending the owner twice).
- **Committed and quota tasks do not appear here.**
- The empty state renders when nothing is pooled.
- **The tab bar shows Capture and Pool, and marks the current one.** *(`one_screen.feature`'s scenario 01 asserts removed routes 404 and its `path` column is not under test — mutating `/stats` to `/statX` still 404s. **Adding `/pool` gives it a `200` row and makes the column falsifiable.** #90 deferred that fix to this slice.)*
- Hostile text in a context tag stays escaped.
- **All 14 existing features pass.**

## Known repo gotchas

1. **`trunk` is green.** Merge `trunk` before your final measurement, not after — it has moved mid-slice repeatedly.
2. **Expect 14 features.**
3. **PR #87's drift was invisible to CI three times.** A restyle touched three templates and no test surface, so nothing noticed. **You are adding a screen to that stylesheet — re-run QA and believe the result.**
4. **`platform/boundary.rs` asserts `capabilities.len() >= 5` and finds six, one of which is `platform`** — documented as *"not a capability"*. It passes by counting a non-capability. **Adding one helps; know the floor is at its limit.**
5. **A new step module means a new dispatch row**, CI-gated on exact scores.
6. **Four named Gherkin traps:** a property is only as strong as the inputs its generator can produce; *if a scenario's point is that nothing happens, its parameters are not under test*; asserting only the outcome where several causes collapse into it; reaching the right end state by the wrong path.
7. **No browser automation.** Say what is uncovered.
8. `trunk` expects four required status checks. Base **`trunk`**; scratch in `./tmp/`.
9. **Open the pull request when QA is done.**

## Open questions

Answer in the pull-request body, in a table, with reasoning, and **name any `T-` row you need in your handoff note.**

1. **What is `poolMeta`?** The canvas shows a count to the right of the title. Total pool tasks, loose tasks, or groups? **Say what you chose and why the number is the useful one.**
2. **How are groups ordered, and items within them?** The canvas draws no control. Most-items-first surfaces the worthwhile trip; alphabetical is stable; newest-first matches the inbox. **`D-menu-is-a-worklist` says manual priority — which the canvas does not draw. Flag the gap rather than inventing a control.**
3. **What is the loose group called, and where does it sit?** The canvas binds `looseLabel` without fixing the text. It must not read as a context tag, and it should not be top.
4. **Where does the tab bar live now that `platform::nav` is gone?** It was `platform::nav`; `D-four-screens` makes the header the route table again. Rebuild there, or somewhere the canvas implies? **Its shape will be inherited by Quota and Committed.**

---
## Source
`docs/design/Trellis.dc.html` — the `isPool` guard · `docs/design/README.md` · `docs/decisions.md` — `D-four-screens`, `D-menu-is-a-worklist`, `D-context-tags-are-the-taxonomy`, `T-qa-binds-tolerantly-to-markup` · #82 (merged, PR #91) — the grouping key · #90 — the deferred `one_screen` fix
