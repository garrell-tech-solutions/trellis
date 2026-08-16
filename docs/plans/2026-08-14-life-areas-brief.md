# Handoff brief — `life-areas`

**Date:** 2026-08-14 · **Issue:** #47 · **Milestone:** M1 — Capture + Triage · **Route:** pipeline

---

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

Open `http://localhost:8080`:

1. The app knows five life areas out of the box: **Work · Fitness · Learning · Family · Home**.
2. Add one the seed does not have — **Side project**. It appears immediately; no restart.
3. Capture "sketch the landing page". Triage it, choosing **Side project**.
4. The task list shows it, tagged with its life area.
5. Restart the server and reload. The life area and the task are both still there.

**Step 2 is the one worth the trip.** `T-life-areas-are-data` settled that adding a life area is never a development task — no rebuild, no config file, no migration. This is where that becomes something the owner can do rather than something a decision log asserts.

## Goal and scope

Make life areas a first-class, user-managed entity, and let triage tag a task with one.

Resolves #36, which sat open for two days blocking the M1 classifier story. **Unblocks S4** on #9.

### The cut

Two halves, shipped together, because the first without the second is a list that does nothing:

- **Manage them** — the app lists life areas and lets the owner add one, without a restart.
- **Use them** — triage lets the owner pick a life area for the task it creates, and the task list shows it.

The issue records an alternative cut splitting these into two slices. **Take the combined cut** unless you find a reason not to; a surface whose only function is to feed a later slice is thin even by `D-visible-slices` standards.

### Out of scope — do not absorb

