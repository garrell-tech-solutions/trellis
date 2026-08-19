# Handoff brief — `schedule-forward-pass`

**Date:** 2026-08-19 · **Issue:** #75 · **Milestone:** M3 — Scheduler Core · **Route:** pipeline

---

**M3 slice 1 of 5, and the first time Trellis puts anything on a schedule.** Everything before this told you when you *could* work; this says when you *will*.

Unblocked by the decisions of 2026-08-18/19: `T-invariants-one-to-five`, `T-hard-refuses-soft-slips` (U2), `T-blocks-do-not-cross-guardrail-seams` (U4).

## Goal

**Place committed tasks into free time, and say why the rest did not fit.** Forward pass only. **No splitting, no pins, no backward pass.**

## Demo (required — `D-visible-slices`)

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

1. Give **Work** `Mon–Fri 09:00–17:00`.
2. Capture and triage a **committed** Work task — *"write the Q3 deck"*, 2h, due Friday, P2.
3. Click **Schedule**. It is placed, inside Work's hours, on a real day and time.
4. Triage a second committed task bigger than the hours that remain. Reload: it appears under **won't fit**, with a reason.
5. Restart and reload. The plan is still there.

## Scope

**In:**
- `schedule()` with the ratified seven-parameter signature — `pins`, `facts` and `prior_plan` **taken and empty**, see below
- **Forward pass**: least slack, priority as tiebreak
- The **infeasibility report** with its closed reason enum
- A **Schedule page**, and its nav entry
- **Invariants 1, 2, 4, 5** under proptest, ≥1000 cases

**Out, and each is a later slice — do not absorb:**
- **Splitting** (S3) — a task that does not fit one free interval whole is unplaceable, not chopped
- **Pins** (S4) — the question of which chunk a pin binds is still open and is why S4 is last
- **Backward pass / hard-deadline-driven placement** (S2) — needs U3
- **Regeneration and idempotence properties** (S5)
- The two-axes capacity attribution that arrived from M2
- Any visual design system. **Plain and ugly remains correct. No calendar grid.**

## On the seven-parameter signature

`schedule(tasks, busy, guardrails, pins, facts, prior_plan, now)` is the ratified contract (`T-fact-plan-line`). This slice **takes all seven and uses four**. `pins`, `facts` and `prior_plan` are accepted and empty.

**That is deliberate and should be stated in code, not worked around.** Do not ship a five-parameter function and widen it later — the signature is the contract, and `prior_plan` in particular exists because the move penalty makes the objective depend on previous placements. **But do not fake them either**: no stub pin source, no invented prior plan. Empty is the honest value at this slice.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `T-invariants-one-to-five` | **The five, ratified 2026-08-18.** 1 no two blocks overlap — with each other, or with facts or pins. 2 a block lies entirely within one allowed window. 3 conservation under splitting. 4 hard deadlines hold on placed tasks. 5 the partition is total. **1, 2, 4 and 5 are yours; 3 is vacuous here** because nothing splits. |
| `D-placed-whole-or-not-at-all` | **No partial placement.** A task that does not fit is unplaceable with a reason — never half-booked. This is what makes "won't fit" a real answer. |
| `T-hard-refuses-soft-slips` | A **hard** deadline is a feasibility constraint: cannot finish by it → unplaceable, `deadline_unreachable`. A **soft** deadline may be overrun: placed late, carrying a **projected finish**. `deadline_type` finally does something. |
| `T-blocks-do-not-cross-guardrail-seams` | A block never spans two life areas' guardrails, even adjacent ones. |
| `D-life-area-owns-its-time` | A task goes in its **own** life area's hours. Borrowing another's is a per-task permission — **`allowed_windows` is not this slice**; assume own-area-only. |
| `T-fact-plan-line` | `proposed`/`published` future blocks are the **Plan layer** — disposable, engine-written. `in_progress`/`completed`/`missed` and pins are **Constraints** — read, never written. This slice writes only Plan. |
| `T-free-time-horizon-fourteen-days` | The horizon is fourteen days **by rule**. `free_intervals` is `scheduler_core::free_time`; **use it, do not reimplement it.** |
| `T-capacity-never-under-reports-demand` | `estimated_minutes` is required at triage for committed. A task with `None` predates the column — it cannot be placed, and **the reason enum does not grow for it**, because the state cannot occur. |
| `T-nav-is-the-site-map` | A new page means a `nav::Page` variant **and** a row in `nav::ALL`. See gotcha 3. |
| **Core vs adapter** | The scheduler is the purest case in the product of *a rule that survives changing HTTP*. It belongs in **`scheduler-core`**, and `cargo tree -p scheduler-core` must still contain no `tokio` and no `sqlx`. |
| `T-422-is-product-wide` · `T-templates-take-view-models` · `T-capability-owns-its-queries` · `T-one-front-door-per-capability` | Unchanged. |

## Acceptance scenarios worth specifying

