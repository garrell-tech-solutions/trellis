#!/usr/bin/env bash
# Executable QA procedure: qa/quota_sessions.md (covers
# features/quota_sessions.feature, and the two halves no acceptance
# scenario can hold -- the expand/Other... disclosures via
# scripts/qa/quota_sessions.cjs, and a real week boundary via --now).
#
# T-qa-binds-tolerantly-to-markup: every extraction binds to a class or id
# this template owns (quota-row, quota-session, quota-session-label,
# quota-sessions-summary, quota-sessions-empty, quota-quick-log-30m,
# quota-quick-log-1h), never to colour, font or attribute order.
#
# Logs every session through the routes (POST /quota/{id}/sessions,
# POST /quota/sessions/{id}, POST /quota/sessions/{id}/delete), never by
# inserting rows: qa/quota_sessions.md's own warning that a row inserted
# with sqlite3 proves nothing about +30m.
#
# --now is a user-interface affordance (platform/clock.rs), not a test
# hook: the server starts believing it is that instant and time advances
# normally from there. Dates below match features/quota_sessions.feature's
# own Examples for -03 (2026-08-24 is a Monday) so this script's fixtures
# and the accepted specification agree on what day of the week each date
# is.
#
# MUST FAIL, NEVER SKIP, if Chrome or playwright-core is unavailable --
# the same rule qa/trip_controls.md and qa/phone_layout.md already carry.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-quota-sessions"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

MONDAY="2026-08-24T14:00:00Z"
TUESDAY="2026-08-25T14:00:00Z"
FRIDAY="2026-08-28T14:00:00Z"
SUNDAY="2026-08-30T14:00:00Z"
NEXT_MONDAY="2026-08-31T14:00:00Z"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

if ! command -v node >/dev/null 2>&1; then
  echo "FAIL: node is not on PATH -- failing closed rather than skipping the quota-sessions browser check" >&2
  exit 1
fi

NODE_MODULES_DIR="$(npm root -g 2>/dev/null || true)"
if [[ -z "$NODE_MODULES_DIR" || ! -d "$NODE_MODULES_DIR/playwright-core" ]]; then
  echo "FAIL: playwright-core is not installed globally (npm install -g playwright-core) -- failing closed rather than skipping the quota-sessions browser check" >&2
  exit 1
fi

qa_get_quota() {
  curl -s "http://$ADDR/quota"
}

# #138 retired the Quota screen's own define form (POST /quota); a quota is
# created only by triaging a capture as kind=quota with a name and an hour
# target now. Submitting raw_text=name keeps this fixture's captures
# distinguishable in the inbox if a scenario ever needs to look, though
# nothing here does.
qa_define_quota() {
  local name="$1" hours="$2" capture_id body
  capture_id="$(qa_submit_capture "$name")"
  body="$(python3 -c 'import json,sys; print(json.dumps({"kind":"quota","name":sys.argv[1],"hours":sys.argv[2]}))' "$name" "$hours")"
  qa_triage "$capture_id" "$body"
}

qa_seed_quota() {
  local name="$1" hours="$2"
  qa_define_quota "$name" "$hours"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: setup -- defining \"$name\" at $hours hours returned status $STATUS:
$BODY" >&2
    FAILURES=1
  fi
}

qa_quota_id() {
  sqlite3 "$DB_PATH" "SELECT id FROM quotas WHERE name = '$1';"
}

qa_log_session() {
  local quota_id="$1" day="$2" minutes="$3" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/quota/$quota_id/sessions" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "day=$day&minutes=$minutes")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_correct_session() {
  local session_id="$1" day="$2" minutes="$3" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/quota/sessions/$session_id" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "day=$day&minutes=$minutes")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_delete_session() {
  local session_id="$1" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/quota/sessions/$session_id/delete")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_quota_row_block() {
  local page="$1" name="$2"
  python3 -c '
import re, sys
page, name = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<div class=\"quota-row\"[^>]*>(?:(?!<div class=\"quota-row\").)*", page, re.S):
    block = m.group(0)
    nm = re.search(r"<div class=\"quota-name\">([^<]*)</div>", block)
    if nm and nm.group(1) == name:
        print(block)
        sys.exit()
