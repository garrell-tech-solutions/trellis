#!/usr/bin/env bash
# Executable QA procedure: qa/committed_field_domains.md (covers
# features/committed_field_domains.feature). Drives the running server
# through its HTTP interface only, and inspects persisted state via a
# read-only sqlite3 query -- never through a project-internal API. The
# capture endpoint's response carries no identifier, so (as in
# capture_endpoint.sh) the capture row is located by its raw text.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-committed-field-domains"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

setup_scenario() { qa_setup_scenario "$BIN" "$1" "$TMP_DIR" "call the dentist"; }

# --- Procedure: deadline round-trip ---
# deadline is stored as epoch milliseconds (T-jiff-epoch-millis); every row
# below names the same instant in a different textual form, so all three must
# store the same value.
run_deadline_round_trip() {
  local submitted_deadline="$1" expected_epoch_ms="$2"
  local name="round-trip-$(echo "$submitted_deadline" | tr -cs 'A-Za-z0-9' '-')"
  setup_scenario "$name" || return
  local body
  body="$(printf '{"kind":"committed","deadline":"%s","commitment":"at","priority":"P1","estimated_minutes":180,"life_area":"Work"}' "$submitted_deadline")"
  qa_triage "$CAPTURE_ID" "$body"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triage of \"$submitted_deadline\" returned status $STATUS, expected 201" >&2
    FAILURES=1
  fi
  local stored
  stored="$(sqlite3 "$DB_PATH" "SELECT deadline FROM tasks WHERE capture_id = $CAPTURE_ID;")"
  if [[ "$stored" != "$expected_epoch_ms" ]]; then
    echo "FAIL: [$name] \"$submitted_deadline\" stored as deadline=$stored, expected $expected_epoch_ms" >&2
    FAILURES=1
  fi
  qa_stop_server
}
run_deadline_round_trip "2026-08-20T17:00:00Z" "1787245200000"
run_deadline_round_trip "2026-08-20T17:00:00.000Z" "1787245200000"
run_deadline_round_trip "2026-08-20T19:00:00+02:00" "1787245200000"

# --- Procedure: invalid deadline ---
run_invalid_deadline() {
  local bad_deadline="$1"
  local name="invalid-deadline-$(echo "$bad_deadline" | tr -cs 'A-Za-z0-9' '-')"
  setup_scenario "$name" || return
  local body
  body="$(python3 -c 'import json,sys; print(json.dumps({"kind":"committed","deadline":sys.argv[1],"commitment":"at","priority":"P1","estimated_minutes":180}))' "$bad_deadline")"
  qa_triage "$CAPTURE_ID" "$body"
  qa_assert_rejected_naming "$name" invalid_field deadline
  qa_stop_server
}
run_invalid_deadline "banana"
run_invalid_deadline "2026-13-45T99:99:99Z"
run_invalid_deadline "'); DROP TABLE tasks;--"

# --- Procedure: invalid commitment ---
# commitment (at | by) replaced deadline_type (hard | soft) in #94's
# required set; the domain check moved with it (qa/committed_field_domains.md's
# corrected #94 note -- deadline_type itself is unread and unvalidated now).
run_invalid_commitment() {
  local bad_commitment="$1"
  local name="invalid-commitment-$bad_commitment"
  setup_scenario "$name" || return
  local body
  body="$(printf '{"kind":"committed","deadline":"2026-08-20T17:00:00Z","commitment":"%s","priority":"P1","estimated_minutes":180}' "$bad_commitment")"
  qa_triage "$CAPTURE_ID" "$body"
  qa_assert_rejected_naming "$name" invalid_field commitment
  qa_stop_server
}
run_invalid_commitment "squishy"
run_invalid_commitment "HARD"

# --- Procedure: invalid priority ---
run_invalid_priority() {
  local bad_priority="$1"
  local name="invalid-priority-$bad_priority"
  setup_scenario "$name" || return
  local body
  body="$(printf '{"kind":"committed","deadline":"2026-08-20T17:00:00Z","commitment":"at","priority":"%s","estimated_minutes":180}' "$bad_priority")"
  qa_triage "$CAPTURE_ID" "$body"
  qa_assert_rejected_naming "$name" invalid_field priority
  qa_stop_server
}
run_invalid_priority "P9"
run_invalid_priority "p1"

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: committed_field_domains"
