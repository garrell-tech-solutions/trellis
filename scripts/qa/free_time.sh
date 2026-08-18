#!/usr/bin/env bash
# Executable QA procedure: qa/free_time.md (covers features/free_time.feature).
# Covers the automatable, curl-only procedures from qa/free_time.md. Drives
# the running server through its HTTP interface only -- the free time URL
# read out of the header's own markup, guardrails set up through the life
# areas page's own controls -- and inspects nothing beyond that; this slice
# writes no durable state of its own.
#
# qa/free_time.md's "By-hand walkthrough" is NOT scripted here: this
# environment has no browser-automation tooling, so it is not claimed as
# manually verified. Every fact it names is covered below over curl instead.
#
# qa/free_time.md's own "nothing else changed" procedure names every other
# QA suite plus app_shell (expected to have changed for the fourth nav
# link); scripts/qa/run.sh already runs every script in this directory
# together, and app_shell.sh already covers the four-link header, so
# neither is re-run here.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-free-time"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_get_life_areas() {
  curl -s "http://$ADDR/life-areas"
}

# The free time page's own URL, read from the header of any page (the
# header is shared, per app_shell) -- not assumed.
qa_free_time_url() {
  local page="$1"
  python3 -c '
import re, sys
page = sys.argv[1]
m = re.search(r"<a href=\"([^\"]*)\"[^>]*>Free time</a>", page)
print(m.group(1) if m else "")
' "$page"
}

qa_get_free_time() {
  local url
  url="$(qa_free_time_url "$(curl -s "http://$ADDR/")")"
  curl -s "http://$ADDR$url"
}

# The markup inside <div id="free-time-row-ID">...</div> whose
# <span class="life-area-name"> exactly matches `name`, or "" if no row
# matches.
qa_free_time_row_block() {
  qa_named_row_block "$1" div "free-time-row-" "$2"
}

# The total hours name's row reports, or "" if the row cannot be found.
qa_free_time_hours() {
  local page="$1" name="$2" block
  block="$(qa_free_time_row_block "$page" "$name")"
  python3 -c '
import re, sys
block = sys.argv[1]
m = re.search(r"— (\d+)h", block)
print(m.group(1) if m else "")
' "$block"
}

# The intervals listed on name's row, one per line, in document order.
qa_free_time_intervals() {
  local page="$1" name="$2" block
  block="$(qa_free_time_row_block "$page" "$name")"
  python3 -c '
import re, sys
block = sys.argv[1]
for m in re.finditer(r"<li>([^<]*)</li>", block):
    print(m.group(1))
' "$block"
}

# --- Procedure: a guardrail projects across the range ---
name="guardrail-projects-across-range"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Work's band returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_free_time)"
  hours="$(qa_free_time_hours "$page" Work)"
  if [[ "$hours" != "16" ]]; then
    echo "FAIL: [$name] expected Work to report 16 hours, got \"$hours\"" >&2
    FAILURES=1
  fi
  intervals="$(qa_free_time_intervals "$page" Work)"
  interval_count="$(printf '%s\n' "$intervals" | grep -c .)"
  if [[ "$interval_count" != "2" ]]; then
    echo "FAIL: [$name] expected 2 intervals listed for Work, found $interval_count:
$intervals" >&2
    FAILURES=1
  fi
  starts="$(printf '%s\n' "$intervals" | python3 -c 'import sys; print("\n".join(l.split()[1].split("-")[0] for l in sys.stdin if l.strip()))')"
  ends="$(printf '%s\n' "$intervals" | python3 -c 'import sys; print("\n".join(l.split()[1].split("-")[1] for l in sys.stdin if l.strip()))')"
  if [[ "$(printf '%s\n' "$starts" | sort -u)" != "09:00" ]]; then
    echo "FAIL: [$name] expected every interval to start at 09:00, got:
$starts" >&2
    FAILURES=1
  fi
  if [[ "$(printf '%s\n' "$ends" | sort -u)" != "17:00" ]]; then
    echo "FAIL: [$name] expected every interval to end at 17:00, got:
$ends" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: empty is a real answer ---
name="empty-is-a-real-answer"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_pool_only "$(qa_get_life_areas)" Work
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking Work never-scheduled returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_free_time)"
  for la in Work Home; do
    hours="$(qa_free_time_hours "$page" "$la")"
    if [[ "$hours" != "0" ]]; then
      echo "FAIL: [$name] expected $la to report 0 hours, got \"$hours\"" >&2
      FAILURES=1
    fi
    intervals="$(qa_free_time_intervals "$page" "$la")"
    if [[ -n "$intervals" ]]; then
      echo "FAIL: [$name] expected $la to list no intervals, got:
