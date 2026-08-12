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
- **Architecture**: `docs/design/brief.md`.
- **Rationale and rejected options**: this file.

---

## Settled — product

| # | Decision | Reasoning |
|---|---|---|
| D1 | Single user, single tenant. | "Family" is a domain of the user's own time, not a shared household schedule. |
| D2 | Pool is the default task kind; committed is the exception. | Motion's schedule-everything model is exactly why it churns. A calendar that is ~40% committed is one the user trusts, because a block exists only because it had to. |
| D3 | Guardrails never yield to deadlines. | A P1 hard deadline with Work hours full raises a conflict; it does not breach the wall. An override that exists will get used, and then the walls are decorative. |
| D4 | Silence on a check-in means **done**. | Assuming "not done" is intuitively safer but is the exact mechanism by which every abandoned task app decayed: unanswered work accumulates and the backlog inflates with phantom obligations. |
| D5 | Kill means archive. Keep the row; build no browsable archive UI. | The row feeds the reckoning ("47 archived this quarter, 31 Learning" is real signal). The moment an archive is browsable it becomes a place to hide from decisions. |
| D6 | A skipped weekly review ages everything one more week. No deeper sweep. | The week the user skips is the week their life was chaotic — the worst possible moment to punish them. Aging is self-correcting. |
| D7 | Inaction archives. Survival requires a deliberate act. | Inverts the property shared by every task app the user has abandoned, where inaction preserved tasks. |
| D8 | Three-strike: on the third deferral, "keep" is removed. Commit or kill. | A task protected weekly but never touched is a lie. This is the one place in the system where friction is a feature; everywhere else, optimize it away. |
| D9 | Pool tasks never appear on the calendar — not even as all-day chips. | The calendar's entire value is that everything on it is true. Ambient chips are the first crack. |
| D10 | The menu returns exactly three options plus a rest option. | A long ranked list is a blank page with extra steps — the same decision fatigue the product exists to eliminate. |
| D11 | Emergent gaps are offered, not filled. | Dragging Thursday's deep work into a random Tuesday hole is behaving badly. Recompute committed work at domain boundaries; surface the menu mid-stream. |
| D12 | Staleness threshold is deliberately unset; instrument first, tune at the first monthly reckoning. Per-domain config. | The pool's turnover rate is unknown until the system runs. Seeded at 21 days / ≥3 offers. |
| D13 | An unmet quota does **not** roll over. A missed week is missed. | Carrying it forward re-creates exactly the accumulating phantom obligation that D4 (silence-means-done) exists to prevent. "You did 1 of 3 runs last week" is a reckoning fact, not a debt. Paired with T11. |

## Settled — technical

