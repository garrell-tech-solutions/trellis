#!/usr/bin/env bash
# Executable QA procedure: qa/stats_ratio.md (covers
# features/stats_ratio.feature). Drives the running server through its HTTP
# interface only -- GET /stats and the capture/triage endpoints established
# by triage_from_page.sh -- and inspects persisted state via a read-only
# sqlite3 query, never a project-internal API. `trellis serve --now` is
# itself a user-interface affordance (a flag on the command the owner already
# runs), so pinning and advancing the server's clock across restarts is in
# scope for curl-only verification.
#
# qa/stats_ratio.md's "By-hand walkthrough" is deliberately NOT scripted
# here, for the same reason inbox_view.sh and triage_from_page.sh don't
# script theirs: this project's stack has no browser-automation tooling, and
# the walkthrough exists to check what a browser renders. That walkthrough
# was performed manually this QA cycle, over the identical HTTP surface a
# browser would drive, and passed in full: three Pool captures and one
# Committed capture triaged through the page left /stats reporting 1
# committed, 3 pool, 0 quota and no share yet (four is below the floor); six
# more Committed captures brought the total to ten and /stats then reported
# 7 of 10 committed, 70%, over the line; every figure survived a server
# restart against the same database.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-stats-ratio"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_get_stats() {
  curl -s "http://$ADDR/stats"
}

# Parses the rendered /stats page into a JSON object: committed, pool, quota
# and in_window (ints), share_percent (int or null), standing ("over",
# "under" or null), and no_share_message (bool) -- everything a procedure's
# Expected Observable Outcomes names.
qa_parse_stats() {
  local page="$1"
  python3 -c '
import re, json, sys
page = sys.argv[1]
result = {
    "committed": None, "pool": None, "quota": None, "in_window": None,
    "share_percent": None, "standing": None,
}
m = re.search(r"<p>(\d+) committed, (\d+) pool and (\d+) quota tasks</p>", page)
if m:
    result["committed"], result["pool"], result["quota"] = (int(g) for g in m.groups())
m = re.search(r"<p>(\d+) tasks in the window</p>", page)
if m:
    result["in_window"] = int(m.group(1))
m = re.search(r"<p>Committed share: (\d+)%</p>", page)
if m:
    result["share_percent"] = int(m.group(1))
m = re.search(r"<p class=\"standing\">(over|under) the line</p>", page)
if m:
    result["standing"] = m.group(1)
result["no_share_message"] = "It has not measured enough tasks yet to report a share." in page
print(json.dumps(result))
' "$page"
}

# Reads one field out of qa_parse_stats's JSON, with a JSON string's
# surrounding quotes stripped -- so "over" and 42 compare against a shell
# string the same way. qa_json_raw_field (not qa_json_field) is used because
# these fields are legitimately JSON null, not just strings.
qa_stats_field() {
  local stats="$1" field="$2" raw
  raw="$(qa_json_raw_field "$stats" "$field")"
  if [[ "$raw" == \"*\" ]]; then
    raw="${raw:1:-1}"
  fi
  echo "$raw"
}

qa_assert_stats_field() {
  local name="$1" field="$2" expected="$3" stats="$4" got
  got="$(qa_stats_field "$stats" "$field")"
  if [[ "$got" != "$expected" ]]; then
    echo "FAIL: [$name] expected $field=$expected, got $got" >&2
    FAILURES=1
  fi
}

# The minimal accepted payload for each kind, per triage/task.rs's field
# domains -- only what triage requires to accept the row, since these
# fixtures exist to populate /stats's denominator, not to exercise triage
# itself (that is committed_field_domains.sh's and quota_triage_validation.sh's job).
qa_fixture_payload_for() {
  case "$1" in
    pool) echo '{"kind":"pool"}' ;;
    committed) echo '{"kind":"committed","deadline":"2026-08-20T17:00:00Z","deadline_type":"hard","priority":"P1"}' ;;
    quota) echo '{"kind":"quota","target_count":3,"target_minutes_each":45,"period":"week"}' ;;
  esac
}

