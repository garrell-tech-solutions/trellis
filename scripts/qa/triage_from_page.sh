#!/usr/bin/env bash
# Executable QA procedure: qa/triage_from_page.md (covers
# features/triage_from_page.feature). Covers the automatable, curl-only
# procedures from qa/triage_from_page.md. Drives the running server through
# its HTTP interface only, and inspects persisted state via a read-only
# sqlite3 query -- never through a project-internal API.
#
# qa/triage_from_page.md's "By-hand walkthrough" is deliberately NOT
# scripted here, for the same reason inbox_view.sh doesn't script its own:
# this project's stack has no browser-automation tooling, and the document
# is explicit that the browser-visible half of "no full page reload" is a
# one-time human check curl cannot honestly replace. That walkthrough was
# performed manually this QA cycle and passed in full: the inbox row for
# "buy milk" offered Pool/Committed/Quota; clicking Pool moved it into the
# task list without a visible reload; the committed form offered deadline
# type and priority as fixed choices; submitting committed with a field
# blank named that field, created nothing, and left the capture in the
# inbox; asking for 0 quota sessions was refused; and everything survived a
# server restart.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-triage-from-page"
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

# Extracts capture_id's pool/committed/quota triage controls from a rendered
# page as one JSON object: each control's hx-post endpoint (read from the
# page's own markup, not assumed), plus the committed form's
# deadline_type/priority <select> options. Per qa/triage_from_page.md's
# "Independent of Implementation" note.
qa_extract_controls() {
  local page="$1" capture_id="$2"
  python3 -c '
import re, json, sys

page, capture_id = sys.argv[1], sys.argv[2]
m = re.search(r"<li id=\"capture-row-" + re.escape(capture_id) + r"\">(.*?)</li>", page, re.S)
if not m:
    print(json.dumps({}))
    sys.exit()
block = m.group(1)

result = {}
for form in re.findall(r"<form\b[^>]*>.*?</form>", block, re.S):
    hx_post_m = re.search(r"hx-post=\"([^\"]+)\"", form)
    endpoint = hx_post_m.group(1) if hx_post_m else None
    if "value=\"pool\"" in form:
        result["pool_endpoint"] = endpoint
    elif "value=\"committed\"" in form:
        result["committed_endpoint"] = endpoint
        dt = re.search(r"<select name=\"deadline_type\">(.*?)</select>", form, re.S)
        result["deadline_type_options"] = re.findall(r"<option value=\"([^\"]+)\"", dt.group(1)) if dt else []
        pr = re.search(r"<select name=\"priority\">(.*?)</select>", form, re.S)
        result["priority_options"] = re.findall(r"<option value=\"([^\"]+)\"", pr.group(1)) if pr else []
    elif "value=\"quota\"" in form:
        result["quota_endpoint"] = endpoint

print(json.dumps(result))
' "$page" "$capture_id"
}

# POSTs a form-encoded triage submission (as the page's controls would) and
# sets STATUS and BODY.
qa_triage_form() {
  local endpoint="$1" data="$2" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "$data")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# --- Procedure: the inbox offers all three kinds ---
name="offers-all-kinds"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "buy milk")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  for endpoint_field in pool_endpoint committed_endpoint quota_endpoint; do
    endpoint="$(qa_json_field "$controls" "$endpoint_field")"
    if [[ -z "$endpoint" ]]; then
      echo "FAIL: [$name] could not find a $endpoint_field control for the capture" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: pool triage through the page ---
name="pool-through-page"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "buy milk")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  endpoint="$(qa_json_field "$controls" pool_endpoint)"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the pool-triage control" >&2
    FAILURES=1
  else
    qa_triage_form "$endpoint" "kind=pool&life_area=Work"
    if [[ "$STATUS" -ge 300 && "$STATUS" -lt 400 ]]; then
      echo "FAIL: [$name] pool triage redirected the browser (status $STATUS)" >&2
      FAILURES=1
    fi
    page="$(qa_get_inbox)"
    inbox="$(qa_html_section "$page" captures)"
    tasks="$(qa_html_section "$page" tasks)"
    if [[ "$inbox" == *"buy milk"* ]]; then
      echo "FAIL: [$name] \"buy milk\" still appears in the inbox after pool triage" >&2
      FAILURES=1
    fi
    if [[ "$tasks" != *"buy milk"* ]]; then
      echo "FAIL: [$name] \"buy milk\" does not appear in the task list after pool triage" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: committed rejection through the page ---