| # | Decision | Reasoning |
|---|---|---|
| T1 | Rust. | The scheduler core is interval arithmetic over sum types, the invariants are property-testable, and the artifact is a static musl binary plus one SQLite file. |
| T2 | SQLite + `sqlx`, WAL mode. | Self-hosted Supabase is ~8 containers solving problems this product does not have. All queries stay inside `scheduler-db` so a Postgres swap stays mechanical. |
| T3 | `jiff`, not `chrono`. | Guardrails are civil wall-clock. `jiff` models zoned vs civil time as distinct types and forces an explicit decision at a DST gap. Store UTC epoch millis, convert at the boundary. |
| T4 | `scheduler-core` must not depend on tokio. | If it compiles without an async runtime, the algorithm has been kept honest. Enforced in CI from M0. |
| T5 | Hand-rolled `reqwest` + `serde` Google Calendar client. | Only six operations are needed; ~300 lines fully controlled beats fighting a generated crate. |
| T6 | Greedy-with-repair, not a constraint solver. | Explainable and fast enough for one user. Keep the interface clean so an optimizer can be swapped in behind it later. |
| T7 | Google Calendar is a render target, never a source of truth for task blocks. | |
| T8 | Capture surfaces for v1: web quick-box and Telegram. CLI and iOS Shortcut deferred. | |
| T9 | Cyclomatic complexity threshold is **8** per function, not the template's 4. | Derived from the domain model rather than borrowed from convention. The largest enums (`Domain`, `BlockState`) have 5 variants, so an exhaustive `match` over one scores 6 — meaning a threshold of 4 would fail on almost every match in a codebase that is deliberately sum-type-heavy. Enforced at 4, the gate would push agents to split clear matches into indirection that is strictly worse to read, and the analyzer would be shaping the architecture instead of guarding it. 8 leaves headroom for one guard above the largest match while still catching genuine branching thickets. **A function over 8 is carrying logic that is not the match — extract that, do not flatten the match.** Revisit if a legitimate enum grows past 6 variants. |
| T10 | Mutation parallelism is 8 for `scheduler-core` and **1** everywhere else. | `scheduler-core` is pure — no I/O, no shared mutable state — so its mutants are safely parallel. Every other crate's mutants open the same SQLite file and truncate each other's database between test setup and assertion. WAL mode does not save you. The symptom is timeouts on mutants that cannot possibly hang, so it reads as flakiness and gets "fixed" by raising the timeout. **Do not raise the timeout.** Use `--jobs 1`, or give each worker a `TMPDIR`-scoped database and record how here. `scheduler-gcal` mutants run against the mock transport, never the live API. |
| T11 | Task `kind` is a **three-variant** sum type — `Committed \| Pool \| Quota` — committed in the schema at M1. Quota fields (`target_count`, `target_minutes_each`, `period`) are nullable. Quota *scheduling* waits until M8. | Resolves O2. The weekly review is itself a recurring commitment, so without a quota kind M8 needs a bespoke recurring mechanism for exactly one task — which is how a special case becomes permanent. Fitness is one of five domains and inherently quota-shaped: no deadline, so it cannot be committed; but as pool it is never placed, and a Fitness guardrail nothing is scheduled into is a wall protecting an empty room. Capacity math is false in the same way C5 makes the Work number false if quota demand is uncounted. And in Rust, adding a variant later turns every `match` into a compile error — that is the *good* case; the bad case is `if committed { .. } else { /* pool */ }`, which silently treats quotas as pool. Behaviour waits because quota semantics are unsettled and M3 is already the riskiest milestone. |
| T12 | Pins are a first-class entity in the Constraints layer: `pin { task_id, start, end, source }`. `pinned` is **not** a column on `Block`. Lands at **M3**, with the `schedule()` signature that consumes it — not at M1. | Resolves C1. Blocks are Plan-layer: disposable, engine-written, deleted wholesale on every recompute (R10). A pin is user-authored intent, so a pin living on a block cannot survive the recompute that deletes its row — which makes M3's regeneration property ("delete all future blocks, re-run, get byte-identical placements") and M6's "a drag creates a pin that survives the next recompute" mutually unsatisfiable. `schedule(tasks, hard_events, guardrails, **pins**, now)` had already made the call implicitly by taking pins as an *input*. Rejected alternative: keep `pinned` on `Block` and exempt pinned rows from deletion — that makes the Plan layer partly durable, the exact fact/plan confusion C2 is separately untangling, and weakens M3's strongest property to "delete all *non-pinned* blocks". Deferred to M3 because pins have no M1 behaviour: no `Block` to drop the column from, no scheduler to consume them, no drag to create one. |
| T13 | Auto-close is recorded as an **event**, not a column: `auto_close_event { task_id, closed_at, remaining_minutes_before, undone_at }`. Lands at **M6** with auto-close itself. | Resolves C4, and goes further than the issue proposed. The AC bundled two requirements into one column: *undo restores exactly*, and *R1 can measure an undo rate*. A column on `task` serves the first and cannot serve the second — it is overwritten on the second close, so the event denominator is wrong and a task that auto-closes repeatedly (the strongest possible evidence that silence does **not** mean done for that work) collapses to a single row. Undo restores from the latest event with `undone_at IS NULL`; undo rate is `count(undone_at IS NOT NULL) / count(*)`, which is what R1's 15%/30% thresholds actually need. Falling back to `estimated_minutes` was never viable: it is correct only for never-started tasks, where undo matters least, and silently inflates every partially-completed one. Additive, so it does not block M1. |
| T14 | `archived_at` is the single archive signal. `status: dropped` is removed. | Resolves N1. Two fields for one state, in a system where D7 makes archiving the *default* outcome reached implicitly from several paths — decay pass, three-strike, and simply closing the review (U7) — means every path gets two chances to set one and forget the other. The resulting half-archived task is alive on whichever surface filters the field that was missed: a task returning from the dead, in a product whose entire value is that the user trusts what it shows. M8's all-surfaces proptest is the test designed to catch this, and it can only assert a clean invariant against one field. `archived_at` also carries strictly more information — the reckoning's "47 archived this quarter, 31 Learning" needs a timestamp, which `status: dropped` cannot supply. |
| T15 | Three layers, dependencies pointing inward: `scheduler-core` holds the rules; `trellis-server::http` translates requests into core inputs; `trellis-server::store` translates core types into rows. Adapters name core types; the core names neither. | The rules had been living inside the axum handlers, expressed as `serde_json::Value` probes and `StatusCode` returns — the function that wrote a task row took a *transport* type as a parameter and returned an *HTTP* type as its error. That leaves no seam to test a rule at: answering "is this triage valid?" required a running server and a SQLite pool. It also made T2's "a Postgres swap stays mechanical" untrue, since the queries were spread across handler modules instead of confined to one layer. The split is what makes `scheduler-core` non-empty for the first time, and gives T4's no-tokio rule something to protect. `store/mod.rs` carries a unit test asserting that no store module names `axum` or `StatusCode`: a layering rule nothing checks is a comment. |
| T16 | `kind` is validated against the three variants at the boundary. A submission naming anything else is rejected with `422 {"unknown_kind": <submitted>}`. | T11 committed to a three-variant sum type, but the M1 implementation read `kind` as `payload.get("kind").and_then(as_str).unwrap_or("")` and stored whatever string arrived, so `{"kind":"banana"}` wrote `banana` into a column whose domain is three values. That is the failure mode T11 named — not the `if committed {} else {}` shape it predicted, but a weaker one, with no discrimination at all. Since `tasks.kind` carries no `CHECK` constraint, the only thing standing between a typo and durable out-of-domain data was the caller. Rejecting is a **behaviour change on input nothing specifies**: no feature, QA procedure or unit test covered an unrecognised kind, and the old permissiveness was an artifact of `unwrap_or("")` rather than a decision. Rejected alternative: keep a fourth catch-all variant to preserve the old behaviour exactly — that reintroduces the stringly-typed hole inside the very type introduced to close it, and makes every future `match` carry an arm that means "we do not know what this is". |