' "$page" "$name"
}

qa_row_readout() {
  qa_between "$1" '<div class="quota-readout">' '</div>'
}

qa_row_note() {
  qa_between "$1" '<div class="quota-note">' '</div>'
}

qa_session_labels_in_order() {
  python3 -c '
import re, sys
for m in re.finditer(r"<span class=\"quota-session-label\">([^<]*)</span>", sys.argv[1]):
    print(m.group(1))
' "$1"
}

qa_session_summary() {
  qa_between "$1" '<div class="quota-sessions-summary">' '</div>'
}

qa_sessions_empty_message() {
  qa_between "$1" '<p class="quota-sessions-empty">' '</p>'
}

qa_day_options() {
  python3 -c '
import re, sys
block = sys.argv[1]
select_match = re.search(r"<select name=\"day\">(.*?)</select>", block, re.S)
if not select_match:
    sys.exit()
for m in re.finditer(r"<option value=\"([^\"]*)\"", select_match.group(1)):
    print(m.group(1))
' "$1"
}

qa_session_id_for_label() {
  local row="$1" label="$2"
  python3 -c '
import re, sys
row, label = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<li class=\"quota-session\" id=\"quota-session-(\d+)\">(?:(?!</li>).)*?<span class=\"quota-session-label\">([^<]*)</span>", row, re.S):
    if m.group(2) == label:
        print(m.group(1))
        sys.exit()
' "$row" "$label"
}

# --- Procedure: the logging loop ---
name="the-logging-loop"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$TUESDAY"; then
  qa_seed_quota "Piano" "4"
  quota_id="$(qa_quota_id "Piano")"

  qa_log_session "$quota_id" "Tue" "60"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] +1h returned status $STATUS: $BODY" >&2
    FAILURES=1
  fi
  page="$(qa_get_quota)"
  row="$(qa_quota_row_block "$page" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "1h / 4h" ]]; then
    echo "FAIL: [$name] expected \"1h / 4h\" after +1h, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi
  if [[ "$(qa_row_note "$row")" != "3h left this week · 25%" ]]; then
    echo "FAIL: [$name] expected \"3h left this week · 25%\", got: $(qa_row_note "$row")" >&2
    FAILURES=1
  fi

  qa_log_session "$quota_id" "Tue" "30"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] +30m returned status $STATUS: $BODY" >&2
    FAILURES=1
  fi
  row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "1h 30m / 4h" ]]; then
    echo "FAIL: [$name] expected \"1h 30m / 4h\" after +30m, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi

  qa_log_session "$quota_id" "Mon" "20"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] Other-logged Monday session returned status $STATUS: $BODY" >&2
    FAILURES=1
  fi
  page="$(qa_get_quota)"
  row="$(qa_quota_row_block "$page" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "1h 50m / 4h" ]]; then
    echo "FAIL: [$name] expected \"1h 50m / 4h\", got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi

  labels="$(qa_session_labels_in_order "$row")"
  expected=$'Mon 20m\nTue 60m\nTue 30m'
  if [[ "$labels" != "$expected" ]]; then
    echo "FAIL: [$name] expected sessions ordered Mon 20m, Tue 60m, Tue 30m, got: $labels" >&2
    FAILURES=1
  fi
  if [[ "$(qa_session_summary "$row")" != "3 sessions · 1h 50m" ]]; then
    echo "FAIL: [$name] expected summary \"3 sessions · 1h 50m\", got: $(qa_session_summary "$row")" >&2
    FAILURES=1
  fi

  # Corroborate in sqlite3: three rows, each carrying enough to know which
  # day of which week it belongs to -- day_ms, not a bare weekday name.
  row_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM quota_sessions WHERE quota_id = $quota_id;")"
  if [[ "$row_count" != "3" ]]; then
    echo "FAIL: [$name] expected 3 session rows in sqlite3, found $row_count" >&2
    FAILURES=1
  fi
  day_ms_type="$(sqlite3 "$DB_PATH" "SELECT typeof(day_ms) FROM quota_sessions LIMIT 1;")"
  if [[ "$day_ms_type" != "integer" ]]; then
    echo "FAIL: [$name] expected quota_sessions.day_ms to be an integer instant, got typeof=$day_ms_type -- a row storing only a weekday name cannot survive a week boundary" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the day picker shrinks to the week so far ---