- A committed task with an estimate, a deadline and a life area with hours **is placed**, inside that life area's guardrail, within the horizon.
- **Invariant 1**: no two blocks overlap — proptest.
- **Invariant 2**: every block lies inside one free interval — proptest.
- **Invariant 4**: every placed **hard**-deadline task finishes at or before its deadline; one that cannot is unplaceable with `deadline_unreachable`. A **soft** one may be placed late and reports a projected finish.
- **Invariant 5**: the partition is **total** — every task is placed or carries a reason from the closed enum. Proptest.
- **`D-placed-whole-or-not-at-all`**: a task larger than any single free interval is unplaceable, **not** partially placed.
- A **pool** task is never placed (`D-no-pool-on-calendar`); a **quota** task is not scheduled at this milestone (M8).
- A life area marked **never scheduled** contributes no hours and its tasks are unplaceable.
- The plan **survives a restart**.
- Hostile text in a task's text stays escaped on the schedule page.
- **All 21 existing acceptance features pass untouched**, except `app_shell`, which **must** change because a page was added.

## Known repo gotchas

1. **`trunk` is green at `27e0aea`.** **Rebase or merge `trunk` before your final measurement, not after** — it has moved mid-slice repeatedly and #55 took `trunk` red 24 seconds after a merge whose checks were honest about an older tree.
2. **Expect 21 features.** Analyzers in order: `scripts/acceptance/run.sh`, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`.
3. **Adding a page: `nav::Page`'s `label` and `path` are exhaustive matches so the compiler sends you there — but `nav::ALL` is the one step it cannot force.** A page missing from it compiles and ships with no link, and the only thing that notices is `app_shell` *not* needing a change.
4. **⚠️ DRY is 2.86% against a fail-closed 3, and Rust alone is 3.55%** — over the line, held under by the shell corpus (#52). It arrived **red in two of the last four slices** and was fixed by real deduplication both times. **That is the standard, not a waiver.** Budget for it.
5. **`platform/boundary.rs` enforces four rules** by walking `src/`. Read the timezone through `settings::current_timezone`; resolve a life-area name through `life_areas::active_id_for_name`.
6. **Migration `0008` is the latest.** You need `0009` for the `block` table.
7. **A new step module means a new dispatch row, CI-gated** with exact scores in `scripts/ci/complexity-baseline.json`. Every arm a one-line delegation; no `(Regex, handler)` table.
8. **Four named Gherkin traps, all of which have bitten this project:**
   - **A property test is only as strong as the inputs it can produce** — #73's sortedness property could not fail because the generator gave every band a distinct weekday. *"≥1000 cases"* does not make a failure reachable. **This is the most relevant one here**, because this slice is mostly proptests.
   - **If a scenario's point is that nothing happens, its parameters cannot be under test** (#71).
   - **Asserting only the outcome where the implementation collapses several causes into it** (#69).
   - **Reaching the right end state by the wrong path** (#70).
9. **Requiring a field is not specified until every transport that supplies it is** (`T-required-fields-are-specified-per-transport`) — #73 shipped a page whose committed form could never succeed while 21 features stayed green, because the suite triages over JSON.
10. **`trellis serve --now <RFC3339>` is an offset, not a freeze.** You need it — a schedule is meaningless without a fixed today.
11. **#66 will fire under load and is not yours.** Re-run quiet and say so.
12. **No browser automation in this stack.** Say what is uncovered.
13. `trunk` expects four required status checks. Base **`trunk`**; scratch in `./tmp/`.
14. **Open the pull request when QA is done.**

## Open questions for you

Answer them **in the pull-request body, in a table, with reasoning**, and **name any `T-` row you need in your handoff note** — #60, #61 and #62 all did, and it is why this project's rationale stopped being written from archaeology.

1. **What is a `Block`, concretely?** `T-fact-plan-line` names its states — `proposed`, `published`, `in_progress`, `completed`, `missed` — and nothing defines its columns. This slice creates the table, so it fixes the shape. Which states does M3 S1 actually write? (Probably `proposed` only.)
2. **Where does the plan live between requests?** Recomputed on every page load, or stored and recomputed on a trigger? `R-incremental-patching` settles that it is **recomputed from scratch, never patched** — but not *when*. The demo's "restart and the plan is still there" reads as stored; recomputing on load would also satisfy it and store nothing.
3. **"Least slack" needs a definition.** Slack is presumably `deadline − now − estimate`. What is the slack of a task with **no deadline**? Committed tasks always have one, but say it rather than leaving it implied.
4. **What does the schedule page show when nothing is placed** — a fresh database, or every task unplaceable? The inbox's *"Nothing to triage"* is the precedent for saying something true rather than showing an empty box.

## Dependencies and sequencing

- **Nothing blocks this.** M2 closed; invariants, U2 and U4 are settled; the pipeline is empty.
- **Blocks nothing directly**, but S2–S5 all build on the `schedule()` it establishes.
- **S2** needs **U3** (backward-pass input), **S3** needs a chunk policy, **S4** needs the pin/split-binding question — all three are open and being settled in parallel.

## Source

- Issue **#11** — M3 epic, the signature, the invariants criterion, the reason enum
- `docs/decisions.md` — `T-invariants-one-to-five`, `D-placed-whole-or-not-at-all`, `T-hard-refuses-soft-slips`, `T-blocks-do-not-cross-guardrail-seams`, `T-fact-plan-line`, `R-incremental-patching`, `D-life-area-owns-its-time`
- `docs/design/architecture.md` — The invariants; The scheduler; Guardrails and free time
- **#7** — U3 remains open and is S2's, not yours
