# Handoff brief — `triage-from-page`

**Date:** 2026-08-13 · **Issue:** #33 · **Milestone:** M1 — Capture + Triage · **Route:** pipeline

---

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

Open `http://localhost:8080`:

1. Capture "buy milk" in the quick-add box — that already works, as of #30.
2. Each inbox item now offers **Pool / Committed / Quota**.
3. Click **Pool**. It leaves the inbox and appears in a task list below.
4. Click **Committed**. Fill in deadline, type and priority. Leave one blank — refused, the reason names the field, nothing is created, the capture stays in the inbox.
5. Click **Quota**. Ask for **0** sessions — refused too.
6. Restart the server and reload. The tasks are still there and the inbox is still short.

**Step 5 is the one worth the trip.** Every rejection rule this project argued over — closed domains, empty-string handling, positive quota targets — has so far only been observable through `curl`. This is where it becomes something the owner can see.

## Goal and scope

Put the triage decision on the page. The endpoint already exists and is fully
validated; this slice gives it a surface and shows the result.

M1 stories 3 and 4, closing acceptance criteria 2 and 3 on #9.

### Out of scope — do not absorb

`/stats` (S3), the keyword classifier (S4), editing or deleting a task, the
calendar, and **any visual design system**. There is still no visual direction
in this repo and inventing one inside a feature slice would bury an
unreviewable decision. Plain and ugly remains correct.

## Decisions already made — confirm, don't re-litigate

Note the new keying: decisions are **slugs** now, not numbers. See
`docs/decisions.md`, "How to read the IDs".

| | |
|---|---|
| `D-visible-slices` | A slice is done when the owner can run it and watch the change. The Demo above is an acceptance criterion, not a garnish. |
| `T-three-task-kinds` | `kind` is `Committed \| Pool \| Quota`. |
| `T-quota-targets-required` | Quota triage requires `target_count`, `target_minutes_each` and `period`. |
| `T-empty-equals-absent` | An empty string and an absent key report identically, as `{"missing_field": <name>}`. |
| `T-period-closed-set` | `period` is `week \| month`. |
| `T-unknown-kind-rejected` | An unrecognised `kind` is `422 {"unknown_kind": <submitted>}`. |
| `T-module-boundary` | Rules in `scheduler-core`; delivery in `trellis-server::http`; rows in `::store`. |
| `T-templates-take-view-models` | Templates render view models, never store row types. `http::view` holds what a page shows. |
| `T-migrations-append-only` | **`0002` is frozen.** The `deadline` CHECK below goes in a new `0003_*`. CI enforces this. |
| `T-complexity-8` | Cyclomatic complexity threshold is 8. |

## Acceptance scenarios worth specifying

Yours to structure. These are the behaviours that matter.

- Each untriaged capture offers all three kinds from the page.
- Triaging removes the capture from the inbox and shows the task in a task list, without a full page reload.
- The committed form refuses a submission missing deadline, deadline_type or priority; the rejection names the field; nothing is created; the capture stays untriaged.
- `deadline_type` and `priority` are **chosen**, not free-typed. The server already closes those domains — the page should not offer a way to violate them.
- Quota requires its three target fields, and refuses a non-positive `target_count`.
- **The page and `POST /captures/{id}/triage` are one code path.** Existing triage acceptance features must pass untouched. If one needed changing, something got rebuilt that should have been reused.
- Hostile capture text stays escaped in the task list, not just the inbox. `#30` proved it for the inbox; this is a second render surface and the same payload should be re-asserted here.

## Folded-in fixes from the PR #31 review

These live in the code this slice already touches, so they ride along rather
than becoming a separate invisible slice.

- **`CHECK` on `tasks.deadline`.** `kind`, `deadline_type`, `priority` and `period` all got one; `deadline` did not — and SQLite's INTEGER *affinity* does not reject text, so `INSERT … deadline='banana'` still succeeds through direct SQL. Verified. `CHECK (deadline IS NULL OR typeof(deadline)='integer')`, **in a new `0003_*` migration**.
- **Quota `target_count` / `target_minutes_each` must be positive.** Both currently accept `0` and negatives — 201, persisted. `T-quota-targets-required`'s own rationale is that the reckoning computes `count(done)/target_count`, so zero is a divide-by-zero and the decision defeats itself.
- **`then_triage_is_rejected` should assert the status it means.** It accepts anything in `400..500`, as does `qa_assert_rejected` in `scripts/qa/lib.sh` — so a malformed-JSON 400 satisfies a test written for a validation 422.
- **`{"kind": 7}` reports `{"unknown_kind": null}`** rather than what was submitted, because `string_field` drops wrong-typed values before the core sees them. `T-unknown-kind-rejected` says report `<submitted>`.

## Known repo gotchas

1. **A slug-migration PR is in flight** touching doc comments in
   `scheduler-core/src/task.rs`, `task_properties.rs` and two step modules —
   the same files this slice edits. Comments versus code, so conflicts should
   be shallow, but **rebase onto `trunk` before you hand off** and expect to
   resolve a few. Three of those comments are currently *wrong* (an off-by-one
   from #37); the migration fixes them. Don't re-introduce old numeric IDs.
2. **Generated acceptance entrypoints are gitignored.** `cargo test --workspace`
   compiles **zero** acceptance tests on a fresh checkout and still reports
   green. Run `scripts/acceptance/run.sh`. Tracked as #26.
3. **Don't copy `task_kinds.feature`'s step style** — its regexes match the
   placeholder `"<(\w+)>"`, so one Examples cell feeds both the request and the
   assertion and the scenario cannot fail under mutation.
   `committed_triage_validation` does it correctly; copy that.
4. **The complexity gate is red** — worst is `steps/triage.rs::dispatch` at 17
   against a threshold of 8. Adding handlers to those dispatchers makes it
   worse. Put new steps in a new module.
5. **`0002` is frozen** and CI will fail a PR that edits it. New migration only.
6. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
7. **Open the pull request when QA is done**, before taking another brief. A
   finished slice sat stranded once because the next brief arrived first.

## Open questions for you

1. **Does triage happen inline in the row, or on a separate page?** Committed
   needs three fields and quota needs three more; that is a lot to expand
   inline, but a separate page costs a round trip and breaks the
   one-page-inbox feel. Whichever you pick becomes the pattern for every form
   in this product — decide deliberately and say why.
2. **Where does the task list live?** Below the inbox on `/`, or its own route?
   #30 established `GET /` as the inbox; a task list is a second concern on the
   same page. Acceptance criteria only require that the task appears.
3. **What does the page do with a rejection?** The endpoint returns
   `422 {"missing_field": …}`. HTMX needs that turned into something visible —
   an error fragment swapped into the form, most likely. Say what you chose;
   it is the first error-display pattern in the product.

## Dependencies and sequencing

- **Depends on #30**, merged — this puts controls on the page it created.
- **Closes M1 acceptance criteria 2 and 3** on #9, taking M1 to 6 of 7.
- After this, only `/stats` (S3) and the classifier (S4) remain in M1 — and S4
  is blocked by **#36**, the contradictory domain list.

## Source

- Issue **#33** — acceptance criteria and the demo
- `docs/decisions.md` — the decisions table above
- **#9** — M1 epic
- PR **#31** review — where the four folded-in fixes came from
- **#26** — the ungated acceptance suite
