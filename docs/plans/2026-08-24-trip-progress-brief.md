# Handoff brief — `trip-progress`

**Date:** 2026-08-24 · **Issue:** #122 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **The fourth defect the owner found by using the product**, and the first one they hit while doing the thing the feature exists for. `D-dogfooding-drives-the-roadmap` was written the same day, off this and its three siblings.

---

## Demo

**On the phone.** Label your pull request `preview` and the box puts it on `:8443` within two minutes.

1. Capture five things, all tagged `@homedepot`. They form a trip.
2. Check off three, one at a time, as if you were finding them in the shop.
3. **Today: on the third check the panel dissolves** and the last two drop into loose ends among everything else — mid-shop, with no record of what you already got.
4. **After: the panel holds.** The three you did stay in place, struck through. The label says **3 of 5 done**. Unchecking one puts it back.

## Goal and scope

`D-a-trip-survives-being-worked`, settled by the owner:

- **A completed item stays in place, struck through**, rather than vanishing.
- **The group's label reports progress** — *"3 of 5 done"* — not a bare count.
- **Formation still requires three *open* items**, so three completed ones never conjure a trip.
- **Persistence counts everything displayed**, so checking things off never dissolves the panel.
- **Unchecking a struck-through item puts it back.**

### Why it happens

`pool/store.rs:31` filters `AND tasks.archived_at IS NULL`, so a completed task leaves the query entirely. `pool.rs:101` then re-tests `list.len() >= TRIP_THRESHOLD` **on every render**. **The screen destroys the list at exactly the moment it is being used for its only purpose.**

### This narrows #103, it does not reverse it

PR #103 rejected counting done items toward the threshold as the **half-pass trap** — *a trip panel of one that reads correctly until you look at the number.* **That was right about the number and wrong about the experience.** The fix is the half it did not consider: **change the label rather than hide the items.**

**The trap must not come back.** A panel reading *"0 of 3 done"* with nothing left to do is the same bug wearing the new label. **Say what happens in that state** — it is also when `#125`'s complete-group button must not appear or must not act.

## The open question, and it is the slice

**When does a struck-through item finally clear?**

`T-archived-at-only` means *recently completed* is already expressible from the stored timestamp — **no new column, no done/killed discriminator.** Candidates:

- **When the group has no open items left** — the trip completes as a unit, matching *"I finished the shop"*. But a group you never quite finish keeps its corpses indefinitely.
- **A time window** — `archived_at > now - window`. Self-clearing, and it needs a defensible constant; `T-free-time-horizon-fourteen-days` is the precedent for one that is argued rather than tuned.
- **On the next visit to the screen** — cheapest, and it means glancing at Pool from the car silently erases your shopping progress. **Probably wrong; say why.**

**Answer it in the pull-request body with reasoning.**

## Read this before writing a column

**#126 bought a permanent schema column for a display preference, on a premise that had already expired**, and could not un-buy it (`T-migrations-append-only`). The chain, in the specifier's own words: *"the acceptance suite speaks only HTTP; to make that assertion true over HTTP the state had to be server-rendered; to be server-rendered across a request it had to be stored."*

**The tier you choose to assert in decides what the implementation must store.** Two things this project already has, which that slice did not use:

- **`scripts/qa/phone_layout.cjs` is gated in CI and drives real Chrome.** It can observe rendered state no HTTP assertion can. It does **not** look at panel or item state today — that is capability sitting unused.
- **The product already runs JavaScript.** `base.html:16-17` loads htmx and an inline script on every page.

**Nothing here obviously needs a column** — `archived_at` already exists and already carries the timestamp. **If you find yourself reaching for a migration, stop and say why in the handoff before writing it.**

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-a-trip-survives-being-worked` | **The whole model. Read the row.** |
| `T-trips-are-derived-not-ranked` | Order stays derived — most items first, alphabetical tiebreak — **so a trip carries no reorder control, here or in any later slice.** **Correction to this brief, 2026-08-24:** an earlier draft said the canvas's *"Raise priority"* / *"Lower priority"* controls are *"not approved"*. **That is wrong and overbroad.** `D-menu-is-a-worklist` names manual priority and the canvas draws six of those controls — **they are approved, for loose ends** (**#95**, *"Reordering belongs to loose ends alone"*). The split is the point: **a trip is a unit you clear in one stop, so its item order is noise; a loose end is a thing you decide about, so it earns a control.** What must not arrive *here* is a reorder control **on a trip**, and `pool-screen-nothing-reorders-05` asserts that absence. |
| `T-archived-at-only` | One nullable timestamp, no discriminator. **This is why the slice is cheap.** |
| `D-bulk-completion-is-explicit` | A complete-group button is settled and is **#125**, not this. |
| `T-set-operations-execute-in-the-store` | The store carries its own `WHERE`/`ORDER BY`. **#108 is open on this exact query and is not yours** — do not absorb it, do not make it worse. |
| `T-canvas-is-authoritative-where-it-speaks` | **Check the canvas before assuming.** It caught my last brief. A struck-through state may or may not be drawn; if it is silent that is a gap to flag, and #120 is open on this same panel. |

## Known repo gotchas

1. **`trunk` is green at `42bc669`.** Cut from `origin/trunk`, not the local `trunk`.
2. **Expect 20 features.** Analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`.
3. **DRY has almost no headroom.** #126 arrived failing at 3.11% and landed at **3.00%** against a threshold of 3; I measure **2.78% overall on a checkout without generated entrypoints, and Rust alone at 3.54%.** **The next slice that adds rendering crosses it.** Fix by extracting genuinely shared helpers, never by waiver.
4. **`T-cross-capability-invariants-need-an-owner`** — `pool::group` buckets by plain string equality, correct **only** because `capture::resolve_tag` canonicalises on the way in. Anything re-entering the grouping goes through the same path.
5. **A fixture that writes `archived_at` directly proves nothing.** Mark done through `POST /pool/tasks/{id}/done`. This slice depends on what that route leaves behind, so the shortcut is more tempting and more wrong than usual.
6. **Update `scripts/ci/complexity-baseline.json` as a step.**
7. **`cargo test --workspace` skips the `#[ignore]`d property tests locally** — CI runs them via `--include-ignored` since #113.
8. **#120 and #125 are open on this same panel.** No overlap in scope, but **say which you expect to land first** rather than both assuming the other.
9. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
10. **Open the pull request when QA is done, and label it `preview`.**

## Source

- Issue **#122** · `crates/trellis-server/src/pool/store.rs:31` · `crates/scheduler-core/src/pool.rs:95-108`
- `docs/decisions.md` — `D-a-trip-survives-being-worked`, `T-archived-at-only`, `T-trips-are-derived-not-ranked`, `D-bulk-completion-is-explicit`
- **#97** / PR **#103** — the half-pass trap, and why the label is the thing to change
- PR **#126** — what asserting at the wrong tier costs, permanently
