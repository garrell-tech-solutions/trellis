# Handoff brief — `trip-persistence`

**Date:** 2026-08-25 · **Issue:** #129 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **The third and last slice on this panel.** #122 made a trip survive being *worked*; #135 gave it controls; this makes it survive being *tidied*. After this, the Pool is finished.
>
> **Read #129's body — it is long, well-argued and mostly right.** This brief corrects two things in it that have gone stale, and hands you a boundary the issue did not know it had.

---

## Demo

**On the phone.** Label the pull request `preview`.

1. Five things tagged `@homedepot`. It is a trip.
2. **Check three off.** The panel holds — that is #127, already true today.
3. **Clear them.** *Today the panel vanishes and the last two scatter into loose ends, while you are still standing in the shop.* After this slice **it holds**: still `@homedepot`, still a panel, two things left to get.
4. **Get the last two.** *Now* it goes — nothing open left, nothing to stand in a shop for.
5. **Reload between every step.** It behaves identically. If it does not, you have answered the storage question wrongly.

## The boundary the issue did not know it had

**Two acceptance scenarios already sit on either side of this change**, and finding them settles most of what #129 called "the whole slice".

`features/trip_progress.feature:126` — **`trip-progress-clearing-can-drop-a-group-05`**:

> 3 tasks at `@homedepot`, 1 done. Clear. → **`the pool screen offers no trips`**, 2 loose ends.

**That scenario asserts the defect.** It is green today and it must go red, then be rewritten: two open items at a tag that has been a trip stay a trip. **#129's Watch 7 says this slice "extends #127's scenarios, it does not reverse them" — that is wrong, and it is the one instruction in the issue not to follow.** Reverse `-05` deliberately, in the open, citing the owner's 2026-08-24 extension. Do not delete it and do not quietly weaken it.

`features/trip_progress.feature:141` — **`trip-progress-fully-done-06`** — is the other side, and it **stays exactly as written**:

> 3 tasks, 3 done. Clear. → **`the pool screen does not mention "@homedepot"`**.

**Those two are the rule.** *Sticky while any item is open; gone the moment none is.* `-06` is the terminating condition already built and already asserted — which is why the runaway #129 feared (*"over a year every frequently-used tag becomes a permanent panel"*) cannot happen on its own. **A tag that empties ends its trip and must re-earn one by reaching three again.**

## What is actually left to decide

Not *whether* trip-ness expires — `-06` answers that. **How you know a tag is inside a run.**

`scheduler_core::pool::group` (`pool.rs:121`) re-derives trip-ness on every render from one set, and `pool::store::list_pool_tasks` never returns a cleared row, so **the current run's history is not in the data the rule can see.** Counting all-time rows instead cannot distinguish a run that ended and restarted from one that never ended — which is the failure #129 correctly names.

**A decision test now exists that did not when #129 was filed.** `T-ephemeral-view-state-rides-the-request` (recorded 2026-08-25, `740d224`) settles the storage question for this project: *does a reload deserve to forget it?* `shown_kind` should have been forgotten and was stored; `cleared_at` should not have been and earned its column.

**Apply it out loud.** A trip that dissolves when you lock your phone in the car park is the same defect this slice exists to fix, one gesture further out — **so the honest answer here may well be a column, and that is allowed.** What is not allowed is reaching for one without the argument. #126's mistake was never *storing something*; it was **not noticing that the premise forcing storage was a choice**.

**If you can derive it cleanly instead, that is better and it is worth the attempt** — `cleared_at` is a millisecond timestamp, not a flag, so the clears are ordered in time. Say what you tried.

**Answer it in the pull-request body with reasoning**, as #129 asks.

## Where the write goes

- **`T-set-operations-execute-in-the-store`.** Whatever the rule is, it is a set operation, and it executes in the store — not a loop in a handler and not a filter in the view.
- **`T-cross-capability-invariants-need-an-owner`.** `pool::group` buckets by plain string equality, and that is correct **only** because `capture::resolve_tag` canonicalises on the way in. If a run's identity is a tag, it inherits that dependency.
- **`T-dead-core-code-earns-its-keep`** does not license leaving the old single-predicate path beside a new one. One rule, one place.

## Scope boundaries — two open issues sit on this code and neither is yours