## Rejected

| # | Rejected | Why |
|---|---|---|
| R1 | Any guardrail override, however well-guarded. | See D3. |
| R2 | A browsable archive UI. | See D5. |
| R3 | Pool tasks on the calendar in any form. | See D9. |
| R4 | Automatic promotion of pool → committed on aging. | A task the system unilaterally puts on the calendar because it is old is a task the user ignores, and ignored blocks destroy calendar credibility. Age raises menu ranking and flags in review; the decision stays with the user. |
| R5 | Multi-tenancy, accounts, auth, RLS. | Single user. |
| R6 | Collaboration, shared calendars, delegation, colleague-visible busy time. | Task blocks are advisory to the user, authoritative to the system. |
| R7 | A plugin or general-purpose extensibility surface. | |
| R8 | Supabase (self-hosted or otherwise). | See T2. If Postgres is ever genuinely needed, run plain Postgres. |
| R9 | Google Calendar push webhooks for v1. | `watch` channels need a public HTTPS endpoint, which fights self-hosting. Poll every 60–120s; a tunnel can be added later if instant reaction is wanted. |
| R10 | Incremental patching of the plan / mutation in place. | The scheduler is recomputed from scratch on every trigger. Same inputs, same output. |

## Open

| # | Question | Status |
|---|---|---|
| O1 | Staleness threshold and offer count. | Deliberately unset. Instrument from M1, tune at first monthly reckoning. See D12. |
| O2 | Recurring quotas in v1. | **Settled 2026-08-12.** Three-variant sum type in the schema at M1, scheduling at M8, no rollover. See T11 and D13. |
| O3 | Weekend check-in density. | Fewer domain transitions means fewer natural check-in points; a coarser Sunday sweep may be needed. Revisit at M6. |
| O4 | Google OAuth refresh-token expiry. | Verify against current Google docs rather than trusting prior notes; policy shifts. Either way, design the token store assuming re-auth happens and surface it loudly. |
| O5 | Menu diversity scoring. | "One quick win, one that matters, one you've been circling" is a stated intent with no defined mechanism. Needs specification before M3.5. |

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
under D4; auto-close undo is unimplementable without storing pre-close
`remaining_minutes`; and per-domain capacity accounting is incoherent with
`allowed_windows`.

### 2026-08-12 — M1 schema decisions settled

O2, C1, C4 and N1 settled in session, unblocking M1. Recorded as T11–T14 and
D13. C2, C3, C5 and the remaining U-series points stay open.

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

Recorded as T15 and T16. The M1 triage slice worked, but all of it lived in the
axum handlers: `scheduler-core` was three lines of doc comment, and the crate
purity rule T4 guards was guarding an empty room.

**One externally visible behaviour change**, called out because it is not a
refactor: triaging with a `kind` outside `pool | committed | quota` — including
omitting `kind` entirely — now returns `422 {"unknown_kind": <submitted>}`
instead of `201` with the arbitrary string written to `tasks.kind`. See T16 for
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
(`capture::dispatch` 14, `triage::dispatch` 17, `when_triaged` 9) against T9's
threshold of 8. Verified identical before and after this change. These are step
dispatchers — regex chains where every arm is a one-line delegation — so T9's
"extract the logic that is not the match" does not straightforwardly apply, and
the fix is more likely to be splitting the step modules by Gherkin phase than
flattening anything. Left alone deliberately rather than absorbed into a
layering change.
