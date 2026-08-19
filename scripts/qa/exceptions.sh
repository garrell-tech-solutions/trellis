#!/usr/bin/env bash
# Executable QA procedure: qa/exceptions.md (covers features/exceptions.feature).
# Covers the automatable, curl-only procedures from qa/exceptions.md. Drives
# the running server through its HTTP interface only -- the exception
# controls' endpoint read from the free time page's own markup, the free
# time URL read from the header -- and inspects nothing beyond that this
# slice's own read-only sqlite3 checks need.
#
# qa/exceptions.md's "By-hand walkthrough" is NOT scripted here: this
# environment has no browser-automation tooling, so it is not claimed as
# manually verified. Every fact it names is covered below over curl instead.
#
# Every procedure pins the clock to 2026-08-17T12:00:00Z (Monday), per
# qa/exceptions.md's own instruction: an exception on a specific date means
# nothing without a fixed today, and every expected total in the doc is
# derived from that day.
#
# qa/exceptions.md's own "nothing else changed" procedure is satisfied by
# scripts/qa/run.sh running every script in this directory together. This
# slice adds no page (the exception controls live on the free time page
# free_time.sh already covers), so app_shell.sh is deliberately untouched --
# per the doc, if app_shell needed a change here, that itself would be the
# bug (a page added without a nav::ALL entry).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-exceptions"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

NOW="2026-08-17T12:00:00Z"

qa_get_life_areas() {
  curl -s "http://$ADDR/life-areas"
}

# POSTs the exception form and sets STATUS and BODY. life_area is the name
# to scope to, or "" for all life areas.
qa_mark_away() {
  local endpoint="$1" start="$2" end="$3" life_area="$4" label="$5" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "start=$start" --data-urlencode "end=$end" \
    --data-urlencode "life_area=$life_area" --data-urlencode "label=$label")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# The markup inside <li id="exception-row-ID">...</li> whose
# <span class="exception-dates"> begins with "start_date to", or "" if no
# row matches. Keyed by start date because that is what every scenario in
# qa/exceptions.md and features/exceptions.feature names an exception by.
qa_exception_row_block() {
  local page="$1" start_date="$2"
  python3 -c '
import re, sys
page, start_date = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<li id=\"exception-row-\d+\">(.*?)</li>", page, re.S):
    block = m.group(1)
    dm = re.search(r"<span class=\"exception-dates\">([^<]*)</span>", block)
    if dm and dm.group(1).startswith(start_date + " to"):
        print(block)
        sys.exit()
print("")
' "$page" "$start_date"
}

# The scope text ("All life areas", or a life area's own name) the
# exception starting start_date shows, or "" if not found.
qa_exception_scope() {
  local page="$1" start_date="$2" block
  block="$(qa_exception_row_block "$page" "$start_date")"
  python3 -c '
import re, sys
block = sys.argv[1]
m = re.search(r"<span class=\"exception-scope\">([^<]*)</span>", block)
print(m.group(1) if m else "")
' "$block"
}

qa_exceptions_row_count() {
  local page="$1"
  python3 -c '
import re, sys
print(len(re.findall(r"<li id=\"exception-row-\d+\">", sys.argv[1])))
' "$page"
}

# Saves Work's Mon-Fri 09:00-17:00 band and confirms the 80h baseline every
# procedure in qa/exceptions.md is derived from. Callers must have started
# the server against --now 2026-08-17T12:00:00Z (Monday 17 August 2026) for
# this number to hold.
qa_setup_work_baseline() {
  local proc_name="$1" hours
  qa_save_guardrail_band "$(qa_get_life_areas)" Work "Mon,Tue,Wed,Thu,Fri" "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$proc_name] saving Work's band returned status $STATUS" >&2
    FAILURES=1
  fi
  hours="$(qa_free_time_hours "$(qa_get_free_time)" Work)"
  if [[ "$hours" != "80" ]]; then
    echo "FAIL: [$proc_name] expected the 80h baseline before any exception, got \"$hours\" -- every expectation below is derived from this number" >&2
    FAILURES=1
  fi
}

