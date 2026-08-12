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
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-committed-triage-validation"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
cleanup() { [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true; }
trap cleanup EXIT

free_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'
}

wait_ready() {
  local port="$1"
  for _ in $(seq 1 50); do
    if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then
      exec 3<&- 3>&-
      return 0
    fi
    sleep 0.1
  done
  return 1
}

# Setup shared by every example row below: fresh db, running server, one
# untriaged capture "call the dentist". Sets DB_PATH, ADDR and CAPTURE_ID.
setup_scenario() {
  local name="$1" port
  DB_PATH="$TMP_DIR/$name.sqlite"
  port="$(free_port)"
  ADDR="127.0.0.1:$port"

  "$BIN" serve --db "$DB_PATH" --addr "$ADDR" >"$TMP_DIR/$name.log" 2>&1 &
  SERVER_PID=$!

  if ! wait_ready "$port"; then
    echo "FAIL: [$name] server never became reachable at $ADDR" >&2
    cat "$TMP_DIR/$name.log" >&2
    FAILURES=1
    return 1
  fi

  local task_count
  task_count="$(sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM tasks;')"
  if [[ "$task_count" != "0" ]]; then
    echo "FAIL: [$name] expected an empty task list before triage, found $task_count" >&2
    FAILURES=1
  fi

  curl -s -o /dev/null -X POST "http://$ADDR/captures" \
    -H 'content-type: application/json' \
    -d '{"raw_text":"call the dentist","source":"web"}'

  CAPTURE_ID="$(sqlite3 "$DB_PATH" "SELECT id FROM captures WHERE raw_text = 'call the dentist';")"
  if [[ -z "$CAPTURE_ID" ]]; then
    echo "FAIL: [$name] no capture row found for \"call the dentist\"" >&2
    FAILURES=1
    return 1
  fi

  if ! untriaged_and_present "$name"; then
    return 1
  fi
}

teardown_scenario() {
  kill "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
  SERVER_PID=""
}

# The untriaged queue has no HTTP affordance of its own; a capture "waiting
# in the untriaged queue" is one whose triaged_at is still unset -- the same
# durable-state definition the accepted specification uses.
untriaged_and_present() {
  local name="$1" triaged_at
  triaged_at="$(sqlite3 "$DB_PATH" "SELECT IFNULL(triaged_at,'') FROM captures WHERE id = $CAPTURE_ID;")"
  if [[ -n "$triaged_at" ]]; then
    echo "FAIL: [$name] expected the capture to still be untriaged, triaged_at=\"$triaged_at\"" >&2
    FAILURES=1
    return 1
  fi
}

triage_response() {
  local body="$1"
  curl -s -w '\n%{http_code}' -X POST "http://$ADDR/captures/$CAPTURE_ID/triage" \
    -H 'content-type: application/json' \
    -d "$body"
}

run_example() {
  local missing_field="$1"
  local name="missing-$missing_field"
  setup_scenario "$name" || return

  local body
  body="$(python3 -c '
import json, sys
payload = {"kind": "committed", "deadline": "2026-08-20T17:00:00Z", "deadline_type": "hard", "priority": "P1"}
del payload[sys.argv[1]]
print(json.dumps(payload))
' "$missing_field")"

  local response status resp_body
  response="$(triage_response "$body")"
  status="${response##*$'\n'}"
  resp_body="${response%$'\n'*}"

  if [[ "$status" -lt 400 || "$status" -ge 500 ]]; then
    echo "FAIL: [$name] expected a client error, got status $status" >&2
    FAILURES=1
  fi

  local named
  named="$(python3 -c '
import json, sys
try:
    body = json.loads(sys.argv[1])
except ValueError:
    print("")
    sys.exit()
print(body.get("missing_field") or "")
' "$resp_body")"
  if [[ "$named" != "$missing_field" ]]; then
    echo "FAIL: [$name] expected the rejection to name \"$missing_field\", body reported missing_field=\"$named\" (body: $resp_body)" >&2
    FAILURES=1
  fi

  local task_count
  task_count="$(sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM tasks;')"
  if [[ "$task_count" != "0" ]]; then
    echo "FAIL: [$name] expected the task list to still be empty, found $task_count row(s)" >&2
    FAILURES=1
  fi

  untriaged_and_present "$name" || true

  teardown_scenario
}

run_example "deadline"
run_example "deadline_type"
run_example "priority"

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: committed_triage_validation"
