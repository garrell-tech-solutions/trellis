# Trellis — Architecture Reference

**What this is:** the vocabulary the rest of the project refers to. Type shapes,
layer names, the scheduler's signature, the closed enums. Nothing else lives
here — the board holds state, milestones hold acceptance criteria, and
`docs/decisions.md` holds rationale.

**What this is not:** the product brief. That is the settled narrative of what
Trellis is for, and it is not in this repo. Every epic's `## Source` line points
at `docs/design/brief.md`, which has never existed; those links should point
here instead.

Every row is marked with what it is:

| | |
|---|---|
| **built** | exists in `crates/`, verifiable today |
| **specified** | fixed by a decision or an epic's acceptance criteria; not built |
| **GAP** | referred to by name elsewhere and defined nowhere. Needs a decision. |

---

## Crates — built

```
crates/scheduler-core     pure rules. no tokio, no sqlx, no async (T-core-no-tokio)
crates/trellis-server     axum + sqlx + askama. binary name `trellis`
crates/acceptance-tests   APS runtime + step handlers; entrypoints generated from features/
```

The brief specified six crates. Three exist. `scheduler-db` collapsed into
`trellis-server` — settled as a module boundary rather than a crate split, and
ratified by `T-package-by-business-domain`: the brief's other three
(`scheduler-web`, `scheduler-bot`, `scheduler-bin`) are role names too, and a
crate earns existence when a capability needs a real boundary — a purity gate,
an independent dependency set — not because a layer has a name. That closes
#24. Renaming the crates that exist is deliberately not part of it:
`scheduler-core` is named twice in the constitution, which no agent may edit.

## Two things are called "layers". They are unrelated.

### Domain layers — specified (`T-fact-plan-line`)

Which rows the scheduler may destroy. The line runs **inside** the `Block`
table, drawn by block state.

| Layer | Members | Behaviour |
|---|---|---|
| **Plan** | `proposed`, `published` future blocks | Disposable. Deleted wholesale and regenerated on every recompute. |
| **Constraints** | `in_progress`, `completed`, `missed` blocks; pins | Immutable inputs. Read by the engine, never written by it. |

"Facts" is informal shorthand for the immutable block subset. It is not a third
layer.

### The module boundary — built (`T-module-boundary`, `T-templates-take-view-models`)