# Captures and triages `count` fresh items as `kind`, against the server the
# most recent qa_start_server started. FIXTURE_SEQ (not name/kind/i alone)
# keeps every fixture's raw_text unique even when a procedure triages the
# same kind in two separate calls -- qa_submit_capture locates a row by its
# raw_text, and a repeat would match more than one row.
FIXTURE_SEQ=0
qa_triage_n_as() {
  local name="$1" kind="$2" count="$3" i cid body
  body="$(qa_fixture_payload_for "$kind")"
  for ((i = 0; i < count; i++)); do
    FIXTURE_SEQ=$((FIXTURE_SEQ + 1))
    cid="$(qa_submit_capture "stats-fixture-$name-$kind-$FIXTURE_SEQ")"
    qa_triage "$cid" "$body"
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] fixture triage as $kind (#$i) returned status $STATUS, body: $BODY" >&2
      FAILURES=1
    fi
  done
}

# --- Procedure: the share, and the counts behind it ---
name="counts-and-share"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_triage_n_as "$name" committed 5
  qa_triage_n_as "$name" pool 4
  qa_triage_n_as "$name" quota 3
  stats="$(qa_parse_stats "$(qa_get_stats)")"
  qa_assert_stats_field "$name" committed 5 "$stats"
  qa_assert_stats_field "$name" pool 4 "$stats"
  qa_assert_stats_field "$name" quota 3 "$stats"
  qa_assert_stats_field "$name" in_window 12 "$stats"
  qa_assert_stats_field "$name" share_percent 42 "$stats"
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: quota is inside the denominator ---
name="quota-in-denominator"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_triage_n_as "$name" committed 6
  qa_triage_n_as "$name" pool 6
  stats="$(qa_parse_stats "$(qa_get_stats)")"
  qa_assert_stats_field "$name" in_window 12 "$stats"
  qa_assert_stats_field "$name" share_percent 50 "$stats"
  qa_triage_n_as "$name" quota 6
  stats="$(qa_parse_stats "$(qa_get_stats)")"
  qa_assert_stats_field "$name" in_window 18 "$stats"
  qa_assert_stats_field "$name" share_percent 33 "$stats"
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the rolling fourteen-day window ---
# Three server runs against one database -- never a fresh one between them.
name="rolling-window"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-1.log" "2026-07-24T09:00:00Z"; then
  qa_triage_n_as "$name" committed 3
  qa_stop_server
  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log" "2026-08-06T09:00:00Z"; then
    qa_triage_n_as "$name" pool 12
    stats="$(qa_parse_stats "$(qa_get_stats)")"
    qa_assert_stats_field "$name" in_window 15 "$stats"
    qa_assert_stats_field "$name" share_percent 20 "$stats"
    qa_stop_server
    if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-3.log" "2026-08-08T09:00:00Z"; then
      stats="$(qa_parse_stats "$(qa_get_stats)")"
      qa_assert_stats_field "$name" in_window 12 "$stats"
      qa_assert_stats_field "$name" share_percent 0 "$stats"
      total="$(qa_task_count)"
      if [[ "$total" != "15" ]]; then
        echo "FAIL: [$name] expected 15 task rows to persist after leaving the window, found $total" >&2
        FAILURES=1
      fi
    else
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: --now offsets the clock, it does not stop it ---
name="now-does-not-freeze"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "2026-07-24T09:00:00Z"; then
  first_id="$(qa_submit_capture "first")"
  second_id="$(qa_submit_capture "second")"
  first_ts="$(sqlite3 "$DB_PATH" "SELECT created_at_ms FROM captures WHERE id = $first_id;")"
  second_ts="$(sqlite3 "$DB_PATH" "SELECT created_at_ms FROM captures WHERE id = $second_id;")"
  if [[ "$first_ts" == "$second_ts" ]]; then
    echo "FAIL: [$name] both captures share created_at_ms=$first_ts -- the clock did not advance" >&2
    FAILURES=1
  elif [[ "$second_ts" -le "$first_ts" ]]; then
    echo "FAIL: [$name] expected \"second\" ($second_ts) to be later than \"first\" ($first_ts)" >&2
    FAILURES=1
  fi
  inbox="$(qa_html_section "$(curl -s "http://$ADDR/")" captures)"
  order="$(python3 -c '
import sys
inbox = sys.argv[1]
print(inbox.find("second") >= 0 and inbox.find("first") >= 0 and inbox.find("second") < inbox.find("first"))
' "$inbox")"
  if [[ "$order" != "True" ]]; then
    echo "FAIL: [$name] expected the inbox to list \"second\" before \"first\" (newest-first)" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the fifty-percent line, reported distinguishably ---
