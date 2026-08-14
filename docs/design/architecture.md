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
still awaiting ratification (#24).

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

Which code may name what. Dependencies point inward.

```
scheduler-core          the rules. names neither adapter.
trellis-server::http    requests -> core inputs; core types -> view models
trellis-server::store   core types -> rows
```

`store/mod.rs` carries a test asserting no store module names `axum` or
`StatusCode`. It is a substring grep over source text — it catches the naive
import and is defeated by a nested submodule or a type alias. Treat it as a
lint, not a proof.

Templates render `http::view` models, never `store` row types.

**Superseded in shape, not in substance** (`T-package-by-business-domain`, #44).
The dependency rule above stands. The `http/` + `store/` *directories* do not —
the tree is being reorganised by business domain, so a capability's delivery,
persistence and view code sit together under its own name. Until #44 merges,
what is described above is what is built.

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

---

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
free_intervals(window, range) -> disjoint, sorted intervals
                                 each a subset of (mask - busy - pins - buffers)
```

Guardrails are civil wall-clock, so DST gaps and folds are real cases, not edge
cases (`T-jiff-epoch-millis`). Guardrails never yield to deadlines
(`D-guardrails-never-yield`): a P1 hard deadline against full windows raises a
conflict, it does not breach the wall.

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

## Life areas — specified (`T-life-areas-are-data`, #47)

Settled 2026-08-14, resolving #36. **Life areas are user-managed rows, editable
from the running app** — not a Rust enum, not a config file. There is no
canonical list to ratify; there is a seed, and then it is the user's.

```
life area   { name, archived_at }        # shape TBD by #47
seed        Work · Fitness · Learning · Family · Home
```

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
```

Rejections are `422`. An unrecognised `kind` reports
`{"unknown_kind": <submitted>}` (`T-unknown-kind-rejected`); a missing or empty
required field reports `{"missing_field": <name>}` — absent and empty are the
same submitter mistake and report identically (`T-empty-equals-absent`).

### Classification — specified (`T-classifier-covers-domain`)

One trait, two implementations: keyword at M1, LLM at M9. Output covers `kind`,
`deadline`, `priority`, `domain` and `title`, each with per-field confidence.

Invoked by a **background worker between capture and triage** — not inside
`POST /captures`, which has a 50ms budget no LLM round trip fits, and not
synchronously at triage, which would put the wait in front of the user.

---

## Gaps index

Everything above marked **GAP**, in the order it blocks work:

| Gap | Blocks | Tracked |
|---|---|---|
| Invariants 1, 3, 4 undefined | M3 cannot be specified | #11 |
| ~~Which five domains, and one concept or two~~ | ~~M1 S4, M9~~ | **closed** — `T-life-areas-are-data`, #47 |
| Per-life-area capacity vs `allowed_windows` | M2 | #6 |
| `Block::missed` unreachable under silence-means-done | M6 | #4 |
| U2 / U3 / U4 — hard vs soft, backward-pass input, window crossing | M3 | #7 |
| Crate layout ratification | nothing; cost grows | #24 |
