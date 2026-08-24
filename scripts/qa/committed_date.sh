#!/usr/bin/env bash
# Executable QA procedure: qa/committed_date.md (covers
# features/committed_date.feature). Drives the running server through its
# HTTP interface only -- the capture screen's committed triage controls
# (endpoint read from the page's own markup, not assumed) and the
# Committed screen -- and inspects nothing beyond what the page renders and
# a read-only sqlite3 query.
#
# THERE IS NO WAY TO SET THE TIMEZONE FROM THE RUNNING APP, per the doc's
# own "Interface used" section: T-timezone-is-a-setting stored it, #88 took
# the page it was edited on, and #85 has not yet brought a replacement.
# Every procedure that needs a non-UTC zone writes `settings.timezone`
# directly via sqlite3 -- reaching past the interface, which is itself the
# finding qa/committed_date.md asks to be reported rather than hidden.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-committed-date"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_get_inbox() {
  curl -s "http://$ADDR/"
}

qa_get_committed() {
  curl -s "http://$ADDR/committed"
}

qa_committed_date_cell_for() {
  qa_between "$(qa_committed_row_for "$1" "$2")" '<div class="committed-date">' '</div>'
}

qa_set_timezone() {
  sqlite3 "$DB_PATH" "UPDATE settings SET timezone = '$1' WHERE id = 1;"
}

# Triages capture_id as committed through the page's own "At a time" or
# "By a day" disclosure (never a raw deadline= field, which the page no
# longer sends) and sets STATUS/BODY. #119 moved committed's fields behind
# a kind button: the fields-form doesn't exist in the row's markup until
# POST .../kind (read from the row's own kind-choice button) has opened
# the panel, so this is a two-step dance now -- open, then find and submit
# to the fields-form's own endpoint from what comes back.
qa_committed_via_page() {
  local capture_id="$1" commitment="$2" date="$3" time="${4:-}" kind_endpoint endpoint data
  kind_endpoint="$(qa_block_control_endpoint "$(qa_capture_row_block "$(qa_get_inbox)" "$capture_id")" 'value="committed"')"
  qa_triage_form "$kind_endpoint" "kind=committed"
  endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$capture_id")" committed)"
  data="kind=committed&deadline_date=$date&commitment=$commitment&priority=P1&estimated_minutes=30"
  if [[ -n "$time" ]]; then
    data="$data&deadline_time=$time"
  fi
  qa_triage_form "$endpoint" "$data"
}

qa_committed_via_api() {
  local capture_id="$1" commitment="$2" deadline="$3"
  qa_triage "$capture_id" "$(python3 -c 'import json,sys; print(json.dumps({"kind":"committed","deadline":sys.argv[2],"commitment":sys.argv[1],"priority":"P1","estimated_minutes":30}))' "$commitment" "$deadline")"
}

# --- Procedure: an at, and a by ---
name="an-at-and-a-by"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  at_id="$(qa_submit_capture "Book the dentist")"
  qa_committed_via_page "$at_id" at "2026-08-25" "08:30"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the at returned status $STATUS" >&2
    FAILURES=1
  fi
  by_id="$(qa_submit_capture "File the tax return")"
  qa_committed_via_page "$by_id" by "2026-08-27"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the by returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_committed)"
  at_cell="$(qa_committed_date_cell_for "$page" "Book the dentist")"
  if [[ "$at_cell" != "TUE 8:30" ]]; then
    echo "FAIL: [$name] expected the at cell \"TUE 8:30\", got: $at_cell" >&2
    FAILURES=1
  fi
  by_cell="$(qa_committed_date_cell_for "$page" "File the tax return")"
  if [[ "$by_cell" != "BY THU" ]]; then
    echo "FAIL: [$name] expected the by cell \"BY THU\", got: $by_cell" >&2
    FAILURES=1
  fi

  # "By Thursday" means before Thursday is over -- the end of that day in
  # UTC, the fresh database's default zone. 2026-08-27 end-of-day UTC.
  by_deadline="$(sqlite3 "$DB_PATH" "SELECT tasks.deadline FROM tasks JOIN captures ON captures.id = tasks.capture_id WHERE captures.raw_text = 'File the tax return';")"
  if [[ "$by_deadline" != "1787875199999" ]]; then
    echo "FAIL: [$name] expected the by's stored deadline at end-of-day UTC (1787875199999), got: $by_deadline" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the timezone decides the day ---
name="the-timezone-decides-the-day"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_set_timezone "America/New_York"

  late_id="$(qa_submit_capture "Book the dentist late")"
  qa_committed_via_page "$late_id" at "2026-08-25" "23:30"
  early_id="$(qa_submit_capture "Book the dentist early")"
  qa_committed_via_page "$early_id" at "2026-08-25" "00:30"

  page="$(qa_get_committed)"
  late_cell="$(qa_committed_date_cell_for "$page" "Book the dentist late")"
  if [[ "$late_cell" != "TUE 23:30" ]]; then
    echo "FAIL: [$name] expected 23:30 on the chosen day to read \"TUE 23:30\", got: $late_cell" >&2
    FAILURES=1
  fi
  early_cell="$(qa_committed_date_cell_for "$page" "Book the dentist early")"
  if [[ "$early_cell" != "TUE 0:30" ]]; then
    echo "FAIL: [$name] expected 00:30 on the chosen day to read \"TUE 0:30\", got: $early_cell" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

