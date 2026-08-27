# QA Procedure: Triage creates tasks in one of three kinds

Covers: `features/task_kinds.feature`

## Interface used

This procedure drives the running server over HTTP only, and inspects persisted
state with a read-only `sqlite3` query against the configured database file.
The read-only query is an inspection of durable state, not a call into a
project API. No project library, module, or test helper is used.

Two user-facing affordances are exercised:

- the capture endpoint (`POST /captures`), already shipped at M0
- the triage affordance for an untriaged capture, which accepts a JSON body
  carrying `kind` plus the fields that kind requires

## Setup — repeat before each procedure below

Each procedure starts from an empty database, matching the feature's
`Background`.

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the Pool, Committed and Quota screens all list nothing before triaging. **The capture page no longer carries a `Tasks` list (#140).**
4. Submit a capture with raw text `buy milk` and observe the capture's
   identifier in the response.

## Procedure — pool

1. Triage the capture as kind `pool`, supplying no deadline, no commitment,
   no priority, and no quota fields.
2. Observe the response status.
3. Query the tasks table for the resulting task.

### Expected Observable Outcomes
- Triage is accepted.
- Exactly one task exists, with kind `pool`.
- Its deadline is empty.
- **The Quota screen offers no quotas.** This replaces the old check that
  `target_count`, `target_minutes_each` and `period` were left empty. **Those
  columns still exist** — `T-migrations-append-only` means they cannot be
  dropped — **but nothing writes them any more**, so the old check would pass
  against any implementation whatsoever. **A check that cannot fail reads
  exactly like coverage** (#90).

## Procedure — committed — repeat once per example row

| deadline             | commitment | priority |
|----------------------|------------|----------|
| 2026-08-20T17:00:00Z | at         | P1       |
| 2026-08-31T09:00:00Z | by         | P3       |

1. Triage the capture as kind `committed`, supplying the row's deadline,
   commitment and priority.
2. Observe the response status.
3. Query the tasks table for the resulting task.

### Expected Observable Outcomes
- Triage is accepted.
- Exactly one task exists, with kind `committed`.
- Its deadline, commitment and priority match the submitted row exactly.
- **The Quota screen offers no quotas**, for the reason given under pool.

## Procedure — quota — repeat once per example row

**What a quota triage carries changed completely in #138.** `target_count`,
`target_minutes_each` and `period` are retired; a quota now carries **a name
and a weekly hour target**, and **triaging is what creates the quota.** The
owner settled that on 2026-08-26 — the reasoning and the whole validation
surface are in `qa/quota_triage_validation.md`.

**`kind` is still a three-variant sum type.** `quota` did not become an
unknown kind; it became the verb that creates a quota.

| name    | hours |
|---------|-------|
| Piano   | 4     |
| Running | 0.5   |

1. Triage the capture as kind `quota`, supplying the row's name and hours.
2. Observe the response status.
3. Read the Quota screen.

### Expected Observable Outcomes
- Triage is accepted.
- **The Quota screen offers exactly that one quota**, meta **`1 quota`**,
  reading **`0m / 4h`** and **`0m / 30m`** respectively.
- **`0.5` is the row that earns its keep.** The canvas's hours input is
  `step="0.5"`, so half an hour is a value the product can really submit, and
  it is the only row where the hours-to-minutes conversion is **visible in the
  readout** rather than implied by it. **If `0.5` reads `0m / 0h` or is
  rejected, the conversion is integer-truncating** and every half-hour target
  the owner ever sets will be wrong.
- The resulting task's deadline is empty.
- **Assert through the screen, not through columns.** Where a quota is stored
  is the architect's to settle; what the owner can see is that the thing they
  triaged is on the Quota screen reading its target.

## Not covered here, deliberately

The M1 acceptance criterion also says quota tasks "are not scheduled". That has
no observable surface at this milestone — there is no scheduler and no block
table — so it cannot be verified end to end until M3. It is recorded as M3/M8's
to assert, not silently skipped.

## Independent of Implementation

This procedure depends only on the triage request/response contract and the
durable task row it leaves behind. It does not depend on handler structure,
how `kind` is represented in Rust, which table a quota lives in, or the ORM in
use.