$intervals" >&2
      FAILURES=1
    fi
    block="$(qa_free_time_row_block "$page" "$la")"
    if [[ -z "$block" ]]; then
      echo "FAIL: [$name] expected $la to still appear on the page" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: overlapping guardrails both report their time ---
name="overlapping-guardrails-both-report"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Work's band returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_save_guardrail_band "$(qa_get_life_areas)" Learning Mon "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Learning's identical band returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_free_time)"
  for la in Work Learning; do
    hours="$(qa_free_time_hours "$page" "$la")"
    if [[ "$hours" != "16" ]]; then
      echo "FAIL: [$name] expected $la to independently report 16 hours, got \"$hours\"" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# Sets the timezone to America/New_York and saves Work's Sun 01:00-04:00
# band -- the setup the spring-forward and fall-back procedures share,
# differing only in which --now the server started against. Prints the
# timezone endpoint, for a caller that wants to change the zone again
# afterwards.
qa_setup_dst_band() {
  local proc_name="$1" endpoint
  endpoint="$(qa_timezone_endpoint "$(qa_get_life_areas)")"
  qa_set_timezone "$endpoint" "America/New_York"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$proc_name] setting the timezone to America/New_York returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Sun "01:00" "04:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$proc_name] saving Work's band returned status $STATUS" >&2
    FAILURES=1
  fi
  echo "$endpoint"
}

# --- Procedure: the spring-forward gap ---
name="spring-forward-gap"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "2027-03-13T12:00:00-05:00"; then
  endpoint="$(qa_setup_dst_band "$name")"
  hours="$(qa_free_time_hours "$(qa_get_free_time)" Work)"
  if [[ "$hours" != "5" ]]; then
    echo "FAIL: [$name] expected Work to report 5 hours across the spring-forward gap, got \"$hours\"" >&2
    FAILURES=1
  fi

  qa_set_timezone "$endpoint" "UTC"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] resetting the timezone to UTC returned status $STATUS" >&2
    FAILURES=1
  fi
  hours="$(qa_free_time_hours "$(qa_get_free_time)" Work)"
  if [[ "$hours" != "6" ]]; then
    echo "FAIL: [$name] expected Work to report 6 hours in UTC (no transition), got \"$hours\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the fall-back fold ---
name="fall-back-fold"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "2027-11-06T12:00:00-04:00"; then
  qa_setup_dst_band "$name" >/dev/null
  hours="$(qa_free_time_hours "$(qa_get_free_time)" Work)"
  if [[ "$hours" != "7" ]]; then
    echo "FAIL: [$name] expected Work to report 7 hours across the fall-back fold, got \"$hours\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a hostile life area name stays escaped ---
name="hostile-text-escaped"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  add_endpoint="$(qa_life_areas_add_endpoint "$(qa_get_life_areas)")"
  qa_add_life_area "$add_endpoint" "<script>alert('boom')</script>"
  # Found by id, not by qa_life_area_control_endpoint's name match: the
  # name it renders under is HTML-escaped, so the raw submitted text never
  # appears verbatim in the markup to match against. The endpoint itself is
  # still read out of that row's own markup, not assumed.
  hostile_id="$(sqlite3 "$DB_PATH" "SELECT id FROM life_areas WHERE name = '<script>alert(''boom'')</script>';")"
  row_block="$(python3 -c '
import re, sys
page, la_id = sys.argv[1], sys.argv[2]
m = re.search(r"<li id=\"life-area-row-" + re.escape(la_id) + r"\">(.*?)</li>", page, re.S)
print(m.group(1) if m else "")
' "$(qa_get_life_areas)" "$hostile_id")"
  guardrail_endpoint="$(qa_block_control_endpoint "$row_block" '>Add band<')"
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$guardrail_endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "mon=on&start=09%3A00&end=17%3A00")"
  STATUS="${response##*$'\n'}"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving the hostile life area's band returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_free_time)"
  if [[ "$page" == *"<script>"* ]]; then
    echo "FAIL: [$name] the free time page contains an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$page" != *"boom"* ]]; then
    echo "FAIL: [$name] the free time page does not contain the word boom -- content may have been stripped instead of escaped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: free_time"
