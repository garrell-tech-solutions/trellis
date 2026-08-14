# Handoff brief — `stats-ratio`

**Date:** 2026-08-14 · **Issue:** #45 · **Milestone:** M1 — Capture + Triage · **Route:** pipeline

---

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

Open `http://localhost:8080`:

1. Capture and triage three things as **Pool**. Capture and triage one as **Committed**.
2. Open `/stats`. It reports the committed:pool ratio — one in four committed.
3. Triage three more as **Committed**. Reload `/stats`. The ratio has crossed 50%, and the page says so distinguishably from the number itself.
4. Restart the server and reload. The figures are unchanged — they are computed from the rows, not held in memory.

**Step 3 is the one worth the trip.** `D-pool-is-default` asserts the calendar should sit around 40% committed, and #20 calls this *"the highest-leverage counter in the product"* — the whole point is that the owner finds out the ratio is drifting **before** M3 builds a scheduler around the assumption. Until this ships, that number does not exist anywhere.

## Goal and scope

Expose the rolling committed:pool ratio at `/stats`. This is risk experiment **R2**'s instrumentation, and #9's acceptance criterion 6 requires it live *"from M1, weeks before it is read."* The clock starts at merge.

Closes AC-6 on #9. This is the **first slice built under the new module tree** — see the gotchas.

### Out of scope — do not absorb

