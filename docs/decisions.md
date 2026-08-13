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

Rows are referenced by prefix throughout the repo and the tracker.

| Prefix | Where | Means |
|---|---|---|
| **D**_n_ | this file | Settled **product** decision |
| **T**_n_ | this file | Settled **technical** decision |
| **R**_n_ | this file | **Rejected** — considered and refused, with why |
| **O**_n_ | this file | **Open** question, not yet answered |
| **C**_n_ | issues #2–#6 | **Correction** — a place the original product brief contradicts itself |
| **U**_n_, N1 | issue #7 | **Underspecified** — the brief is silent and an agent will guess |

`O2` is titled `OQ2` on issue #1. Same question; the issue title predates this
scheme.

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
| D14 | **Every pipeline slice ends in something the user can run and see.** A slice is not done when its tests pass; it is done when the owner can start the app and watch the new behaviour happen. A slice with no visible surface carries the thinnest surface that exposes it. Size does not matter — smaller is better. | This is the owner's professional standard with their own clients, not a preference about this project: continuous visible delivery is the customer's best case, and building for fifteen slices before the customer can try anything is the failure mode it exists to prevent. Three things it buys, all of which the plan was otherwise deferring. **Feedback:** #20's risk register (R1/R2/R3) names three assumptions that are all behavioural — silence-means-done, the ~40% committed ratio, whether the menu beats a list. None can be tested by building a correct scheduler; they need the owner living with the thing, so calendar time is the scarce resource, not engineering time. #9's AC-6 already conceded this by requiring `/stats` live "weeks before it is read". **Clear thinking:** a working system is a better argument about what to build next than a roadmap is. **Value:** the product is useful as an inbox long before it is useful as a scheduler. Cost, accepted knowingly: some slices grow a surface they would not otherwise need, and early surfaces are plain and will be reworked. Rejected alternative: keep the horizontal milestone cut and add UI at the end — that is precisely the fifteen-slice wait, and it defers every behavioural risk to the point where acting on what is learned is most expensive. |

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
| T17 | **The fact/plan line runs inside the `Block` table, by block state.** `proposed` and `published` future blocks are the **Plan layer** — disposable, engine-written, deleted wholesale and regenerated on every recompute. `in_progress`, `completed` and `missed` blocks, plus pins, are the **Constraints layer** — immutable facts, read by the engine and never written by it. The determinism property is therefore: *delete every `proposed`/`published` future block, re-run with the same facts, get byte-identical placements.* The signature is `schedule(tasks, busy, guardrails, pins, facts, prior_plan, now) -> placements`. | Resolves C2 (#3), ratified by the owner 2026-08-12. As originally written — "delete every block and regenerate" — the property was not merely wrong but **untestable**, because the move penalty makes the objective depend on previous placements and past blocks are immutable inputs. Wiping the table would destroy history and pins alongside the plan. Drawing the line inside the table rather than splitting it keeps one query surface while making the disposable set precisely definable. Two naming rules come with it, because the vocabulary was in use before it was defined: (1) **"Plan" and "Constraints" are the layer names**; "Facts" is informal shorthand for the immutable block subset, not a third layer. (2) **"Layer" is reserved for this domain split** — T15's code organisation is the **module boundary**, not layers, because a `Block` row is otherwise Plan-layer and store-layer at once and the word stops carrying information. Note T15's inline five-argument rendering of `schedule()` predates this and is an abbreviation, not a competing decision; the seven-argument form above is the contract, and #11's AC-1 already says "corrected signature per C2". |
| T18 | Quota triage requires `target_count`, `target_minutes_each` and `period`, rejected the same way `committed`'s three fields are. | T11 made the *columns* nullable for a schema reason — one `tasks` table shared by three kinds, most columns unused per row. That is a storage fact, not a triage-time permission. A quota row with no target can never be scheduled at M8 (nothing to place) and can never appear in D13's reckoning (`count(done)/target_count` has no denominator) — the same "wall protecting an empty room" failure T11 named for Fitness-as-pool, relocated to quota-with-no-target instead of pool. Requiring the fields at triage costs nothing today (no M1 surface depends on omitting them) and closes the hole before a real quota row can be created. |
| T19 | An empty string and an absent key report identically — both use the existing `{"missing_field": <name>}` shape. | `require()` treated `Some("")` as present, which is the empty-string half of the hole this slice closes; the other half is deciding what the closed case reports. Distinguishing "you sent nothing" from "you sent an empty string" is a distinction a client rarely intends — an unfilled HTML form field and an absent field are the same submitter mistake. One shape, one code path in `require()`, rather than a second rejection variant carrying no information the caller can act on differently. |
| T20 | `period` is a closed set: `week \| month`. | `deadline_type` and `priority` were closed in this same slice (T-series validated sum types in `scheduler-core`) for the reason D3 states — the fields the M3 scheduler branches on cannot carry undefined values. `period` is exactly that kind of field for quota scheduling at M8: "3 sessions per `fortnight`" is not a case M8's cadence math is written to handle, and typos (`"weekk"`) currently store the same way a legitimate value would. Only `week` is exercised by any M1 example; `month` is added now because closing the set later, after a real quota row exists, is the same free-now/expensive-later trade T3 already made for `deadline`. |

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

### 2026-08-12 — Delivery shape, and the vocabulary gap

**D14 changes what a slice is.** Every pipeline slice now ends in something the
owner can run and see. This is a resequencing, not new scope: M1's remaining
work is unchanged, but it is cut so that each merge is observable rather than
grouped by component. M1's story 2 — "Untriaged queue UI" — was already in
scope and simply had not been sliced yet.

The constraint that forced the point: at the time of writing the application
serves **two routes, both POST, both JSON**, no `askama` dependency and no
templates. There is no GET route. Trellis cannot be opened in a browser at all;
the only way to observe it is `curl`. Under the previous milestone cut that
remained true until roughly M6.

**T17 settles the layer vocabulary** (C2, #3). Worth recording why it sat open
so long: T12 was *settled* while standing on C2's *unratified* proposal, and
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
Recorded as T18–T20 (renumbered from an initial T17–T19 to avoid colliding
with T17 above, settled independently the same day).

- **Quota target required at triage (T18).** T11's nullable columns were a
  schema-sharing fact, not a triage-time permission; leaving target fields
  optional at triage would have let a quota row exist that M8 can never
  schedule and D13's reckoning can never report on.
- **Empty and absent report identically (T19).** Both remain
  `{"missing_field": <name>}`. This also settles the empty-string half of the
  defect issue #29 raised against `require()` — the fix is one path, not two.
- **`period` closed to `week | month` (T20).** Matches how `deadline_type` and
  `priority` are closed in the same slice, for the same D3 reason: M8's cadence
  math cannot branch on an unvalidated string.

### 2026-08-12 — triage-validation harness seams and the complexity gate

The triage-validation slice landed the closed domains in `scheduler-core`,
which is where T15 says they belong — the layering held with no correction
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

DRY: 3.26% → 2.62%, back under the 3% threshold it had crossed.

**On the complexity gate, deliberately not "fixed".** Three violations remain,
all pure regex dispatch chains: `steps/capture.rs::dispatch` (14),
`steps/triage.rs::dispatch` (17), `steps/mod.rs::dispatch` (9). Every arm is a
one-line delegation, so by T9's own rule — *a function over 8 is carrying
logic that is not the match; extract that, do not flatten the match* — there
is nothing to extract. The only way to move the number is a
`(Regex, handler)` table, and because the handlers are `async` with differing
arities, a uniform table in Rust needs a boxed-future wrapper function per
step: roughly forty wrappers to replace forty one-line branches, which is
exactly the "indirection that is strictly worse to read" T9 refuses. **Do not
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
field; a deadline naming no real instant is rejected (T3); empty and absent
produce the *identical* rejection (T18, asserted as an equality between the
two outcomes rather than against a fixed expectation, so it survives a change
to either); a missing field is reported before an invalid one; and equivalent
textual spellings of one instant store one deadline. The last three were
checked against deliberate breakages of `require()`, of the check ordering in
`committed_from`, and confirmed to fail — a property that cannot fail is not
coverage.
