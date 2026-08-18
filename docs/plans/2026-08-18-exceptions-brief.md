# Handoff brief — `exceptions`

**Date:** 2026-08-18 · **Issue:** #61 · **Milestone:** M2 — Guardrails + Free-slot Algebra · **Route:** pipeline

> **M2 slice 3 of 4.** `free-time` (#60, PR #70) merged, so `free_intervals(guardrail, range)` exists with a 1000-case proptest behind it.
>
> **This is the first thing in Trellis that ever subtracts from a guardrail.** Everything to date has only ever *added* availability. `free_intervals` currently subtracts nothing, and its own module header says so: *"M2 has none of those — dated exceptions arrive at `#61`, pins at M3, calendar busy at M4."* You are the first subtrahend, and the shape you choose is the one M3's pins and M4's calendar busy will follow.

---

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

1. Give **Work** `Mon–Fri 09:00–17:00`. Open **Free time** — the next fortnight shows **80h** of Work.
2. Mark **20–24 August** as away.
3. Reload. **Those five days are gone from Work's free time**, and the total is **40h**.
4. Restart the server. Still gone.

**Step 3 is the point.** It is the first time anything in this product removes availability, and the free-time page shows it happening.

## Goal and scope

**A weekly guardrail is the normal week. This is how the owner says a particular week is not normal.**

### Where it plugs in

`scheduler_core::free_time` is the home:

```rust
pub struct Interval { start_ms: i64, end_ms: i64 }   // half-open, instants
pub struct Range { start: Date, end: Date }
pub fn free_intervals(guardrail: Guardrail<'_>, range: Range) -> Vec<Interval>
```

`free_intervals` today projects and sorts, and subtracts nothing. **Adding subtraction is this slice's core work**, and it should be shaped so that M3's pins and M4's calendar busy are additional inputs of the same kind, not a second mechanism. `Interval` is deliberately not named after a producer, for exactly this reason.

### Carried forward from PR #70 — an acceptance scenario I owe you

The architect found that `life_areas::guardrails` handed out bands **regardless of `pool_only`**, and `/free-time` reported **32h for a life area the owner had marked never scheduled**. Fixed there. But the specifier identified why its own scenario could not have caught it, and **deferred the regression test to this slice** because #61 is already editing these intervals and adding it to #70 would have staled a freshly-stamped mutation manifest:

> `free-time-empty-is-an-answer-03` marks a life area never-scheduled **from a clean state**, so it reports 0h whether the rule is honoured or not. Catching this needed a life area that **has bands and is then marked**.

**Please carry that ordering as a scenario.** The general shape is worth holding onto beyond this one case: *a scenario that reaches the right end state by the wrong path can be green against an implementation that enforces nothing.*

### Out of scope — do not absorb

The capacity view (#62), `allowed_windows` / cross-life-area borrowing (M3), placing anything, pins, **calendar busy — M4 owns that and this is not an early version of it**, recurring or annual exceptions, and any visual design system. **Do not build a calendar picker.**

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-guardrails-never-yield` | A guardrail is never breached. **An exception narrows availability by the owner's deliberate act — it is not a mechanism for work to escape a wall.** Open question 2 is where that could go wrong; read it. |
| `T-fold-counts-both-passes` | A repeated wall-clock hour inside a guardrail is free time **twice**; the spring-forward hour is not counted. **An exception range crossing a transition is a real case**, not an edge case — and the totals in `free_time.feature` are what would change if you get it wrong. |
| `T-free-time-horizon-fourteen-days` | The horizon is fourteen days **by rule** — exactly two weeks, so every weekday appears exactly twice and every total in the feature is a fixed number. An exception outside the window changes nothing; one inside it changes a stated total. |
| `T-jiff-epoch-millis` | Civil wall-clock at the boundary, UTC epoch millis in storage. "20 August" is a **civil date in the owner's zone**. |
| `T-timezone-is-a-setting` | One stored zone, read through `settings::current_timezone` — never `settings::store`, or `platform/boundary.rs` rule 4 fails the build. |
| `D-life-area-owns-its-time` | Guardrails may overlap between life areas. Relevant to open question 1. |
| `D-kill-means-archive` | This project keeps rows. A cancelled or past exception is very likely archived rather than deleted — confirm, and say what a delete-looking action actually does. |
| `T-migrations-append-only` | New migration only, CI-enforced. **You take `0007`.** |
| `T-nav-is-the-site-map` | **If you add a page, you add a `nav::Page` variant *and* a row in `nav::ALL`** — and then `app_shell` **must** change. See gotcha 3. |
| `T-422-is-product-wide` | `422` means exactly one thing: validation rejection, body is the re-rendered fragment. |
| `T-forms-swap-one-fragment` · `T-templates-take-view-models` · `T-capability-owns-its-queries` · `T-one-front-door-per-capability` | Unchanged, all four. |
| **No visual design system** | Plain and ugly remains correct. |

## Acceptance scenarios worth specifying

- A date or date range is marked as an exception from the running app, and **it changes what `free_intervals` returns for those dates**.
- An exception survives a restart; one whose dates have passed stops mattering with no cleanup step.
- **Scope is explicit and visible** — an exception applies to one life area or to all of them, and which is shown on the page rather than implied. See open question 1.
- **Overlapping exceptions produce no negative-length interval and no double subtraction.** The proptest in `crates/scheduler-core/tests/free_time_properties.rs` already asserts disjoint, sorted and positive-duration — **extend it rather than writing a second**, so subtraction is covered by the same property that covers projection.
- An exception crossing a **DST transition** behaves per `T-fold-counts-both-passes`.
- **The deferred ordering scenario**: a life area that has bands and is *then* marked never-scheduled reports no free time.
- Hostile text in an exception's label stays escaped.
- **All 19 existing acceptance features pass untouched** — unless you add a page, in which case `app_shell` legitimately changes and 18 are untouched.

## Known repo gotchas

1. **`trunk` is green at `5fda4de`** (the #70 merge). Confirm CI on your base commit — and note `trunk` has moved twice mid-slice in the last week (#68 during `guardrails`, my decisions commits during others). **Rebase or merge `trunk` before your final measurement**, not after: #55 took `trunk` red 24 seconds after a merge whose checks were honest about an older tree.
2. **Expect 19 features.** Analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`.
3. **If you add a page, `app_shell` must change — and if it doesn't, that is the bug.** #70 wrote that inversion into the feature and QA doc: a page missing from `nav::ALL` compiles and ships with no link, and the only thing that notices is `app_shell` *not* needing an update.
4. **⚠️ DRY is 2.93% against a fail-closed threshold of 3** (#68 closed the fail-open). #70 tripped it at 3.03% and fixed it by real deduplication rather than a waiver — **that is the standard**. And the headline understates the risk: **Rust alone was 3.43%** when last split, diluted under the line by the shell corpus (#52). Budget for deduplication.
5. **`platform/boundary.rs` enforces four rules**, walking `src/`. Read the timezone through `settings::current_timezone`.
6. **A new step module means a new dispatch row, CI-gated**, with exact scores in `scripts/ci/complexity-baseline.json`. Keep every dispatch arm a one-line delegation; do not flatten a dispatcher into a `(Regex, handler)` table.
7. **Three Gherkin traps, all of which have bitten this project:**
   - **Reaching the right end state by the wrong path** (#70, above) — the newest and subtlest.
   - **Asserting only the outcome where the implementation collapses several causes into it** (#59, twelve survivors).
   - **Echoing a placeholder in both input and assertion**, or mutating on an axis the mutator cannot move (`task_kinds.feature`, `life-areas-duplicate-03`).
8. **`cargo test --workspace` compiles zero acceptance tests on a fresh checkout.** Run `scripts/acceptance/run.sh` yourself. Property tests need `-- --include-ignored`.
9. **`trellis serve --now <RFC3339>` is an offset, not a freeze.** You need it for any dated scenario — an exception on "20 August" means nothing without a fixed today.
10. **#66 will fire under load and is not yours.** It failed at 612 ms against its 50 ms budget at load average 15.5 during #70, and passed quiet. Re-run quiet and say so.
11. **No browser automation in this stack.** Anything only visible in a browser is uncovered; say so.
12. `trunk` expects four required status checks. Base branch **`trunk`**; scratch in `./tmp/`.
13. **Open the pull request when QA is done.**

## Open questions for you

Answer them **in the pull-request body, in a table, with reasoning** — and if one of them wants a `T-` row, **say so in your handoff note the way #60 did.** That is now the cheapest way this project has of keeping its rationale honest.

1. **Is an exception per life area, or global?** *"I am away"* removes every life area's hours; *"no gym this week"* removes one. Both are real. Global-only is smaller and probably covers the common case; supporting both costs a nullable `life_area_id` whose meaning must be stated. Say what the page shows.
2. **May an exception *add* hours, or only remove them?** *"Working this Saturday"* is a real need and a different thing from an absence. **This is the one place this slice could quietly breach `D-guardrails-never-yield`** — an additive exception is an override with a friendly name, *unless* it is understood as the owner redrawing the wall for a day, which is legitimate. **Decide deliberately and record the reasoning; do not implement whichever is easier.**
3. **What does an exception do to work already scheduled into those hours?** Nothing today — nothing is scheduled until M3. But **the shape chosen here is what M6's reality-flex loop inherits**, so say what you expect rather than leaving it undefined.
4. **Whole dates only, or a time range?** Whole-date is smaller. The first time the owner wants a dentist appointment on Tuesday morning they will want the other. `T-quota-targets-required`'s precedent says require at the boundary what downstream cannot function without — but this may genuinely be a later need, and **M4's calendar busy may make half-day exceptions redundant**, which is an argument for not building them twice.

## Dependencies and sequencing

- **Nothing blocks this.** #60 merged, `trunk` green, pipeline empty.
- **Blocks #62** (capacity): its supply number is the sum of these intervals, so shipping capacity before exceptions would over-report availability on every exception day — a number wrong in the direction of *"you have more time than you do"*, which is the failure M2 exists to prevent.
- **The subtraction shape you choose is inherited by M3** (pins, `T-pins-in-constraints`) and **M4** (calendar busy).
- **#63 and #65 are ops PRs** that may land independently.

## Source

- Issue **#61** — acceptance criteria and the demo
- Issue **#10** — M2 epic and the cut
- `docs/decisions.md` — `D-guardrails-never-yield` above all, plus `T-fold-counts-both-passes`, `T-free-time-horizon-fourteen-days`, `T-jiff-epoch-millis`, `T-timezone-is-a-setting`, `D-kill-means-archive`
- `docs/design/architecture.md` — Guardrails and free time
- **#60** / PR #70 — the algebra you extend, the deferred scenario, and the ordering lesson
- **#52** — why the DRY number understates the risk · **#66** — the flake that is not yours