for pair in "$MONDAY|Mon" "$TUESDAY|Mon,Tue" "$FRIDAY|Mon,Tue,Wed,Thu,Fri" "$SUNDAY|Mon,Tue,Wed,Thu,Fri,Sat,Sun"; do
  now="${pair%%|*}"
  expected_days="${pair##*|}"
  name="day-picker-at-$now"
  if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$now"; then
    qa_seed_quota "Piano" "4"
    row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
    days="$(qa_day_options "$row" | paste -sd, -)"
    if [[ "$days" != "$expected_days" ]]; then
      echo "FAIL: [$name] expected day options \"$expected_days\", got: \"$days\"" >&2
      FAILURES=1
    fi
    today="$(qa_day_options "$row" | tail -1)"
    selected="$(python3 -c '
import re, sys
m = re.search(r"<option value=\"([^\"]*)\" selected>", sys.argv[1])
print(m.group(1) if m else "")
' "$row")"
    if [[ "$selected" != "$today" ]]; then
      echo "FAIL: [$name] expected today (\"$today\") pre-selected in the day picker, got: \"$selected\"" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
  qa_stop_server
done

# A guard that only exists in the dropdown is not a guard: post a future
# day directly from a server that believes it is Tuesday.
name="future-day-refused-server-side"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$TUESDAY"; then
  qa_seed_quota "Piano" "4"
  quota_id="$(qa_quota_id "Piano")"
  qa_log_session "$quota_id" "Sat" "30"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] expected 422 posting a Saturday session from a server that believes it is Tuesday, got $STATUS" >&2
    FAILURES=1
  fi
  row_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM quota_sessions;")"
  if [[ "$row_count" != "0" ]]; then
    echo "FAIL: [$name] expected no session row created, found $row_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# A session of no minutes is not a session -- quota-sessions-a-session-
# must-be-positive-09.
name="a-session-must-be-positive"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$TUESDAY"; then
  qa_seed_quota "Piano" "4"
  quota_id="$(qa_quota_id "Piano")"
  for bad_minutes in 0 -5; do
    qa_log_session "$quota_id" "Tue" "$bad_minutes"
    if [[ "$STATUS" != "422" ]]; then
      echo "FAIL: [$name] expected 422 logging $bad_minutes minutes, got $STATUS" >&2
      FAILURES=1
    fi
  done
  row_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM quota_sessions;")"
  if [[ "$row_count" != "0" ]]; then
    echo "FAIL: [$name] expected no session row created, found $row_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: correcting and deleting ---