# --- Procedure: an exception removes those dates ---
for case in "2026-08-24:2026-08-28:40" "2026-08-24:2026-08-24:72" "2026-08-22:2026-08-23:80" "2026-09-10:2026-09-20:80"; do
  IFS=':' read -r from to expected_hours <<< "$case"
  name="exception-removes-dates-$from-$to"
  if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$NOW"; then
    qa_setup_work_baseline "$name"
    endpoint="$(qa_exceptions_add_endpoint "$(qa_get_free_time)")"
    qa_mark_away "$endpoint" "$from" "$to" "" ""
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] marking $from to $to away returned status $STATUS" >&2
      FAILURES=1
    fi
    page="$(qa_get_free_time)"
    hours="$(qa_free_time_hours "$page" Work)"
    if [[ "$hours" != "$expected_hours" ]]; then
      echo "FAIL: [$name] expected Work to report ${expected_hours}h after marking $from to $to away, got \"$hours\"" >&2
      FAILURES=1
    fi
    intervals="$(qa_free_time_intervals "$page" Work)"
    for excluded_date in "$from" "$to"; do
      if [[ "$intervals" == *"$excluded_date"* ]]; then
        echo "FAIL: [$name] expected no interval on $excluded_date, got:
$intervals" >&2
        FAILURES=1
      fi
    done
    row_count="$(qa_exceptions_row_count "$page")"
    if [[ "$row_count" != "1" ]]; then
      echo "FAIL: [$name] expected the exception to still be listed, found $row_count row(s)" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
  qa_stop_server
done

# --- Procedure: scope is explicit, and visible ---
name="scope-explicit-and-visible"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$NOW"; then
  qa_setup_work_baseline "$name"
  qa_save_guardrail_band "$(qa_get_life_areas)" Fitness "Mon,Tue,Wed,Thu,Fri" "06:00" "07:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Fitness's band returned status $STATUS" >&2
    FAILURES=1
  fi
  endpoint="$(qa_exceptions_add_endpoint "$(qa_get_free_time)")"
  qa_mark_away "$endpoint" "2026-08-24" "2026-08-28" "Fitness" ""
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] marking Fitness away returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_free_time)"
  if [[ "$(qa_free_time_hours "$page" Work)" != "80" ]]; then
    echo "FAIL: [$name] expected Work to stay at 80h when only Fitness was marked away" >&2
    FAILURES=1
  fi
  if [[ "$(qa_free_time_hours "$page" Fitness)" != "5" ]]; then
    echo "FAIL: [$name] expected Fitness to drop to 5h" >&2
    FAILURES=1
  fi

  endpoint="$(qa_exceptions_add_endpoint "$page")"
  qa_mark_away "$endpoint" "2026-09-01" "2026-09-02" "" ""
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] marking all life areas away returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_free_time)"
  scope="$(qa_exception_scope "$page" "2026-08-24")"
  if [[ "$scope" != "Fitness" ]]; then
    echo "FAIL: [$name] expected the 2026-08-24 exception to show scope \"Fitness\", got \"$scope\"" >&2
    FAILURES=1
  fi
  scope="$(qa_exception_scope "$page" "2026-09-01")"
  if [[ "$scope" != "All life areas" ]]; then
    echo "FAIL: [$name] expected the 2026-09-01 exception to show scope \"All life areas\", got \"$scope\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a past exception stops mattering by itself ---
name="past-exception-stops-mattering"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$NOW"; then
  qa_setup_work_baseline "$name"
  endpoint="$(qa_exceptions_add_endpoint "$(qa_get_free_time)")"
  qa_mark_away "$endpoint" "2026-08-10" "2026-08-14" "" ""
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] marking a past range away returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_free_time)"
  if [[ "$(qa_free_time_hours "$page" Work)" != "80" ]]; then
    echo "FAIL: [$name] expected Work to stay at 80h for an exception entirely before today" >&2
    FAILURES=1
  fi
  if [[ "$(qa_exceptions_row_count "$page")" != "1" ]]; then
    echo "FAIL: [$name] expected the past exception to still be listed" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: removing an exception restores the hours ---
name="removing-restores-hours"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$NOW"; then
  qa_setup_work_baseline "$name"
  page="$(qa_get_free_time)"
  endpoint="$(qa_exceptions_add_endpoint "$page")"
  qa_mark_away "$endpoint" "2026-08-24" "2026-08-28" "" ""
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] marking the range away returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_free_time)"
  if [[ "$(qa_free_time_hours "$page" Work)" != "40" ]]; then
    echo "FAIL: [$name] setup expected Work at 40h before removal" >&2
    FAILURES=1
  fi
  row_block="$(qa_exception_row_block "$page" "2026-08-24")"
  remove_endpoint="$(qa_block_control_endpoint "$row_block" '>Remove<')"
  curl -s -o /dev/null -X POST "http://$ADDR$remove_endpoint"
  page="$(qa_get_free_time)"
  if [[ "$(qa_free_time_hours "$page" Work)" != "80" ]]; then
    echo "FAIL: [$name] expected Work back to 80h after removing the exception" >&2
    FAILURES=1
  fi
  if [[ "$(qa_exceptions_row_count "$page")" != "0" ]]; then
    echo "FAIL: [$name] expected the exceptions list to be empty after removal" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: overlapping exceptions subtract their union ---
