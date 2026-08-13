# QA Procedure: Triage rejects a kind outside pool, committed and quota

Covers: `features/unknown_kind_rejection.feature`

## Interface used

HTTP only, plus read-only `sqlite3` inspection of persisted state. No project
library, module, or test helper is used.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. Confirm the task list is empty.
4. Submit a capture with raw text `buy milk` and observe the capture's
   identifier in the response.
5. Confirm the capture is present in the untriaged queue.

## Procedure — unrecognised kind — repeat once per example row

Example rows:

| bad_kind |
|----------|
| someday  |
| later    |

For each row:

1. Triage the capture, submitting `kind` as the row's `bad_kind` and no other
   fields.
2. Observe the response status and body.
3. Query the tasks table and count its rows.
4. Re-read the untriaged queue.

### Expected Observable Outcomes
- Triage is rejected.
- The response body reports an unknown kind, and echoes back exactly what was
  submitted (`bad_kind`) — a client sending a typo'd kind can tell what it
  sent, not just that it was rejected.
- The tasks table is still empty and the capture is still present, untriaged,
  in the queue.

## Procedure — kind not named

1. Triage the capture, submitting a request body that names no `kind` at all
   (an empty JSON object).
2. Observe the response status and body.
3. Query the tasks table and count its rows.

### Expected Observable Outcomes
- Triage is rejected, with the same "unknown kind" rejection shape as a named
  but unrecognised kind, reporting no kind was submitted (rather than, for
  example, being silently treated as `pool`).
- The tasks table is still empty.

## Independent of Implementation

This procedure depends only on the triage endpoint's rejection contract and on
durable state remaining unchanged on rejection. This is existing, shipped
behaviour (T-unknown-kind-rejected); this procedure gives it QA coverage it
did not previously have, without changing it.
