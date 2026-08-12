#!/usr/bin/env bash
# Executable QA procedure: qa/quota_triage_validation.md (covers
# features/quota_triage_validation.feature). Drives the running server
# through its HTTP interface only, and inspects persisted state via a
# read-only sqlite3 query -- never through a project-internal API. The
# capture endpoint's response carries no identifier, so (as in
# capture_endpoint.sh) the capture row is located by its raw text.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-quota-triage-validation"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

setup_scenario() { qa_setup_scenario "$BIN" "$1" "$TMP_DIR" "go to the gym"; }

# --- Procedure: required field omitted ---
run_missing_field() {
  local missing_field="$1"
  local name="missing-$missing_field"
  setup_scenario "$name" || return
  local body
  body="$(python3 -c '
import json, sys
payload = {"kind": "quota", "target_count": 3, "target_minutes_each": 45, "period": "week"}
del payload[sys.argv[1]]
print(json.dumps(payload))
' "$missing_field")"
  qa_triage "$CAPTURE_ID" "$body"
  qa_assert_rejected_naming "$name" missing_field "$missing_field"
  qa_stop_server
}
run_missing_field "target_count"
run_missing_field "target_minutes_each"
run_missing_field "period"

# --- Procedure: period left empty ---
name="period-empty"
if setup_scenario "$name"; then
  qa_triage "$CAPTURE_ID" '{"kind":"quota","target_count":3,"target_minutes_each":45,"period":""}'
  qa_assert_rejected_naming "$name" missing_field "period"
fi
qa_stop_server

# --- Procedure: invalid period ---
run_invalid_period() {
  local bad_period="$1"
  local name="invalid-period-$bad_period"
  setup_scenario "$name" || return
  local body
  body="$(printf '{"kind":"quota","target_count":3,"target_minutes_each":45,"period":"%s"}' "$bad_period")"
  qa_triage "$CAPTURE_ID" "$body"
  qa_assert_rejected_naming "$name" invalid_field period
  qa_stop_server
}
run_invalid_period "fortnight"
run_invalid_period "Week"

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: quota_triage_validation"
