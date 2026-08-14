#!/usr/bin/env bash
# Executable QA procedure: qa/unknown_kind_rejection.md (covers
# features/unknown_kind_rejection.feature). Drives the running server
# through its HTTP interface only, and inspects persisted state via a
# read-only sqlite3 query -- never through a project-internal API. The
# capture endpoint's response carries no identifier, so (as in
# capture_endpoint.sh) the capture row is located by its raw text.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-unknown-kind-rejection"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

setup_scenario() { qa_setup_scenario "$BIN" "$1" "$TMP_DIR" "buy milk"; }

# --- Procedure: unrecognised kind ---
run_unrecognised_kind() {
  local bad_kind="$1"
  local name="unrecognised-$bad_kind"
  setup_scenario "$name" || return

  local body
  body="$(python3 -c 'import json,sys; print(json.dumps({"kind": sys.argv[1]}))' "$bad_kind")"
  qa_triage "$CAPTURE_ID" "$body"
  qa_assert_rejected_naming "$name" unknown_kind "$bad_kind"
  qa_stop_server
}
run_unrecognised_kind "someday"
run_unrecognised_kind "later"

# --- Procedure: kind not named ---
name="kind-absent"
if setup_scenario "$name"; then
  qa_triage "$CAPTURE_ID" '{}'

  if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
    echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
    FAILURES=1
  fi
  # An absent kind is reported as unknown_kind: null, not silently treated
  # as pool or omitted from the body -- qa_json_raw_field (unlike the
  # string-only qa_json_field) can tell "null" from "key absent".
  raw="$(qa_json_raw_field "$BODY" unknown_kind)"
  if [[ "$raw" != "null" ]]; then
    echo "FAIL: [$name] expected the rejection body to report unknown_kind: null, got unknown_kind: ${raw:-<absent>} (body: $BODY)" >&2
    FAILURES=1
  fi
  qa_assert_durable_state_unchanged "$name"
fi
qa_stop_server

# --- Procedure: kind submitted as the wrong JSON type ---
name="wrong-json-type"
if setup_scenario "$name"; then
  qa_triage "$CAPTURE_ID" '{"kind": 7}'

  if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
    echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
    FAILURES=1
  fi
  # The rejection must echo back the submitted number 7, not null -- a
  # wrong-typed kind is still something the client can see reflected.
  raw="$(qa_json_raw_field "$BODY" unknown_kind)"
  if [[ "$raw" != "7" ]]; then
    echo "FAIL: [$name] expected the rejection body to report unknown_kind: 7, got unknown_kind: ${raw:-<absent>} (body: $BODY)" >&2
    FAILURES=1
  fi
  qa_assert_durable_state_unchanged "$name"
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: unknown_kind_rejection"
