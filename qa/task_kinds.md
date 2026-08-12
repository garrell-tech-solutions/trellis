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

Each procedure starts from an empty task list, matching the feature's
`Background`.

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the task list is empty before triaging.
4. Submit a capture with raw text `buy milk` and observe the capture's
   identifier in the response.

## Procedure — pool

1. Triage the capture as kind `pool`, supplying no deadline, no deadline type,
   no priority, and no quota fields.
2. Observe the response status.
3. Query the tasks table for the resulting task.

### Expected Observable Outcomes
- Triage is accepted.
- Exactly one task exists, with kind `pool`.
- Its deadline is empty.
- Its quota target fields (`target_count`, `target_minutes_each`, `period`) are
  all empty.

## Procedure — committed — repeat once per example row

| deadline             | deadline_type | priority |
|----------------------|---------------|----------|
| 2026-08-20T17:00:00Z | hard          | P1       |
| 2026-08-31T09:00:00Z | soft          | P3       |

1. Triage the capture as kind `committed`, supplying the row's deadline,
   deadline type and priority.
2. Observe the response status.
3. Query the tasks table for the resulting task.

### Expected Observable Outcomes
- Triage is accepted.
- Exactly one task exists, with kind `committed`.
- Its deadline, deadline type and priority match the submitted row exactly.
- Its quota target fields are all empty.

## Procedure — quota — repeat once per example row

| target_count | target_minutes_each |
|--------------|---------------------|
| 3            | 45                  |
| 1            | 90                  |

1. Triage the capture as kind `quota`, supplying the row's target count, target
   minutes each, and period `week`.
2. Observe the response status.
3. Query the tasks table for the resulting task.

### Expected Observable Outcomes
- Triage is accepted.
- Exactly one task exists, with kind `quota`.
- Its target count and target minutes each match the submitted row exactly, and
  its period is `week`.
- Its deadline is empty.

## Not covered here, deliberately

The M1 acceptance criterion also says quota tasks "are not scheduled". That has
no observable surface at this milestone — there is no scheduler and no block
table — so it cannot be verified end to end until M3. It is recorded as M3/M8's
to assert, not silently skipped.

## Independent of Implementation

This procedure depends only on the triage request/response contract and the
durable task row it leaves behind. It does not depend on handler structure,
how `kind` is represented in Rust, whether quota fields live in one table or
several, or the ORM in use.