Which code may name what. Dependencies point inward, and the tree names
business domains rather than technical roles (`T-package-by-business-domain`,
#44). What went away is `http/` and `store/` as *top-level directories*; the
dependency rule itself is unchanged.

```
crates/trellis-server/src/
  capture/    http.rs  store.rs
  triage/     http.rs  store.rs
  inbox/      http.rs  lists.rs  store.rs  view.rs
  life_areas/ mod.rs  http.rs  store.rs  view.rs
  stats/      http.rs  store.rs
  platform/   app.rs  assets.rs  boundary.rs  clock.rs  db.rs  request.rs
              response.rs  test_support.rs
```

Five capabilities and one bucket named so a reader can tell it is not one.
`scheduler-core` holds the rules and names neither adapter; a domain's `http`
turns requests into core inputs and core types into view models; its `store`
turns core types into rows and is the only production SQL; its `view` is what
a template renders, never a `store` row type. `capture` and `triage` reach
into `inbox::view` and `inbox::lists` because the inbox is the surface they
act on.

A domain has a `view` only when its page shape is its own: `inbox`'s rows are
assembled from two queries and carry a slot for an in-flight rejection, while
`stats` renders `scheduler_core::ratio`'s answer directly, and a struct
copying that field for field would be a view model in name only.

**A capability read by other capabilities gets one front door.** Three
capabilities need the pickable life areas — the management page lists them,
the inbox fragment offers them in each capture's triage forms, and a
quick-added capture renders its own row with the same forms. They call
`life_areas::active_options`, in that capability's `mod.rs`, rather than each
composing `store::list_active` with `LifeAreaOption::from` themselves: a
reader that knows which query *and* which mapping to combine is holding a
copy of another capability's internals, and three of those drift. Reaching
across for a *type* (`inbox::view::CaptureRow`, `life_areas::view::
LifeAreaOption`) stays fine; it is reaching across for the recipe that does
not.

**The rule that decides what goes in the core**, and the one this tree is
easiest to get wrong: a rule that survives changing HTTP for something else
belongs in `scheduler-core`. The window's length, the sample floor, the
fifty-percent line and the share arithmetic are all that, so they are
`scheduler_core::ratio`, not `stats/`. The core is the *enforced* pure
boundary — `cargo tree -p scheduler-core` is a gate with an acceptance test
behind it — while a pure-by-convention module sitting beside the adapters is
one import away from stopping being pure. `platform/boundary.rs` checks all of it by **walking** `src/` rather
than naming a directory — the previous check globbed `src/store/*.rs` and
would have stopped covering anything the moment that directory dissolved,
which is the failure `T-module-boundary` named against itself. It asserts that
no persistence module names `axum` or `StatusCode` (the old rule); that
nothing outside a `store.rs` or `platform/db.rs` writes production SQL (the
half the old check never had); that no top-level directory carries a
technical-role name; and a floor on each, so an empty walk fails rather than
passes. It is still a substring scan over source text — defeated by a type
alias or a macro, so treat it as a lint, not a proof.

## Three things are called "domain". They are unrelated.

The same problem as "layer", one word over, and it cost a full round trip
between the PM and the owner on 2026-08-14 before anyone noticed the two sides
were discussing different subjects.

| Term | Means | Where |
|---|---|---|
| **domain** (bare) | A **life area** — Work, Fitness, Learning, Family, Home. The thing guardrails wall off, capacity counts, and the reckoning groups by. | `T-life-areas-are-data`, `D-single-user`, `D-pool-is-default`, #47 |
| **business domain** (always both words) | The **code-packaging axis**. A capability the product provides, used to name directories and crates. | `T-package-by-business-domain`, #44 |
| **domain model** | Ordinary usage: the shapes below. Not a third concept, just the phrase. | this file |

Bare "domain" is always the life area. If you mean packaging, write both words.

## And "window" is not used at all

The fourth overloaded word, caught before M2 wrote it into code rather than
after. It was already ambiguous inside M2's own acceptance criteria — a mask in
`free_intervals(window, range)`, a rolling time span in "14-day capacity view",
and a time span again in `/stats`.

| Term | Means |
|---|---|
| **guardrail** | A life area's weekly mask: the hours in which its work may be scheduled (`D-life-area-owns-its-time`). |
| **range** | A span of time — the 14 days a capacity view covers, the fortnight `/stats` counts over. |
| ~~window~~ | **Do not use.** Say which of the two you mean. |

The signature is `free_intervals(guardrail, range)`. `allowed_windows` keeps its
name for now because it is the field #6 and #11 both cite; when it is built it
should be `borrowed_guardrails` or similar.

---

### The clock — built

`Clock` is a value the composition root hands to `build_app`, not a global a
handler reaches for. It carries an offset from the real wall clock, which is
what makes `trellis serve --now <RFC3339>` an offset rather than a freeze: the
server starts believing it is that instant and time advances normally from
there. One clock per server, so two servers in one process — or two tests —
disagree without disturbing each other.

## Domain model — built

Source of truth: `crates/scheduler-core/src/task.rs`.

```rust
pub enum TaskKind {
    Pool,
    Committed { deadline: i64, deadline_type: DeadlineType, priority: Priority },
    Quota     { target_count: i64, target_minutes_each: i64, period: Period },
}

pub enum DeadlineType { Hard, Soft }
pub enum Priority     { P1, P2, P3, P4 }
pub enum Period       { Week, Month }
```

`deadline` is **UTC epoch milliseconds** (`T-jiff-epoch-millis`), parsed with
`jiff` at the boundary. Never a string, in the type or the column.

Each variant carries exactly the fields that kind means, so "a pool task has no
deadline" is a fact about the type rather than a claim about one payload.
`TaskAttributes` is the nullable row-shaped projection; at most one attribute
group is ever populated.

### Schema — built

```sql
captures(id, raw_text, source, created_at_ms, triaged_at)
tasks(id, capture_id, kind, deadline, deadline_type, priority,
      target_count, target_minutes_each, period, archived_at, created_at_ms)
```

`kind`, `deadline_type`, `priority` and `period` carry `CHECK` constraints.
`deadline` does **not** — SQLite's INTEGER affinity does not reject text, so a
non-HTTP write path can still store a string there. Tracked in #33.

`archived_at` is the single archive signal; there is no `status` column
(`T-archived-at-only`). Nothing writes it yet — every archive route arrives at
M8.

A capture row is never deleted. Triage stamps `triaged_at`.

**Migrations are append-only** (`T-migrations-append-only`) and CI enforces it.
SQLite has no `ALTER COLUMN`, so a type change means the table-rebuild pattern
inside a *new* migration.

---

## The scheduler — specified, not built

### Signature (C2, #3)

```
schedule(tasks, busy, guardrails, pins, facts, prior_plan, now) -> placements
```

Seven parameters. `T-fact-plan-line` renders it inline with five; that is an
abbreviation written before C2 was ratified, and this form is the contract.

`prior_plan` exists because the move penalty makes the objective depend on
previous placements. `facts` exists because past blocks constrain the next run.

### Determinism (M3, #11)

> Delete every `proposed`/`published` future block, re-run with the same facts,
> get byte-identical placements.

Plus idempotence: two consecutive runs on unchanged inputs agree byte for byte.
The plan is recomputed from scratch on every trigger — never patched
(`R-incremental-patching`).

### Passes

Backward pass over hard-deadline tasks by latest-feasible start; forward pass by
least slack with priority as tiebreak; splitting under chunk policy. Approach is
greedy-with-repair, not a solver (`T-greedy-with-repair`) — the interface stays
clean so an optimiser can be swapped in behind it.

### Infeasibility report — closed enum (M3, #11)

```
no_window · capacity_exceeded · deadline_unreachable · chunk_policy_unsatisfiable
```

Every unplaceable task is named with one of these. The placed/unplaceable
partition is total.

### Pins — specified, lands at M3 (`T-pins-in-constraints`)

```
pin { task_id, start, end, source }
```

A first-class Constraints-layer entity, not a column on `Block`. Two questions
it raises are unanswered: which chunk a pin binds when a task splits, and
whether a pin ever expires. The reason enum above has no code for "a stale pin
is in the way".

### Guardrails and free time — specified (M2, #10)

```
free_intervals(guardrail, range) -> disjoint, sorted intervals
                                    each a subset of (mask - busy - pins - buffers)
```

**Each life area carries one guardrail** — the hours its work may be scheduled
in (`D-life-area-owns-its-time`). A task goes in its own life area's hours by
default and may not go outside them; borrowing another's is an explicit
per-task permission. **Guardrails may overlap in clock time**, and where they
do their life areas compete, resolved by deadline and priority. A life area
with no guardrail must be marked **pool-only** — never placed, only offered by
the menu (`T-life-areas-are-data`'s well-formedness rule, made concrete).

Reservation is the default and sharing is opt-in, which is what lets one
mechanism serve both jobs the settled decisions demand: **containment**, since
`D-guardrails-never-yield` means a P1 hard deadline against a full guardrail
raises a conflict rather than breaching the wall; and **reservation**, since
`T-three-task-kinds` warns that *a Fitness guardrail nothing is scheduled into
is a wall protecting an empty room*. Fitness's 06:00 is protected by nothing
having claimed it, not by a protection rule.

Guardrails are civil wall-clock, so DST gaps and folds are real cases, not edge
cases (`T-jiff-epoch-millis`).

### Capacity — specified (M2, #10; `T-capacity-two-axes`)

**Consumed from the guardrail occupied, attributed to the task's life area.**

```
Work: 5h of 8h used — 2h of that is Learning you allowed in.
Learning: 2h done this week.
```

Two numbers because there are two questions: *how much of this wall is left* is
about the clock, *how much Learning did I do* is about the work. Charging a
borrowed hour to only one of them makes the other lie — and the availability
lie is the expensive one, since it reports free time that is physically
occupied. Quota demand **counts** toward capacity (`T-three-task-kinds`); pool
consumes **nothing**, because pool is never placed
(`D-no-pool-on-calendar`).

---

## The invariants

M3's strongest acceptance criterion asserts "Invariants 1–5 hold under
`proptest`, ≥1000 cases". Two are described somewhere in the project. Three are
not, and until they are written down that criterion cannot be specified.

| # | Statement | |
|---|---|---|
| 1 | — | **GAP** |
| 2 | A block lies entirely within **one** allowed window. | specified (U4, #7) |
| 3 | — | **GAP** |
| 4 | — | **GAP** |
| 5 | The placed/unplaceable partition is **total**; every unplaceable task carries a reason from the closed enum. | specified (#11) |

Reconstruction offered and **not yet ratified** — do not build against these:
non-overlap; conservation under splitting; hard deadlines hold.

Invariant 2 has a known cost, accepted deliberately: a 2h task allowed in both
Work and Personal, with Work ending and Personal starting at 17:00, cannot use
those two contiguous free hours.

---

## Life areas — built (`T-life-areas-are-data`, #47)

Settled 2026-08-14, resolving #36. **Life areas are user-managed rows, editable
from the running app** — not a Rust enum, not a config file. There is no
canonical list to ratify; there is a seed, and then it is the user's.

```sql
life_areas(id, name, archived_at)        -- name is UNIQUE COLLATE NOCASE
seed        Work · Fitness · Learning · Family · Home
tasks.life_area_id                       -- nullable column, required at triage
```

**Two names are the same name when they match once trimmed and case-folded**,
and the column's `UNIQUE COLLATE NOCASE` is where that is enforced — not a
function in `scheduler_core`. "Work" and "work" as two indistinguishable
picker entries is the failure the rule exists to prevent, and a constraint
the database checks cannot be bypassed by a write path that forgot to call
something. Two consequences: a database swap has to carry the collation
across (`T-sqlite-sqlx`'s "mechanical" Postgres move needs `CITEXT` or a
functional unique index), and if the rule ever outgrows a collation — Unicode
folding — it moves into the core and the constraint becomes the backstop.

`tasks.life_area_id` is nullable for the reason `T-quota-targets-required`
already established: SQLite cannot add a `NOT NULL` column without a default
to a table that may hold rows, so the requirement lives at the triage
boundary instead. `None` therefore means "written before this migration", not
"has no life area".

The seed keeps every life area cited in a settled decision's reasoning — Fitness
(`T-three-task-kinds`), Learning (`D-kill-means-archive`), Family
(`D-single-user`), Work (`D-pool-is-default`) — and adds Home for errand and
admin traffic, which neither candidate set housed.

**One concept, not two.** The capture-tagging vocabulary and the scheduler's
life area are the same list. Every consumer of one is a scheduler concern —
guardrails, capacity, the reckoning, menu diversity (#41) — so two lists would
make the mapping between them the real list.

**Why not a closed enum**, given `kind`, `period`, `deadline_type` and `priority`
all are: those are fields the scheduler **branches on**, a `match` with a
different body per variant. A life area is a **lookup key, not a discriminant** —
every one is handled identically. A set you `match` on must be closed; a set you
index by need not be.

**The invariant that replaces the enum:** a life area is well-formed only once
it has a guardrail, or is explicitly marked pool-only. Enforced at M2, when
guardrails exist. A non-exhaustive `match` never expressed this — it reports
missing arms, not a missing wall.

`T-complexity-8`'s threshold of 8 is unaffected: it is derived from *"the largest
enums (`Domain`, `BlockState`) have 5 variants"*, and `BlockState` still has
exactly five, so the derivation stands on that one alone.

---

## Capture and triage — built

```
GET  /                        the inbox: untriaged captures, newest first
GET  /static/htmx.min.js      vendored; no Node build step
POST /captures                raw text in. JSON -> 201 JSON; form-encoded -> 201
                              HTML fragment. One route, content-negotiated.
                              50ms budget, asserted by capture_endpoint.feature.
POST /captures/{id}/triage    capture -> task
GET  /stats                   the committed share of the last fourteen days
                              (R2, #45). Rules in `scheduler_core::ratio`.
GET  /life-areas              manage the set: list, add, archive (#47)
POST /life-areas              add one. Duplicate or blank -> 422 + the list
                              fragment carrying the message.
POST /life-areas/{id}/archive retire one. Never deleted (D-kill-means-archive).
```

Triage requires a life area for every kind. The name is validated in two
steps that meet at `scheduler_core::task::WellFormedTriage`: the core decides
kind first and then that *some* life area was named, and the adapter resolves
that name against the `life_areas` table — a question needing the database,
so `unknown_life_area` is the one rejection the core does not produce.

Rejections are `422`. An unrecognised `kind` reports
`{"unknown_kind": <submitted>}` (`T-unknown-kind-rejected`); a missing or empty
required field reports `{"missing_field": <name>}` — absent and empty are the
same submitter mistake and report identically (`T-empty-equals-absent`).

### Classification — specified, **M9 only** (`T-classifier-covers-domain`, `D-manual-triage-until-llm`)

**Nothing classifies a capture before M9. Triage is fully manual until then**,
and the life-area picker carries no preselection — a default that is merely
first-by-id is a silent wrong answer the user never chose
(`D-manual-triage-until-llm`, 2026-08-17). The keyword implementation
`T-classifier-covers-domain` scheduled for M1 is cancelled, not deferred.

What lands at M9: one trait, output covering `kind`, `deadline`, `priority`,
`domain` and `title`, each with per-field confidence. Invoked by a **background
worker between capture and triage** — not inside `POST /captures`, which has a
50ms budget no LLM round trip fits, and not synchronously at triage, which
would put the wait in front of the user. Failure or timeout falls back to
**empty fields and manual triage**, which is the shipped M1 product rather than
a second classifier nobody validated.

`captures` therefore carries no classification columns and gains none until M9 —
a schema element with no observable behaviour has nothing to specify against.

---

## Gaps index

Everything above marked **GAP**, in the order it blocks work:

| Gap | Blocks | Tracked |
|---|---|---|
| Invariants 1, 3, 4 undefined | M3 cannot be specified | #11 |
| ~~Which five domains, and one concept or two~~ | ~~M1 S4, M9~~ | **closed** — `T-life-areas-are-data`, #47 |
| ~~Per-life-area capacity vs `allowed_windows`~~ | ~~M2~~ | **closed** — `T-capacity-two-axes` + `D-life-area-owns-its-time`, #6 |
| `Block::missed` unreachable under silence-means-done | M6 | #4 |
| U2 / U3 / U4 — hard vs soft, backward-pass input, window crossing | M3 | #7 |
| ~~Crate layout ratification~~ | ~~nothing; cost grows~~ | **closed** — `T-package-by-business-domain`, #44 |