name="overlap-subtracts-union"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$NOW"; then
  qa_setup_work_baseline "$name"
  endpoint="$(qa_exceptions_add_endpoint "$(qa_get_free_time)")"
  qa_mark_away "$endpoint" "2026-08-24" "2026-08-28" "" ""
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] marking the first range away returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_mark_away "$endpoint" "2026-08-26" "2026-08-29" "" ""
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] marking the overlapping range away returned status $STATUS" >&2
    FAILURES=1
  fi
  hours="$(qa_free_time_hours "$(qa_get_free_time)" Work)"
  if [[ "$hours" != "40" ]]; then
    echo "FAIL: [$name] expected 40h (the union, not double-subtracted 16h), got \"$hours\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: an exception across a DST transition ---
for dst_case in "2027-03-13T12:00:00-05:00|2027-03-21|2027-03-21|2" "2027-11-06T12:00:00-04:00|2027-11-14|2027-11-14|4"; do
  IFS='|' read -r dst_now from to expected_hours <<< "$dst_case"
  name="dst-transition-$from"
  if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$dst_now"; then
    endpoint="$(qa_timezone_endpoint "$(qa_get_life_areas)")"
    qa_set_timezone "$endpoint" "America/New_York"
    if [[ "$STATUS" != "200" ]]; then
      echo "FAIL: [$name] setting the timezone returned status $STATUS" >&2
      FAILURES=1
    fi
    qa_save_guardrail_band "$(qa_get_life_areas)" Learning Sun "01:00" "04:00"
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] saving Learning's band returned status $STATUS" >&2
      FAILURES=1
    fi

    exceptions_endpoint="$(qa_exceptions_add_endpoint "$(qa_get_free_time)")"
    qa_mark_away "$exceptions_endpoint" "$from" "$to" "" ""
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] marking the transition Sunday away returned status $STATUS" >&2
      FAILURES=1
    fi
    hours="$(qa_free_time_hours "$(qa_get_free_time)" Learning)"
    if [[ "$hours" != "$expected_hours" ]]; then
      echo "FAIL: [$name] expected Learning to report ${expected_hours}h with the ordinary Sunday removed, got \"$hours\"" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
  qa_stop_server
done

# --- Procedure: a life area given hours and then marked never scheduled ---
name="bands-then-never-scheduled"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$NOW"; then
  qa_setup_work_baseline "$name"
  qa_save_pool_only "$(qa_get_life_areas)" Work
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking Work never-scheduled returned status $STATUS" >&2
    FAILURES=1
  fi
  hours="$(qa_free_time_hours "$(qa_get_free_time)" Work)"
  if [[ "$hours" != "0" ]]; then
    echo "FAIL: [$name] expected Work to report 0h once marked never-scheduled after already having bands, got \"$hours\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a backwards range is refused ---
name="backwards-range-refused"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$NOW"; then
  qa_setup_work_baseline "$name"
  endpoint="$(qa_exceptions_add_endpoint "$(qa_get_free_time)")"
  qa_mark_away "$endpoint" "2026-08-28" "2026-08-24" "" ""
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] expected 422, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"the last day precedes the first"* ]]; then
    echo "FAIL: [$name] expected the rejection to say the last day precedes the first, got:
$BODY" >&2
    FAILURES=1
  fi
  page="$(qa_get_free_time)"
  if [[ "$(qa_exceptions_row_count "$page")" != "0" ]]; then
    echo "FAIL: [$name] expected nothing stored after a rejected range" >&2
    FAILURES=1
  fi
  if [[ "$(qa_free_time_hours "$page" Work)" != "80" ]]; then
    echo "FAIL: [$name] expected Work to stay at 80h after a rejected range" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile text in a label stays escaped ---
name="hostile-label-escaped"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "$NOW"; then
  qa_setup_work_baseline "$name"
  endpoint="$(qa_exceptions_add_endpoint "$(qa_get_free_time)")"
  qa_mark_away "$endpoint" "2026-08-24" "2026-08-28" "" "<script>alert('boom')</script>"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] marking away with a hostile label returned status $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" == *"<script>"* ]]; then
    echo "FAIL: [$name] the exceptions list contains an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"boom"* ]]; then
    echo "FAIL: [$name] the exceptions list does not contain the word boom -- content may have been stripped instead of escaped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: exceptions"
