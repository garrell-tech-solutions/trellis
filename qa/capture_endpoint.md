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

## Procedure — the 50 ms budget, which now lives here

**Moved out of the acceptance and unit suites in #88**
(`T-latency-is-a-qa-assertion`). It is a property of the deployed system, and
`qa/one_screen.md` carries the same procedure with the full reasoning.

1. Start the server against a fresh database **on a quiet machine**. Record
   the load average before you begin.
2. Send ten capture requests and record each response time.

### Expected Observable Outcomes
- Every response is **201**, every row persists, and every response arrives
  **within 50 ms**.
- **A reading taken under load is not evidence.** Say what the load was and
  re-run if it was high — this assertion cost four spurious re-runs in six
  slices while it lived in the suites, and once silently voided the whole
  `trellis-server` crate's mutation coverage by failing `cargo-mutants`'
  unmutated baseline at 1.885 s.
- 50 ms remains the **design constraint** — `capture`'s module header cites
  it and `T-classifier-covers-domain` reasons from it. Only where it is
  asserted changed.
