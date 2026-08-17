# Handoff brief — `dismiss-capture`

**Date:** 2026-08-17 · **Issue:** #48 · **Milestone:** M1 — Capture + Triage · **Route:** pipeline

> **This is the last M1 slice.** AC-5 and story S4 — the keyword classifier — were cut to M9 on 2026-08-17 (`D-manual-triage-until-llm`). When this merges, M1 closes.

---

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

Open `http://localhost:8080`:

1. Capture junk — `asdfgh`. **Dismiss it from its row.** It leaves the inbox.
2. Capture `buy milk`. **The life-area picker now starts on nothing.** Click **Pool** without choosing one — it is refused, and the message appears on that row.
3. Choose **Home**, click **Pool**. It moves to the task list, tagged Home.
4. Restart the server and reload. The inbox is empty, `buy milk` is still in the task list tagged Home, and `asdfgh` has not come back.

**Two steps are worth the trip, for different reasons.** Step 1 is the feature: until now a capture had exactly one way out of the inbox — become a task — so junk was permanent. Step 2 is where `D-manual-triage-until-llm` becomes something the owner can see: the picker used to default to Work by accident of `ORDER BY id ASC`, so every unattended pool capture filed into Work.

## Goal and scope

**Give the inbox a "no", and stop the picker answering for the user.**

Closes **AC-4** on #9 (a capture row is never deleted) and carries **AC-2**'s remaining assertion. Both are M1's last open criteria.

### The cut — two halves, shipped together

- **Dismiss.** A capture can be dismissed from the page. It leaves the inbox, creates no task, and **the row stays**.
- **No preselection.** The life-area `<select>` on all three triage forms starts on a blank option. A submission naming no life area is refused.

They ship together because the second is three lines in a template that #48 is already editing, and splitting it would cost a whole pipeline trip for a template change. If you find they genuinely conflict, say so rather than absorbing the conflict.

**The second half is smaller than it looks, and reuses rather than adds.** `#47` already made `life_area` required for all three kinds, so a submission naming none is *already* rejected — `{"missing_field": "life_area"}`, via `T-empty-equals-absent`, which makes an empty string and an absent key report identically. Today the page simply cannot produce that submission, because the `<select>` has no empty option. Adding one makes an existing, specified rejection reachable from the page. **Do not invent a new rejection variant for this.**

### Out of scope — do not absorb

Un-dismissing, any dismissed-captures view, editing or deleting a **task**, life-area management beyond the picker's default (`#47` shipped that), the classifier (gone to M9), `/stats` (#45), and **any visual design system**. There is still no visual direction in this repo; inventing one inside a feature slice buries an unreviewable decision. **Plain and ugly remains correct.**

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-kill-means-archive` | Keep the row; **build no browsable archive UI.** *"The moment an archive is browsable it becomes a place to hide from decisions."* So: no dismissed-captures list, no un-dismiss affordance. |
| `D-three-strike` | Friction is a feature in exactly one place in this system, and this is not it. **No confirmation dialog on dismiss.** |
| `D-inaction-archives` | Dismissal is the deliberate act; it is not the same as leaving a capture alone. |
| `D-manual-triage-until-llm` | **New, 2026-08-17.** Nothing classifies a capture before M9. The picker carries no preselection, because a default the user never chose is silently wrong — `D-inaction-archives` applied to triage. Read the entry before arguing that a sensible default would be friendlier. |
| `D-pool-is-default` + amended **AC-2** | Pool must be **cheapest**: no expanding `<details>` form, and strictly fewer inputs than committed or quota. It is no longer "a single action" — that phrasing was amended on 2026-08-17 precisely because a manual life-area pick makes pool two inputs against committed's six. **Do not restore a default to get back to one click.** |
| `T-migrations-append-only` | `0001`–`0004` are frozen and CI enforces it. New migration only. **You take `0005`** — `0004` went to life-areas. |
| `T-archived-at-only` | Directly relevant to open question 1. *"Two fields for one state … every path gets two chances to set one and forget the other."* |
| `T-forms-swap-one-fragment` | Forms inline in the row they act on; `#lists` is one shared fragment with one id; a rejection re-renders **that same fragment** carrying the error, as **422**. htmx is configured to swap on 422 for the whole page. This is the fourth surface to follow it. |
| `T-templates-take-view-models` | Templates render `http::view` models, never `store` row types. |
| `T-capability-owns-its-queries` | A business domain owns the SQL it issues, not the table it touches. Dismissal's query is dismissal's, wherever you decide dismissal lives — not borrowed from `capture`. |
| `T-one-front-door-per-capability` | **New, recorded 2026-08-16.** If you need another capability's data, call its `mod.rs` front door (`life_areas::active_options`), never its `store` composed with its `view` yourself. |
| `T-core-owns-validation-order` | **New, recorded 2026-08-16.** Ordered validation composes in `scheduler-core` and returns one result; the adapter keeps only the part needing a database. |
| `T-complexity-8` | Threshold 8, **with a CI gate behind it.** See gotcha 3. |

