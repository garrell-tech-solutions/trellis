# QA Procedure: Committed triage validates the values of deadline, commitment and priority

> **#94 note, corrected.** `commitment` (at | by) replaced `deadline_type`
> (hard | soft) in the required set, and **the domain check moved with it** —
> the feature now validates `commitment`, and **nothing validates
> `deadline_type` any more.** The column survives in the schema, unread and
> unchecked, on #88's ground that dropping one throws away what the owner
> already typed.
>
> An earlier version of this note claimed `deadline_type` was still
> domain-validated when sent, and told you to send it explicitly. **That was
> wrong** and QA caught it: it would have had you testing a rejection that no
> longer exists.

Covers: `features/committed_field_domains.feature`

## Interface used

HTTP only, plus read-only `sqlite3` inspection of persisted state. No project
library, module, or test helper is used.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the task list is empty.
4. Submit a capture with raw text `call the dentist` and observe the capture's
   identifier in the response.
5. Confirm the capture is present in the untriaged queue.

## Procedure — deadline round-trip — repeat once per example row

Example rows — a deadline submitted in a valid but varying textual form, and
the instant it names, as milliseconds since the Unix epoch:

| submitted_deadline         | expected_epoch_ms |
|-----------------------------|--------------------|
| 2026-08-20T17:00:00Z        | 1787245200000      |
| 2026-08-20T17:00:00.000Z    | 1787245200000      |
| 2026-08-20T19:00:00+02:00   | 1787245200000      |

For each row:

1. Triage the capture as kind `committed`, supplying the row's
   `submitted_deadline`, a valid `commitment`, and a valid `priority`.
2. Observe the response status.
3. Query the `deadline` column of the resulting task row directly (it is
   stored as milliseconds since the epoch, not as the submitted text).

### Expected Observable Outcomes
- Triage is accepted.
- The stored `deadline` equals the row's `expected_epoch_ms` exactly, for
  every textual form in the table — the three example rows all name the same
  instant, so all three must produce the same stored value. A procedure that
  only checks one form would miss an implementation that stores the submitted
  string's byte length or offset incorrectly.

## Procedure — invalid deadline — repeat once per example row

Example rows — a `deadline` value that does not parse to a real instant:

| bad_deadline              |
|----------------------------|
| banana                     |
| 2026-13-45T99:99:99Z       |
| '); DROP TABLE tasks;--    |

For each row:

1. Triage the capture as kind `committed`, supplying the row's `bad_deadline`
   and a valid `commitment`/`priority`.
2. Observe the response status and body.
3. Query the tasks table and count its rows.
4. Re-read the untriaged queue.

### Expected Observable Outcomes
- Triage is rejected, and the response body identifies `deadline` as the
  invalid field.
- The second example row is a syntactically plausible timestamp
  (`2026-13-45T99:99:99Z` has the right shape) that names no real instant —
  confirm the rejection is a genuine parse failure, not a shape/regex check
  that a plausible-looking string can slip past.
- The third example row is a SQL-injection-shaped string; confirm the tasks
  table still exists and is still empty afterward, not merely that the
  request was rejected.
- The tasks table is still empty and the capture is still present, untriaged,
  in the queue, for every row.

## Procedure — invalid commitment — repeat once per example row

Example rows:

| bad_commitment    |
|--------------------|
| squishy            |
| HARD               |

For each row:

1. Triage the capture as kind `committed`, supplying a valid `deadline` and
   `priority`, and the row's `bad_commitment`.
2. Observe the response status and body.
3. Query the tasks table and count its rows.

### Expected Observable Outcomes
- Triage is rejected, and the response body identifies `commitment` as the
  invalid field.
- The second row confirms the domain check is case-sensitive: `HARD` is not
  accepted in place of `hard`.
- The tasks table is still empty.

## Procedure — invalid priority — repeat once per example row

Example rows:

| bad_priority |
|--------------|
| P9           |
| p1           |

For each row:

1. Triage the capture as kind `committed`, supplying a valid `deadline` and
   `commitment`, and the row's `bad_priority`.
2. Observe the response status and body.
3. Query the tasks table and count its rows.

### Expected Observable Outcomes
- Triage is rejected, and the response body identifies `priority` as the
  invalid field.
- The second row confirms the domain check is case-sensitive: `p1` is not
  accepted in place of `P1`.
- The tasks table is still empty.

## Independent of Implementation

This procedure depends only on the triage endpoint's rejection contract, the
stored representation of an accepted deadline as epoch milliseconds, and
durable state remaining unchanged on rejection. It does not depend on which
date/time library performs the parse, or how `commitment`/`priority` are
represented internally.
