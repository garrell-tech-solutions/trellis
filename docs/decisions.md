# Trellis — Decisions Log

Append-only. Newest at the bottom of each section.

This file exists to record **why**, and especially **what was rejected and on
what grounds**. The project board tracks what we are doing; it cannot tell a
future contributor that a plausible-sounding idea was already considered and
refused. If you are about to propose something listed under "Rejected", read
the reasoning first — and if you still disagree, add a dated entry arguing
against it rather than quietly reversing it.

- **State** (what is in flight, what is done): the project board —
  https://github.com/orgs/garrell-tech-solutions/projects/4
- **Phases and acceptance criteria**: GitHub Milestones and their epic issues.
- **Architecture**: `docs/design/brief.md` — **does not exist yet.** Every epic
  links to it. Until it lands, the vocabulary it would define lives nowhere; see
  the 2026-08-12 vocabulary note below.
- **Rationale and rejected options**: this file.

## How to read the IDs

Decisions are keyed by a **stable slug**, never a number. `T-jiff-epoch-millis`,
not `T3`.

| Prefix | Where | Means |
|---|---|---|
| `D-*` | this file | Settled **product** decision |
| `T-*` | this file | Settled **technical** decision |
| `R-*` | this file | **Rejected** — considered and refused, with why |
| `C`_n_ | issues #2–#6 | **Correction** — a place the original brief contradicts itself |
| `U`_n_ | issue #7 | **Underspecified** — the brief is silent and an agent will guess |

**Open questions are issues, not rows here.** A question with no answer is
state, and state lives on the board. Settled decisions live in this file because
every agent reads it at startup, from a worktree, offline — and because the
point of the "Rejected" section is that someone about to re-propose an idea
trips over the reasoning without having to know it exists.

### Why slugs

Sequential numbers were allocated on branches, and branch order is not merge
order. That failed twice in two days. `T17` was claimed independently by two
branches; then a third branch renumbered its source comments to a scheme that
never existed, leaving every citation off by one — and each wrong number
resolved to a *real* decision, so nothing looked broken.

A slug is derived from content, so two branches cannot silently collide, and a
citation that is wrong does not quietly resolve to somebody else's decision.
Slugs are never renumbered, so a reference in a source comment stays true for
the life of the file. **CI enforces that every slug cited under `crates/`
exists here.**

### Former numbering

Commits, pull requests and issues written before 2026-08-13 cite the old
numbers. This table is the only reason to keep them.

| Slug | Was |
|---|---|
| `D-gaps-offered-not-filled` | D11 |
| `D-guardrails-never-yield` | D3 |
| `D-inaction-archives` | D7 |
| `D-kill-means-archive` | D5 |
| `D-menu-of-three` | D10 |
| `D-no-pool-on-calendar` | D9 |
| `D-pool-is-default` | D2 |
| `D-quota-no-rollover` | D13 |
| `D-silence-means-done` | D4 |
| `D-single-user` | D1 |
| `D-skipped-review-ages` | D6 |
| `D-staleness-unset` | D12 |
| `D-three-strike` | D8 |
| `D-visible-slices` | D14 |
| `R-auto-promote-on-age` | R4 |
| `R-browsable-archive` | R2 |
| `R-collaboration` | R6 |
| `R-gcal-webhooks` | R9 |
| `R-guardrail-override` | R1 |
| `R-incremental-patching` | R10 |
| `R-multi-tenancy` | R5 |
| `R-plugin-surface` | R7 |
| `R-pool-on-calendar` | R3 |
| `R-supabase` | R8 |
| `T-archived-at-only` | T14 |
| `T-auto-close-event` | T13 |
| `T-capture-surfaces` | T8 |
| `T-classifier-covers-domain` | T22 |
| `T-complexity-8` | T9 |
| `T-core-no-tokio` | T4 |
| `T-empty-equals-absent` | T18 |
| `T-fact-plan-line` | T20 |
| `T-gcal-is-render-target` | T7 |
| `T-greedy-with-repair` | T6 |
| `T-handrolled-gcal-client` | T5 |
| `T-jiff-epoch-millis` | T3 |
| `T-migrations-append-only` | T21 |
| `T-module-boundary` | T15 |
| `T-mutation-parallelism` | T10 |
| `T-period-closed-set` | T19 |
| `T-pins-in-constraints` | T12 |
| `T-quota-targets-required` | T17 |
| `T-rust` | T1 |
| `T-sqlite-sqlx` | T2 |
| `T-templates-take-view-models` | T23 |
| `T-three-task-kinds` | T11 |
| `T-unknown-kind-rejected` | T16 |

Open questions moved to issues: O1 → #38 · O2 → #1 (settled) · O3 → #39 ·
O4 → #40 · O5 → #41 · O6 → #36. Issue #1 is titled `OQ2`; same question.

---

## Settled — product