name="committed-rejected"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "call the dentist")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  endpoint="$(qa_json_field "$controls" committed_endpoint)"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the committed-triage control" >&2
    FAILURES=1
  else
    # deadline omitted; deadline_type and priority supplied.
    qa_triage_form "$endpoint" "kind=committed&deadline_type=hard&priority=P1"
    if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
      echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
      FAILURES=1
    fi
    if [[ "$BODY" != *"deadline is required"* ]]; then
      echo "FAIL: [$name] expected the response to name deadline as required, got:
$BODY" >&2
      FAILURES=1
    fi
    task_count="$(qa_task_count)"
    if [[ "$task_count" != "0" ]]; then
      echo "FAIL: [$name] expected the task list to still be empty, found $task_count row(s)" >&2
      FAILURES=1
    fi
    if qa_capture_untriaged "$capture_id"; then
      : # expected: still untriaged
    else
      echo "FAIL: [$name] expected the capture to still be untriaged" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the committed form's closed choices ---
name="committed-closed-choices"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "call the dentist")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  deadline_types="$(python3 -c 'import json,sys; print(",".join(json.loads(sys.argv[1]).get("deadline_type_options", [])))' "$controls")"
  priorities="$(python3 -c 'import json,sys; print(",".join(json.loads(sys.argv[1]).get("priority_options", [])))' "$controls")"
  if [[ "$deadline_types" != "hard,soft" ]]; then
    echo "FAIL: [$name] expected the deadline type choices to be exactly hard,soft, got \"$deadline_types\"" >&2
    FAILURES=1
  fi
  if [[ "$priorities" != "P1,P2,P3,P4" ]]; then
    echo "FAIL: [$name] expected the priority choices to be exactly P1,P2,P3,P4, got \"$priorities\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: quota rejection through the page ---
name="quota-rejected"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "go to the gym")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  endpoint="$(qa_json_field "$controls" quota_endpoint)"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the quota-triage control" >&2
    FAILURES=1
  else
    # target_count omitted; target_minutes_each and period supplied.
    qa_triage_form "$endpoint" "kind=quota&target_minutes_each=45&period=week"
    if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
      echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
      FAILURES=1
    fi
    if [[ "$BODY" != *"target_count is required"* ]]; then
      echo "FAIL: [$name] expected the response to name target_count as required, got:
$BODY" >&2
      FAILURES=1
    fi
    task_count="$(qa_task_count)"
    if [[ "$task_count" != "0" ]]; then
      echo "FAIL: [$name] expected the task list to still be empty, found $task_count row(s)" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile capture text stays escaped in the task list ---
name="hostile-text-in-tasks"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "<script>alert('boom')</script>")"
  controls="$(qa_extract_controls "$(qa_get_inbox)" "$capture_id")"
  endpoint="$(qa_json_field "$controls" pool_endpoint)"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the pool-triage control" >&2
    FAILURES=1
  else
    qa_triage_form "$endpoint" "kind=pool&life_area=Work"
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] setup triage returned status $STATUS, expected 201" >&2
      FAILURES=1
    fi
    tasks="$(qa_html_section "$(qa_get_inbox)" tasks)"
    if [[ "$tasks" == *"<script>"* ]]; then
      echo "FAIL: [$name] task list contains an unescaped <script> tag" >&2
      FAILURES=1
    fi
    if [[ "$tasks" != *"boom"* ]]; then
      echo "FAIL: [$name] task list does not contain the word \"boom\" -- content may have been stripped instead of escaped" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: triage_from_page"
