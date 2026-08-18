# Handoff brief — `capacity`

**Date:** 2026-08-18 · **Issue:** #62 · **Milestone:** M2 — Guardrails + Free-slot Algebra · **Route:** pipeline

> **M2 slice 4 of 4. This closes the milestone.**
>
> `exceptions` (#61, PR #71) merged, so free time is now guardrails **minus** what the owner has removed. This slice turns that into the number that stops Thursday at 4pm: **hours you have, against hours you have promised.**

---

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

1. Give **Fitness** a guardrail of `Sat 09:00–11:00`. **Free time** shows **4h** — two hours, two Saturdays, because the horizon is exactly two weeks (`T-free-time-horizon-fourteen-days`).
2. Capture and triage a **committed** Fitness task, estimated **3h**. Open **Capacity**: *"Fitness — 3h needed, 4h available."*
3. Triage a second committed Fitness task, estimated **2h**. Reload: *"Fitness — 5h needed, 4h available. **1h over.**"*
4. Widen Fitness's guardrail to `Sat 09:00–12:00`, or kill a task. Reload — **the warning clears.**

**Step 3 is the whole slice.** You found out on a quiet Tuesday, not at 4pm on the Saturday.

*(The arithmetic above holds on any day of any week — `T-free-time-horizon-fourteen-days` guarantees exactly two of every weekday in the window. The previous brief's demo used real dates and got them wrong; this one is built not to need them.)*

## Goal and scope

**Supply against demand, per life area, over the next fourteen days, with over-commitment called out.**

### The correction to this epic's original acceptance criterion — read this first

#10's AC once read: *"reports minutes per guardrail and separately attributes them per life area (`T-capacity-two-axes`), matching a fixture including at least one task whose life area differs from the guardrail it was **placed in**."*

**That cannot be built here. Placement needs a scheduler and there is none until M3** — no `block` table, nothing occupying anything. `T-capacity-two-axes` is correct and settled; it simply has no observable behaviour at M2, and this project settled on 2026-08-12 that a schema element with no observable behaviour has nothing to specify against. **The two-axes attribution moved to #11 (M3)**; M2 keeps the half that carries the warning. Both epics say so.

### ⚠️ The hidden cost, confirmed against the tree

`tasks` has **no estimate column**:

```
id · capture_id · kind · deadline · deadline_type · priority
target_count · target_minutes_each · period · life_area_id · archived_at · created_at_ms
```

Quota carries `target_minutes_each`, so **quota demand is computable today**. **Committed tasks carry no minutes at all**, so the committed half of the number cannot be computed. **This slice almost certainly has to add `estimated_minutes` and require it at triage for committed tasks** — a triage-surface change riding in M2's last slice, and the largest unknown in it.

**Two alternatives look closed by precedent, and you should confirm rather than explore them:**

- **Count only quota demand.** Makes the number wrong in the direction of *"you have more time than you do"* — the exact failure this milestone exists to prevent, and the reason #6 was settled at all.
- **Default committed tasks to some estimate.** A default the owner never chose, silently wrong, invisible on every surface. Refused twice already: `D-manual-triage-until-llm` for the life-area picker, `T-timezone-is-a-setting` for the host zone.

**Only committed needs it.** Pool consumes nothing (`D-no-pool-on-calendar` — never placed), and quota already carries its minutes.

**Predict the fixture drift:** `T-life-area-required-at-triage` added a required triage field and broke five already-green QA scripts. Adding a second required field will do the same. That is drift, not regression — reproduce each, confirm, fix the fixture.

### Out of scope — do not absorb

Placement, blocks, scheduling, the two-axes attribution (M3), `allowed_windows` / borrowing (M3), the weekly reckoning (M8), **and any visual design system. No charts.**

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `T-capacity-two-axes` | Consumed from the guardrail, attributed to the life area. **Settled, and M3's to demonstrate.** Build nothing here that contradicts it. |
| `T-free-time-horizon-fourteen-days` | **Capacity's horizon is the same fourteen days by rule**, because supply is the sum of those intervals. Not a coincidence, unlike `/stats`'. |
| `T-availability-only-subtracts` | Supply is free time **after** exceptions. Shipping capacity before exceptions would have over-reported every exception day — that is why #61 came first. |
| `T-guardrail-well-formedness` | A life area marked *never scheduled* has no hours. **It must report no capacity at all, not zero-available-and-over** — it opted out of being scheduled, which is not the same as being full. |
| `T-three-task-kinds` | **Quota demand counts.** *"Capacity math is false in the same way C5 makes the Work number false if quota demand is uncounted."* |
| `D-no-pool-on-calendar` | **Pool consumes nothing.** A backlog of 200 pool items must not make the number look catastrophic. |
| `D-quota-no-rollover` | A missed week is missed. Last week's unmet quota is **not** this fortnight's demand. |
| `T-archived-at-only` | Archived tasks and archived life areas are excluded. |
| `T-nav-is-the-site-map` | **A new page means a `nav::Page` variant *and* a row in `nav::ALL`** — and then `app_shell` **must** change. See gotcha 3. |
| **Core vs adapter** | The arithmetic — proration, what counts as demand, where the warning fires — survives changing HTTP, so it belongs in **`scheduler-core`**. `scheduler_core::ratio` is the precedent: `/stats`' rules live in the core, not in `stats/`. |
| `T-templates-take-view-models` · `T-422-is-product-wide` · `T-capability-owns-its-queries` · `T-one-front-door-per-capability` | Unchanged. |
| **No visual design system** | Plain and ugly remains correct. |

## Acceptance scenarios worth specifying

- Per life area over fourteen days: **hours available** (the sum of its `free_intervals`) against **hours needed**.
- **Over-commitment is called out**, not left to the reader to subtract.
- **Quota demand counts**, prorated across the horizon — see open question 2.
- **Pool consumes nothing.**
- **A never-scheduled life area reports no capacity**, not zero-and-over.
- **Exceptions reduce supply** — mark a week away and watch the available number fall. This is the first slice where two of M2's mechanisms compose, and it is worth a scenario of its own.
- Archived tasks and archived life areas are excluded.
- Matches a **hand-computed fixture**.
- **All 20 existing acceptance features pass untouched** — except `app_shell`, which **must** change if you add a page.

## Known repo gotchas

1. **`trunk` is green at `1b81fdb`** (the #71 merge). **Rebase or merge `trunk` before your final measurement, not after** — it has moved mid-slice repeatedly, and #55 took `trunk` red 24 seconds after a merge whose checks were honest about an older tree.
2. **Expect 20 features.** Analyzers in order: `scripts/acceptance/run.sh`, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`.
3. **If you add a page, `app_shell` must change — and if it doesn't, that is the bug.** A page missing from `nav::ALL` compiles and ships with no link, and `app_shell` *not* needing an update is the only thing that notices. #70 wrote that inversion down; #71 checked itself against it and correctly did not add a page.
4. **⚠️ DRY is 2.89% against a fail-closed 3.** It has arrived **red in two of the last three slices** (3.03%, 3.28%) and been fixed by real deduplication both times — **that is the standard, not a waiver.** And the headline understates it: Rust alone was 3.43% when last split, diluted by the shell corpus (#52).
5. **`platform/boundary.rs` enforces four rules**, walking `src/`. Read the timezone through `settings::current_timezone`; resolve a life-area name through `life_areas::active_id_for_name` — #71 deleted a byte-identical duplicate of exactly that query.
6. **A new step module means a new dispatch row, CI-gated**, exact scores in `scripts/ci/complexity-baseline.json`. Every dispatch arm a one-line delegation; no `(Regex, handler)` table.
7. **Three named Gherkin traps, all of which have bitten this project:**
   - **If a scenario's point is that nothing happens, its parameters cannot be under test** (#71, four survivors — dates chosen to be inert).
   - **Asserting only the outcome where the implementation collapses several causes into it** (#59, twelve survivors).
   - **Reaching the right end state by the wrong path** (#70 — a rule can be unenforced and the scenario still green).
8. **A removal control wants a test that the thing was there first.** Two handlers have now survived being replaced with a no-op (`remove_guardrail_band`, `remove_exception`) because the assertions were equally true of an empty response.
9. **`cargo test --workspace` compiles zero acceptance tests on a fresh checkout.** Property tests need `-- --include-ignored`.
10. **`trellis serve --now <RFC3339>` is an offset, not a freeze.**
11. **#66 will fire under load and is not yours** — 612ms against a 50ms budget at load average 15.5 during #70, green quiet. Re-run quiet and say so.
12. **No browser automation in this stack.** Say what is uncovered.
13. `trunk` expects four required status checks. Base **`trunk`**; scratch in `./tmp/`.
14. **Open the pull request when QA is done.**

## Open questions for you

Answer them **in the pull-request body, in a table, with reasoning**, and **name any `T-` row you need in your handoff note** — #60 and #61 both did, and it is now the cheapest thing this project does for its own rationale.

1. **`estimated_minutes` — confirm the shape.** Required at triage for **committed** only? Nullable in storage for the reason `T-quota-targets-required` established (SQLite cannot add a `NOT NULL` column without a default to a table that may hold rows), required at the boundary? What does an existing committed task with no estimate do to the number — and there will be some, because rows predate the column. **Say what `None` means**, the way `T-life-area-required-at-triage` had to.
2. **How is quota demand prorated across fourteen days?** *"3 runs a week"* over a fortnight is 6. Over a `period: month` it is not an integer. State the arithmetic; `D-quota-no-rollover` means you are never carrying a debt forward, only counting the horizon.
3. **What is "over"?** Exactly over, or over by a margin? A guardrail 100% full is a schedule with no slack, so `D-guardrails-never-yield` suggests warning before 100%. `D-staleness-unset`'s precedent — *instrument first, tune at the first reckoning* — says pick a seed, make it configurable, and do not agonise.
4. **Its own page, or joined to `/stats`?** Both are read-only instruments over the same fortnight. Two pages is more nav; one page conflates R2's committed:pool ratio with the over-commitment warning. **#58's open question 5 settled the general rule — every route-table page gets a link — so this is about whether capacity is a page, not about whether it gets one.**

## Dependencies and sequencing

- **Nothing blocks this.** #61 merged, `trunk` green, pipeline empty.
- **This closes M2** (#10) on merge.
- Its demand arithmetic is read again by **M8**'s reckoning (#18); the two-axes half is picked up by **M3** (#11).
- **#63 and #65 are ops PRs** that may land independently.

## Source

- Issue **#62** — acceptance criteria and the demo
- Issue **#10** — M2 epic, the cut, and the amended AC
- `docs/decisions.md` — `T-capacity-two-axes`, `T-free-time-horizon-fourteen-days`, `T-availability-only-subtracts`, `T-guardrail-well-formedness`, `T-three-task-kinds`, `D-quota-no-rollover`, `D-no-pool-on-calendar`
- `docs/design/architecture.md` — Capacity; Guardrails and free time
- **#6** (closed) — the contradiction this number exists to resolve
- **#52** — why the DRY headline understates the risk · **#66** — the flake that is not yours