name="correcting-and-deleting"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$TUESDAY"; then
  qa_seed_quota "Piano" "4"
  quota_id="$(qa_quota_id "Piano")"
  qa_log_session "$quota_id" "Mon" "25"
  qa_log_session "$quota_id" "Tue" "35"

  row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "1h / 4h" ]]; then
    echo "FAIL: [$name] expected \"1h / 4h\" after seeding, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi
  monday_id="$(qa_session_id_for_label "$row" "Mon 25m")"
  if [[ -z "$monday_id" ]]; then
    echo "FAIL: [$name] could not find the Monday session's id in the rendered row" >&2
    FAILURES=1
  fi

  # Change the Monday session to Tuesday (day only).
  qa_correct_session "$monday_id" "Tue" "25"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] correcting the day returned status $STATUS: $BODY" >&2
    FAILURES=1
  fi
  row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "1h / 4h" ]]; then
    echo "FAIL: [$name] expected \"1h / 4h\" unchanged after a day-only correction, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi

  # Change it to 45 minutes.
  qa_correct_session "$monday_id" "Tue" "45"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] correcting the minutes returned status $STATUS: $BODY" >&2
    FAILURES=1
  fi
  row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "1h 20m / 4h" ]]; then
    echo "FAIL: [$name] expected \"1h 20m / 4h\" after correcting to 45 minutes, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi

  # Delete it.
  qa_delete_session "$monday_id"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] deleting returned status $STATUS: $BODY" >&2
    FAILURES=1
  fi
  page="$(qa_get_quota)"
  row="$(qa_quota_row_block "$page" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "35m / 4h" ]]; then
    echo "FAIL: [$name] expected \"35m / 4h\" after deleting, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi
  last_id="$(qa_session_id_for_label "$row" "Tue 35m")"
  if [[ -z "$last_id" ]]; then
    echo "FAIL: [$name] could not find the remaining Tuesday session's id" >&2
    FAILURES=1
  fi

  # Delete the last one -- the quota itself must survive.
  qa_delete_session "$last_id"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] deleting the last session returned status $STATUS: $BODY" >&2
    FAILURES=1
  fi
  page="$(qa_get_quota)"
  row="$(qa_quota_row_block "$page" "Piano")"
  if [[ -z "$row" ]]; then
    echo "FAIL: [$name] expected the quota itself to still be there after deleting every session, got:
$page" >&2
    FAILURES=1
  fi
  if [[ "$(qa_row_readout "$row")" != "0m / 4h" ]]; then
    echo "FAIL: [$name] expected \"0m / 4h\" with nothing logged, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi
  if [[ "$(qa_sessions_empty_message "$row")" != "No sessions yet this week. Log one above when you have done it." ]]; then
    echo "FAIL: [$name] expected the empty-week message, got: $(qa_sessions_empty_message "$row")" >&2
    FAILURES=1
  fi
  if [[ "$(qa_session_summary "$row")" != "nothing logged" ]]; then
    echo "FAIL: [$name] expected summary \"nothing logged\", got: $(qa_session_summary "$row")" >&2
    FAILURES=1
  fi

  # T-set-operations-execute-in-the-store, as a proxy: log and delete
  # twenty and confirm nothing suggests a second write path or an N-row
  # round trip that should have been one statement.
  for i in $(seq 1 20); do
    qa_log_session "$quota_id" "Tue" "5"
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] logging session $i of 20 returned status $STATUS" >&2
      FAILURES=1
    fi
  done
  ids="$(sqlite3 "$DB_PATH" "SELECT id FROM quota_sessions WHERE quota_id = $quota_id ORDER BY id;")"
  start_ns="$(date +%s%N)"
  for id in $ids; do
    qa_delete_session "$id"
  done
  end_ns="$(date +%s%N)"
  ms="$(( (end_ns - start_ns) / 1000000 ))"
  echo "[$name] deleting 20 sessions one at a time took ${ms}ms" >&2
  row_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM quota_sessions WHERE quota_id = $quota_id;")"
  if [[ "$row_count" != "0" ]]; then
    echo "FAIL: [$name] expected zero session rows left after deleting all twenty, found $row_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: Monday starts again at zero ---
name="monday-starts-again-at-zero"
DB_FOR_RESTART="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB_FOR_RESTART" "$TMP_DIR/$name.log" "$TUESDAY"; then
  qa_seed_quota "Piano" "4"
  quota_id="$(qa_quota_id "Piano")"
  qa_log_session "$quota_id" "Mon" "25"
  qa_log_session "$quota_id" "Tue" "35"

  row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "1h / 4h" ]]; then
    echo "FAIL: [$name] expected \"1h / 4h\" before the week rolls over, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi
  last_week_session_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM quota_sessions;")"
else
  FAILURES=1
fi
qa_stop_server

if qa_start_server "$BIN" "$DB_FOR_RESTART" "$TMP_DIR/$name-restarted.log" "$NEXT_MONDAY"; then
  page="$(qa_get_quota)"
  row="$(qa_quota_row_block "$page" "Piano")"
  if [[ -z "$row" ]]; then
    echo "FAIL: [$name] expected Piano to still exist after the week rolled over, got:
