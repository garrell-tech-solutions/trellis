#!/usr/bin/env bash
# Executable QA procedure: qa/task_kinds.md (covers features/task_kinds.feature).
# Drives the running server through its HTTP interface only, and inspects
# persisted state via a read-only sqlite3 query -- never through a project-
# internal API. The capture endpoint's response carries no identifier, so
# (as in capture_endpoint.sh) the capture row is located by its raw text.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-task-kinds"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

setup_scenario() { qa_setup_scenario "$BIN" "$1" "$TMP_DIR" "buy milk"; }

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
  count="$(qa_task_count)"
  if [[ "$count" != "1" ]]; then
    echo "FAIL: [$name] expected exactly one task, found $count" >&2
    FAILURES=1
    return 1
  fi
}

assert_status_created() {
  local name="$1" body="$2"
  qa_triage "$CAPTURE_ID" "$body"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triage returned status $STATUS, expected 201" >&2
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
qa_stop_server

# --- Scenario: committed ---
# deadline is stored as epoch milliseconds (T-jiff-epoch-millis);
# expected_deadline_ms is the instant the submitted deadline text names,
# computed the same way the triage boundary parses a submission.
run_committed_example() {
  local deadline="$1" deadline_type="$2" priority="$3" expected_deadline_ms="$4"
  local name="committed-$deadline_type"
  setup_scenario "$name" || return
  local body
  body="$(printf '{"kind":"committed","deadline":"%s","deadline_type":"%s","priority":"%s"}' \
    "$deadline" "$deadline_type" "$priority")"
  assert_status_created "$name" "$body"
  if assert_one_task "$name"; then
    IFS='|' read -r kind row_deadline row_deadline_type row_priority target_count target_minutes_each period < <(task_row)
    [[ "$kind" == "committed" ]] || { echo "FAIL: [$name] expected kind committed, got \"$kind\"" >&2; FAILURES=1; }
    if [[ "$row_deadline" != "$expected_deadline_ms" || "$row_deadline_type" != "$deadline_type" || "$row_priority" != "$priority" ]]; then
      echo "FAIL: [$name] expected deadline=$expected_deadline_ms deadline_type=$deadline_type priority=$priority, got deadline=$row_deadline deadline_type=$row_deadline_type priority=$row_priority" >&2
      FAILURES=1
    fi
    [[ -z "$target_count$target_minutes_each$period" ]] || {
      echo "FAIL: [$name] expected no quota target, got target_count=\"$target_count\" target_minutes_each=\"$target_minutes_each\" period=\"$period\"" >&2
      FAILURES=1
    }
  fi
  qa_stop_server
}
run_committed_example "2026-08-20T17:00:00Z" "hard" "P1" "1787245200000"
run_committed_example "2026-08-31T09:00:00Z" "soft" "P3" "1788166800000"

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
  qa_stop_server
}
run_quota_example "3" "45"
run_quota_example "1" "90"

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: task_kinds"