name="a-second-zone-far-from-the-first"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_set_timezone "Pacific/Auckland"

  cid="$(qa_submit_capture "Book the dentist")"
  qa_committed_via_page "$cid" at "2026-08-25" "23:30"
  cell="$(qa_committed_date_cell_for "$(qa_get_committed)" "Book the dentist")"
  if [[ "$cell" != "TUE 23:30" ]]; then
    echo "FAIL: [$name] expected the rendered day to follow the zone setting (Pacific/Auckland), got: $cell" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the JSON transport is unchanged ---
name="the-json-transport-is-unchanged"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  at_id="$(qa_submit_capture "Book the dentist")"
  qa_committed_via_api "$at_id" at "2026-08-25T08:30:00Z"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a JSON at with an RFC 3339 instant returned status $STATUS" >&2
    FAILURES=1
  fi

  by_id="$(qa_submit_capture "File the tax return")"
  qa_committed_via_api "$by_id" by "2026-08-27T17:00:00Z"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a JSON by carrying a time returned status $STATUS -- the page stops asking, the API must not stop accepting" >&2
    FAILURES=1
  fi

  by_cell="$(qa_committed_date_cell_for "$(qa_get_committed)" "File the tax return")"
  if [[ "$by_cell" != "BY THU" ]]; then
    echo "FAIL: [$name] expected the by (with a time, over JSON) to still render as a by, got: $by_cell" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: rejections still name the field ---
name="rejections-still-name-the-field"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  # API, deadline omitted.
  CAPTURE_ID="$(qa_submit_capture "call the dentist 1")"
  qa_triage "$CAPTURE_ID" '{"kind":"committed","commitment":"by","priority":"P1","estimated_minutes":30}'
  qa_assert_rejected_naming "$name-api-deadline" missing_field deadline

  # API, commitment omitted.
  CAPTURE_ID="$(qa_submit_capture "call the dentist 2")"
  qa_triage "$CAPTURE_ID" '{"kind":"committed","deadline":"2026-08-27T17:00:00Z","priority":"P1","estimated_minutes":30}'
  qa_assert_rejected_naming "$name-api-commitment" missing_field commitment

  # Page, deadline_date omitted -- still reported as "deadline", the core
  # field name, not the page's own field name.
  cid="$(qa_submit_capture "call the dentist 3")"
  kind_endpoint="$(qa_block_control_endpoint "$(qa_capture_row_block "$(qa_get_inbox)" "$cid")" 'value="committed"')"
  qa_triage_form "$kind_endpoint" "kind=committed"
  endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$cid")" committed)"
  qa_triage_form "$endpoint" "kind=committed&commitment=by&priority=P1&estimated_minutes=30"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name-page-deadline] expected 422, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"deadline is required"* ]]; then
    echo "FAIL: [$name-page-deadline] expected the rejection to name deadline, got:
$BODY" >&2
    FAILURES=1
  fi

  # Page, commitment omitted.
  cid="$(qa_submit_capture "call the dentist 4")"
  kind_endpoint="$(qa_block_control_endpoint "$(qa_capture_row_block "$(qa_get_inbox)" "$cid")" 'value="committed"')"
  qa_triage_form "$kind_endpoint" "kind=committed"
  endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$cid")" committed)"
  qa_triage_form "$endpoint" "kind=committed&deadline_date=2026-08-27&priority=P1&estimated_minutes=30"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name-page-commitment] expected 422, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"commitment is required"* ]]; then
    echo "FAIL: [$name-page-commitment] expected the rejection to name commitment, got:
$BODY" >&2
    FAILURES=1
  fi

  task_count="$(qa_task_count)"
  if [[ "$task_count" != "0" ]]; then
    echo "FAIL: [$name] expected no tasks created across all four rejections, found $task_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the cell says which day, beyond this week ---
name="the-cell-says-which-day-beyond-this-week"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "2026-08-24T09:00:00Z"; then
  near_id="$(qa_submit_capture "Renew the passport soon")"
  qa_committed_via_page "$near_id" by "2026-08-27"
  far_id="$(qa_submit_capture "Renew the passport later")"
  qa_committed_via_page "$far_id" by "2026-09-17"

  page="$(qa_get_committed)"
  near_cell="$(qa_committed_date_cell_for "$page" "Renew the passport soon")"
  if [[ "$near_cell" != "BY THU" ]]; then
    echo "FAIL: [$name] expected a by 3 days out to read \"BY THU\", got: $near_cell" >&2
    FAILURES=1
  fi
  far_cell="$(qa_committed_date_cell_for "$page" "Renew the passport later")"
  if [[ "$far_cell" != "BY 17 SEP" ]]; then
    echo "FAIL: [$name] expected a by 3 weeks out to read \"BY 17 SEP\" (BY THU cannot tell two Thursdays apart), got: $far_cell" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: committed_date"
