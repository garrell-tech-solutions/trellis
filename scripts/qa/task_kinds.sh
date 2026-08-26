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
# kind|deadline|commitment|priority|target_count|target_minutes_each|period,
# with NULLs rendered as the empty string.
task_row() {
  sqlite3 -separator '|' "$DB_PATH" \
    "SELECT kind, IFNULL(deadline,''), IFNULL(commitment,''), IFNULL(priority,''), \
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

# #138: target_count/target_minutes_each/period are retired and nothing
# writes them any more, so the columns being empty would pass against any
# implementation whatsoever (#90). The Quota screen offering no quotas is
# the live check.
qa_get_quota() {
  curl -s "http://$ADDR/quota"
}

qa_quota_meta() {
  qa_between "$1" '<div class="quota-meta">' '</div>'
}

assert_no_quotas() {
  local name="$1" meta
  meta="$(qa_quota_meta "$(qa_get_quota)")"
  if [[ "$meta" != "none yet" ]]; then
    echo "FAIL: [$name] expected the Quota screen to offer no quotas (meta \"none yet\"), got: $meta" >&2
    FAILURES=1
  fi
}

# --- Scenario: pool ---
if setup_scenario "pool"; then
  assert_status_created "pool" '{"kind":"pool","life_area":"Work"}'
  if assert_one_task "pool"; then
    IFS='|' read -r kind deadline _dt _prio _tc _tme _period < <(task_row)
    [[ "$kind" == "pool" ]] || { echo "FAIL: [pool] expected kind pool, got \"$kind\"" >&2; FAILURES=1; }
    [[ -z "$deadline" ]] || { echo "FAIL: [pool] expected no deadline, got \"$deadline\"" >&2; FAILURES=1; }
    assert_no_quotas "pool"
  fi
fi
qa_stop_server

# --- Scenario: committed ---
# deadline is stored as epoch milliseconds (T-jiff-epoch-millis);
# expected_deadline_ms is the instant the submitted deadline text names,
# computed the same way the triage boundary parses a submission.
run_committed_example() {
  local deadline="$1" commitment="$2" priority="$3" expected_deadline_ms="$4"
  local name="committed-$commitment"
  setup_scenario "$name" || return
  local body
  body="$(printf '{"kind":"committed","deadline":"%s","commitment":"%s","priority":"%s","estimated_minutes":180,"life_area":"Work"}' \
    "$deadline" "$commitment" "$priority")"
  assert_status_created "$name" "$body"
  if assert_one_task "$name"; then
    IFS='|' read -r kind row_deadline row_commitment row_priority _tc _tme _period < <(task_row)
    [[ "$kind" == "committed" ]] || { echo "FAIL: [$name] expected kind committed, got \"$kind\"" >&2; FAILURES=1; }
    if [[ "$row_deadline" != "$expected_deadline_ms" || "$row_commitment" != "$commitment" || "$row_priority" != "$priority" ]]; then
      echo "FAIL: [$name] expected deadline=$expected_deadline_ms commitment=$commitment priority=$priority, got deadline=$row_deadline commitment=$row_commitment priority=$row_priority" >&2
      FAILURES=1
    fi
    assert_no_quotas "$name"
  fi
  qa_stop_server
}
run_committed_example "2026-08-20T17:00:00Z" "at" "P1" "1787245200000"
run_committed_example "2026-08-31T09:00:00Z" "by" "P3" "1788166800000"

# --- Scenario: quota ---
# #138: what a quota triage carries changed completely. target_count,
# target_minutes_each and period are retired; a quota now carries a name
# and a weekly hour target, and triaging is what creates the quota.
# "Assert through the screen, not through columns" (qa/task_kinds.md) --
# where a quota is stored is the architect's to settle.
qa_quota_readout() {
  local page="$1" quota_name="$2"
  python3 -c '
import re, sys
page, name = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<div class=\"quota-row\"[^>]*>(?:(?!<div class=\"quota-row\").)*", page, re.S):
    block = m.group(0)
    nm = re.search(r"<div class=\"quota-name\">([^<]*)</div>", block)
    if nm and nm.group(1) == name:
        ro = re.search(r"<div class=\"quota-readout\">([^<]*)</div>", block)
        print(ro.group(1) if ro else "")
        sys.exit()
' "$page" "$quota_name"
}

run_quota_example() {
  local quota_name="$1" hours="$2" expected_readout="$3"
  local name="quota-${quota_name// /-}"
  setup_scenario "$name" || return
  local body
  body="$(python3 -c 'import json,sys; print(json.dumps({"kind":"quota","name":sys.argv[1],"hours":sys.argv[2]}))' "$quota_name" "$hours")"
  assert_status_created "$name" "$body"
  if assert_one_task "$name"; then
    IFS='|' read -r kind deadline _dt _prio _tc _tme _period < <(task_row)
    [[ "$kind" == "quota" ]] || { echo "FAIL: [$name] expected kind quota, got \"$kind\"" >&2; FAILURES=1; }
    [[ -z "$deadline" ]] || { echo "FAIL: [$name] expected no deadline, got \"$deadline\"" >&2; FAILURES=1; }

    local page meta readout
    page="$(qa_get_quota)"
    meta="$(qa_quota_meta "$page")"
    if [[ "$meta" != "1 quota" ]]; then
      echo "FAIL: [$name] expected meta \"1 quota\", got: $meta" >&2
      FAILURES=1
    fi
    readout="$(qa_quota_readout "$page" "$quota_name")"
    if [[ "$readout" != "$expected_readout" ]]; then
      echo "FAIL: [$name] expected the Quota screen to read \"$expected_readout\" for \"$quota_name\", got: \"$readout\"" >&2
      FAILURES=1
    fi
  fi
  qa_stop_server
}
run_quota_example "Piano" "4" "0m / 4h"
# 0.5 is the row that earns its keep: the canvas's hours input is
# step="0.5", so half an hour is a value the product can really submit, and
# it is the only row where the hours-to-minutes conversion is visible in
# the readout rather than implied by it. If this reads "0m / 0h" or is
# rejected, the conversion is integer-truncating.
run_quota_example "Running" "0.5" "0m / 30m"

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: task_kinds"
