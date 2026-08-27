# QA Procedure: Committed triage requires deadline, commitment, priority and an estimate

> **`commitment` replaced `deadline_type` on the committed form in #94.** A
> committed item is an **at** (a fixed block) or a **by** (a deadline with
> slack) — `D-committed-is-at-or-by`. `deadline_type`'s only behaviour lived
> in the scheduler `D-dogfood-first` paused, so it changed nothing while
> costing a field. **The column is still in the schema and still
> domain-validated when sent; it is simply no longer asked for or read.**

Covers: `features/committed_triage_validation.feature`

## Interface used

HTTP only, plus read-only `sqlite3` inspection of persisted state. No project
library, module, or test helper is used.

## Setup — repeat before each example row

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the Pool, Committed and Quota screens all list nothing. **There is no `Tasks` list on the capture page any more (#140)** — the destination screens are where a write is now visible.
4. Submit a capture with raw text `call the dentist` and observe the capture's
   identifier in the response.
5. Confirm the capture is present in the untriaged queue.

## Procedure — repeat once per example row

Example rows — the field omitted from an otherwise complete committed
submission:

| missing_field |
|---------------|
| deadline      |
| commitment    |
| priority      |

For each row:

1. Triage the capture as kind `committed`, supplying every required field
   **except** the row's `missing_field`.
2. Observe the response status.
3. Observe the response body.
4. Query the tasks table and count its rows.
5. Re-read the untriaged queue.

### Expected Observable Outcomes
- Triage is rejected — the response reports a client error, not success.
- The response body names the omitted field, so the user can tell which one was
  missing. A rejection that does not say what was wrong fails this procedure
  even if the status code is correct.
- The tasks table is still empty. A rejected triage must not leave a partial
  task behind.
- The capture is still present in the untriaged queue, and is still untriaged —
  a rejected triage must not consume the capture.

## Procedure — required field left empty — repeat once per example row

Example rows — the field submitted as an empty string in an otherwise complete
committed submission:

| empty_field   |
|---------------|
| deadline      |
| commitment    |
| priority      |

For each row:

1. Triage the capture as kind `committed`, supplying every required field with
   its normal value, except the row's `empty_field`, which is submitted as
   `""`.
2. Observe the response status.
3. Observe the response body.
4. Query the tasks table and count its rows.
5. Re-read the untriaged queue.

### Expected Observable Outcomes
- Triage is rejected, and the response body names the empty field — the same
  rejection shape as the row's field being omitted entirely. An empty string
  is not a value the field is willing to accept; it must not be treated as
  "present."
- The tasks table is still empty and the capture is still present, untriaged,
  in the queue.

## Independent of Implementation

This procedure depends only on the triage endpoint's rejection contract and on
two pieces of durable state remaining unchanged. It does not depend on which
validation library is used, whether validation runs before or after
deserialization, or how the error is represented internally.
