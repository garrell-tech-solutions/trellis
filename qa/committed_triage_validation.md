# QA Procedure: Committed triage requires deadline, deadline type and priority

Covers: `features/committed_triage_validation.feature`

## Interface used

HTTP only, plus read-only `sqlite3` inspection of persisted state. No project
library, module, or test helper is used.

## Setup — repeat before each example row

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the task list is empty.
4. Submit a capture with raw text `call the dentist` and observe the capture's
   identifier in the response.
5. Confirm the capture is present in the untriaged queue.

## Procedure — repeat once per example row

Example rows — the field omitted from an otherwise complete committed
submission:

| missing_field |
|---------------|
| deadline      |
| deadline_type |
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

## Independent of Implementation

This procedure depends only on the triage endpoint's rejection contract and on
two pieces of durable state remaining unchanged. It does not depend on which
validation library is used, whether validation runs before or after
deserialization, or how the error is represented internally.
