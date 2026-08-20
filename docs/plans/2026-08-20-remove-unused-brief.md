# Handoff brief — `remove-unused`

**Date:** 2026-08-20 · **Issue:** #88 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **This slice deletes code and adds none.** It is the largest deletion this project has done and it carries three passengers, all named below.
>
> **Sequencing:** `context-tags` (#82) is **parked** and its pull request **#89 is closed** — it was built against modules this slice removes. It returns after you, on a clean tree.

---

## Three passengers — deliberate, not scope creep

**1. `T-latency-is-a-qa-assertion` — approved 2026-08-20 to ride in the next slice specified, which is this one.**

The capture **50 ms** budget moves **out of the acceptance and unit suites and into QA**, measured against a real server on a quiet machine. It stays as the stated design constraint — `capture`'s module header cites it and `T-classifier-covers-domain` reasons from it. Two places to change:

- `features/capture_endpoint.feature` — `capture-endpoint-persists-quickly-01`, *"the response is received within 50 milliseconds"*
- `platform::app::tests::capture_request_persists_a_row_and_responds_within_50ms`

**Why it is not merely a flake.** It fired spuriously in four of six slices — but the harm that decided it was silent: under mutation-run contention the request took **1.885 s**, which failed `cargo-mutants`' unmutated **baseline**, so it refused to test a single mutant and **the whole `trellis-server` crate had zero mutation coverage with no error connecting cause to effect.**

**2 and 3. Two commits salvaged from closed PR #89**, both the architect's and both unrelated to context tags:

- **`91ba030`** — mutation testing on nextest, with `crates/acceptance-tests` dropped from per-line mutation. **650 of 1296 mutants lived in that one crate**, where killing one means unit-testing that an assertion helper fails when it should — a test of a test of the product — at ~90% of the total cost, already covered by Gherkin acceptance mutation.
- **`e49b9a8`** — the fix for the wall-clock assertion above, plus two other findings from the first real run.

**Cherry-pick them rather than re-deriving them.** They are on the closed branch and they were measured, not reasoned.

---

> **Scope widened 2026-08-20.** This was route-removal with the modules left dead in the tree. The owner's call — *"we can mark the unused for removal, it's still in git"* — makes it the demolition. **The deferral was argued on the wrong ground:** the evidence that decides whether guardrails were worth having is two weeks of real use, not the code, and git keeps the code. Restoring it is a revert.

## Goal

**Only what the owner uses exists.**

```
GONE   routes    /stats  /life-areas  /free-time  /capacity  /schedule
       server    life_areas  free_time  capacity  exceptions  stats  schedule
       core      guardrail  free_time  life_area  capacity  exception
       features  capacity  free_time  guardrails  exceptions  life_areas
                 life_area_triage  timezone_setting  schedule
       + their QA scripts and step modules

STAYS  /  (capture + triage)   — and /menu when #85 lands
       scheduler_core::schedule and ::interval  — M3's, paused, sound
       every table, untouched
```

## No migration. Leave the tables.

`tasks.life_area_id` carries a foreign key into `life_areas`, so dropping that table means **rebuilding `tasks`** — and `T-migrations-append-only` makes that a new numbered migration with the full table-rebuild pattern. **That is the only construction in this job, and it buys nothing:** unused tables cost SQLite nothing, unread columns cost nothing, and leaving them preserves the owner's data as a second safety net beside git.

**This slice deletes code and adds none.** If you find yourself writing `0010`, stop and say why.

## The header disappears

**`T-nav-is-the-site-map`: every page in the route table gets a header link.** Removing routes collapses the header on its own — **nothing needs superseding.** With the Menu unbuilt there is **one** screen, so there is nothing to navigate between and **the header should not render at all** until #85 brings the second one back.

## Consequences to state rather than discover

- **The timezone becomes unsettable.** It lives on the life-areas page. It is stored as `America/New_York` and persists, so nothing is lost — but `D-menu-is-a-worklist` puts controls inline, so **the Menu owns it from #85.** Do not build a settings page. Until then it is a one-way door.
- **R2's counter goes dark.** `#20` calls the committed:pool ratio *"the highest-leverage counter in the product"*. Removing `/stats` and `scheduler_core::ratio` stops it being computed — **but it was always computed on demand from `tasks` rows, which stay.** Nothing is lost that cannot be recomputed the moment a surface wants it. **Say this in the pull request** so nobody believes the history was thrown away.
- **`scheduler_core::schedule` survives with no caller.** It imports `interval` and `task` only — never `guardrail` or `free_time` — so it compiles fine. Its input was free intervals, which are going. **Leave it.** M3 is paused, not cancelled, and #75's properties still hold.

## Acceptance scenarios worth specifying

- Each removed path returns **404**.
- **No header renders while one screen exists.**
- Capture and triage are unaffected — this is the whole point of the slice and the thing most at risk.
- **`app_shell.feature` shrinks substantially.** Ten scenarios, most of them per-page assertions for pages that no longer exist. **That is the rule working**, and #70's inversion applies in reverse: if `app_shell` did *not* need changing, a page is still reachable that should not be.
- **Deleted features are deleted, not skipped or left failing.** Eight of them specify code that goes with them. **Say so in the pull request, by name** — a deleted specification should be a stated act, not a diff nobody reads.

## Known repo gotchas

1. **#82 is in flight** and touches `capture_row.html`, `inbox.html`, `lists.html`. This touches `app.rs`, `nav.rs`, `lib.rs`, whole modules and eight features. **Merge `trunk` before measuring, not after.**
2. **PR #87 landed a 636-line stylesheet and restyled three templates without touching any feature, QA script or step module.** CI stayed green, so selectors survived — but this slice deletes a lot of what remains. **Re-run everything and believe the result, not the expectation.**
3. **`platform/boundary.rs` walks `src/`** and asserts a **floor** on what it finds. Deleting five capability directories may take it under its own minimum — *"only N capabilities found, so this check covers almost nothing"* is exactly the failure it was built to report. **The floor may need lowering, and that is a decision, not a formality.**
4. **The complexity baseline pins exact scores** for step modules you are deleting. Rows will go stale; `scripts/ci/complexity_baseline.sh` fails on a stale row as well as a new one.
5. **DRY may move sharply** — deleting 3,650 lines changes the denominator. Down is fine; report the number either way.
6. `trunk` expects four required status checks. Base **`trunk`**; scratch in `./tmp/`.

## Open questions

1. **Does `platform::nav` survive with one page?** A site map of one is arguably not a site map. Deleting means rebuilding at #85; keeping means a module rendering nothing for a fortnight.
2. **Does `scheduler_core::ratio` go with `stats`, or stay?** It is pure, tested, and has no caller once the page goes. `T-core-no-tokio`'s purity gate does not care. **Staying is dead code; going means #20's counter is rebuilt from scratch when it returns.**
3. **What happens to `settings`?** It holds only the timezone, whose only reader (`free_time`) is being deleted. It has a front door and one row. **Keep it** — #85 needs it — but say so deliberately rather than deleting it as collateral.

---
## Source
`docs/decisions.md` — `D-menu-is-a-worklist`, `D-context-tags-are-the-taxonomy`, `D-dogfood-first`, `T-nav-is-the-site-map`, `T-migrations-append-only` · #85 (restores the second screen) · #11 (M3, paused) · #20 (R2)

