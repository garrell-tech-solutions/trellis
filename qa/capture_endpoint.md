# QA Procedure: Capture endpoint accepts and persists captures

Covers: `features/capture_endpoint.feature`

## Setup
1. Start the trellis server against a fresh database (empty `captures` table).
2. Confirm the server is reachable before sending any capture request.

## Procedure — repeat once per example row
Example rows:

| raw_text         | source   |
|-------------------|----------|
| buy milk          | web      |
| call the dentist  | telegram |

For each row:
1. Send an HTTP POST to the capture endpoint with a JSON body of
   `{"raw_text": "<raw_text>", "source": "<source>"}`, timing the request.
2. Observe the HTTP response status code.
3. Observe the elapsed time from request sent to response received.
4. Query the `captures` table (via the project's own inspection affordance,
   e.g. a `sqlite3` shell against the configured database file — this is a
   read-only inspection of persisted state, not a call into a project API)
   for a row matching the submitted `raw_text` and `source`.

## Expected Observable Outcomes
- Response status is `201` for every example row.
- Elapsed time is under 50 milliseconds for every example row.
- Exactly one matching row exists in `captures` per submitted example after
  its request completes.

## Independent of Implementation
This procedure only depends on the HTTP capture endpoint's request/response
contract and the durable row it leaves behind — not on internal handler
structure, ORM choice, or module layout.