run_standing_case() {
  local case_name="$1" committed_n="$2" pool_n="$3" expected_percent="$4" expected_standing="$5"
  local name="standing-$case_name"
  if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
    qa_triage_n_as "$name" committed "$committed_n"
    qa_triage_n_as "$name" pool "$pool_n"
    stats="$(qa_parse_stats "$(qa_get_stats)")"
    qa_assert_stats_field "$name" share_percent "$expected_percent" "$stats"
    qa_assert_stats_field "$name" standing "$expected_standing" "$stats"
  else
    FAILURES=1
  fi
  qa_stop_server
}
run_standing_case under-25 5 15 25 under
run_standing_case exactly-50 10 10 50 under
run_standing_case over-55 11 9 55 over

# --- Procedure: below ten tasks, and an empty database ---
name="below-floor"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  stats="$(qa_parse_stats "$(qa_get_stats)")"
  qa_assert_stats_field "$name" committed 0 "$stats"
  qa_assert_stats_field "$name" pool 0 "$stats"
  qa_assert_stats_field "$name" quota 0 "$stats"
  if [[ "$(qa_stats_field "$stats" no_share_message)" != "true" ]]; then
    echo "FAIL: [$name] expected an empty database to report no measured share yet" >&2
    FAILURES=1
  fi
  if [[ "$(qa_get_stats)" == *"0%"* ]]; then
    echo "FAIL: [$name] an empty database must not print a bare 0%" >&2
    FAILURES=1
  fi

  qa_triage_n_as "$name" committed 4
  qa_triage_n_as "$name" pool 5
  stats="$(qa_parse_stats "$(qa_get_stats)")"
  qa_assert_stats_field "$name" in_window 9 "$stats"
  if [[ "$(qa_stats_field "$stats" no_share_message)" != "true" ]]; then
    echo "FAIL: [$name] nine tasks is below the floor of ten; expected no share yet" >&2
    FAILURES=1
  fi
  if [[ "$(qa_get_stats)" == *"44%"* ]]; then
    echo "FAIL: [$name] nine tasks below the floor must not print 44%" >&2
    FAILURES=1
  fi

  qa_triage_n_as "$name" pool 1
  stats="$(qa_parse_stats "$(qa_get_stats)")"
  qa_assert_stats_field "$name" in_window 10 "$stats"
  qa_assert_stats_field "$name" share_percent 40 "$stats"
  qa_assert_stats_field "$name" standing under "$stats"
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the figures are computed from the rows, not held in memory ---
name="rows-not-memory"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-1.log"; then
  qa_triage_n_as "$name" committed 6
  qa_triage_n_as "$name" pool 6
  before="$(qa_get_stats)"
  qa_stop_server
  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log"; then
    after="$(qa_get_stats)"
    if [[ "$before" != "$after" ]]; then
      echo "FAIL: [$name] /stats figures changed after restarting against the same database" >&2
      echo "before: $before" >&2
      echo "after:  $after" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the stats page changes nothing else ---
name="stats-is-read-only"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  buy_milk_id="$(qa_submit_capture "buy milk")"
  qa_triage_n_as "$name" committed 3
  captures_before="$(sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM captures;')"
  tasks_before="$(sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM tasks;')"

  first_read="$(qa_get_stats)"
  page="$(curl -s "http://$ADDR/")"
  inbox="$(qa_html_section "$page" captures)"
  tasks_section="$(qa_html_section "$page" tasks)"
  if [[ "$inbox" != *"buy milk"* ]]; then
    echo "FAIL: [$name] \"buy milk\" is missing from the inbox after reading /stats" >&2
    FAILURES=1
  fi
  if ! qa_capture_untriaged "$buy_milk_id"; then
    echo "FAIL: [$name] \"buy milk\" was triaged merely by reading /stats" >&2
    FAILURES=1
  fi
  committed_rows="$(grep -o "stats-fixture-$name-committed-" <<<"$tasks_section" | wc -l | tr -d ' ')"
  if [[ "$committed_rows" != "3" ]]; then
    echo "FAIL: [$name] expected 3 committed rows in the task list, found $committed_rows" >&2
    FAILURES=1
  fi

  second_read="$(qa_get_stats)"
  if [[ "$first_read" != "$second_read" ]]; then
    echo "FAIL: [$name] two consecutive /stats reads produced different figures" >&2
    FAILURES=1
  fi

  captures_after="$(sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM captures;')"
  tasks_after="$(sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM tasks;')"
  if [[ "$captures_before" != "$captures_after" || "$tasks_before" != "$tasks_after" ]]; then
    echo "FAIL: [$name] row counts changed after reading /stats (captures $captures_before->$captures_after, tasks $tasks_before->$tasks_after)" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: stats_ratio"