- **#108** is open on `list_pool_tasks` — grouping and per-group limits run in memory over an unbounded fetch. **You may change that query where this rule requires it. Do not fix #108 while you are there**, and say in the handoff if what you did makes #108 easier or harder.
- **#95** (reordering loose ends) touches the same screen and is **not** in this slice. `T-trips-are-derived-not-ranked` still forbids a reorder control on a trip, and `pool-screen-nothing-reorders-05` asserts that absence.

## How this composes with what already shipped

**#125's complete-group button is no longer hypothetical — it merged in #135.** Completing a group empties it, which under the new rule ends the trip. **Walk that path explicitly:** complete-group → zero open → the panel goes, per `-06`. `trip_controls.feature:176` already asserts a trip with nothing open offers no complete-group control; make sure the two rules do not disagree about which state the panel is in between the completion and the render.

**Per-item undo still works after a group completion** — `trip_controls.feature:216` asserts it. **Check what unchecking does to a trip you just ended**: an item coming back at a tag with zero open is a tag going from no-trip to some-trip with one item, and `-06` says a trip that ended must re-earn one at three. **That is a real edge and the issue never mentions it.** Decide it and assert it.

## Three corrections to #129's Watch list

1. **"DRY headroom is zero — #127 landed at exactly 3.00%" is stale.** #130 shipped in PR #131: the gate measures product code only (`T-dry-measures-product-code`). The last two slices measured **1.86%** and **1.81%** against a 3% threshold. **You have real headroom** — which is not an invitation to copy a step module wholesale.
2. **"All 21 acceptance features"** — there are **23**. `trip_controls` was the 23rd.
3. **Watch 7's "does not reverse them"** — see the boundary section. It reverses exactly one scenario, on purpose.

## Watch

1. **`T-a-check-must-be-seen-to-fail`.** The rewritten `-05` must be observed red against today's code before it is trusted green. It is the whole slice; a green-on-first-run assertion here proves nothing.
2. **A trip with no open items must leave the screen entirely**, not linger as an empty panel — `-06`, unchanged.
3. **Loose ends are unchanged.** #127 settled that a completed loose end leaves at once, deliberately: *a loose end has no panel to dissolve.* Do not extend trip-ness to them.
4. **A fixture that writes `cleared_at` directly proves nothing.** Go through the route, as #103 and #135 were both told.
5. **`scripts/qa/trip_controls.cjs` drives real Chrome and is CI-gated** (`ci.yml:452`). If your answer has any client-side component, that is the tier that can see it — and step 10 is the pattern to follow.
6. **PR #137 (merged today) re-did the palette and typeface**, touching `trellis.css`, `base.html` and the colour feature — **no pool logic**. Cut from current `trunk` and expect the panel to look different from every screenshot in #122, #127 and #135.
7. **All 23 acceptance features pass**, and every #127 and #135 scenario except `-05` must still hold.
8. Base branch is **`trunk`**; cut from `origin/trunk`. Scratch in `./tmp/`. **Open the pull request when QA is done and label it `preview`.**

## Out of scope

The threshold itself (three is settled, owner-confirmed), `VISIBLE_TRIP_ITEMS`, reordering (**#95**), the query's in-memory grouping (**#108**), loose-end behaviour, dismissing or killing a task (`T-archived-at-only` still has no discriminator), the Quota screen (**#93**, next slice), and **any visual design system** — #137 just settled that and it is not yours to revisit.

## Source

- Issue **#129** — read it in full; it frames the question well
- `crates/scheduler-core/src/pool.rs:121` — `group`, the single predicate, and the `#122` doc comment explaining why one count does two jobs
- `crates/trellis-server/src/pool/store.rs` — `list_pool_tasks` (`cleared_at IS NULL` is why history is invisible) and `clear_done`
- `features/trip_progress.feature:126` (`-05`, reverse it) and `:141` (`-06`, keep it)
- `features/trip_controls.feature:176, 216` — the states this must compose with
- `docs/decisions.md` — `D-a-trip-survives-being-worked` (extended by the owner 2026-08-24), `T-ephemeral-view-state-rides-the-request` (`740d224`), `T-set-operations-execute-in-the-store`, `T-trips-are-derived-not-ranked`, `T-a-check-must-be-seen-to-fail`, `T-migrations-append-only`
- **#122** / PR **#127** — the slice this completes · **#125** / PR **#135** — the controls it must not contradict