The keyword classifier (S4 — this unblocks it, it does not do it), capture dismissal (#48), guardrails and capacity (M2), the reckoning (M8), `/stats` reporting by life area, and **any visual design system**. There is still no visual direction in this repo and inventing one inside a feature slice would bury an unreviewable decision. Plain and ugly remains correct.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `T-life-areas-are-data` | Life areas are user-managed rows, editable from the running app. **Not** a Rust enum, **not** a config file. A life area is a lookup key, not a discriminant — which is why the closed-set precedent (`T-three-task-kinds`, `T-unknown-kind-rejected`, `T-period-closed-set`) does **not** apply here. Read the entry before arguing with this. |
| Seed | **Work · Fitness · Learning · Family · Home.** The four cited in settled decisions, plus Home for errand and admin traffic. A seed, not a ratification. |
| `D-single-user` | One user. No accounts, no sharing, no permissions on a life area. |
| `D-kill-means-archive` | This project archives; it does not delete. See open question 1. |
| `T-migrations-append-only` | `0002`, `0003` and every migration on `trunk` are frozen, and CI enforces it. New migration only. |
| `T-package-by-business-domain` | The tree names capabilities. Life areas are a capability, so they get their own directory — not a slot in someone else's. |
| `T-capability-owns-its-queries` | A capability owns the SQL it issues, not the table it touches. If `triage` needs to validate a life area id, that query is `triage`'s, in `triage/store.rs` — not a function borrowed from the life-areas module. |
| **Core vs adapter** | *A rule that survives changing HTTP for something else belongs in `scheduler-core`.* Introduced by `stats-ratio` and now the standing test. Whether a life-area name is valid is that kind of rule; how it is submitted is not. |
| `T-templates-take-view-models` | Templates render view models, never `store` row types. |
| `T-forms-swap-one-fragment` | Forms inline in the row they act on; a page region rendered by more than one handler is one shared fragment; a rejection re-renders that fragment carrying the error, as `422`. This is the third page to follow it. |
| `T-complexity-8` | Threshold 8. **And see gotcha 2 — this now has a CI gate behind it.** |

## Acceptance scenarios worth specifying

Yours to structure. These are the behaviours that matter.

- A fresh database offers the five seeded life areas.
- A life area added from the page is immediately available to triage, with no restart.
- Triage tags the task with the chosen life area, and the task list shows it.
- The tag survives a restart.
- A life area cannot be added twice — see open question 3 on what "twice" means.
- Removing a life area does not orphan or delete the tasks in it (open question 1).
- Hostile text in a life-area name stays escaped wherever it renders. `#30` proved this for the inbox and `#33` for the task list; a life area is a third render surface and the same payload should be re-asserted here.
- **Existing acceptance features pass untouched.** Capture, triage rejection, the inbox and `/stats` behave identically. If one needed changing, something got rebuilt that should have been reused.

## Known repo gotchas

The tooling changed substantially today. Gotchas 1–3 are new since the last brief.

1. **`trunk` must be green before you start.** It was red at the time of writing (#55). Confirm CI is green on your base commit; if it is not, stop and report rather than inheriting someone else's failure.
2. **A new step module means a new dispatch chain, and there is now a CI gate on that.** `scripts/ci/complexity-baseline.json` pins the accepted over-threshold set with **exact** scores, and `scripts/ci/complexity_baseline.sh` fails when the set grows, when a score moves in either direction, or when a row goes stale. Adding `steps/life_areas.rs` will add a row and will move `steps/mod.rs::dispatch` by one. **Both are legitimate and both must be recorded in the same commit, with reasons.** Read that script's header — *"adding a row here is a decision, not a formality."*
3. **Keep the dispatch chain a chain.** This is what broke `trunk` today: `stats_ratio::dispatch` scored 17 because three of its arms inlined a result unwrap before delegating instead of being one-line delegations. `T-complexity-8`: *a function over 8 is carrying logic that is not the match — extract that, do not flatten the match.* Put the logic in the handler; keep the arm one line. **Do not flatten any dispatcher into a `(Regex, handler)` table** — the 2026-08-12 log entry settles that, and the threshold is in the constitution and cannot be raised.
4. **Run the analyzers in order.** `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`. `coverage.sh` and `crap.sh` report nonsense without the generated acceptance entrypoints. Expect **12** features before your change.
5. **`cargo test --workspace` still compiles zero acceptance tests on a fresh checkout** — the entrypoints are gitignored. CI now gates the real suite (#26), but locally you must run `scripts/acceptance/run.sh` yourself.
6. **`platform/boundary.rs` will fail your build** if you write production SQL outside a `store.rs` or `platform/db.rs`, name a persistence module in delivery vocabulary, or create a top-level directory with a technical-role name. It walks `src/`, so your new capability directory is covered the moment it exists.
7. **`trellis serve --now <RFC3339>`** exists — an offset, not a freeze. `scripts/qa/lib.sh`'s `qa_start_server` takes it as an optional fourth argument. You probably do not need it; know it is there.
8. **Migration numbering is first-come.** #48 is also queued for a new number. Rebase onto `trunk` before assuming one is free.
9. **Don't copy `task_kinds.feature`'s step style** — its regexes match the placeholder `"<(\w+)>"`, so one Examples cell feeds both the request and the assertion and the scenario cannot fail under mutation. `committed_triage_validation` does it correctly; copy that.
10. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
11. **Open the pull request when QA is done**, before taking another brief.

## Open questions for you

1. **What does removing a life area do — and is removal in this slice at all?** This project archives rather than deletes, every time it has faced the question: `D-kill-means-archive` ("keep the row"), `T-archived-at-only`, and a capture row is never deleted. So an `archived_at` on the life area, hidden from pickers but still resolving for tasks that reference it, is the consistent answer. But **the demo does not require removal**, and a slice that only adds is smaller. Decide whether removal is in scope; if it is, say what an archived life area does to a task still in it.
2. **Is a life area required at triage, or optional?** `T-quota-targets-required` reasoned that a field the downstream cannot function without should be required at the boundary rather than left nullable by storage convenience — which points at required. Against it: S4 exists to pre-fill this, and requiring it before the classifier ships puts the work on the owner at the moment `D-menu-of-three` says to remove friction. Both are defensible; the argument matters more than the answer.
3. **Uniqueness and case.** Is "Work" the same life area as "work"? Every closed domain in this codebase is case-sensitive by explicit test (`Priority::parse("p1") == None`), but a user-typed name is a different situation from a wire enum. Say what you chose and where the check lives.
4. **Does a capture carry a life area too, or only a task?** `T-classifier-covers-domain` has the classifier filling `domain` on the **capture**, between capture and triage. If only tasks carry it, S4's output has nowhere to land — and retrofitting that column after S4 depends on it is the expensive order, which is the argument `T-three-task-kinds` already made once.

## Dependencies and sequencing

- **Blocked by #55** until `trunk` is green.
- Depends on #44 (merged, PR #49) for the module tree, and #33 (merged, PR #43) for the triage surface this extends.
- **Unblocks S4** on #9, which has been blocked since 2026-08-13.
- **#48 (capture dismissal) is queued behind this** and touches the same inbox page. They do not run concurrently.
- Does not interact with `/stats` (#45, merged) — that counts task *kinds*, not life areas.

## Source

- Issue **#47** — acceptance criteria and the demo
- Issue **#36** — the question this settles, and two claims it made that turned out false
- `docs/decisions.md` — `T-life-areas-are-data` above all, plus the 2026-08-14 version note
- `docs/design/architecture.md` — the module tree, the schema, the three meanings of "domain", and the Life areas section
- **#9** — M1 epic, story S4
- **#55** — why `trunk` may still be red when you read this