| # | Decision | Reasoning |
|---|---|---|
| D-single-user | Single user, single tenant. | "Family" is a domain of the user's own time, not a shared household schedule. |
| D-pool-is-default | Pool is the default task kind; committed is the exception. | Motion's schedule-everything model is exactly why it churns. A calendar that is ~40% committed is one the user trusts, because a block exists only because it had to. |
| D-guardrails-never-yield | Guardrails never yield to deadlines. | A P1 hard deadline with Work hours full raises a conflict; it does not breach the wall. An override that exists will get used, and then the walls are decorative. |
| D-silence-means-done | Silence on a check-in means **done**. | Assuming "not done" is intuitively safer but is the exact mechanism by which every abandoned task app decayed: unanswered work accumulates and the backlog inflates with phantom obligations. |
| D-kill-means-archive | Kill means archive. Keep the row; build no browsable archive UI. | The row feeds the reckoning ("47 archived this quarter, 31 Learning" is real signal). The moment an archive is browsable it becomes a place to hide from decisions. |
| D-skipped-review-ages | A skipped weekly review ages everything one more week. No deeper sweep. | The week the user skips is the week their life was chaotic — the worst possible moment to punish them. Aging is self-correcting. |
| D-inaction-archives | Inaction archives. Survival requires a deliberate act. | Inverts the property shared by every task app the user has abandoned, where inaction preserved tasks. |
| D-three-strike | Three-strike: on the third deferral, "keep" is removed. Commit or kill. | A task protected weekly but never touched is a lie. This is the one place in the system where friction is a feature; everywhere else, optimize it away. |
| D-no-pool-on-calendar | Pool tasks never appear on the calendar — not even as all-day chips. | The calendar's entire value is that everything on it is true. Ambient chips are the first crack. |
| D-menu-of-three | The menu returns exactly three options plus a rest option. | A long ranked list is a blank page with extra steps — the same decision fatigue the product exists to eliminate. |
| D-gaps-offered-not-filled | Emergent gaps are offered, not filled. | Dragging Thursday's deep work into a random Tuesday hole is behaving badly. Recompute committed work at domain boundaries; surface the menu mid-stream. |
| D-staleness-unset | Staleness threshold is deliberately unset; instrument first, tune at the first monthly reckoning. Per-domain config. | The pool's turnover rate is unknown until the system runs. Seeded at 21 days / ≥3 offers. |
| D-quota-no-rollover | An unmet quota does **not** roll over. A missed week is missed. | Carrying it forward re-creates exactly the accumulating phantom obligation that D-silence-means-done (silence-means-done) exists to prevent. "You did 1 of 3 runs last week" is a reckoning fact, not a debt. Paired with T-three-task-kinds. |
| D-visible-slices | **Every pipeline slice ends in something the user can run and see.** A slice is not done when its tests pass; it is done when the owner can start the app and watch the new behaviour happen. A slice with no visible surface carries the thinnest surface that exposes it. Size does not matter — smaller is better. | This is the owner's professional standard with their own clients, not a preference about this project: continuous visible delivery is the customer's best case, and building for fifteen slices before the customer can try anything is the failure mode it exists to prevent. Three things it buys, all of which the plan was otherwise deferring. **Feedback:** #20's risk register (R-guardrail-override/R-browsable-archive/R-pool-on-calendar) names three assumptions that are all behavioural — silence-means-done, the ~40% committed ratio, whether the menu beats a list. None can be tested by building a correct scheduler; they need the owner living with the thing, so calendar time is the scarce resource, not engineering time. #9's AC-6 already conceded this by requiring `/stats` live "weeks before it is read". **Clear thinking:** a working system is a better argument about what to build next than a roadmap is. **Value:** the product is useful as an inbox long before it is useful as a scheduler. Cost, accepted knowingly: some slices grow a surface they would not otherwise need, and early surfaces are plain and will be reworked. Rejected alternative: keep the horizontal milestone cut and add UI at the end — that is precisely the fifteen-slice wait, and it defers every behavioural risk to the point where acting on what is learned is most expensive. |

## Settled — technical

