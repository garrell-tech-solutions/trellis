#!/usr/bin/env bash
# Executable QA procedure: qa/task_kinds.md (covers features/task_kinds.feature).
# Drives the running server through its HTTP interface only, and inspects
# persisted state via a read-only sqlite3 query -- never through a project-
# internal API. The capture endpoint's response carries no identifier, so
# (as in capture_endpoint.sh) the capture row is located by its raw text.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-task-kinds"
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

# Setup shared by every scenario below: fresh db, running server, one
# untriaged capture "buy milk". Sets DB_PATH, ADDR and CAPTURE_ID.
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
    -d '{"raw_text":"buy milk","source":"web"}'

  CAPTURE_ID="$(sqlite3 "$DB_PATH" "SELECT id FROM captures WHERE raw_text = 'buy milk';")"
  if [[ -z "$CAPTURE_ID" ]]; then
    echo "FAIL: [$name] no capture row found for \"buy milk\"" >&2
    FAILURES=1
    return 1
  fi
}

teardown_scenario() {
  kill "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
  SERVER_PID=""
}

triage_status() {
  local body="$1" result
  result="$(curl -s -o /dev/null -w '%{http_code}' -X POST "http://$ADDR/captures/$CAPTURE_ID/triage" \
    -H 'content-type: application/json' \
    -d "$body")"
  echo "$result"
}

# Prints the resulting task row for CAPTURE_ID as
# kind|deadline|deadline_type|priority|target_count|target_minutes_each|period,
# with NULLs rendered as the empty string.
task_row() {
  sqlite3 -separator '|' "$DB_PATH" \
    "SELECT kind, IFNULL(deadline,''), IFNULL(deadline_type,''), IFNULL(priority,''), \
            IFNULL(target_count,''), IFNULL(target_minutes_each,''), IFNULL(period,'') \
     FROM tasks WHERE capture_id = $CAPTURE_ID;"
}

assert_one_task() {
  local name="$1" count
  count="$(sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM tasks;')"
  if [[ "$count" != "1" ]]; then
    echo "FAIL: [$name] expected exactly one task, found $count" >&2
    FAILURES=1
    return 1
  fi
}

assert_status_created() {
  local name="$1" body="$2" status
  status="$(triage_status "$body")"
  if [[ "$status" != "201" ]]; then
    echo "FAIL: [$name] triage returned status $status, expected 201" >&2
    FAILURES=1
  fi
}

# --- Scenario: pool ---
if setup_scenario "pool"; then
  assert_status_created "pool" '{"kind":"pool"}'
  if assert_one_task "pool"; then
    IFS='|' read -r kind deadline _dt _prio target_count target_minutes_each period < <(task_row)
    [[ "$kind" == "pool" ]] || { echo "FAIL: [pool] expected kind pool, got \"$kind\"" >&2; FAILURES=1; }
    [[ -z "$deadline" ]] || { echo "FAIL: [pool] expected no deadline, got \"$deadline\"" >&2; FAILURES=1; }
    [[ -z "$target_count$target_minutes_each$period" ]] || {
      echo "FAIL: [pool] expected no quota target, got target_count=\"$target_count\" target_minutes_each=\"$target_minutes_each\" period=\"$period\"" >&2
      FAILURES=1
    }
  fi
fi
teardown_scenario

# --- Scenario: committed ---
run_committed_example() {
  local deadline="$1" deadline_type="$2" priority="$3"
  local name="committed-$deadline_type"
  setup_scenario "$name" || return
  local body
  body="$(printf '{"kind":"committed","deadline":"%s","deadline_type":"%s","priority":"%s"}' \
    "$deadline" "$deadline_type" "$priority")"
  assert_status_created "$name" "$body"
  if assert_one_task "$name"; then
    IFS='|' read -r kind row_deadline row_deadline_type row_priority target_count target_minutes_each period < <(task_row)
    [[ "$kind" == "committed" ]] || { echo "FAIL: [$name] expected kind committed, got \"$kind\"" >&2; FAILURES=1; }
    if [[ "$row_deadline" != "$deadline" || "$row_deadline_type" != "$deadline_type" || "$row_priority" != "$priority" ]]; then
      echo "FAIL: [$name] expected deadline=$deadline deadline_type=$deadline_type priority=$priority, got deadline=$row_deadline deadline_type=$row_deadline_type priority=$row_priority" >&2
      FAILURES=1
    fi
    [[ -z "$target_count$target_minutes_each$period" ]] || {
      echo "FAIL: [$name] expected no quota target, got target_count=\"$target_count\" target_minutes_each=\"$target_minutes_each\" period=\"$period\"" >&2
      FAILURES=1
    }
  fi
  teardown_scenario
}
run_committed_example "2026-08-20T17:00:00Z" "hard" "P1"
run_committed_example "2026-08-31T09:00:00Z" "soft" "P3"

# --- Scenario: quota ---
run_quota_example() {
  local target_count="$1" target_minutes_each="$2"
  local name="quota-${target_count}x${target_minutes_each}"
  setup_scenario "$name" || return
  local body
  body="$(printf '{"kind":"quota","target_count":%s,"target_minutes_each":%s,"period":"week"}' \
    "$target_count" "$target_minutes_each")"
  assert_status_created "$name" "$body"
  if assert_one_task "$name"; then
    IFS='|' read -r kind deadline _dt _prio row_target_count row_target_minutes_each row_period < <(task_row)
    [[ "$kind" == "quota" ]] || { echo "FAIL: [$name] expected kind quota, got \"$kind\"" >&2; FAILURES=1; }
    if [[ "$row_target_count" != "$target_count" || "$row_target_minutes_each" != "$target_minutes_each" || "$row_period" != "week" ]]; then
      echo "FAIL: [$name] expected target_count=$target_count target_minutes_each=$target_minutes_each period=week, got target_count=$row_target_count target_minutes_each=$row_target_minutes_each period=$row_period" >&2
      FAILURES=1
    fi
    [[ -z "$deadline" ]] || { echo "FAIL: [$name] expected no deadline, got \"$deadline\"" >&2; FAILURES=1; }
  fi
  teardown_scenario
}
run_quota_example "3" "45"
run_quota_example "1" "90"

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: task_kinds"