Life areas (#47), capture dismissal (#48), the keyword classifier (S4), any notification or alerting mechanism (nothing in this product can notify until M7), any change to capture or triage behaviour, and **any visual design system**. There is still no visual direction in this repo and inventing one inside a feature slice would bury an unreviewable decision. Plain and ugly remains correct.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-visible-slices` | The Demo above is an acceptance criterion, not a garnish. |
| `D-pool-is-default` | Pool is the default kind; committed is the exception. ~40% committed is the target this counter measures against. |
| `T-three-task-kinds` | `kind` is `Committed \| Pool \| Quota` — **three**, which is why the denominator is an open question below. |
| `T-package-by-business-domain` | The tree names capabilities. A new surface gets its own directory, not a slot in a technical-role folder. |
| `T-capability-owns-its-queries` | A capability owns the SQL it issues, not the table it touches. Counting tasks for `/stats` is `/stats`'s query, in its own `store.rs` — **not** a function added to `triage/store.rs` because that file already touches `tasks`. |
| `T-templates-take-view-models` | Templates render `http::view`-style view models, never `store` row types. A ratio is a computed thing; it is a view model, not a row. |
| `T-jiff-epoch-millis` | `jiff`, not `chrono`. Timestamps are UTC epoch millis in the column, converted at the boundary. **This slice is the product's first real time-window arithmetic** — see open question 3. |
| `T-forms-swap-one-fragment` | If this page grows any interactive control, it follows the established pattern. It probably has none. |
| **Route is `/stats`** | Settled by AC-6's own wording. Not a section on `/` — `#30` established `/` as the inbox and `#33` already added a task list to it; a third concern on that page is not what the criterion asks for. |
| **Display only** | The alarm "at >50%" is a visible report, not a notification. Nothing in this product can notify until M7 (Telegram), so there is no mechanism to route it to and nothing to decide. |

## Acceptance scenarios worth specifying

Yours to structure. These are the behaviours that matter.

- `/stats` reports the committed:pool ratio over the rolling window.
- A task triaged **outside** the window is excluded from the figures. This is the scenario that proves there is a window at all — without it, "rolling" is untested and the implementation could be counting every row ever.
- Crossing the 50% committed threshold is reported **distinguishably from the ratio itself**. A reader must be able to tell "the number is 62%" from "62% is over the line" without doing the comparison themselves.
- An empty database does not divide by zero, and does not report `0%` as though it had measured something.
- A small sample is not presented as a measurement — see open question 4.
- **`/stats` changes nothing else.** Capture and triage behave identically; existing acceptance features pass untouched. If one needed changing, something got rebuilt that should have been reused.

## Known repo gotchas

1. **The module tree changed today (#44, PR #49) and you are the first slice into it.** `crates/trellis-server/src/` is now `capture/ inbox/ platform/ triage/` — each capability holding its own `http.rs` and `store.rs`, with `platform/` for the composition root, connection, clock, assets and shared response mapping. **Read `crates/trellis-server/src/platform/boundary.rs` before you place a single file**; its module doc explains the rule better than this brief can.
2. **`platform/boundary.rs` is a test that will fail your build**, deliberately, if you: write production SQL anywhere but a `store.rs` or `platform/db.rs`; name a persistence module in delivery vocabulary (`axum`, `StatusCode`); or create a top-level directory with a technical-role name (`http`, `store`, `api`, `services`, `models`, …). It walks `src/`, so a new capability directory is covered the moment it exists — you do not register it anywhere.
3. **Generated acceptance entrypoints are gitignored.** `cargo test --workspace` compiles **zero** acceptance tests on a fresh checkout and still reports green. Run `scripts/acceptance/run.sh` — it should report **11** features before your change. Tracked as #26.
4. **Run `scripts/analyzers/coverage.sh` and `crap.sh` *after* `scripts/acceptance/run.sh`, never before.** Without the generated entrypoints they report ~86.79% coverage, 10 complexity violations and CRAP 5 — every one an artefact of the step handlers never executing. Discovered during #49; it will waste an hour if you meet it cold.
5. **The complexity gate is red at 5 violations**, all regex dispatch chains in `crates/acceptance-tests/src/steps/`. This is deliberate and documented (`T-complexity-8`, 2026-08-12 entry): there is nothing to extract and a table would cost ~40 boxed-future wrappers. **Put new steps in a new module** rather than adding arms to an existing dispatcher, which would make it worse.
6. **The DRY gate measures prose** (#50) — jscpd scans markdown, so editing documentation moves the number. If DRY shifts, check whether the clones are `.md` before chasing anything.
7. **Migration numbering is first-come.** This slice probably needs none. If it does: `0002` and `0003` are frozen and CI enforces it (`T-migrations-append-only`), and #47 and #48 are also queued for a new number — rebase onto `trunk` before assuming one is free.
8. **Don't copy `task_kinds.feature`'s step style** — its regexes match the placeholder `"<(\w+)>"`, so one Examples cell feeds both the request and the assertion and the scenario cannot fail under mutation. `committed_triage_validation` does it correctly; copy that.
9. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
10. **Open the pull request when QA is done**, before taking another brief. A finished slice sat stranded once because the next brief arrived first.

## Open questions for you

1. **What is the denominator?** "committed:pool" names two kinds; `T-three-task-kinds` made it three. Is quota counted with pool, counted separately, or excluded? `D-pool-is-default`'s claim is about *calendar occupancy* — *"a calendar that is ~40% committed is one the user trusts"* — and quota tasks do get placed at M8, so "everything that is not committed" and "pool" are not the same set. R2's 50% threshold was written before this distinction existed. Whatever you choose, the page should make it legible; a bare percentage whose denominator is ambiguous is a number that will be misread for months.
2. **What timestamps the window — the task or the capture?** `tasks.created_at_ms` is triage time; `captures.created_at_ms` is capture time. R2 says *"instrument the ratio at triage"*, which points at the task row. They diverge whenever a capture sits in the inbox for days, which is the normal case.
3. **What is a "2-week rolling window", exactly?** This is the product's first genuine time arithmetic and `T-jiff-epoch-millis` exists for it. Is it 14 × 24h back from now, or 14 calendar days in a civil zone? `T-jiff-epoch-millis` chose `jiff` precisely because *"guardrails are civil wall-clock"* and it *"forces an explicit decision at a DST gap"* — so the civil reading has precedent, but **nothing in this product configures a timezone yet.** If you conclude one is needed, that is an owner decision, not a guess: stop and say so rather than defaulting to UTC silently. A defaulted timezone is a description that outruns the thing it describes, and this project has paid for three of those in a week.
4. **What does it show below a useful sample size?** There are single-digit tasks in the database. A bare "100% committed" from two rows is a number that will be believed and should not be. Suppress it, annotate with n, show raw counts alongside — your call, but it needs making rather than inheriting. `D-staleness-unset` is the precedent for the shape of this answer: instrument first, and be honest that you have not measured enough yet.

## Dependencies and sequencing

- **Depends on #44**, merged (PR #49) — you are the first slice into the new tree.
- Depends on #33, merged (PR #43), for the task rows this counts.
- **Closes AC-6 on #9**, taking M1 to 5 of 7 acceptance criteria.
- Independent of #47 (life areas) and #48 (dismissal) in behaviour — but all three touch the same pages, so they run one at a time, not concurrently. #47 is next after this.
- Does not unblock anything structurally. It starts R2's clock, and R2's data is what should bias S4's classifier.

## Source

- Issue **#45** — acceptance criteria and the demo
- **#20** — the risk register, R2 in full
- **#9** — M1 epic, AC-6
- `docs/decisions.md` — the decisions table above
- `docs/design/architecture.md` — the module tree, the schema, and the three meanings of "domain"
- **#26** — why `cargo test --workspace` is not enough
- **#50** — why the DRY number moves when you edit documentation
