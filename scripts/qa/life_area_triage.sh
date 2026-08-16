#!/usr/bin/env bash
# Executable QA procedure: qa/life_area_triage.md (covers
# features/life_area_triage.feature). Drives the running server through its
# HTTP interface only -- GET /, its triage controls (endpoints read from the
# page's own markup, not assumed), and GET /life-areas for setup -- and
# inspects persisted state via a read-only sqlite3 query, never a
# project-internal API.
#
# qa/life_area_triage.md's by-hand browser walkthrough lives in
# qa/life_areas.md (it covers both slices) and life_areas.sh's header notes
# it was performed manually this QA cycle; it is not repeated here. Every
# procedure below is curl-only, per the procedure document itself.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-life-area-triage"
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

# The pool triage control's hx-post endpoint and its life_area <select>
# options, for capture_id, read from the page's own markup.
qa_extract_pool_control() {
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
    if "value=\"pool\"" not in form:
        continue
    hx = re.search(r"hx-post=\"([^\"]+)\"", form)
    result["endpoint"] = hx.group(1) if hx else None
    la = re.search(r"<select name=\"life_area\">(.*?)</select>", form, re.S)
    result["life_area_options"] = re.findall(r"<option value=\"([^\"]+)\"", la.group(1)) if la else []

print(json.dumps(result))
' "$page" "$capture_id"
}

# POSTs a form-encoded triage submission and sets STATUS and BODY.
qa_triage_form() {
  local endpoint="$1" data="$2" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "$data")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# --- Procedure: triage offers every life area ---
name="offers-every-life-area"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "sketch the landing page")"
  add_endpoint="$(qa_life_areas_add_endpoint "$(curl -s "http://$ADDR/life-areas")")"
  qa_add_life_area "$add_endpoint" "Side project"
  control="$(qa_extract_pool_control "$(qa_get_inbox)" "$capture_id")"
  options="$(python3 -c 'import json,sys; print(",".join(json.loads(sys.argv[1]).get("life_area_options", [])))' "$control")"
  if [[ "$options" != "Work,Fitness,Learning,Family,Home,Side project" ]]; then
    echo "FAIL: [$name] expected exactly the six choices Work,Fitness,Learning,Family,Home,Side project, got: $options" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the tag follows the task ---