| # | Decision | Reasoning |
|---|---|---|
| T-rust | Rust. | The scheduler core is interval arithmetic over sum types, the invariants are property-testable, and the artifact is a static musl binary plus one SQLite file. |
| T-sqlite-sqlx | SQLite + `sqlx`, WAL mode. | Self-hosted Supabase is ~8 containers solving problems this product does not have. All queries stay inside `scheduler-db` so a Postgres swap stays mechanical. |
| T-jiff-epoch-millis | `jiff`, not `chrono`. | Guardrails are civil wall-clock. `jiff` models zoned vs civil time as distinct types and forces an explicit decision at a DST gap. Store UTC epoch millis, convert at the boundary. |
| T-core-no-tokio | `scheduler-core` must not depend on tokio. | If it compiles without an async runtime, the algorithm has been kept honest. Enforced in CI from M0. |
| T-handrolled-gcal-client | Hand-rolled `reqwest` + `serde` Google Calendar client. | Only six operations are needed; ~300 lines fully controlled beats fighting a generated crate. |
| T-greedy-with-repair | Greedy-with-repair, not a constraint solver. | Explainable and fast enough for one user. Keep the interface clean so an optimizer can be swapped in behind it later. |
| T-gcal-is-render-target | Google Calendar is a render target, never a source of truth for task blocks. | |
| T-capture-surfaces | Capture surfaces for v1: web quick-box and Telegram. CLI and iOS Shortcut deferred. | |
| T-complexity-8 | Cyclomatic complexity threshold is **8** per function, not the template's 4. | Derived from the domain model rather than borrowed from convention. The largest enums (`Domain`, `BlockState`) have 5 variants, so an exhaustive `match` over one scores 6 — meaning a threshold of 4 would fail on almost every match in a codebase that is deliberately sum-type-heavy. Enforced at 4, the gate would push agents to split clear matches into indirection that is strictly worse to read, and the analyzer would be shaping the architecture instead of guarding it. 8 leaves headroom for one guard above the largest match while still catching genuine branching thickets. **A function over 8 is carrying logic that is not the match — extract that, do not flatten the match.** Revisit if a legitimate enum grows past 6 variants. |
| T-mutation-parallelism | Mutation parallelism is 8 for `scheduler-core` and **1** everywhere else. | `scheduler-core` is pure — no I/O, no shared mutable state — so its mutants are safely parallel. Every other crate's mutants open the same SQLite file and truncate each other's database between test setup and assertion. WAL mode does not save you. The symptom is timeouts on mutants that cannot possibly hang, so it reads as flakiness and gets "fixed" by raising the timeout. **Do not raise the timeout.** Use `--jobs 1`, or give each worker a `TMPDIR`-scoped database and record how here. `scheduler-gcal` mutants run against the mock transport, never the live API. |
| T-three-task-kinds | Task `kind` is a **three-variant** sum type — `Committed \| Pool \| Quota` — committed in the schema at M1. Quota fields (`target_count`, `target_minutes_each`, `period`) are nullable. Quota *scheduling* waits until M8. | Resolves #1. The weekly review is itself a recurring commitment, so without a quota kind M8 needs a bespoke recurring mechanism for exactly one task — which is how a special case becomes permanent. Fitness is one of five domains and inherently quota-shaped: no deadline, so it cannot be committed; but as pool it is never placed, and a Fitness guardrail nothing is scheduled into is a wall protecting an empty room. Capacity math is false in the same way C5 makes the Work number false if quota demand is uncounted. And in Rust, adding a variant later turns every `match` into a compile error — that is the *good* case; the bad case is `if committed { .. } else { /* pool */ }`, which silently treats quotas as pool. Behaviour waits because quota semantics are unsettled and M3 is already the riskiest milestone. |
| T-pins-in-constraints | Pins are a first-class entity in the Constraints layer: `pin { task_id, start, end, source }`. `pinned` is **not** a column on `Block`. Lands at **M3**, with the `schedule()` signature that consumes it — not at M1. | Resolves C1. Blocks are Plan-layer: disposable, engine-written, deleted wholesale on every recompute (R-incremental-patching). A pin is user-authored intent, so a pin living on a block cannot survive the recompute that deletes its row — which makes M3's regeneration property ("delete all future blocks, re-run, get byte-identical placements") and M6's "a drag creates a pin that survives the next recompute" mutually unsatisfiable. `schedule(tasks, hard_events, guardrails, **pins**, now)` had already made the call implicitly by taking pins as an *input*. Rejected alternative: keep `pinned` on `Block` and exempt pinned rows from deletion — that makes the Plan layer partly durable, the exact fact/plan confusion C2 is separately untangling, and weakens M3's strongest property to "delete all *non-pinned* blocks". Deferred to M3 because pins have no M1 behaviour: no `Block` to drop the column from, no scheduler to consume them, no drag to create one. |
| T-auto-close-event | Auto-close is recorded as an **event**, not a column: `auto_close_event { task_id, closed_at, remaining_minutes_before, undone_at }`. Lands at **M6** with auto-close itself. | Resolves C4, and goes further than the issue proposed. The AC bundled two requirements into one column: *undo restores exactly*, and *R-guardrail-override can measure an undo rate*. A column on `task` serves the first and cannot serve the second — it is overwritten on the second close, so the event denominator is wrong and a task that auto-closes repeatedly (the strongest possible evidence that silence does **not** mean done for that work) collapses to a single row. Undo restores from the latest event with `undone_at IS NULL`; undo rate is `count(undone_at IS NOT NULL) / count(*)`, which is what R-guardrail-override's 15%/30% thresholds actually need. Falling back to `estimated_minutes` was never viable: it is correct only for never-started tasks, where undo matters least, and silently inflates every partially-completed one. Additive, so it does not block M1. |
| T-archived-at-only | `archived_at` is the single archive signal. `status: dropped` is removed. | Resolves N1. Two fields for one state, in a system where D-inaction-archives makes archiving the *default* outcome reached implicitly from several paths — decay pass, three-strike, and simply closing the review (U7) — means every path gets two chances to set one and forget the other. The resulting half-archived task is alive on whichever surface filters the field that was missed: a task returning from the dead, in a product whose entire value is that the user trusts what it shows. M8's all-surfaces proptest is the test designed to catch this, and it can only assert a clean invariant against one field. `archived_at` also carries strictly more information — the reckoning's "47 archived this quarter, 31 Learning" needs a timestamp, which `status: dropped` cannot supply. |
| T-module-boundary | Three modules, dependencies pointing inward: `scheduler-core` holds the rules; `trellis-server::http` translates requests into core inputs; `trellis-server::store` translates core types into rows. Adapters name core types; the core names neither. | The rules had been living inside the axum handlers, expressed as `serde_json::Value` probes and `StatusCode` returns — the function that wrote a task row took a *transport* type as a parameter and returned an *HTTP* type as its error. That leaves no seam to test a rule at: answering "is this triage valid?" required a running server and a SQLite pool. It also made T-sqlite-sqlx's "a Postgres swap stays mechanical" untrue, since the queries were spread across handler modules instead of confined to one layer. The split is what makes `scheduler-core` non-empty for the first time, and gives T-core-no-tokio's no-tokio rule something to protect. `store/mod.rs` carries a unit test asserting that no store module names `axum` or `StatusCode`: a layering rule nothing checks is a comment. |
| T-unknown-kind-rejected | `kind` is validated against the three variants at the boundary. A submission naming anything else is rejected with `422 {"unknown_kind": <submitted>}`. | T-three-task-kinds committed to a three-variant sum type, but the M1 implementation read `kind` as `payload.get("kind").and_then(as_str).unwrap_or("")` and stored whatever string arrived, so `{"kind":"banana"}` wrote `banana` into a column whose domain is three values. That is the failure mode T-three-task-kinds named — not the `if committed {} else {}` shape it predicted, but a weaker one, with no discrimination at all. Since `tasks.kind` carries no `CHECK` constraint, the only thing standing between a typo and durable out-of-domain data was the caller. Rejecting is a **behaviour change on input nothing specifies**: no feature, QA procedure or unit test covered an unrecognised kind, and the old permissiveness was an artifact of `unwrap_or("")` rather than a decision. Rejected alternative: keep a fourth catch-all variant to preserve the old behaviour exactly — that reintroduces the stringly-typed hole inside the very type introduced to close it, and makes every future `match` carry an arm that means "we do not know what this is". |
| T-quota-targets-required | Quota triage requires `target_count`, `target_minutes_each` and `period`, rejected the same way `committed`'s three fields are. | T-three-task-kinds made the *columns* nullable for a schema reason — one `tasks` table shared by three kinds, most columns unused per row. That is a storage fact, not a triage-time permission. A quota row with no target can never be scheduled at M8 (nothing to place) and can never appear in D-quota-no-rollover's reckoning (`count(done)/target_count` has no denominator) — the same "wall protecting an empty room" failure T-three-task-kinds named for Fitness-as-pool, relocated to quota-with-no-target instead of pool. Requiring the fields at triage costs nothing today (no M1 surface depends on omitting them) and closes the hole before a real quota row can be created. |
| T-empty-equals-absent | An empty string and an absent key report identically — both use the existing `{"missing_field": <name>}` shape. | `require()` treated `Some("")` as present, which is the empty-string half of the hole this slice closes; the other half is deciding what the closed case reports. Distinguishing "you sent nothing" from "you sent an empty string" is a distinction a client rarely intends — an unfilled HTML form field and an absent field are the same submitter mistake. One shape, one code path in `require()`, rather than a second rejection variant carrying no information the caller can act on differently. |
| T-period-closed-set | `period` is a closed set: `week \| month`. | `deadline_type` and `priority` were closed in this same slice (both became validated sum types in `scheduler-core`) for the reason D-guardrails-never-yield states — the fields the M3 scheduler branches on cannot carry undefined values. `period` is exactly that kind of field for quota scheduling at M8: "3 sessions per `fortnight`" is not a case M8's cadence math is written to handle, and typos (`"weekk"`) currently store the same way a legitimate value would. Only `week` is exercised by any M1 example; `month` is added now because closing the set later, after a real quota row exists, is the same free-now/expensive-later trade T-jiff-epoch-millis already made for `deadline`. |
| T-fact-plan-line | **The fact/plan line runs inside the `Block` table, by block state.** `proposed` and `published` future blocks are the **Plan layer** — disposable, engine-written, deleted wholesale and regenerated on every recompute. `in_progress`, `completed` and `missed` blocks, plus pins, are the **Constraints layer** — immutable facts, read by the engine and never written by it. The determinism property is therefore: *delete every `proposed`/`published` future block, re-run with the same facts, get byte-identical placements.* The signature is `schedule(tasks, busy, guardrails, pins, facts, prior_plan, now) -> placements`. | Resolves C2 (#3), ratified by the owner 2026-08-12. As originally written — "delete every block and regenerate" — the property was not merely wrong but **untestable**, because the move penalty makes the objective depend on previous placements and past blocks are immutable inputs. Wiping the table would destroy history and pins alongside the plan. Drawing the line inside the table rather than splitting it keeps one query surface while making the disposable set precisely definable. Two naming rules come with it, because the vocabulary was in use before it was defined: (1) **"Plan" and "Constraints" are the layer names**; "Facts" is informal shorthand for the immutable block subset, not a third layer. (2) **"Layer" is reserved for this domain split** — T-module-boundary's code organisation is the **module boundary**, not layers, because a `Block` row is otherwise Plan-layer and store-layer at once and the word stops carrying information. Note T-module-boundary's inline five-argument rendering of `schedule()` predates this and is an abbreviation, not a competing decision; the seven-argument form above is the contract, and #11's AC-1 already says "corrected signature per C2". |
| T-migrations-append-only | **Never edit a migration that has been applied. Add a new numbered one.** Enforced in CI: migration files present on `trunk` may be added to, never modified or deleted (#32). Because SQLite has no `ALTER COLUMN`, a type change means the table-rebuild pattern — create the corrected table, `INSERT INTO new SELECT … FROM old`, drop the old, rename — inside a *new* migration. | `sqlx` records each applied migration by checksum, so editing one makes every existing database refuse to boot: `migration N was previously applied but has been modified`, with no fallback and no remediation path. The `triage-validation` slice did exactly this, changing `deadline TEXT` to `INTEGER` inside `0002_tasks.sql`; the damage was zero only because no database happened to exist at that moment. D-visible-slices removes that luck — the owner now runs the app every slice and will always have a live database. A convention is not enough here, because SQLite's missing `ALTER COLUMN` makes editing the old file the path of least resistance every single time: the correct route is roughly fifteen lines of rebuild in a new file, the wrong route is a one-word edit in an old one. The brief that authorised it reasoned "`tasks` has no production data, so this is free today" — which conflates *no production data* with *no existing database*, and is the specific mistake the CI gate exists to make unmakeable. Note `features/migrations.feature` cannot catch this: its idempotency scenario re-runs the *same* migration set, never a changed one. |
| T-classifier-covers-domain | **Domain and title categorization folds into the existing classifier trait (M1 keyword impl, M9 LLM impl) — not a second pipeline.** The trait's output grows `domain` and `title` alongside `kind`/`deadline`/`priority`, carrying per-field confidence the same way. The keyword implementation guesses `domain` by keyword match and passes `title` through unchanged; M9's LLM implementation improves both behind the same trait. **Classification is invoked from a background worker after capture, not inside `POST /captures` and not synchronously at triage.** | Two asks arrived separately — classify a capture's task *kind*, and tag it with a *domain* and a cleaned-up *title* — and they are the same mechanism: classify raw text, cheap rules first, LLM later, fall back safely, measure against labelled data. M9 (#19) already commits to exactly that shape, so a standalone categorization worker would duplicate the trait, the fallback and the evaluation harness for a second field set. Growing the trait's output at M1 rather than at M9 is the same argument T-three-task-kinds made for the three-variant `kind`: retrofitting an output shape after M9 depends on it is the expensive order. **On invocation timing**, the apparent conflict between "a trait implies a synchronous call" and "a background poller" dissolves — a trait describes *swappability*, not *when it is called*, and a worker can call a synchronous `classify()` perfectly well. What is genuinely settled is *where*: `POST /captures` has a 50ms budget asserted by `capture_endpoint.feature`, which no LLM round trip fits inside; and blocking triage on a network call puts the latency in front of the user at the one moment they are waiting. So the worker fills the fields between capture and triage. Provider is **OpenRouter** behind a hand-rolled `reqwest`+`serde` client, model pinned by `OPENROUTER_MODEL` with a cheap default — swappable without a code change, consistent with T-handrolled-gcal-client's precedent against generated SDKs. |
| T-templates-take-view-models | **Templates render view models, never store row types.** `http::view` holds what a page shows; `store` holds what a query returned. Handlers map between them. | The inbox slice had `inbox.html` and `capture_row.html` rendering `store::capture::UntriagedCapture` directly — a `sqlx::FromRow` struct, documented as "a capture as the inbox view needs it", which is persistence described in terms of a page. The tell was in `create_capture`: to return the new row's markup it **hand-built an `UntriagedCapture`** for a capture it had just written and never read back, because the template demanded that type. A struct being fabricated to satisfy a renderer is no longer a row. Left alone, every later view inherits the pattern and the templates end up bound to the schema — and the coupling bites in both directions, since `store` would grow a view-shaped type per page. The two structs carry the same single field today and the mapping is one line; that is precisely why this is the cheap moment to draw the line, before the triage screen needs a row id to aim an action at and the calendar needs formatted times that are not columns. This applies T-module-boundary's module boundary to the delivery side — it is not a new boundary, and "layer" stays reserved for the Plan/Constraints domain split (T-fact-plan-line). |

**Renumbered on merge, then superseded.** This branch allocated numeric IDs that `trunk` had already given to other decisions, and its source comments were left citing the stale numbers. Both problems are gone: decisions are keyed by slug now, and the citations were migrated with a CI gate behind them. Kept as the record of why.

## Rejected

| # | Rejected | Why |
|---|---|---|
| R-guardrail-override | Any guardrail override, however well-guarded. | See D-guardrails-never-yield. |
| R-browsable-archive | A browsable archive UI. | See D-kill-means-archive. |
| R-pool-on-calendar | Pool tasks on the calendar in any form. | See D-no-pool-on-calendar. |
| R-auto-promote-on-age | Automatic promotion of pool → committed on aging. | A task the system unilaterally puts on the calendar because it is old is a task the user ignores, and ignored blocks destroy calendar credibility. Age raises menu ranking and flags in review; the decision stays with the user. |
| R-multi-tenancy | Multi-tenancy, accounts, auth, RLS. | Single user. |
| R-collaboration | Collaboration, shared calendars, delegation, colleague-visible busy time. | Task blocks are advisory to the user, authoritative to the system. |
| R-plugin-surface | A plugin or general-purpose extensibility surface. | |
| R-supabase | Supabase (self-hosted or otherwise). | See T-sqlite-sqlx. If Postgres is ever genuinely needed, run plain Postgres. |
| R-gcal-webhooks | Google Calendar push webhooks for v1. | `watch` channels need a public HTTPS endpoint, which fights self-hosting. Poll every 60–120s; a tunnel can be added later if instant reaction is wanted. |
| R-incremental-patching | Incremental patching of the plan / mutation in place. | The scheduler is recomputed from scratch on every trigger. Same inputs, same output. |

---

## Version notes

### 2026-08-12 — Initial log

Extracted from the settled product brief. Board created, milestones M0–M9 plus
spike S3 opened.

**Source-of-truth split adopted:** the board holds state, milestones and epic
issues hold acceptance criteria, `docs/design/brief.md` holds architecture, and
this file holds rationale. No overlap, so nothing needs syncing. A
`docs/roadmap/` document was drafted and removed as duplicative of the board —
note this departs from the `swarmforge/roles/PM.prompt` convention, which
expects a roadmap document under `docs/roadmap/`; the departure is deliberate.

**Spec corrections raised against the brief, awaiting decision** (C1–C5, tracked
as issues): pins belong in the Constraints layer rather than as a bool on Block;
the "delete every block and regenerate" property is false as written and needs
the fact/plan line drawn inside the Block table; `Block::missed` is unreachable
under D-silence-means-done; auto-close undo is unimplementable without storing pre-close
`remaining_minutes`; and per-domain capacity accounting is incoherent with
`allowed_windows`.

### 2026-08-12 — M1 schema decisions settled

#1, C1, C4 and N1 settled in session, unblocking M1. Recorded as `T-three-task-kinds`, `T-pins-in-constraints`, `T-auto-close-event`,
`T-archived-at-only` and `D-quota-no-rollover`. C2, C3, C5 and the remaining U-series points stay open.

**M1's blocker list is now empty**, but not because all four were answered in
M1's favour — two of them turned out to belong downstream:

- **C1 → M3.** Pins have no M1 behaviour. There is no `Block` to drop `pinned`
  from until M3, no scheduler to consume a pin, and no calendar drag to create
  one until M6. The contradiction C1 identifies is entirely between M3's
  signature and M6's drag handling.
- **C4 → M6.** An event table is additive, so it lands with auto-close rather
  than with the task schema.

Both were deferred on the same principle: **a schema element with no observable
behaviour has nothing to specify against.** The specifier's Gherkin describes
externally visible behaviour, so a table nothing reads or writes yet cannot be
covered by an acceptance test or by an end-to-end QA suite — QA has no user
interface affordance to drive it. Adding such a table early buys nothing and
commits to a shape before the behaviour that would validate it exists.

**M1 epic (#9) needs rewording accordingly.** Its acceptance criterion 7 —
"Schema includes `remaining_minutes_at_auto_close` (C4), `pin` as its own table
(C1), `archived_at` as the single archive signal (N1)" — should retain only the
`archived_at` clause. M6 and M3 pick up the other two.

**One gap left open deliberately:** `archived_at` also has no M1 write path.
Nothing at M1 archives a task — capture *dismissal* stamps the capture row, not
a task, and every genuine archive route (decay pass, three-strike, closing the
review) arrives at M8. The column is specified at M1 because triage creates the
row it lives on, but its *behaviour* — an archived task appearing in none of
menu, pool or review — is M8's proptest to own.

**Two questions C1 raises that its issue does not answer**, needed before M3:

1. A pin binds `task_id` to one interval, but M3 splits tasks into chunks. For a
   6h task split across three days, which chunk does the pin bind — and chunks
   have no identity to refer to. Or does pinning suppress splitting entirely?
2. Pins have no defined death. Does a pin outlive its deadline? Can the user
   clear one? A forgotten pin is an invisible constraint that makes the
   scheduler look broken, and the infeasibility report's closed reason enum
   (`no_window`, `capacity_exceeded`, `deadline_unreachable`,
   `chunk_policy_unsatisfiable`) has no code for "a stale pin is in the way".

### 2026-08-12 — M1 layering

Recorded as T-module-boundary and T-unknown-kind-rejected. The M1 triage slice worked, but all of it lived in the
axum handlers: `scheduler-core` was three lines of doc comment, and the crate
purity rule T-core-no-tokio guards was guarding an empty room.

**One externally visible behaviour change**, called out because it is not a
refactor: triaging with a `kind` outside `pool | committed | quota` — including
omitting `kind` entirely — now returns `422 {"unknown_kind": <submitted>}`
instead of `201` with the arbitrary string written to `tasks.kind`. See T-unknown-kind-rejected for
why this was treated as a schema-integrity hole rather than behaviour worth
preserving. Everything the acceptance criteria *do* specify is unchanged,
including the `{"missing_field": ...}` rejection body and the order the three
committed fields are reported in.

**What the sum type bought immediately.** "A pool task has no deadline" and "a
quota task has no deadline" were previously assertions about the payload a test
happened to send; the handler would have stored a deadline on a pool task had
one been supplied. They are now properties of the type, and
`crates/scheduler-core/tests/task_properties.rs` asserts them against arbitrary
inputs rather than the two example rows.

**Known-red gate, untouched and pre-existing:** `scripts/analyzers/complexity.sh`
reports three violations, all in `crates/acceptance-tests/src/steps/`
(`capture::dispatch` 14, `triage::dispatch` 17, `when_triaged` 9) against T-complexity-8's
threshold of 8. Verified identical before and after this change. These are step
dispatchers — regex chains where every arm is a one-line delegation — so T-complexity-8's
"extract the logic that is not the match" does not straightforwardly apply, and
the fix is more likely to be splitting the step modules by Gherkin phase than
flattening anything. Left alone deliberately rather than absorbed into a
layering change.

### 2026-08-12 — Delivery shape, and the vocabulary gap

**D-visible-slices changes what a slice is.** Every pipeline slice now ends in something the
owner can run and see. This is a resequencing, not new scope: M1's remaining
work is unchanged, but it is cut so that each merge is observable rather than
grouped by component. M1's story 2 — "Untriaged queue UI" — was already in
scope and simply had not been sliced yet.

The constraint that forced the point: at the time of writing the application
serves **two routes, both POST, both JSON**, no `askama` dependency and no
templates. There is no GET route. Trellis cannot be opened in a browser at all;
the only way to observe it is `curl`. Under the previous milestone cut that
remained true until roughly M6.

**T-fact-plan-line settles the layer vocabulary** (C2, #3). Worth recording why it sat open
so long: T-pins-in-constraints was *settled* while standing on C2's *unratified* proposal, and
M3's acceptance criteria (#11) were written in vocabulary that nothing in the
repo defined. Every agent reading "Plan layer" was inferring it.

**Still missing, and now the largest documented gap:** `docs/design/brief.md`.
Nineteen issues link to it; it has never existed. The specific casualty is
**invariants 1–5**, asserted by number in #11's strongest acceptance criterion
("Invariants 1-5 hold under `proptest` … >= 1000 cases"). Only two are
described anywhere in the repo or the tracker:

- **Invariant 2** — a block lies entirely within **one** allowed window
  (recovered from U4).
- **Invariant 5** — the placed/unplaceable partition is **total**, every
  unplaceable task carrying a reason from the closed enum (recovered from #11
  AC-6).

1, 3 and 4 exist nowhere. Reconstruction offered and awaiting ratification:
non-overlap; conservation under splitting; hard deadlines hold. Until they are
written down M3 cannot be specified, because the specifier cannot write Gherkin
for a criterion whose terms are undefined.

### 2026-08-12 — triage-validation open questions settled

Issue #29's three open questions settled with the user before specification.
Recorded as `T-quota-targets-required`, `T-empty-equals-absent` and
`T-period-closed-set`. (They carried numeric IDs at the time, and were
renumbered once on merge to dodge a collision — see "Former numbering".)

- **Quota target required at triage (`T-quota-targets-required`).** T-three-task-kinds's nullable columns were a
  schema-sharing fact, not a triage-time permission; leaving target fields
  optional at triage would have let a quota row exist that M8 can never
  schedule and D-quota-no-rollover's reckoning can never report on.
- **Empty and absent report identically (`T-empty-equals-absent`).** Both remain
  `{"missing_field": <name>}`. This also settles the empty-string half of the
  defect issue #29 raised against `require()` — the fix is one path, not two.
- **`period` closed to `week | month` (`T-period-closed-set`).** Matches how `deadline_type` and
  `priority` are closed in the same slice, for the same D-guardrails-never-yield reason: M8's cadence
  math cannot branch on an unvalidated string.

### 2026-08-12 — triage-validation harness seams and the complexity gate

The triage-validation slice landed the closed domains in `scheduler-core`,
which is where T-module-boundary says they belong — the layering held with no correction
needed. The architectural work this round was in the acceptance harness, which
had grown two missing seams.

**The definition of a valid submission had no home.** Closing four field
domains meant every "…is rejected because X is wrong" scenario needed a
payload valid in every respect *except* X. Written inline, the canonical
committed payload became a literal repeated twelve times across five step
modules, so closing one more field's domain would mean finding and editing
every copy. `steps/payloads.rs` now holds the canonical payload per kind plus
`with_field`/`without_field`, and a scenario states only what it varies.

**Nothing knew how a scenario reaches the application.** `capture` and
`triage` had each grown a copy of "build the router from the world's pool,
POST, record the outcome", and the copies had already drifted: one timed the
round trip and discarded the body, the other kept the body and discarded the
timing — so a step could assert only what its own module happened to record.
`steps/app_client.rs` returns status, body and elapsed together.

DRY: 3.26% → 2.66%, back under the 3% threshold it had crossed. *(Corrected
2026-08-13: this line read 2.62%, a transcription slip; the architect's own
commit message has 2.66%, which is what re-measurement confirms.)*

**On the complexity gate, deliberately not "fixed".** Three violations stand,
all pure regex dispatch chains: `steps/capture.rs::dispatch` (14),
`steps/triage.rs::dispatch` (17), `steps/mod.rs::dispatch` (9). *(Corrected
2026-08-13: this read "three violations **remain**", which implies all three
predate the slice. Two do. `steps/mod.rs::dispatch` is **introduced here** —
5 on the base, 9 after adding four step modules to the top-level dispatcher.
The total held at three only because `when_triaged` dropped off in the same
change. The argument below is unaffected; the framing hid an added violation,
which is exactly how a red gate gets waived on the next read.)* Every arm is a
one-line delegation, so by T-complexity-8's own rule — *a function over 8 is carrying
logic that is not the match; extract that, do not flatten the match* — there
is nothing to extract. The only way to move the number is a
`(Regex, handler)` table, and because the handlers are `async` with differing
arities, a uniform table in Rust needs a boxed-future wrapper function per
step: roughly forty wrappers to replace forty one-line branches, which is
exactly the "indirection that is strictly worse to read" T-complexity-8 refuses. **Do not
flatten these into a registry to make the number go down.** If they are ever
worth changing it should be for a cohesion reason — splitting a step module by
Gherkin phase — not for the metric.

The distinction is visible in this slice: `steps/triage.rs::when_triaged` also
scored 9 and *did* come off the list, because it was genuinely doing two
things (driving HTTP, and recording onto `World`) and separating them was
worth doing on its own merits. The number moved as a side effect of an
improvement, which is the only reason it should ever move.

**Property coverage doubled, 6 → 12**, all in
`crates/scheduler-core/tests/task_properties.rs`: every value inside a closed
domain round-trips; anything outside one is rejected as invalid naming that
field; a deadline naming no real instant is rejected (T-jiff-epoch-millis); empty and absent
produce the *identical* rejection (T-empty-equals-absent, asserted as an equality between the
two outcomes rather than against a fixed expectation, so it survives a change
to either); a missing field is reported before an invalid one; and equivalent
textual spellings of one instant store one deadline. The last three were
checked against deliberate breakages of `require()`, of the check ordering in
`committed_from`, and confirmed to fail — a property that cannot fail is not
coverage.

### 2026-08-13 — Decision-ID collision on merge

`triage-validation` and `trunk` independently allocated **T-quota-targets-required**. The slice
branched from `a0889b4` before trunk's entry existed, so both took what was
correctly the next free number at the time; neither side erred.

Resolved by renumbering **trunk's** decision — the fact/plan layer model, C2/#3,
now `T-fact-plan-line` — and leaving the slice's three (now
`T-quota-targets-required`, `T-empty-equals-absent`, `T-period-closed-set`)
untouched. That direction was chosen purely
on blast radius: the slice's numbers are cited in nine places in `crates/`
(`scheduler-core/src/task.rs`, `task_properties.rs`, two step modules), while
trunk's had three references, all in documentation. Renumbering the cheaper side
kept a merge fix out of product code.

**The underlying problem is unfixed:** the log has no ID allocation mechanism,
so any two concurrent branches will collide again the moment both add a
decision. Options if it recurs — allocate IDs only at merge time, prefix them
per branch, or drop sequential numbering for dated slugs. Not worth solving
until it costs more than this did.

*It recurred within hours, and cost more. Sequential numbering was dropped the
same day — see "Decisions are keyed by slug" below.*

### 2026-08-13 — Migration discipline, and two corrections

**T-migrations-append-only** makes editing an applied migration a CI failure (#32). Recorded because
the near-miss was invisible: `triage-validation` shipped an in-place edit to
`0002_tasks.sql`, and it cost nothing only because no database existed at that
moment. Under D-visible-slices that luck is gone.

Two corrections to the `triage-validation` entries above, both found by
re-measuring the claims rather than reading them:

- The DRY figure read **2.62%**; it is **2.66%**. Transcription slip — the
  architect's commit message had it right.
- "Three violations **remain**" implied all three predate the slice. Two do;
  `steps/mod.rs::dispatch` went 5 → 9 *in* that slice. Corrected to "stand".

Neither changes an argument, and both are small. They are recorded rather than
quietly patched because the previous slice's log entry contained a claim that
was simply false — "verified identical before and after" when the count had
gone 1 → 3 — and the habit worth building is that measurable assertions in this
file get measured.

### 2026-08-13 — Capture categorization folded into the classifier trait

A brainstorming session designed LLM auto-categorization for captures, then
found mid-design that it overlapped M9. Reconciled as **T-classifier-covers-domain**: one trait, two
implementations, output grown to carry `domain` and `title`. Source:
`docs/plans/2026-08-13-capture-categorization-handoff.md`.

Two corrections to that proposal, applied here rather than inherited:

- **The invocation-timing question it left open is partly a false conflict.**
  It framed "trait implies synchronous" against "background poller" as a
  design fork. A trait describes swappability, not call timing; a worker can
  call a synchronous `classify()`. The real constraint is *where*, and that is
  already settled by evidence — `capture_endpoint.feature` asserts a 50ms
  budget on `POST /captures`, which no LLM round trip fits inside, and
  blocking triage puts the wait in front of the user at the moment they are
  waiting. Recorded in T-classifier-covers-domain as the worker filling fields between capture and
  triage.
- **The provisional domain list contradicts this log**, and that is *not*
  settled — raised as **#36**. The proposal lists Work, Health, Home, Learning,
  Social. This file already names Fitness (T-three-task-kinds), Learning (D-kill-means-archive), Family (D-single-user)
  and Work. Two different sets of five, and nobody noticed because the domain
  set has never been written down in one place. Which is the same root cause
  as the missing invariants: `docs/design/brief.md` does not exist, so the
  vocabulary lives in whichever entry happened to mention it.

#36 also surfaces a harder question the proposal states as a settled
constraint: capture domains as extensible plain data versus `Domain` as a
5-variant enum that T-complexity-8's complexity threshold is derived from. One concept or
two is a real decision, and it needs making before the M1 classifier story is
specified.

### 2026-08-13 — inbox-view: the first page, and where its data comes from

The slice that finally made Trellis openable in a browser also introduced the
first template, and with it the first chance to get the delivery side of T-module-boundary's
module boundary wrong. It very nearly did. Recorded as **T-templates-take-view-models** — this branch's
own history gave it a number `trunk` had already spent, and its source comments
cited the stale numbering; see "Decision-ID collision on merge" above. Both are
moot now that decisions are keyed by slug and a CI gate checks the citations.

**What the templates were rendering.** `InboxTemplate` held
`Vec<store::capture::UntriagedCapture>` and `capture_row.html` read
`capture.raw_text` — so the HTML was bound to a `sqlx::FromRow` struct and, by
extension, to the columns `list_untriaged` happens to select. The store type's
own doc comment gave the direction away: "a capture as the inbox view needs
it". Persistence was being described in terms of a page.

The decisive evidence was in `create_capture`, not the inbox. To hand the
quick-add box back the new row's markup, it constructed an `UntriagedCapture`
by hand from the submitted text — a database row shape, for a capture it had
just written and deliberately not read back, existing only because the
template's type demanded it. A struct being fabricated to satisfy a renderer
has stopped being a row.

`http::view` now holds the view models. It depends on nothing, so a page's
data can be built and rendered without a database; handlers do the mapping,
one line each. `UntriagedCapture` keeps its name and goes back to describing
the query.

This is small on purpose — one field, one line of mapping. The point is not
the size of today's duplication but the direction of tomorrow's: this is the
first of many pages, and the alternative precedent is templates bound to the
schema by M6.

**Property coverage** gained the inbox's projection invariant
(`store::capture`, `#[ignore]`d per convention): for any queue and any triaged
subset of it, `list_untriaged` returns *exactly* the untriaged captures in
*exactly* newest-first order. The three example tests sample two captures and
one triaged one; the property pins both halves of the query. Checked by
breaking `ORDER BY id DESC` to `ASC` and by dropping the `WHERE` — each fails
the property, neither fails every example test.

**Complexity is unchanged at 4 violations**, all still the regex dispatch
chains covered by the 2026-08-12 entry; `steps/inbox_view.rs::dispatch` is the
new one and is the same shape. The reasoning there stands: nothing to extract,
and a table would cost ~40 boxed-future wrappers. DRY sits at 2.0%.

### 2026-08-13 — Decisions are keyed by slug; open questions are issues

Two failures in two days made the numbering untenable. `T17` was claimed
independently by `trunk` and by the `triage-validation` branch, which forked
before the other existed. Then `inbox-view` renumbered its source comments to a
scheme that had never been settled, leaving every citation in
`scheduler-core/src/task.rs` off by one — and because each wrong number
resolved to a *real* decision, the comments read as correct. A dangling
reference announces itself; a reference to the wrong true thing does not.

The cause is structural: an integer allocated on a branch is a distributed
counter with no coordinator, and branch order is not merge order. Renaming is
not a fix, it is the next collision deferred.

**Two changes.** Settled decisions are keyed by content-derived slug, so two
branches cannot silently pick the same key and nothing is ever renumbered.
Open questions leave this file for the tracker — they were duplicated between
the Open table and issues #4, #6, #7 and #36 already, which is the same
duplicated state the project rejected when it deleted `docs/roadmap/`.

Rejected alternative: move settled decisions to GitHub issues too, which would
also make collisions impossible since issue numbers are server-allocated. It
loses more than it buys. Every agent reads this file at startup, from a
worktree, often offline; the rationale should ship with the clone. And the
"Rejected" section works precisely because someone about to re-propose a
refused idea trips over the reasoning while reading — a closed issue has to be
searched for by someone who already suspects it exists.

Enforcement matters more than the convention: CI fails if a slug cited under
`crates/` is absent from this file. The off-by-one above was undetectable;
under the gate it is a failed build. Same shape as
`T-migrations-append-only` — the rule is only real once something checks it.

### 2026-08-13 — triage-from-page: a payload nobody could use, and a shared fragment

The slice extended the delivery module correctly — `view::CaptureRow` grew the
`id` and `error` that `T-templates-take-view-models` predicted it would need,
and `store::TaskWithCaptureText` stayed a row rather than becoming a second
view model. Two things wanted straightening.

**`TriageRejection::UnknownKind` carried a payload no adapter could use.**
Both consumers destructured it as `_` and reported `kind_submitted` from the
request instead, with a comment explaining that the core's copy is not the one
to trust: JSON `{"kind": 7}` reaches the core as `None`, because reading it as
a string is what turned it into one. So the core was carrying a value that is
never better than the adapter's and sometimes strictly worse, and the only
documentation it had was a warning not to use it. It is now a unit variant.
The core says *this kind is not one of the three*; what arrived is the
adapter's to report, because the adapter is the only place it still exists.

A sum-type payload every consumer ignores is not extra information, it is a
second source of truth that loses. Behaviour is unchanged — both responses
were already built from the request.

**The `#lists` fragment now lives beside its two renderers, not inside one.**
`ListsTemplate` and `build_lists` sat in `http::inbox`, so the triage endpoint
depended on the inbox *page* to render the fragment they share. `http::lists`
holds both; `inbox` and `triage` are now siblings over it. Same move as
`view`, `payloads` and `app_client`: when a second caller appears, the shared
thing gets its own home rather than one caller reaching into the other.

**Property coverage** gained the invariant this slice's own brief asserts —
"the page and `POST /captures/{id}/triage` are one code path, not two". That
is now two hand-written, field-by-field translations into the same
`TriageFields`, which is exactly where the claim can quietly stop being true:
add a field to the core, wire it into one transport, and it still compiles and
still passes every example test that does not use that field. The property
submits one arbitrary submission through both and requires the results to be
identical. Checked by dropping `period` from the form mapping alone — it
fails, shrinking to the single field.

**Wording repair, called out rather than made quietly:** `T-module-boundary`
opened with "Three layers", written before `T-fact-plan-line` reserved *layer*
for the Plan/Constraints domain split. Changed to "Three modules". The
substance is untouched; the row was contradicting the naming rule that
supersedes it, in the one document agents consult to learn the vocabulary.

**Complexity is 5**, all still regex dispatch chains in the acceptance step
modules — `triage_from_page::dispatch` is the new one, same shape. The
2026-08-12 entry's reasoning stands. DRY 1.96%, CRAP 0.
