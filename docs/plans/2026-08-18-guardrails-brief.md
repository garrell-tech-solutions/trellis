# Handoff brief — `guardrails`

**Date:** 2026-08-18 · **Issue:** #59 · **Milestone:** M2 — Guardrails + Free-slot Algebra · **Route:** pipeline

> **M2 slice 1 of 4**, and the first slice of a new milestone. **The first time Trellis knows anything about *when*.**
>
> M1 closed on 2026-08-17; `app-shell` (#58, PR #67) merged after it. Everything M2 builds reads what this slice defines.

---

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

Open `http://localhost:8080` and click **Life areas** in the header:

1. Each of the five life areas shows **"no guardrail"**.
2. Give **Work** `Mon–Fri 09:00–17:00`. It appears on the page.
3. Give **Fitness** `Mon/Wed/Fri 06:00–07:00` **and** `Sat 09:00–11:00` — **two bands, one life area**.
4. Add **Side project** and mark it **pool-only**. It is well-formed with no hours at all.
5. Try to leave **Home** with neither a guardrail nor a pool-only mark — **refused**, and the message says which of the two is missing.
6. Restart the server and reload. Everything is still there.

**Step 5 is the one worth the trip.** `T-life-areas-are-data` has said since 2026-08-14 that *"a life area is well-formed only once it has a guardrail, or is explicitly marked pool-only — enforced at M2, when guardrails exist."* This is when that stops being a promise.

## Goal and scope

**Each life area carries its own weekly guardrail — the hours in which its work may be scheduled.** A life area with no guardrail must be explicitly marked pool-only. A life area that is neither is refused.

### The model, settled — read `D-life-area-owns-its-time` before modelling anything

```
Work         Mon-Fri 09:00-17:00
Fitness      Mon/Wed/Fri 06:00-07:00, Sat 09:00-11:00
Learning     Mon-Fri 20:00-22:00
Family       Sat-Sun 10:00-18:00
Home         Sat 09:00-12:00, Sun 14:00-17:00
Side project pool-only — no guardrail, never placed, only offered by the menu
```

A task goes in its own life area's hours **by default** and may not go outside them. Borrowing another life area's hours is an explicit **per-task** permission — that is M3's, when there is a scheduler to honour it, and **not this slice's**. Guardrails **may overlap** in clock time; where they do, their life areas compete, resolved at scheduling. Fitness's 06:00 is protected because nothing else claims it, not because a rule protects it.

### This slice owns the timezone, and that is the hidden cost

**Trellis has no concept of a timezone anywhere.** `/stats` deferred it explicitly — its window is *"14 × 24h from `tasks.created_at_ms`, no timezone, zone deferred to M2"* (#45). A civil wall-clock guardrail is meaningless without one: `Mon 09:00` is not an instant until you know where the owner is.

So this slice introduces it, and it is the smallest surface that can. `D-single-user` means **one zone**, not one per guardrail. See open question 3 — this is the part most likely to be underestimated.

### Out of scope — do not absorb

`free_intervals` and any interval subtraction (**#60**), dated exceptions (**#61**), the capacity view (**#62**), anything about *placing* work, `allowed_windows` / cross-life-area borrowing (M3), DST handling beyond storing civil time honestly (#60 owns it), and **any visual design system**. Plain and ugly remains correct.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-life-area-owns-its-time` | One guardrail per life area; own hours by default; borrowing is per-task; guardrails may overlap and compete; no guardrail means pool-only. **The whole model. Read the row.** |
| `T-life-areas-are-data` | Life areas are user-managed rows; adding one is never a development task — and now neither is walling it. |
| `D-guardrails-never-yield` | A guardrail is never breached, not even by a P1 hard deadline. **Nothing in this slice should make an override representable.** |
| `T-jiff-epoch-millis` | `jiff`; civil and zoned are distinct types. Guardrails are **civil wall-clock**. Do not store a civil band as an epoch instant and leave #60 with nothing to fix. |
| `T-nav-is-the-site-map` | **Every page in the route table gets a header link.** If this slice adds a page, it adds its nav entry — that is settled and not a judgment call. |
| `T-422-is-product-wide` | `422` means exactly one thing everywhere: a validation rejection whose body is the re-rendered fragment. The override is global in `base.html`. **An endpoint that cannot honour that must not return 422.** |
| `T-forms-swap-one-fragment` | Forms inline in the row they act on; one shared fragment per region with one id; rejection re-renders that fragment. |
| `T-templates-take-view-models` | Templates render `http::view` models, never `store` rows. |
| `T-capability-owns-its-queries` | A business domain owns the SQL it issues, not the table it touches. |
| `T-one-front-door-per-capability` | A capability read by others exposes one function in its `mod.rs`. **Now enforced** — see gotcha 4. |
| `T-migrations-append-only` | New migration only, CI-enforced. `0005` is taken; **you take `0006`.** |
| **Core vs adapter** | A rule that survives changing HTTP belongs in `scheduler-core`. *Is this band well-formed? Do a life area's own bands overlap? Is this life area well-formed?* are all that shape. |
| **No visual design system** | Plain and ugly remains correct. |

## Acceptance scenarios worth specifying

- A life area's guardrail is a **weekly** mask of civil wall-clock bands, authored and edited from the running app. **Multiple bands per life area, multiple days per band.**
- A life area may instead be marked **pool-only** — well-formed with no hours.
- **A life area that is neither walled nor pool-only is refused, and the rejection names which is missing.**
- Guardrails survive a restart.
- An **archived** life area keeps its guardrail rather than losing it (`D-kill-means-archive` — keep the row).
- Guardrails are civil wall-clock: `09:00` means nine o'clock in the owner's zone on whatever day it lands.
- Hostile text in any field the guardrail form accepts stays escaped.
- **Every existing acceptance feature passes untouched.** Sixteen of them. If one needed changing, something got rebuilt that should have been reused.

## Known repo gotchas

1. **`trunk` is green at `18c3ab2`** (the #67 merge). Confirm CI on your base commit before starting.
2. **Expect 16 features** before your change. Run the analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh` — the first two report nonsense without the generated acceptance entrypoints.
3. **Adding a page is now cheap and structural, so do it properly.** `platform::nav` holds `Page`, and `label`/`path` are exhaustive matches, so the compiler will send you there. **The one step the compiler cannot force is adding your variant to `nav::ALL`** — and `app::every_header_link_reaches_the_page_it_names` walks that list through the real router. A page missing from `ALL` compiles and ships with no link.
4. **`platform/boundary.rs` enforces four rules**, walking `src/`: no persistence module names `axum` or `StatusCode`; nothing outside a `store.rs` or `platform/db.rs` writes production SQL; no top-level directory carries a technical-role name; **no capability names another capability's `store` in production**. Anything you add is covered the moment it exists.
5. **`inbox::lists` is private** and `inbox::store`'s membership queries are `pub(super)`. Reaching past a front door does not compile.
6. **Templates now `{% extends "base.html" %}`** and receive `nav: Vec<NavLink>`. A new page follows that shape; do not hand-roll `<html>`.
7. **A new step module means a new dispatch row, and there is a CI gate on it.** `scripts/ci/complexity-baseline.json` pins the accepted over-threshold set with **exact** scores and fails when the set grows, when a score moves *in either direction*, or when a row goes stale. **Keep every dispatch arm a one-line delegation.** Do not flatten any dispatcher into a `(Regex, handler)` table — settled 2026-08-12, and `T-complexity-8`'s threshold is in the constitution and cannot be raised.
8. **DRY is at 2.78%, threshold 3.** This slice adds a page, a form and a store; watch it. A guardrail band form repeated per weekday is the obvious way to cross it.
9. **`cargo test --workspace` compiles zero acceptance tests on a fresh checkout** — entrypoints are gitignored. Run `scripts/acceptance/run.sh` locally yourself.
10. **#66 is open and not yours:** `capture-endpoint-persists-quickly-01`'s 50 ms budget is load-sensitive. If it fails under load, re-run quiet and say so rather than chasing it.
11. **Don't copy `task_kinds.feature`'s step style** — its regexes match `"<(\w+)>"`, so one Examples cell feeds both request and assertion and the scenario cannot fail under mutation. `committed_triage_validation` does it correctly.
12. **A scenario asserting invariance under exactly the transformation the mutator applies is untestable by that mutator and looks fully covered.** Found on `life-areas-duplicate-03`, and `app_shell` handled it by writing literal scenarios per page. If a scenario here is about sameness across weekdays, consider the same.
13. **There is no browser automation in this stack.** Anything that can only be seen in a browser is uncovered; say so rather than implying otherwise.
14. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
15. **Open the pull request when QA is done**, before taking another brief.

## Open questions for you

Settle these and **answer them in the pull-request body, in a table, with reasoning** — PR #64 established that and #67 repeated it, and it is the reason this project's rationale log has stopped being written from archaeology.

1. **How is a weekly mask stored and authored?** A row per band (`life_area_id, weekday, start, end`) is the obvious storage and makes "Mon–Fri 09:00–17:00" five rows. **Authoring matters as much as storage** — five separate forms to say "weekdays" is friction the owner pays every single time, and `D-menu-of-three`'s instinct is to remove friction everywhere except the one place it is a feature.
2. **May a life area's own bands overlap or touch?** `Mon 09:00–12:00` plus `Mon 11:00–17:00` is either a mistake to reject or a shorthand to merge. Say which, and where the check lives. Note this is *within* one life area — overlap *between* life areas is settled as legal.
3. **Where does the owner's timezone live?** There is no settings surface and no `settings` table. Options: a one-row settings table, an env var or CLI flag beside `--now`, or a column on something that exists. **A flag is cheapest and least honest** — the zone is data the owner changes when they move, not deployment config. Every civil-time feature from here reads whatever you choose, so this is the decision in this slice with the longest reach.
4. **Is "pool-only" a column, or the absence of any band?** Absence is one fewer field and cannot disagree with itself. A column says it out loud and distinguishes *deliberately pool-only* from *not finished setting up*. `T-archived-at-only` is the precedent for one signal — but it argued against two fields for **one** state, and these may be two states. `T-capture-leaves-inbox-once` made the same call the other way last week and is worth reading alongside it.

## Dependencies and sequencing

- **Nothing blocks this.** #58 merged, `trunk` green, pipeline empty.
- **Blocks the rest of M2**: **#60** (free time) needs guardrails to project, **#61** (exceptions) needs #60, **#62** (capacity) needs #61.
- **#63 and #65 are ops PRs** that may land independently. #63 touches module visibility — if it lands first, your new capability is inside a stricter boundary.
- Depends on #47 (PR #57) for the life areas being walled, and #58 (PR #67) for the page frame.

## Source

- Issue **#59** — acceptance criteria and the demo
- Issue **#10** — M2 epic, the cut, and the two costs flagged during it
- `docs/decisions.md` — `D-life-area-owns-its-time` above all, plus `T-life-areas-are-data`, `D-guardrails-never-yield`, `T-jiff-epoch-millis`, `T-nav-is-the-site-map`, `T-422-is-product-wide`
- `docs/design/architecture.md` — Guardrails and free time; Capacity; the module boundary
- **#6** (closed) — the contradiction this model resolves, and why "Personal" in it is not a life area