## Acceptance scenarios worth specifying

Yours to structure — these are the behaviours that matter.

- A capture can be dismissed from its inbox row; it leaves the inbox and **no task is created**.
- **The capture row is not deleted.** Row count is monotonic across arbitrary sequences of capture, triage and dismissal — `#9` AC-4 asks for a *property*, not an example.
- A dismissed capture does not reappear after a restart.
- **A capture cannot be both triaged and dismissed.** See open question 1; this is a schema question before it is a handler one.
- The life-area picker offers no preselection on all three triage forms, **including the row rendered by the quick-add box** (`capture_row.html` is rendered by `create_capture` too, so this should be automatic — assert it anyway).
- Submitting a triage with no life area chosen is refused, the message lands on that row, and **the response is 422** carrying the re-rendered fragment.
- **Pool requires no expanding form** and takes strictly fewer inputs than committed or quota — AC-2's remaining half.
- Hostile text stays escaped on any new surface. `#30`, `#33` and `#47` each re-asserted this; a dismissal affordance is not a render surface, but check whether your changes add one.
- **Existing acceptance features pass untouched.** Capture, triage, rejection, the inbox, `/stats` and life areas behave identically. If one needed changing, something got rebuilt that should have been reused.

## Known repo gotchas

1. **`trunk` is green at `77869c3`.** Confirm CI is green on your base commit before starting; do not inherit someone else's failure.
2. **You take migration `0005`.** `0004_life_areas.sql` is taken. Nothing else is queued behind you, so the number is safe — but rebase onto `trunk` before assuming it.
3. **A new step module means a new dispatch chain, and there is a CI gate on it.** `scripts/ci/complexity-baseline.json` pins the accepted over-threshold set with **exact** scores; `scripts/ci/complexity_baseline.sh` fails when the set grows, when a score moves *in either direction*, or when a row goes stale. Adding `steps/dismiss.rs` adds a row and moves `steps/mod.rs::dispatch` by one. **Both are legitimate and both must be recorded in the same commit, with reasons.** Read that script's header — *"adding a row here is a decision, not a formality."*
4. **Keep the dispatch chain a chain.** Every arm a one-line delegation; put logic in the handler. This is what broke `trunk` on 2026-08-14 (`stats_ratio::dispatch` scored 17 because three arms unwrapped a result before delegating). `T-complexity-8`: *a function over 8 is carrying logic that is not the match — extract that, do not flatten the match.* **Do not flatten any dispatcher into a `(Regex, handler)` table** — settled 2026-08-12, and the threshold is in the constitution and cannot be raised.
5. **Run the analyzers in order.** `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`. The first two report nonsense without the generated acceptance entrypoints. **Expect 14 features before your change** (12 in the last brief — life-areas added two).
6. **`cargo test --workspace` compiles zero acceptance tests on a fresh checkout** — the entrypoints are gitignored. CI gates the real suite (#26); locally you must run `scripts/acceptance/run.sh` yourself.
7. **`platform/boundary.rs` will fail your build** if you write production SQL outside a `store.rs` or `platform/db.rs`, name a persistence module in delivery vocabulary, or create a top-level directory with a technical-role name. It walks `src/`, so anything you add is covered the moment it exists.
8. **The five QA fixture scripts already send `life_area` explicitly**, so the blank option should not churn them again. Confirm rather than assume — life-areas broke five scripts this way and each had to be re-verified as fixture drift rather than regression.
9. **Don't copy `task_kinds.feature`'s step style** — its regexes match the placeholder `"<(\w+)>"`, so one Examples cell feeds both the request and the assertion and the scenario cannot fail under mutation. `committed_triage_validation` does it correctly; copy that.
10. **A scenario asserting invariance under exactly the transformation the mutator applies is untestable by that mutator, and looks fully covered.** Found on `life-areas-duplicate-03`, whose `"Work"/"work"/"WORK"` Examples table shared its only axis with the Gherkin mutator's single-character case flip. If a scenario here is about invariance, split it into literal scenarios.
11. **`trellis serve --now <RFC3339>`** exists — an offset, not a freeze. `scripts/qa/lib.sh`'s `qa_start_server` takes it as an optional fourth argument.
12. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
13. **Open the pull request when QA is done**, before taking another brief.

## Open questions for you

The owner has explicitly delegated these to the pipeline — settle them and say what you chose, the way `life-areas` settled its four. **Say it somewhere the decisions log can pick it up**; the last two slices answered their briefs' questions only in commit messages and code, and both had to be reconstructed afterwards.

1. **One column or two, and what forbids the impossible state?** The obvious shape is `dismissed_at` beside `triaged_at`, with the inbox query becoming `WHERE triaged_at IS NULL AND dismissed_at IS NULL`. **But two nullable timestamps permit a row that is both triaged and dismissed** — exactly what `T-archived-at-only` was written about: *"two fields for one state … every path gets two chances to set one and forget the other,"* producing a row alive on whichever surface filters the field that was missed. The counter-argument is that triage and dismissal are genuinely *different outcomes*, not one state spelled twice, so two columns may be right. If two, something must forbid both being set — a `CHECK`, or a single `left_inbox_at` plus an outcome discriminator. **Decide deliberately; `T-migrations-append-only` makes this expensive to reverse.**
2. **Does the row-count property belong in `store` or as its own proptest?** `store::capture` already carries an `#[ignore]`d projection property — for any queue and any triaged subset, `list_untriaged` returns exactly the untriaged captures newest-first. Extending it to cover dismissal is probably right rather than writing a second; say which.
3. **What does the blank life-area option say, and is it a real `<option>` or a `required` attribute?** A `<option value="" selected disabled>` and a `<option value="">— choose —</option>` behave differently: the first cannot be submitted at all, the second submits empty and hits the existing `missing_field` path. **The second is what the demo describes and what exercises the specified rejection** — but browser-side prevention is a legitimate argument, so make the call rather than inheriting mine, and note that a browser-prevented submission is a scenario acceptance tests cannot drive over HTTP.
4. **What does the inbox say when everything is dismissed?** It reads *"Nothing to triage. Add a capture above to get started."* That is still true; confirm it needs no variant.

## Dependencies and sequencing

- **Nothing blocks this.** `#44` (PR #49) and `#47` (PR #57) are both merged; `trunk` is green.
- **Nothing is queued behind it.** The pipeline is empty and no other slice touches these files.
- Depends on `#47` (PR #57) for the life-area picker this changes, `#33` (PR #43) for the triage surface, `#44` (PR #49) for the module tree.
- **Closes M1 on merge.** `#9`'s only remaining criteria are AC-2 and AC-4, both here.

## Source

- Issue **#48** — acceptance criteria, the demo, and the AC-2 fold-in
- Issue **#9** — M1 epic; AC-2 as amended 2026-08-17, AC-4
- `docs/decisions.md` — `D-manual-triage-until-llm` and `T-archived-at-only` above all, plus the 2026-08-16 and 2026-08-17 version notes
- `docs/design/architecture.md` — the module tree, the schema, the Life areas section, and the three meanings of "domain"
- `docs/plans/2026-08-14-life-areas-brief.md` — the previous slice, whose picker this changes