name="tag-follows-task"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  landing_id="$(qa_submit_capture "sketch the landing page")"
  milk_id="$(qa_submit_capture "buy milk")"
  qa_triage "$landing_id" '{"kind":"pool","life_area":"Learning"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging \"sketch the landing page\" into Learning returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_triage "$milk_id" '{"kind":"pool","life_area":"Home"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging \"buy milk\" into Home returned status $STATUS" >&2
    FAILURES=1
  fi
  tasks="$(qa_html_section "$(qa_get_inbox)" tasks)"
  if [[ "$tasks" != *"sketch the landing page (Learning)"* ]]; then
    echo "FAIL: [$name] expected the task list to tag \"sketch the landing page\" with Learning, got:
$tasks" >&2
    FAILURES=1
  fi
  if [[ "$tasks" != *"buy milk (Home)"* ]]; then
    echo "FAIL: [$name] expected the task list to tag \"buy milk\" with Home, got:
$tasks" >&2
    FAILURES=1
  fi
  learning_id="$(sqlite3 "$DB_PATH" "SELECT id FROM life_areas WHERE name = 'Learning';")"
  home_id="$(sqlite3 "$DB_PATH" "SELECT id FROM life_areas WHERE name = 'Home';")"
  row_learning="$(sqlite3 "$DB_PATH" "SELECT life_area_id FROM tasks WHERE capture_id = $landing_id;")"
  row_home="$(sqlite3 "$DB_PATH" "SELECT life_area_id FROM tasks WHERE capture_id = $milk_id;")"
  if [[ "$row_learning" != "$learning_id" ]]; then
    echo "FAIL: [$name] expected the landing-page task's life_area_id to be Learning's id ($learning_id), got $row_learning" >&2
    FAILURES=1
  fi
  if [[ "$row_home" != "$home_id" ]]; then
    echo "FAIL: [$name] expected the buy-milk task's life_area_id to be Home's id ($home_id), got $row_home" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a life area is required ---
name="life-area-required"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  CAPTURE_ID="$(qa_submit_capture "buy milk")"
  qa_triage "$CAPTURE_ID" '{"kind":"pool"}'
  qa_assert_rejected_naming "$name-pool" missing_field life_area

  CAPTURE_ID="$(qa_submit_capture "call the dentist")"
  qa_triage "$CAPTURE_ID" '{"kind":"committed","deadline":"2026-08-20T17:00:00Z","deadline_type":"hard","priority":"P1"}'
  qa_assert_rejected_naming "$name-committed" missing_field life_area

  CAPTURE_ID="$(qa_submit_capture "go to the gym")"
  qa_triage "$CAPTURE_ID" '{"kind":"quota","target_count":3,"target_minutes_each":45,"period":"week"}'
  qa_assert_rejected_naming "$name-quota" missing_field life_area

  task_count="$(qa_task_count)"
  if [[ "$task_count" != "0" ]]; then
    echo "FAIL: [$name] expected no tasks after three life-area-less rejections, found $task_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a life area that does not exist is refused ---
name="unknown-life-area"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  CAPTURE_ID="$(qa_submit_capture "buy milk")"
  qa_triage "$CAPTURE_ID" '{"kind":"pool","life_area":"Gardening"}'
  if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
    echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
    FAILURES=1
  fi
  named="$(qa_json_field "$BODY" unknown_life_area)"
  if [[ "$named" != "Gardening" ]]; then
    echo "FAIL: [$name] expected unknown_life_area=\"Gardening\", got \"$named\" (body: $BODY)" >&2
    FAILURES=1
  fi
  qa_assert_durable_state_unchanged "$name"
  ghost_row_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM life_areas WHERE name = 'Gardening';")"
  if [[ "$ghost_row_count" != "0" ]]; then
    echo "FAIL: [$name] naming an unknown life area must not create one, found $ghost_row_count row(s)" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: an archived life area is refused at the boundary, not merely hidden ---
name="archived-refused-at-boundary"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  CAPTURE_ID="$(qa_submit_capture "buy milk")"
  life_areas_page="$(curl -s "http://$ADDR/life-areas")"
  archive_endpoint="$(python3 -c '
import re, sys
page = sys.argv[1]
for m in re.finditer(r"<li id=\"life-area-row-\d+\">\n(.*?)\n</li>", page, re.S):
    block = m.group(1)
    if block.split("<form", 1)[0].strip() != "Learning":
        continue
    hx = re.search(r"hx-post=\"([^\"]+)\"", block)
    print(hx.group(1) if hx else "")
    break
else:
    print("")
' "$life_areas_page")"
  curl -s -o /dev/null -X POST "http://$ADDR$archive_endpoint"
  # By hand, not through the picker (which no longer offers Learning) --
  # exactly what the procedure requires: a request composed directly must
  # still be refused.
  qa_triage "$CAPTURE_ID" '{"kind":"pool","life_area":"Learning"}'
  if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
    echo "FAIL: [$name] triaging into an archived life area should be rejected, got status $STATUS" >&2
    FAILURES=1
  fi
  qa_assert_durable_state_unchanged "$name"
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a hostile life area name stays escaped on the task row ---
name="hostile-life-area-on-task-row"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  add_endpoint="$(qa_life_areas_add_endpoint "$(curl -s "http://$ADDR/life-areas")")"
  hostile_name="<script>alert('boom')</script>"
  qa_add_life_area "$add_endpoint" "$hostile_name"
  CAPTURE_ID="$(qa_submit_capture "buy milk")"
  triage_body="$(python3 -c 'import json,sys; print(json.dumps({"kind":"pool","life_area":sys.argv[1]}))' "$hostile_name")"
  qa_triage "$CAPTURE_ID" "$triage_body"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup triage into the hostile life area returned status $STATUS, expected 201" >&2
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
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the tag survives a restart ---
name="tag-survives-restart"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-1.log"; then
  qa_triage "$(qa_submit_capture "sketch the landing page")" '{"kind":"pool","life_area":"Learning"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup triage into Learning returned status $STATUS" >&2
    FAILURES=1
  fi
  tasks="$(qa_html_section "$(qa_get_inbox)" tasks)"
  if [[ "$tasks" != *"sketch the landing page (Learning)"* ]]; then
    echo "FAIL: [$name] expected the tag to render before a restart, got:
$tasks" >&2
    FAILURES=1
  fi
  qa_stop_server
  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log"; then
    tasks="$(qa_html_section "$(qa_get_inbox)" tasks)"
    if [[ "$tasks" != *"sketch the landing page (Learning)"* ]]; then
      echo "FAIL: [$name] expected the tag to still render after a restart, got:
$tasks" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: triage otherwise behaves identically ---
name="triage-otherwise-unchanged"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  CAPTURE_ID="$(qa_submit_capture "call the dentist")"
  qa_triage "$CAPTURE_ID" '{"kind":"committed","deadline_type":"hard","priority":"P1","life_area":"Work"}'
  qa_assert_rejected_naming "$name-committed" missing_field deadline

  CAPTURE_ID="$(qa_submit_capture "go to the gym")"
  qa_triage "$CAPTURE_ID" '{"kind":"quota","target_minutes_each":45,"period":"week","life_area":"Work"}'
  qa_assert_rejected_naming "$name-quota" missing_field target_count

  cid3="$(qa_submit_capture "buy milk")"
  qa_triage "$cid3" '{"kind":"pool","life_area":"Work"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a valid pool triage with a life area should succeed, got status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_inbox)"
  inbox="$(qa_html_section "$page" captures)"
  tasks="$(qa_html_section "$page" tasks)"
  if [[ "$inbox" == *"buy milk"* ]]; then
    echo "FAIL: [$name] \"buy milk\" should have left the inbox for the task list" >&2
    FAILURES=1
  fi
  if [[ "$tasks" != *"buy milk"* ]]; then
    echo "FAIL: [$name] \"buy milk\" should appear in the task list" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: life_area_triage"
