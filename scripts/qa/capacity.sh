#!/usr/bin/env bash
# Executable QA procedure: qa/capacity.md (covers features/capacity.feature).
# Covers the automatable, curl-only procedures from qa/capacity.md. Drives
# the running server through its HTTP interface only -- the capacity URL
# read from the header, guardrails and exceptions set up through the life
# areas and free time pages' own controls -- and inspects nothing beyond
# read-only sqlite3 checks this slice's own procedures need.
#
# qa/capacity.md's "By-hand walkthrough" is NOT scripted here: this
# environment has no browser-automation tooling, so it is not claimed as
# manually verified. Every fact it names is covered below over curl instead,
# including its step 6 (the committed form alone carries the estimate
# field), which the "the triage form asks committed alone for an estimate"
# procedure below checks by reading the rendered markup.
#
# qa/capacity.md's own "a committed task with no estimate" procedure is NOT
# scripted here either, per the doc's own instruction: triage now requires
# the field, so there is no way to create an unestimated committed task from
# any interface this procedure is allowed to use. It is verified by the unit
# test capacity::tests::an_unestimated_committed_task_is_not_counted_as_zero_but_is_surfaced,
# confirmed passing in this QA cycle's `cargo test --workspace`; QA could not
# drive it end-to-end, as the doc says to report.
#
# qa/capacity.md's own "nothing else changed" procedure is satisfied by
# scripts/qa/run.sh running every script in this directory together;
# app_shell.sh and committed_triage_validation.sh were updated separately in
# this QA cycle for the fifth nav link and the fourth required field.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-capacity"
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

# The capacity page's own URL, read from the header of any page -- not
# assumed.
qa_capacity_url() {
  local page="$1"
  python3 -c '
import re, sys
page = sys.argv[1]
m = re.search(r"<a href=\"([^\"]*)\"[^>]*>Capacity</a>", page)
print(m.group(1) if m else "")
' "$page"
}

qa_get_capacity() {
  local url
  url="$(qa_capacity_url "$(curl -s "http://$ADDR/")")"
  curl -s "http://$ADDR$url"
}

# The markup inside <div id="capacity-row-ID">...</div> whose
# <span class="life-area-name"> exactly matches `name`, or "" if no row
# matches (an archived life area, for instance).
qa_capacity_row_block() {
  qa_named_row_block "$1" div "capacity-row-" "$2"
}

# The fields name's capacity row reports, as a JSON object:
# never_scheduled (bool), needed_hours, available_hours, percent
# (null if never_scheduled), over_hours (null if not over), unestimated
# (0 if the row names none). "" (not found) if the row itself is missing.
qa_capacity_fields() {
  local page="$1" name="$2" block
  block="$(qa_capacity_row_block "$page" "$name")"
  if [[ -z "$block" ]]; then
    echo ""
    return
  fi
  python3 -c '
import re, json, sys
block = sys.argv[1]
if "never scheduled" in block:
    print(json.dumps({"never_scheduled": True}))
    sys.exit()
m = re.search(r"—\s*([\d.]+)h needed,\s*([\d.]+)h available,\s*(\d+)%", block)
over_m = re.search(r",\s*([\d.]+)h over", block)
un_m = re.search(r"\((\d+) without an estimate\)", block)
print(json.dumps({
    "never_scheduled": False,
    "needed_hours": float(m.group(1)) if m else None,
    "available_hours": float(m.group(2)) if m else None,
    "percent": int(m.group(3)) if m else None,
    "over_hours": float(over_m.group(1)) if over_m else None,
    "unestimated": int(un_m.group(1)) if un_m else 0,
}))
' "$block"
}

# Reads one field out of qa_capacity_fields' JSON, printing "MISSING" if the
# row itself was not found and "null"/a bare value otherwise.
qa_capacity_field() {
  local fields="$1" field="$2"
  if [[ -z "$fields" ]]; then
    echo "MISSING"
    return
  fi
  python3 -c '
import json, sys
d = json.loads(sys.argv[1])
print(d.get(sys.argv[2]))
' "$fields" "$field"
}

qa_triage_committed() {
  local capture_id="$1" life_area="$2" estimated_minutes="$3"
  qa_triage "$capture_id" "$(python3 -c 'import json,sys; print(json.dumps({"kind":"committed","deadline":"2026-08-20T17:00:00Z","deadline_type":"hard","priority":"P1","estimated_minutes":int(sys.argv[2]),"life_area":sys.argv[1]}))' "$life_area" "$estimated_minutes")"
}

# --- Procedure: the triage form asks committed alone for an estimate ---
name="only-committed-form-asks-for-an-estimate"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  cid="$(qa_submit_capture "buy milk")"
  page="$(curl -s "http://$ADDR/")"
  if [[ "$page" != *"name=\"estimated_minutes\""* ]]; then
    echo "FAIL: [$name] expected the committed form to carry an estimated_minutes input" >&2
    FAILURES=1
  fi
  block="$(qa_capture_row_block "$page" "$cid")"
  pool_form="$(python3 -c '
