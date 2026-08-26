# Handoff brief — `quota-sessions`

**Date:** 2026-08-26 · **Issue:** #93 (the other half) · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **This is not a new slice. It is the rest of `quota-screen`.**
>
> **Branch from `origin/slice/quota-screen`, not from `origin/trunk`.** PR #144 is open, blocked, and this work turns it green. Push to that branch and the same pull request completes.

---

## Why the branch, and it is the whole shape of this handoff

`quota-screen` stopped at the line the brief drew, and stopped honestly. **The cost of stopping there was three things nobody predicted individually:**

1. **PR #144 cannot merge.** `acceptance suite + analyzers` is red on `quota_sessions_acceptance`.
2. **The language-mutation run is gone.** `run.sh` globs `features/*.feature`, so the deferred spec generates an entrypoint and runs red, and `cargo-mutants` aborts on a failed baseline rather than degrading.
3. **The owner could not preview it** (**#146**). `ops/preview.sh` selects on run-level `--status success`; the run failed, so the poller bailed and the phone never showed the fourth tab — **even though the binary built fine and is sitting in that failed run.**

**Finishing on the same branch fixes all three at once and produces one green, complete, previewable pull request.** Merging #144 red would put a failing suite on `trunk`, which is the worst option available and is not on the table.

## Demo

**On the phone.** #144 is already labelled `preview`; once the suite is green the poller will pick it up on its own.

1. **Tap the fourth tab.** Define `Piano`, 4 hours a week — that part already works.
2. **Tap `+1h`.** The bar moves. The readout says one hour against four.
3. **Tap `+30m`.** Now an hour and a half.
4. **Use `Other`** to log the 20 minutes you did on Monday. **The day picker offers Monday and today, and nothing later.**
5. **Expand the quota.** `This week` lists all three, with a summary.
6. **Fix the day you got wrong. Delete the one you did not do.** Both stick across a reload.
7. **It is Monday.** It reads zero against four again.

## The specification already exists — implement it, do not re-specify it

`features/quota_sessions.feature` is on the branch, **nine scenarios, fully reasoned in a header worth reading twice.** It is red only because nothing implements it.

**Four things in that header that are decisions, not suggestions:**

- **`-03`: the day picker offers only days that have already happened.** Settled by the owner 2026-08-25. On Tuesday it offers Monday and Tuesday. The reasoning is the sharp part — `D-logging-is-retrospective-and-separate` refused a timer because *"an unstarted timer silently reports zero — inaction producing a false number rather than a missing one"*, and **offering Saturday on Tuesday is that same failure inverted**: hours logged against a day that has not happened fill the bar and the week reads as met.
- **The canvas draws all seven days, and the spec calls that a gap rather than a statement.** `days: DAYS` is a static constant beside `const TODAY = "Tue"` — the mock has no notion of the week passing, so it could not have drawn the distinction. **Checked rather than assumed, which is the practice `T-canvas-is-authoritative-where-it-speaks` kept.**
- **`-08`: Monday starts again at zero**, and the consequence was shown to the owner and accepted: **you cannot log Sunday evening's practice on Monday morning.** Recorded so whoever meets it knows it was chosen.
- **A session earns its table. The expanded row and its open `Other` panel do not.** `T-ephemeral-view-state-rides-the-request` with `D-a-trip-survives-being-tidied` beside it — **the test says when storage is legitimate, never that it is required.**

## The one thing blocking the spec from being implementable

**`the server believes it is "<now>"` does not resolve its placeholder.** The step exists at `steps/triage.rs:10`, but the handler passes the captured text straight to the instant parser without looking `<now>` up in the example row — every existing caller writes a literal timestamp. **Four rows across `-03` and `-08` fail with `bad pinned instant "<now>"` rather than `unsupported step`**, which is a different red and worth telling apart while reading the file.

**Fix the handler, not the Gherkin.** Every other step module already carries the four-line `resolve` helper for exactly this. **Do not write the timestamps out as literals** — *which instant the server believes in* is the parameter this feature most needs to mutate, and a week boundary asserted against one hardcoded Monday is a week boundary asserted once.

## Monday needs a timezone, and you are inheriting a trap

Use `settings::current_timezone` + `scheduler_core::timezone::resolve`, the path `committed/body.rs:30-32` takes (`T-timezone-is-a-setting`). **Do not reach for UTC and do not add a second notion of the owner's zone.**

**#118 is open because `/timezone` is `POST`-only and reachable from no page.** The live database reads `America/New_York`, so the owner is fine today, **but the schema default is `UTC`** (`0006_guardrails.sql:39`). This makes a *second* capability depend on a setting nobody can edit. **Say so in the handoff; do not fix #118 here.**

## Add `/quota` to the two browser gates — a scope call, deliberately made

`scripts/qa/colour.cjs` and `scripts/qa/phone_layout.cjs` each enumerate a **hardcoded three-screen list**, so `/quota` has never been measured for palette conformance, dark mode, contrast, tap targets at 390px, or the no-horizontal-scroll rule.

**PR #144 assigned this to those slices rather than itself, which was defensible then and is not now**: the screen is about to be finished and merged, and **a whole screen escaping its gate is exactly how #137's four-day-old palette settlement quietly stops being true.** One line in each list.

**If adding it turns either gate red, that is the point** — find it before merge, not after. Report what it found rather than adjusting the screen silently to suit a check.

## Where the writes go

- **`T-set-operations-execute-in-the-store`.** *This week's* sessions for a quota, and the total against the target, are set operations. **One statement, not a fetch-and-sum in a handler** — and the week boundary is part of the query, not a filter applied afterwards.
- **`T-one-front-door-per-capability`.** Logging a session, correcting one and deleting one are **one capability with one front door.** Three routes may call it; three write paths may not.
- **`T-422-is-product-wide`.** `-09` — a session of no minutes is not a session — is a `422` whose body is the re-rendered fragment it failed against.
- **`T-forms-swap-one-fragment`.** The quick-log controls, `This week` and the readout all change together; that is one fragment with one id.
- **`T-migrations-append-only`.** `0014` is applied and immutable. Sessions get `0015`.

## Watch

1. **`T-a-check-must-be-seen-to-fail`.** Nine scenarios go from red to green, which is **not** the same as being seen to fail — they have never been observed failing *against an implementation*. **Break each rule deliberately once it works** and record the message, the way `0416241` did with six.
2. **#145 — the manifest cannot record a survivor.** `quota_sessions.feature`'s manifest will read 100% whether or not it is. **Do not trust it, do not hand-edit it, and do not let it stand in for the mutation evidence.** Report what the tool actually printed.
3. **The language-mutation run returns the moment the suite is green.** It has been unavailable for a whole slice — expect it to have something to say.
4. **`quota::check_name` runs in memory** (`store::existing_names` has no `WHERE`) — a known, tracked exception in the Gaps index. **Not yours.** It needs the quota identity ruling that belongs with #138.
5. **DRY was 2.05% at #144** against a 3% product-code threshold. A second step module for the same screen is the thing most likely to cross it — **extract a shared family early rather than at the end.**
6. Base is **`origin/slice/quota-screen`**. Scratch in `./tmp/`. **The pull request already exists (#144) and is already labelled `preview`** — push to the branch; do not open a second one.

## Out of scope

`Filed here`, the triage change, retiring `TaskKind::Quota` and deciding `period` (**#138**); reorder controls (**#139**), **whose absence must not be asserted — on a quota it is undecided, not settled**; **#118**; **#145**; **#146**; **#108**; and any visual design system.

## Source

- `features/quota_sessions.feature` (on `slice/quota-screen`) — **the specification. Its header is the brief for the hard parts.**
- `qa/quota_sessions.md` — the QA document already written alongside it
- `crates/acceptance-tests/src/steps/triage.rs:10` — the clock step that cannot resolve `<now>`
- `crates/trellis-server/src/quota/` and `crates/scheduler-core/src/quota.rs` — the entity to build on
- `crates/trellis-server/src/committed/body.rs:30-32` — how a screen resolves the owner's timezone
- `docs/decisions.md` — `D-logging-is-retrospective-and-separate`, `D-quota-no-rollover`, `D-quotas-are-selected-not-typed`, `T-timezone-is-a-setting`, `T-set-operations-execute-in-the-store`, `T-ephemeral-view-state-rides-the-request`, `D-a-trip-survives-being-tidied`
- **#146** — why the owner could not see the fourth tab, and why finishing on this branch is what fixes it