$page" >&2
    FAILURES=1
  fi
  if [[ "$(qa_row_readout "$row")" != "0m / 4h" ]]; then
    echo "FAIL: [$name] expected \"0m / 4h\" on the new week, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi
  if [[ "$(qa_row_note "$row")" != "4h left this week · 0%" ]]; then
    echo "FAIL: [$name] expected \"4h left this week · 0%\" on the new week, got: $(qa_row_note "$row")" >&2
    FAILURES=1
  fi
  if [[ "$(qa_sessions_empty_message "$row")" != "No sessions yet this week. Log one above when you have done it." ]]; then
    echo "FAIL: [$name] expected the empty-week message on the new week, got: $(qa_sessions_empty_message "$row")" >&2
    FAILURES=1
  fi

  # Last week's sessions must still be in the database -- the counter
  # does not carry, but the reset must not have deleted rows.
  session_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM quota_sessions;")"
  if [[ "$session_count" != "$last_week_session_count" ]]; then
    echo "FAIL: [$name] expected last week's $last_week_session_count session row(s) to survive the reset, found $session_count -- a Monday reset must not delete history (D-kill-means-archive's concern)" >&2
    FAILURES=1
  fi

  # Log 30m for the new Monday; the new week counts normally.
  quota_id="$(qa_quota_id "Piano")"
  qa_log_session "$quota_id" "Mon" "30"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] logging on the new Monday returned status $STATUS: $BODY" >&2
    FAILURES=1
  fi
  row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "30m / 4h" ]]; then
    echo "FAIL: [$name] expected \"30m / 4h\" on the new week, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi

  # Try to log last week's Sunday session on this Monday -- the picker
  # must not offer last week, and the total must not move.
  days="$(qa_day_options "$row")"
  if [[ "$days" != "Mon" ]]; then
    echo "FAIL: [$name] expected the picker to offer only \"Mon\" on the new week, got: $days" >&2
    FAILURES=1
  fi
  qa_log_session "$quota_id" "Sun" "20"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] expected 422 posting last week's Sunday directly on the new Monday, got $STATUS" >&2
    FAILURES=1
  fi
  row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
  if [[ "$(qa_row_readout "$row")" != "30m / 4h" ]]; then
    echo "FAIL: [$name] expected the total unchanged (\"30m / 4h\") after the refused Sunday post, got: $(qa_row_readout "$row")" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the disclosures, in a browser ---
name="the-disclosures-in-a-browser"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$TUESDAY"; then
  qa_seed_quota "Piano" "4"
  qa_seed_quota "Running" "3"
  qa_seed_quota "Rust" "5"
  qa_log_session "$(qa_quota_id "Piano")" "Tue" "30"
  qa_log_session "$(qa_quota_id "Running")" "Tue" "20"
  qa_log_session "$(qa_quota_id "Rust")" "Tue" "45"

  if ! NODE_PATH="$NODE_MODULES_DIR" node "$SCRIPT_DIR/quota_sessions.cjs" "http://$ADDR"; then
    FAILURES=1
  fi

  # Check the schema: no column holds which row is expanded or whether
  # Other... is open (T-ephemeral-view-state-rides-the-request); a logged
  # session is the opposite and does earn its own column (day_ms).
  quota_columns="$(sqlite3 "$DB_PATH" "PRAGMA table_info(quotas);" | cut -d'|' -f2)"
  if echo "$quota_columns" | grep -qi "expand\|open"; then
    echo "FAIL: [$name] found a column on quotas naming expand/open state, which must ride the request instead:
$quota_columns" >&2
    FAILURES=1
  fi
  session_columns="$(sqlite3 "$DB_PATH" "PRAGMA table_info(quota_sessions);" | cut -d'|' -f2)"
  if ! echo "$session_columns" | grep -qi "day_ms"; then
    echo "FAIL: [$name] expected quota_sessions to carry a day_ms instant column, got: $session_columns" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: quota_sessions"
