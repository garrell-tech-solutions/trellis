#!/usr/bin/env bash
# Executable QA procedure: qa/committed_triage_validation.md (covers
# features/committed_triage_validation.feature). Drives the running server
# through its HTTP interface only, and inspects persisted state via a
# read-only sqlite3 query -- never through a project-internal API. The
# capture endpoint's response carries no identifier, so (as in
# capture_endpoint.sh) the capture row is located by its raw text.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-committed-triage-validation"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

setup_scenario() { qa_setup_scenario "$BIN" "$1" "$TMP_DIR" "call the dentist"; }

# Runs one example row: triages as committed with a complete payload except
# that field_name is either omitted (mode=omit) or submitted as "" (mode=empty).
run_example() {
  local field_name="$1" mode="$2"
  local name="$mode-$field_name"
  setup_scenario "$name" || return

  local body
  body="$(python3 -c '
import json, sys
field, mode = sys.argv[1], sys.argv[2]
payload = {"kind": "committed", "deadline": "2026-08-20T17:00:00Z", "commitment": "at", "priority": "P1", "estimated_minutes": 180}
if mode == "omit":
    del payload[field]
else:
    payload[field] = ""
print(json.dumps(payload))
' "$field_name" "$mode")"

  qa_triage "$CAPTURE_ID" "$body"
  qa_assert_rejected_naming "$name" missing_field "$field_name"

  qa_stop_server
}

for field in deadline commitment priority estimated_minutes; do
  run_example "$field" "omit"
done
for field in deadline commitment priority estimated_minutes; do
  run_example "$field" "empty"
done

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: committed_triage_validation"