import re, sys
block = sys.argv[1]
for form in re.findall(r"<form\b[^>]*>.*?</form>", block, re.S):
    if "value=\"pool\"" in form:
        print(form)
        break
' "$block")"
  quota_form="$(python3 -c '
import re, sys
block = sys.argv[1]
for form in re.findall(r"<form\b[^>]*>.*?</form>", block, re.S):
    if "value=\"quota\"" in form:
        print(form)
        break
' "$block")"
  if [[ "$pool_form" == *"estimated_minutes"* ]]; then
    echo "FAIL: [$name] the pool form should not ask for an estimate" >&2
    FAILURES=1
  fi
  if [[ "$quota_form" == *"estimated_minutes"* ]]; then
    echo "FAIL: [$name] the quota form should not ask for an estimate" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: supply against demand ---
name="supply-against-demand"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Fitness Sat "09:00" "11:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Fitness's band returned status $STATUS" >&2
    FAILURES=1
  fi
  cid="$(qa_submit_capture "run a 5k")"
  qa_triage_committed "$cid" Fitness 180
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the committed task returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_capacity)"
  fields="$(qa_capacity_fields "$page" Fitness)"
  if [[ "$(qa_capacity_field "$fields" needed_hours)" != "3.0" ]]; then
    echo "FAIL: [$name] expected 3h needed, got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" available_hours)" != "4.0" ]]; then
    echo "FAIL: [$name] expected 4h available, got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" percent)" != "75" ]]; then
    echo "FAIL: [$name] expected 75%, got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" over_hours)" != "None" ]]; then
    echo "FAIL: [$name] expected no over-hours warning, got: $fields" >&2
    FAILURES=1
  fi
  free_time_hours="$(qa_free_time_hours "$(qa_get_free_time)" Fitness)"
  if [[ "$free_time_hours" != "4" ]]; then
    echo "FAIL: [$name] expected the free time page to also report 4h for Fitness, got \"$free_time_hours\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: over-commitment is named, and clears ---
name="over-commitment-named-and-clears"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_life_areas)"
  qa_save_guardrail_band "$page" Fitness Sat "09:00" "11:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Fitness's band returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_triage_committed "$(qa_submit_capture "run a 5k")" Fitness 180
  qa_triage_committed "$(qa_submit_capture "swim laps")" Fitness 120
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the second committed task returned status $STATUS" >&2
    FAILURES=1
  fi
  fields="$(qa_capacity_fields "$(qa_get_capacity)" Fitness)"
  if [[ "$(qa_capacity_field "$fields" needed_hours)" != "5.0" ]]; then
    echo "FAIL: [$name] expected 5h needed, got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" percent)" != "125" ]]; then
    echo "FAIL: [$name] expected 125%, got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" over_hours)" != "1.0" ]]; then
    echo "FAIL: [$name] expected 1h over, got: $fields" >&2
    FAILURES=1
  fi

  # Widen Fitness's guardrail (remove the 2h band, add a 3h one) and confirm
  # the warning clears -- per the doc, confirming the clear matters as much
  # as confirming the warning.
  page="$(qa_get_life_areas)"
  remove_endpoint="$(qa_life_area_control_endpoint "$page" Fitness '>Remove<')"
  curl -s -o /dev/null -X POST "http://$ADDR$remove_endpoint"
  qa_save_guardrail_band "$(qa_get_life_areas)" Fitness Sat "09:00" "12:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] widening Fitness's band returned status $STATUS" >&2
    FAILURES=1
  fi
  fields="$(qa_capacity_fields "$(qa_get_capacity)" Fitness)"
  if [[ "$(qa_capacity_field "$fields" over_hours)" != "None" ]]; then
    echo "FAIL: [$name] expected the over-hours warning to clear after widening the guardrail, got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" available_hours)" != "6.0" ]]; then
    echo "FAIL: [$name] expected 6h available after widening to Sat 09:00-12:00, got: $fields" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: pool consumes nothing ---
name="pool-consumes-nothing"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Fitness Sat "09:00" "11:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Fitness's band returned status $STATUS" >&2
    FAILURES=1
  fi
  for raw_text in "run a 5k" "swim laps"; do
    qa_triage "$(qa_submit_capture "$raw_text")" '{"kind":"pool","life_area":"Fitness"}'
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] triaging \"$raw_text\" as pool returned status $STATUS" >&2
      FAILURES=1
    fi
  done
  fields="$(qa_capacity_fields "$(qa_get_capacity)" Fitness)"
  if [[ "$(qa_capacity_field "$fields" needed_hours)" != "0.0" ]]; then
    echo "FAIL: [$name] expected 0h needed for two pool tasks, got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" available_hours)" != "4.0" ]]; then
    echo "FAIL: [$name] expected 4h available, got: $fields" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: quota demand counts, prorated ---
