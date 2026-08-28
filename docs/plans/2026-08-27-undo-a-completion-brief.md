# Handoff brief — `undo-a-completion`

**Date:** 2026-08-27 · **Issue:** #111 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> Base is `origin/trunk` at `0d76854`. **#157 (`quota-screen-redesign`, the owner's own) merged while this was being written** — it touches `quota/view.rs`, `quota_body.html`, `trellis.css` and three QA scripts, and **nothing this slice needs.** Confirm trunk is green before you start.

---

## The filing is stale in the way that matters. Read this before reading it.

**#111 says:** *"`grep` finds no unarchive, restore or undo path anywhere in the tree."*

**Undo already exists.** `mark_done::unmark_task_done` is a working front door, `POST /pool/tasks/{id}/undone` is a live route, and `pool_body.html:38` renders the control: a trip item's checkbox flips to `/undone` when `item.done`. **#122 built it.** The filing predates it.

**So this slice is not "build undo". It is "two surfaces cannot reach the undo that exists", and the reason is the same in both:**

| surface | control | why |
|---|---|---|
| **item inside a trip** | ✅ checkbox flips to `/undone` | **the struck row stays visible** (#122) |
| **loose end** | ❌ always posts `/done` (`pool_body.html:59`) | the row leaves the screen |
| **committed row** | ❌ always posts `/done` (`committed_body.html:17`) | the row leaves the screen |

**There is no `/committed/tasks/{id}/undone` route at all.**

**Undo works where the row survives the swap. That is the whole finding, and it is also the design problem** — you cannot put a control on a row that is gone.

## The loose-end half is a core rule, not a template

`scheduler_core::pool` — `loose.extend(list.into_iter().filter(|t| !t.done))` (`:181`), `None if !task.done => loose.push(task)` (`:223`), and the doc comment says it outright: *"A **loose** end that is done is filtered out entirely"* (`:38`), *"a done loose end leaves the screen"* (`:214`).

**That is deliberate, decided, and asserted** — `trip-progress-loose-ends-unchanged-08`. **This slice narrows that assertion; it does not delete it**, the same way #95 narrows `pool-screen-05` rather than removing it. **Say in the handoff what the scenario means afterwards.**

## Demo

**On the phone.** Label the pull request `preview`.

1. **Pool.** Tick a loose end — say `buy milk`. It leaves the list, as it does today.
2. **Something says so, and offers a way back.** Tap it. **`buy milk` is in the pool again**, unstruck.
3. **Committed.** Same gesture, same way back.
4. **Inside a trip: nothing changes.** A struck item still unchecks in place. **This slice adds a second route to undo, it does not replace the first.**
5. **Mistap something and get it back without leaving the screen.** That is the whole point: today the item you lose is the one you were about to do.

## The one question this slice exists to answer

**Where does "a way back" live when the row is gone, and how long does it last?**

`T-ephemeral-view-state-rides-the-request` is the rule, and **the precedent is in the building already**: `#pool-body` and `#quota-body` both read client state off the DOM in an `htmx:configRequest` hook and echo it back as `expanded=` on every request. **A "just completed, still undoable" identity is the same shape** — it rides the request, nothing is stored, and a fresh page load starts clean.

**Three things to settle explicitly rather than by whichever template was easiest:**

1. **Transient strip, or the row staying struck-and-visible for a while?** The second is what trips already do and costs no new concept; the first is the phone convention. **A permanent archive view is out of scope and `R-browsable-archive` should be read before proposing one.**
2. **Does it survive a reload or a tab change?** *"Honest answer either way"* — but say it plainly. **State that only exists in a fragment disappears in ways the owner will not predict**, and this project has now got that call right twice and wrong twice.
3. **Does undo restore position?** Pool order is derived (`T-trips-are-derived-not-ranked`), so an undone item returns wherever the rule puts it — **which may not be where it left. That is probably correct and should be stated rather than discovered.**

**Do not buy a column without saying why in the handoff first.** `T-ephemeral-view-state-rides-the-request` **permits** storage for a durable consequence of a deliberate act; it never requires it, and `D-a-trip-survives-being-tidied` is the slice that proved deriving first was worth one slice's thought against a permanent migration (`T-migrations-append-only`).

## Where the writes go

- **`T-cross-capability-invariants-need-an-owner`.** `mark_done` is *"the one place `archived_at` is written from"* — its own words. **Committed's undo goes through `mark_task_done`'s existing inverse, not a second write path.** `mark_done` is the only capability in the tree that already declares `mod store;` privately, and `capabilities go through front doors` is now a CI gate that will say so.
- **`unmark_task_done` already returns `bool`** — *"`false` when the task was already open, already cleared, or does not exist."* **The double-undo guard exists; use it rather than adding one.** An undo that resurrects an item a later action already changed is a correctness bug, not a UI wrinkle.
- **`T-forms-swap-one-fragment`.** The row leaving and the way-back appearing are one change: one fragment, one id.

## Watch

1. **CRAP is the live trap, today.** `pool_screen::dispatch` came down 31 → 28 yesterday to clear a CRAP failure at threshold 30, **which is an exclusive bound — `crap.sh:138` is `if crap >= threshold`, so 30 fails too.** **At 100% coverage CRAP *is* the cyclomatic count** and, unlike `T-complexity-8`, **it has no baseline to record an accepted dispatcher.** You have **two arms of headroom** in that module. New steps for this slice are the thing most likely to spend them.
2. **Run the analyzers before handoff.** `scripts/analyzers/crap.sh` is listed in `stack.prompt` and was not run on the last slice; it reached a pull request. **#156** covers the five gates under `scripts/ci/` that the constitution never names — **that is a separate gap and it does not excuse this one.**
3. **A fixture that unarchives by writing SQL directly proves nothing.** The trap #103 was warned about. **Exercise it through the route.**
4. **`scripts/qa/phone_layout.cjs` can see a rendered control at 390×844.** A transient affordance is exactly the kind of thing that has never been checkable here before — **and tap-target size is part of that gate.**
5. **#136 is adjacent and may close for free.** It records that the `htmx:configRequest` hook keeping a trip expanded *"is executed by no check"*, and the fix is revert-and-watch. **If this slice touches that hook, say whether #136 is satisfied — do not assume either way.**
6. **`T-a-check-must-be-seen-to-fail`.** Break each new rule once and record the message. **#149 is the open example of not doing it.**
7. Scratch in `./tmp/`. **New pull request, labelled `preview`.**

## Out of scope

An archive browser (`R-browsable-archive`) · undo for anything other than completion · bulk undo · **#154** · **#148** · **#118** · **#95** · any visual design system.

**And group completion.** `#125`'s complete-a-trip button has no single-gesture undo, which `trip-persistence` flagged and `D-a-trip-survives-being-tidied` worked around by keeping the panel one tap longer. **That is a real debt and it is not this slice** — but if the shape you build makes it obvious, say so.

## Source

- `crates/trellis-server/src/mark_done/mod.rs:40-45` — `unmark_task_done`, the inverse that already exists
- `crates/trellis-server/templates/pool_body.html:38` vs `:59` — the trip item that can undo, and the loose end that cannot
- `crates/trellis-server/templates/committed_body.html:17` — no undo, and no route behind it
- `crates/trellis-server/src/platform/app.rs:77-82` — `/pool/tasks/{id}/undone` exists; the committed pair does not
- `crates/scheduler-core/src/pool.rs:38,181,214,223` — *"a done loose end leaves the screen"*, and where it is enforced
- `features/trip_progress.feature` — `trip-progress-loose-ends-unchanged-08`, the scenario this narrows
- `docs/decisions.md` — `T-archived-at-only`, `D-kill-means-archive`, `R-browsable-archive`, `T-cross-capability-invariants-need-an-owner`, `T-ephemeral-view-state-rides-the-request`, `T-trips-are-derived-not-ranked`, `T-forms-swap-one-fragment`
- `docs/decisions-history.md` — 2026-08-25 `trip-persistence`, for why deriving was tried before a column
- **#97** / PR **#103** — the completion control this reverses · **#122** — the undo that already exists · **#125** — the group-completion debt
