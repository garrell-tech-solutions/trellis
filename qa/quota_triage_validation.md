# QA Procedure: Quota triage requires target count, target minutes each and period

Covers: `features/quota_triage_validation.feature`

## Interface used

HTTP only, plus read-only `sqlite3` inspection of persisted state. No project
library, module, or test helper is used.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the task list is empty.
4. Submit a capture with raw text `go to the gym` and observe the capture's
   identifier in the response.
5. Confirm the capture is present in the untriaged queue.

## Procedure — required field omitted — repeat once per example row

Example rows — the field omitted from an otherwise complete quota submission
(`target_count: 3`, `target_minutes_each: 45`, `period: week`):

| missing_field       |
|----------------------|
| target_count         |
| target_minutes_each  |
| period               |

For each row:

1. Triage the capture as kind `quota`, supplying every field **except** the
   row's `missing_field`.
2. Observe the response status and body.
3. Query the tasks table and count its rows.
4. Re-read the untriaged queue.

### Expected Observable Outcomes
- Triage is rejected, and the response body names the omitted field.
- The tasks table is still empty and the capture is still present, untriaged,
  in the queue.

## Procedure — period left empty

1. Triage the capture as kind `quota`, supplying `target_count: 3` and
   `target_minutes_each: 45`, with `period` submitted as `""`.
2. Observe the response status and body.
3. Query the tasks table and count its rows.

### Expected Observable Outcomes
- Triage is rejected, and the response body names `period` — the same
  rejection as `period` being omitted entirely.
- The tasks table is still empty.

## Procedure — invalid period — repeat once per example row

Example rows:

| bad_period |
|------------|
| fortnight  |
| Week       |

For each row:

1. Triage the capture as kind `quota`, supplying `target_count: 3` and
   `target_minutes_each: 45`, and the row's `bad_period`.
2. Observe the response status and body.
3. Query the tasks table and count its rows.

### Expected Observable Outcomes
- Triage is rejected, and the response body identifies `period` as the
  invalid field.
- The second row confirms the domain check is case-sensitive: `Week` is not
  accepted in place of `week`.
- The tasks table is still empty.

## Procedure — non-positive target_count — repeat once per example row

Example rows:

| bad_target_count |
|-------------------|
| 0                  |
| -1                 |

For each row:

1. Triage the capture as kind `quota`, supplying the row's `bad_target_count`,
   a valid `target_minutes_each: 45`, and `period: week`.
2. Observe the response status and body.
3. Query the tasks table and count its rows.

### Expected Observable Outcomes
- Triage is rejected, and the response body identifies `target_count` as the
  invalid field. A quota task with a zero or negative target can never be
  completed, so it can never be reported on at the reckoning — `0` is not a
  meaningful target, it is a divide-by-zero waiting to happen.
- The tasks table is still empty.

## Procedure — non-positive target_minutes_each — repeat once per example row

Example rows:

| bad_target_minutes_each |
|---------------------------|
| 0                          |
| -5                         |

For each row:

1. Triage the capture as kind `quota`, supplying a valid `target_count: 3`,
   the row's `bad_target_minutes_each`, and `period: week`.
2. Observe the response status and body.
3. Query the tasks table and count its rows.

### Expected Observable Outcomes
- Triage is rejected, and the response body identifies `target_minutes_each`
  as the invalid field.
- The tasks table is still empty.

## Independent of Implementation

This procedure depends only on the triage endpoint's rejection contract and on
durable state remaining unchanged on rejection. It does not depend on how the
quota target is represented internally, or which fields live in which table.
