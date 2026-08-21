#!/usr/bin/env bash
# Executable QA procedure: qa/one_screen.md (covers features/one_screen.feature).
# Drives the running server through its HTTP interface only -- the removed
# paths, the inbox at GET /, and the capture endpoint -- and inspects
# persisted state via a read-only sqlite3 query, never a project-internal
# API.
#
# qa/one_screen.md's by-hand browser walkthrough is NOT scripted here, for
# the reason every earlier by-hand walkthrough in this directory was not:
# no browser-automation tooling in this environment.
#
# qa/one_screen.md's own "capture and triage are untouched" procedure is
# NOT re-implemented here; it names the twelve QA suites scripts/qa/run.sh
# already runs alongside this one in the same cycle, and restating their
# assertions in a second script is how the two copies drift.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-one-screen"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_get() {
  curl -s "http://$ADDR$1"
}

qa_status() {
  curl -s -o /dev/null -w '%{http_code}' "http://$ADDR$1"
}

# --- Procedure: every removed path is gone ---
name="every-removed-path-is-gone"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for path in /stats /life-areas /free-time /capacity /schedule; do
    status="$(qa_status "$path")"
    if [[ "$status" != "404" ]]; then
      echo "FAIL: [$name] $path returned $status, expected 404 -- not 200, not a redirect, not 500" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: no header renders ---
name="no-header-renders"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get /)"
  if [[ "$page" == *"<header"* || "$page" == *"<nav"* ]]; then
    echo "FAIL: [$name] expected no <header> or <nav> in the page, got:
$page" >&2
    FAILURES=1
  fi
  if [[ "$page" == *"<a "* ]]; then
    echo "FAIL: [$name] expected no link to another page anywhere in the page, got:
$page" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the timezone setting is a one-way door now, but survives ---
# Not one of qa/one_screen.md's own numbered procedures, but the doc's
# "Consequences to state rather than discover" section explicitly asks QA
# to confirm the stored value is still there. The life-areas page that used
# to read it back is gone, so this is sqlite3-only, per the doc's Interface
# section (the surviving /timezone endpoint itself is untested by any QA
# doc -- a deliberate scope decision the brief states, not a gap this
# script invents coverage for).
name="settings-and-task-rows-survive"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-1.log"; then
  before_tz="$(sqlite3 "$DB" "SELECT timezone FROM settings WHERE id = 1;")"
  if [[ -z "$before_tz" ]]; then
    echo "FAIL: [$name] expected the settings row to carry a timezone" >&2
    FAILURES=1
  fi
  capture_id="$(qa_submit_capture "buy milk")"
  qa_triage "$capture_id" '{"kind":"pool"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup triage returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_stop_server

  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log"; then
    after_tz="$(sqlite3 "$DB" "SELECT timezone FROM settings WHERE id = 1;")"
    if [[ "$after_tz" != "$before_tz" ]]; then
      echo "FAIL: [$name] the timezone changed across a restart with no write in between: before=$before_tz after=$after_tz" >&2
      FAILURES=1
    fi
    # R2's ratio ("#20") is recomputed on demand from `tasks` rows, which
    # this slice deliberately leaves in place even with /stats gone.
    task_count="$(sqlite3 "$DB" "SELECT COUNT(*) FROM tasks;")"
    if [[ "$task_count" != "1" ]]; then
      echo "FAIL: [$name] expected the triaged task row to survive the restart, found $task_count" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the capture latency budget, now measured here ---
name="capture-latency-budget"
load_average="$(awk '{print $1}' /proc/loadavg)"
echo "[$name] load average before starting: $load_average" >&2
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  over_budget=0
  for i in $(seq 1 10); do
    result="$(curl -s -o /dev/null -w '%{http_code} %{time_total}' \
      -X POST "http://$ADDR/captures" \
      -H 'content-type: application/json' \
      -d "$(python3 -c 'import json,sys; print(json.dumps({"raw_text": f"item {sys.argv[1]}", "source": "web"}))' "$i")")"
    status="${result%% *}"
    time_total="${result#* }"
    time_ms="$(awk -v t="$time_total" 'BEGIN { printf "%.1f", t * 1000 }')"
    if [[ "$status" != "201" ]]; then
      echo "FAIL: [$name] request $i returned status $status, expected 201" >&2
      FAILURES=1
    fi
    if (( $(awk -v t="$time_total" 'BEGIN { print (t < 0.05) ? 0 : 1 }') )); then
      echo "FAIL: [$name] request $i took ${time_ms}ms, expected under 50ms (load average was $load_average)" >&2
      over_budget=1
      FAILURES=1
    fi
  done
  if [[ "$over_budget" != "0" ]]; then
    echo "[$name] a reading taken under load is not evidence -- re-run this script alone on a quiet machine before treating an overage as a regression" >&2
  fi
  row_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM captures;")"
  if [[ "$row_count" != "10" ]]; then
    echo "FAIL: [$name] expected 10 persisted rows, found $row_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: one_screen"
