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
  local page="$1" capture_id="$2" block
  block="$(qa_capture_row_block "$page" "$capture_id")"
  python3 -c '
import re, json, sys
block = sys.argv[1]

result = {}
for form in re.findall(r"<form\b[^>]*>.*?</form>", block, re.S):
    if "value=\"pool\"" not in form:
        continue
    hx = re.search(r"hx-post=\"([^\"]+)\"", form)
    result["endpoint"] = hx.group(1) if hx else None
    la = re.search(r"<select name=\"life_area\">(.*?)</select>", form, re.S)
    result["life_area_options"] = re.findall(r"<option value=\"([^\"]+)\"", la.group(1)) if la else []

print(json.dumps(result))
' "$block"
}

# For capture_id's pool/committed/quota triage forms: the form's hx-post
# endpoint, and whether its life_area <select> preselects anything -- the
# value of its first <option> (the placeholder) and whether any <option>
# anywhere in the select carries `selected`. A blank first option with no
# `selected` anywhere is what "preselects nothing" means -- absent an
# explicit `selected`, a browser defaults to the first <option> in document
# order, so the placeholder being first is what makes that default carry no
# life area.
qa_extract_life_area_shapes() {
  local page="$1" capture_id="$2" block
  block="$(qa_capture_row_block "$page" "$capture_id")"
  python3 -c '
import re, json, sys
block = sys.argv[1]

result = {}
for kind in ("pool", "committed", "quota"):
    form_m = re.search(r"<form\b[^>]*>(?:(?!</form>).)*?value=\"" + kind + r"\"(?:(?!</form>).)*?</form>", block, re.S)
    if not form_m:
        result[kind] = None
        continue
    form = form_m.group(0)
    hx = re.search(r"hx-post=\"([^\"]+)\"", form)
    select_m = re.search(r"<select name=\"life_area\">(.*?)</select>", form, re.S)
    if not select_m:
        result[kind] = None
        continue
    options = re.findall(r"<option value=\"([^\"]*)\"([^>]*)>", select_m.group(1))
    result[kind] = {
        "endpoint": hx.group(1) if hx else None,
        "first_option_value": options[0][0] if options else None,
        "any_selected": any("selected" in attrs for _, attrs in options),
        "life_area_options": [value for value, _ in options if value != ""],
    }

print(json.dumps(result))
' "$block"
}

# Reads one field for `kind` out of qa_extract_life_area_shapes' JSON.
# Prints "MISSING" if the form itself was not found, so a caller comparing
# against an expected value fails loudly instead of matching an empty string.
qa_shape_field() {
  local shapes="$1" kind="$2" field="$3"
  python3 -c '
import json, sys
d = json.loads(sys.argv[1]).get(sys.argv[2])
if d is None:
    print("MISSING")
    sys.exit()
value = d.get(sys.argv[3])
print(",".join(value) if isinstance(value, list) else value)
' "$shapes" "$kind" "$field"
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

# --- Procedure: a life area is optional ---
# T-life-area-required-at-triage was superseded by
# D-context-tags-are-the-taxonomy (#82): all three kinds now succeed with no
# life area, over the JSON transport.
name="life-area-optional"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  cid_pool="$(qa_submit_capture "buy milk")"
  qa_triage "$cid_pool" '{"kind":"pool"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name-pool] expected a pool triage with no life area to succeed, got status $STATUS (body: $BODY)" >&2
    FAILURES=1
  fi

  cid_committed="$(qa_submit_capture "call the dentist")"
  qa_triage "$cid_committed" '{"kind":"committed","deadline":"2026-08-20T17:00:00Z","deadline_type":"hard","priority":"P1","estimated_minutes":180}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name-committed] expected a committed triage with no life area to succeed, got status $STATUS (body: $BODY)" >&2
    FAILURES=1
  fi

  cid_quota="$(qa_submit_capture "go to the gym")"
  qa_triage "$cid_quota" '{"kind":"quota","target_count":3,"target_minutes_each":45,"period":"week"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name-quota] expected a quota triage with no life area to succeed, got status $STATUS (body: $BODY)" >&2
    FAILURES=1
  fi

  task_count="$(qa_task_count)"
  if [[ "$task_count" != "3" ]]; then
    echo "FAIL: [$name] expected three tasks after three life-area-less triages, found $task_count" >&2
    FAILURES=1
  fi
  for cid in "$cid_pool" "$cid_committed" "$cid_quota"; do
    life_area_id="$(sqlite3 "$DB_PATH" "SELECT IFNULL(life_area_id,'NULL') FROM tasks WHERE capture_id = $cid;")"
    if [[ "$life_area_id" != "NULL" ]]; then
      echo "FAIL: [$name] expected capture $cid's task to have no life area (not a defaulted one), got life_area_id=$life_area_id" >&2
      FAILURES=1
    fi
  done
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
  archive_endpoint="$(qa_life_area_control_endpoint "$life_areas_page" "Learning" ">Archive<")"
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

# --- Procedure: the picker chooses nothing for the user ---
name="picker-preselects-nothing"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "buy milk")"
  shapes="$(qa_extract_life_area_shapes "$(qa_get_inbox)" "$capture_id")"
  for kind in pool committed quota; do
    first="$(qa_shape_field "$shapes" "$kind" first_option_value)"
    if [[ "$first" != "" ]]; then
      echo "FAIL: [$name] expected the $kind form's life area picker to start on a blank placeholder, first option was \"$first\"" >&2
      FAILURES=1
    fi
    selected="$(qa_shape_field "$shapes" "$kind" any_selected)"
    if [[ "$selected" != "False" ]]; then
      echo "FAIL: [$name] expected no <option> in the $kind form's life area picker to carry selected" >&2
      FAILURES=1
    fi
    options="$(qa_shape_field "$shapes" "$kind" life_area_options)"
    if [[ "$options" != "Work,Fitness,Learning,Family,Home" ]]; then
      echo "FAIL: [$name] expected the $kind form to offer exactly Work,Fitness,Learning,Family,Home (the placeholder is not a life area), got: $options" >&2
      FAILURES=1
    fi
  done

  quick_add_response="$(curl -s -X POST "http://$ADDR/captures" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "raw_text=call the dentist" --data-urlencode "source=web")"
  quick_add_id="$(sqlite3 "$DB_PATH" "SELECT id FROM captures WHERE raw_text = 'call the dentist';")"
  quick_shapes="$(qa_extract_life_area_shapes "$quick_add_response" "$quick_add_id")"
  first="$(qa_shape_field "$quick_shapes" pool first_option_value)"
  selected="$(qa_shape_field "$quick_shapes" pool any_selected)"
  if [[ "$first" != "" || "$selected" != "False" ]]; then
    echo "FAIL: [$name] expected the quick-added row's pool picker to preselect nothing (first=\"$first\", any_selected=$selected)" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: submitting the page with the placeholder still selected succeeds ---