name="quota-demand-weekly"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Learning "Mon,Tue,Wed,Thu,Fri" "20:00" "22:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Learning's band returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_triage "$(qa_submit_capture "read")" '{"kind":"quota","target_count":3,"target_minutes_each":40,"period":"week","life_area":"Learning"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the weekly quota task returned status $STATUS" >&2
    FAILURES=1
  fi
  fields="$(qa_capacity_fields "$(qa_get_capacity)" Learning)"
  if [[ "$(qa_capacity_field "$fields" needed_hours)" != "4.0" ]]; then
    echo "FAIL: [$name] expected 4h needed (3 sessions x 40min x 2 weeks), got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" available_hours)" != "20.0" ]]; then
    echo "FAIL: [$name] expected 20h available, got: $fields" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

name="quota-demand-monthly"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Learning "Mon,Tue,Wed,Thu,Fri" "20:00" "22:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Learning's band returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_triage "$(qa_submit_capture "read")" '{"kind":"quota","target_count":10,"target_minutes_each":45,"period":"month","life_area":"Learning"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the monthly quota task returned status $STATUS" >&2
    FAILURES=1
  fi
  fields="$(qa_capacity_fields "$(qa_get_capacity)" Learning)"
  if [[ "$(qa_capacity_field "$fields" needed_hours)" != "3.5" ]]; then
    echo "FAIL: [$name] expected 3.5h needed (14/30 of 10x45min), got: $fields" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a life area that opted out ---
name="never-scheduled-life-area"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_pool_only "$(qa_get_life_areas)" Fitness
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking Fitness never-scheduled returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_triage_committed "$(qa_submit_capture "run a 5k")" Fitness 180
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging a committed task into the never-scheduled life area returned status $STATUS" >&2
    FAILURES=1
  fi
  fields="$(qa_capacity_fields "$(qa_get_capacity)" Fitness)"
  if [[ "$(qa_capacity_field "$fields" never_scheduled)" != "True" ]]; then
    echo "FAIL: [$name] expected Fitness to report never_scheduled, got: $fields" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: an exception lowers what is available ---
name="exception-lowers-available"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log" "2026-08-17T12:00:00Z"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Fitness Sat "09:00" "11:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Fitness's band returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_triage_committed "$(qa_submit_capture "run a 5k")" Fitness 180
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the committed task returned status $STATUS" >&2
    FAILURES=1
  fi
  fields="$(qa_capacity_fields "$(qa_get_capacity)" Fitness)"
  if [[ "$(qa_capacity_field "$fields" needed_hours)" != "3.0" || "$(qa_capacity_field "$fields" available_hours)" != "4.0" || "$(qa_capacity_field "$fields" over_hours)" != "None" ]]; then
    echo "FAIL: [$name] setup expected 3h needed, 4h available, no warning before the exception, got: $fields" >&2
    FAILURES=1
  fi

  exceptions_endpoint="$(qa_exceptions_add_endpoint "$(qa_get_free_time)")"
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$exceptions_endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "start=2026-08-22" --data-urlencode "end=2026-08-22" \
    --data-urlencode "life_area=" --data-urlencode "label=")"
  ex_status="${response##*$'\n'}"
  if [[ "$ex_status" != "201" ]]; then
    echo "FAIL: [$name] marking 2026-08-22 away returned status $ex_status" >&2
    FAILURES=1
  fi

  fields="$(qa_capacity_fields "$(qa_get_capacity)" Fitness)"
  if [[ "$(qa_capacity_field "$fields" needed_hours)" != "3.0" ]]; then
    echo "FAIL: [$name] expected 3h still needed after the exception, got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" available_hours)" != "2.0" ]]; then
    echo "FAIL: [$name] expected 2h available after removing one Saturday, got: $fields" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capacity_field "$fields" over_hours)" != "1.0" ]]; then
    echo "FAIL: [$name] expected 1h over after the exception tipped Fitness over, got: $fields" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: archived things are excluded ---
name="archived-excluded"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_life_areas)"
  qa_save_guardrail_band "$page" Fitness Sat "09:00" "11:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Fitness's band returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_triage_committed "$(qa_submit_capture "run a 5k")" Fitness 180
  archive_endpoint="$(qa_life_area_control_endpoint "$(qa_get_life_areas)" Fitness ">Archive<")"
  curl -s -o /dev/null -X POST "http://$ADDR$archive_endpoint"
  fields="$(qa_capacity_fields "$(qa_get_capacity)" Fitness)"
  if [[ -n "$fields" ]]; then
    echo "FAIL: [$name] expected archived Fitness not to be reported at all, got: $fields" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: capacity"