# features/life_area_triage.feature's own life-area-triage-page-optional-09
# inverted this scenario under #82, the same way life-area-optional above
# did for the JSON transport: `life_area` present-but-empty (the placeholder,
# never touched) is T-empty-equals-absent's case, and an absent life area is
# no longer rejected either way.
name="page-with-empty-life-area-succeeds"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  pool_id="$(qa_submit_capture "buy milk")"
  pool_endpoint="$(qa_shape_field "$(qa_extract_life_area_shapes "$(qa_get_inbox)" "$pool_id")" pool endpoint)"
  qa_triage_form "$pool_endpoint" "kind=pool&life_area="
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name-pool] expected 201, got $STATUS (body: $BODY)" >&2
    FAILURES=1
  fi

  committed_id="$(qa_submit_capture "call the dentist")"
  committed_endpoint="$(qa_shape_field "$(qa_extract_life_area_shapes "$(qa_get_inbox)" "$committed_id")" committed endpoint)"
  qa_triage_form "$committed_endpoint" "kind=committed&deadline=2026-08-20T17%3A00%3A00Z&deadline_type=hard&priority=P1&estimated_minutes=180&life_area="
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name-committed] expected 201, got $STATUS (body: $BODY)" >&2
    FAILURES=1
  fi

  quota_id="$(qa_submit_capture "go to the gym")"
  quota_endpoint="$(qa_shape_field "$(qa_extract_life_area_shapes "$(qa_get_inbox)" "$quota_id")" quota endpoint)"
  qa_triage_form "$quota_endpoint" "kind=quota&target_count=3&target_minutes_each=45&period=week&life_area="
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name-quota] expected 201, got $STATUS (body: $BODY)" >&2
    FAILURES=1
  fi

  task_count="$(qa_task_count)"
  if [[ "$task_count" != "3" ]]; then
    echo "FAIL: [$name] expected three tasks after three empty-life-area page submissions, found $task_count" >&2
    FAILURES=1
  fi
  for cid in "$pool_id" "$committed_id" "$quota_id"; do
    if qa_capture_untriaged "$cid"; then
      echo "FAIL: [$name] expected capture $cid to have left the inbox" >&2
      FAILURES=1
    fi
    life_area_id="$(sqlite3 "$DB_PATH" "SELECT IFNULL(life_area_id,'NULL') FROM tasks WHERE capture_id = $cid;")"
    if [[ "$life_area_id" != "NULL" ]]; then
      echo "FAIL: [$name] expected capture $cid's task to have no life area, got life_area_id=$life_area_id" >&2
      FAILURES=1
    fi
  done
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
