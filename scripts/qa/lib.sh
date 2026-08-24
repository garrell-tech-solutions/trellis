#!/usr/bin/env bash
# Shared helpers for scripts/qa/*.sh: server lifecycle and HTTP/durable-state
# helpers common to every QA procedure that drives the running trellis server
# over its HTTP interface only, inspecting persisted state via a read-only
# sqlite3 query -- never through a project-internal API.

qa_free_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'
}

qa_wait_ready() {
  local port="$1"
  for _ in $(seq 1 50); do
    if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then
      exec 3<&- 3>&-
      return 0
    fi
    sleep 0.1
  done
  return 1
}

# Starts the trellis server against a database (fresh, or an existing file to
# restart against) and waits until it is reachable. Sets DB_PATH, ADDR and
# SERVER_PID. now_iso, when given, is passed as --now so the server starts
# believing it is that instant; the clock then advances normally from there
# (stats_ratio's "--now offsets the clock, it does not stop it"). now_iso
# applies to this process only -- pass it again on every restart that needs
# it, per qa/stats_ratio.md.
qa_start_server() {
  local bin="$1" db_path="$2" log_file="$3" now_iso="${4:-}" port
  DB_PATH="$db_path"
  port="$(qa_free_port)"
  ADDR="127.0.0.1:$port"
  if [[ -n "$now_iso" ]]; then
    "$bin" serve --db "$DB_PATH" --addr "$ADDR" --now "$now_iso" >"$log_file" 2>&1 &
  else
    "$bin" serve --db "$DB_PATH" --addr "$ADDR" >"$log_file" 2>&1 &
  fi
  SERVER_PID=$!
  if ! qa_wait_ready "$port"; then
    echo "server never became reachable at $ADDR" >&2
    cat "$log_file" >&2
    return 1
  fi
}

qa_stop_server() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  SERVER_PID=""
}

# Submits a capture over HTTP and prints its id. The capture endpoint's
# response carries no identifier, so the row is located by its raw text --
# safe because each scenario starts from a fresh, empty database. raw_text is
# JSON-encoded and SQL-escaped properly, so arbitrary text (quotes, hostile
# markup) is safe to pass.
qa_submit_capture() {
  local raw_text="$1" sql_escaped
  curl -s -o /dev/null -X POST "http://$ADDR/captures" \
    -H 'content-type: application/json' \
    -d "$(python3 -c 'import json,sys; print(json.dumps({"raw_text": sys.argv[1], "source": "web"}))' "$raw_text")"
  sql_escaped="${raw_text//\'/\'\'}"
  sqlite3 "$DB_PATH" "SELECT id FROM captures WHERE raw_text = '$sql_escaped';"
}

# POSTs a form-encoded triage submission (as a page triage control would)
# and sets STATUS and BODY. Shared by every script that drives a triage form
# directly rather than through lib.sh's JSON qa_triage.
qa_triage_form() {
  local endpoint="$1" data="$2" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "$data")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# POSTs a triage body for capture_id and sets STATUS and BODY.
qa_triage() {
  local capture_id="$1" body="$2" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/captures/$capture_id/triage" \
    -H 'content-type: application/json' \
    -d "$body")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# Prints a string field from a JSON object body, or nothing if absent, null,
# non-string, or the body does not parse.
qa_json_field() {
  local body="$1" field="$2"
  python3 -c '
import json, sys
try:
    obj = json.loads(sys.argv[1])
except ValueError:
    print("")
    sys.exit()
value = obj.get(sys.argv[2])
print(value if isinstance(value, str) else "")
' "$body" "$field"
}

# Prints a field's raw JSON-encoded value from a JSON object body (e.g. `7`,
# `null`, `"someday"`), or nothing if the field is absent or the body does
# not parse. Unlike qa_json_field, this is not string-only -- use it when the
# field may legitimately carry a non-string value.
qa_json_raw_field() {
  local body="$1" field="$2"
  python3 -c '
import json, sys
try:
    obj = json.loads(sys.argv[1])
except ValueError:
    sys.exit()
if sys.argv[2] in obj:
    print(json.dumps(obj[sys.argv[2]]))
' "$body" "$field"
}

qa_task_count() {
  sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM tasks;'
}

# Extracts the markup inside <li id="capture-row-ID">...</li> from a
# rendered page -- or from a response that already is that li, such as the
# quick-add box's own reply -- or prints nothing if the row is not found.
# Shared by every script that reads a capture row's triage or dismiss
# controls from the page's own markup rather than assuming their shape.
qa_capture_row_block() {
  local page="$1" capture_id="$2"
  python3 -c '
import re, sys
page, capture_id = sys.argv[1], sys.argv[2]
m = re.search(r"<li id=\"capture-row-" + re.escape(capture_id) + r"\"[^>]*>(.*?)</li>", page, re.S)
print(m.group(1) if m else "")
' "$page" "$capture_id"
}

# The hx-post endpoint of whichever <form> in `block` contains `marker`
# (e.g. "value=\"pool\"" for a pool triage control, ">Archive<" for an
# archive control), or "" if none matches. The shared primitive behind
# qa_row_control_endpoint and qa_life_area_control_endpoint, which differ
# only in how they find their block.
qa_block_control_endpoint() {
  local block="$1" marker="$2"
  python3 -c '
import re, sys
block, marker = sys.argv[1], sys.argv[2]
for form in re.findall(r"<form\b[^>]*>.*?</form>", block, re.S):
    if marker not in form:
        continue
    hx = re.search(r"hx-post=\"([^\"]+)\"", form)
    print(hx.group(1) if hx else "")
    sys.exit()
print("")
' "$block" "$marker"
}

# The hx-post endpoint of the control in capture_id's row whose <form>
# contains `marker` (e.g. "value=\"pool\"" for the pool triage control,
# ">Dismiss<" for the dismiss control), read from the page's own markup --
# not assumed. Prints nothing if the row or a matching control cannot be
# found.
qa_row_control_endpoint() {
  local page="$1" capture_id="$2" marker="$3"
  qa_block_control_endpoint "$(qa_capture_row_block "$page" "$capture_id")" "$marker"
}

# The hx-post endpoint of the <form class="fields"...> in `block` whose
# hidden `kind` input names `kind` (#119: once a row's kind button has
# opened its panel, the panel's own triage form is what this finds --
# qa_block_control_endpoint would find the kind BUTTON's own endpoint
# instead, since that form appears first in document order and also
# carries `value="{kind}"`). "" if no such form is open in block.
qa_open_panel_endpoint() {
  local block="$1" kind="$2"
  python3 -c '
import re, sys
block, kind = sys.argv[1], sys.argv[2]
for form in re.findall(r"<form class=\"fields\"[^>]*>.*?</form>", block, re.S):
    if "value=\"" + kind + "\"" in form:
        hx = re.search(r"hx-post=\"([^\"]+)\"", form)
        print(hx.group(1) if hx else "")
        sys.exit()
print("")
' "$block" "$kind"
}

# The markup inside <tag id="id_prefix-ID">...</tag> whose
# <span class="life-area-name"> exactly matches `name`, read from a rendered
# page -- or from a response that already is that element -- or "" if no
# row matches. Keyed by name rather than id because that is what every
# caller has in hand. Shared by every per-life-area row a page renders
# (life_area_row.html's <li>, free_time.html's <div>), so a third such row
# is one more call rather than one more near-identical parser.
qa_named_row_block() {
  local page="$1" tag="$2" id_prefix="$3" name="$4"
  python3 -c '
import re, sys
page, tag, id_prefix, name = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
pattern = r"<" + tag + r" id=\"" + id_prefix + r"\d+\">(.*?)</" + tag + r">"
for m in re.finditer(pattern, page, re.S):
    block = m.group(1)
    nm = re.search(r"<span class=\"life-area-name\">([^<]*)</span>", block)
    if nm and nm.group(1) == name:
        print(block)
        sys.exit()
print("")
' "$page" "$tag" "$id_prefix" "$name"
}

# The markup inside <li id="life-area-row-ID">...</li> whose
# <span class="life-area-name"> exactly matches `name`. life_area_row.html's
# guardrail markup means a row's first <form> is no longer reliably the one
# a caller wants, so control lookups go through this and
# qa_life_area_control_endpoint rather than assuming which form comes first.
qa_life_area_row_block() {
  qa_named_row_block "$1" li "life-area-row-" "$2"
}

# The hx-post endpoint of the control in name's life-area row whose <form>
# contains `marker` (e.g. ">Archive<", ">Add band<"), read from the page's
# own markup -- not assumed. Prints nothing if the row or a matching control
# cannot be found.
qa_life_area_control_endpoint() {
  local page="$1" name="$2" marker="$3"
  qa_block_control_endpoint "$(qa_life_area_row_block "$page" "$name")" "$marker"
}

# The state text a life area's own row shows: "no guardrail",
# "never scheduled - menu only", or "" if it carries bands instead (a row
# with bands shows the bands themselves, not a state sentence).
qa_life_area_state() {
  local page="$1" name="$2" block
  block="$(qa_life_area_row_block "$page" "$name")"
  python3 -c '
import re, sys
block = sys.argv[1]
m = re.search(r"<span>([^<]*)</span>", block)
print(m.group(1) if m else "")
' "$block"
}

# The guardrail bands listed on name's row, one label per line, in document
# order -- "" (no lines) if the row carries none.
qa_guardrail_band_labels() {
  local page="$1" name="$2" block
  block="$(qa_life_area_row_block "$page" "$name")"
  python3 -c '
import re, sys
block = sys.argv[1]
for m in re.finditer(r"<div>([^<]*)\n<form", block):
    print(m.group(1).strip())
' "$block"
}

# POSTs name's guardrail band form (weekday checkboxes on "<days>", a
# comma-separated list of Mon/Tue/.../Sun) and sets STATUS and BODY.
qa_save_guardrail_band() {
  local page="$1" name="$2" days="$3" start="$4" end="$5" endpoint data response
  endpoint="$(qa_life_area_control_endpoint "$page" "$name" '>Add band<')"
  data="start=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' "$start")"
  data="$data&end=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' "$end")"
  IFS=',' read -ra day_list <<< "$days"
  for day in "${day_list[@]}"; do
    day="$(echo "$day" | tr -d ' ' | tr '[:upper:]' '[:lower:]')"
    [[ -z "$day" ]] && continue
    data="$data&$day=on"
  done
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "$data")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# POSTs name's pool-only form and sets STATUS and BODY.
qa_save_pool_only() {
  local page="$1" name="$2" endpoint response
  endpoint="$(qa_life_area_control_endpoint "$page" "$name" '>Save<')"
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "pool_only=on")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# The #timezone fragment's own hx-post endpoint, read from the page's
# markup -- not assumed.
qa_timezone_endpoint() {
  local page="$1"
  python3 -c '
import re, sys
page = sys.argv[1]
m = re.search(r"<div id=\"timezone\">.*?<form hx-post=\"([^\"]+)\"", page, re.S)
print(m.group(1) if m else "")
' "$page"
}

# POSTs the timezone control with zone and sets STATUS and BODY.
qa_set_timezone() {
  local endpoint="$1" zone="$2" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "zone=$zone")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
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

# The exceptions form's own hx-post endpoint, read from the free time
# page's markup -- not assumed.
qa_exceptions_add_endpoint() {
  local page="$1"
  python3 -c '
import re, sys
page = sys.argv[1]
m = re.search(r"<form hx-post=\"([^\"]+)\" hx-target=\"#exceptions-list\"", page)
print(m.group(1) if m else "")
' "$page"
}

# Extracts the contents of <ul id="html_id">...</ul> from a page, or prints
# nothing if not found. The inbox page renders both the capture list and the
# task list on one page (triage-from-page), so an assertion about one must
# not accidentally match text that legitimately belongs to the other.
qa_html_section() {
  local page="$1" html_id="$2"
  python3 -c '
import re, sys
m = re.search(r"<ul id=\"" + re.escape(sys.argv[2]) + r"\">(.*?)</ul>", sys.argv[1], re.S)
print(m.group(1) if m else "")
' "$page" "$html_id"
}

# The text strictly between the first occurrence of `start` and the next
# occurrence of `end` after it, or "" if `start` is absent or `end` never
# follows it. General-purpose scoping for the div/span-wrapped fields a
# view-model template renders one value into (e.g. `<div class="foo">value
# </div>`), shared by every script that needs one without a full HTML
# parser.
qa_between() {
  local text="$1" start="$2" end="$3"
  python3 -c '
import sys
text, start, end = sys.argv[1], sys.argv[2], sys.argv[3]
i = text.find(start)
if i == -1:
    print("")
    sys.exit()
i += len(start)
j = text.find(end, i)
print(text[i:j] if j != -1 else "")
' "$text" "$start" "$end"
}

# The <ul class="committed-rows">...</ul> content of a committed-screen
# page, or the whole page if the marker is absent -- shared by every
# script that reads the Committed screen (committed_screen.sh,
# committed_date.sh).
qa_committed_rows_scope() {
  qa_between "$1" '<ul class="committed-rows">' '</ul>'
}

# The <li class="committed-row...">...</li> block whose text contains
# needle, scoped to the committed-rows list, or "" if none matches.
# Creates one pool task, tagged if a second argument is given, and returns
# nothing -- callers that need the id use qa_submit_capture themselves.
# Shared by mark_done.sh and pool_screen.sh.
qa_pool_task() {
  local raw_text="$1" tag="${2:-}" capture_id body
  capture_id="$(qa_submit_capture "$raw_text")"
  if [[ -n "$tag" ]]; then
    body="$(python3 -c 'import json,sys; print(json.dumps({"kind":"pool","context_tag":sys.argv[1]}))' "$tag")"
  else
    body='{"kind":"pool"}'
  fi
  qa_triage "$capture_id" "$body"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: setup -- triaging \"$raw_text\" as pool returned status $STATUS" >&2
    FAILURES=1
  fi
}

# The <ul class="loose">...</ul> content of a Pool-screen page. Shared by
# mark_done.sh and pool_screen.sh.
qa_loose_section() {
  qa_between "$1" '<ul class="loose">' '</ul>'
}

# The <li class="loose-item">...</li> block containing needle, within the
# loose section already extracted by qa_loose_section. Shared by
# mark_done.sh and pool_screen.sh.
qa_loose_row_containing() {
  local loose_section="$1" needle="$2"
  python3 -c '
import re, sys
section, needle = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<li class=\"loose-item\">(.*?)</li>", section, re.S):
    if needle in m.group(1):
        print(m.group(1))
        sys.exit()
' "$loose_section" "$needle"
}

qa_committed_row_for() {
  local page="$1" needle="$2" scope
  scope="$(qa_committed_rows_scope "$page")"
  python3 -c '
import re, sys
scope, needle = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<li class=\"committed-row[^\"]*\">(.*?)</li>", scope, re.S):
    if needle in m.group(1):
        print(m.group(1))
        sys.exit()
' "$scope" "$needle"
}

# True (exit 0) if capture_id is still present and untriaged. Migration 0005
# renamed captures.triaged_at to left_inbox_at (it now means "left the
# inbox", by either triage or dismissal, not just triage) -- this still
# means "untriaged" for every caller here because none of them dismiss the
# capture they are checking.
qa_capture_untriaged() {
  local capture_id="$1"
  [[ -z "$(sqlite3 "$DB_PATH" "SELECT IFNULL(left_inbox_at,'') FROM captures WHERE id = $capture_id;")" ]]
}

# Setup shared by every scenario/example row across the triage QA scripts:
# fresh db, running server, an empty task list, one untriaged capture with
# raw_text. Sets DB_PATH, ADDR, SERVER_PID and CAPTURE_ID. Callers must have
# a FAILURES variable in scope (the convention every scripts/qa/*.sh uses).
qa_setup_scenario() {
  local bin="$1" name="$2" log_dir="$3" raw_text="$4"
  if ! qa_start_server "$bin" "$log_dir/$name.sqlite" "$log_dir/$name.log"; then
    FAILURES=1
    return 1
  fi

  local task_count
  task_count="$(qa_task_count)"
  if [[ "$task_count" != "0" ]]; then
    echo "FAIL: [$name] expected an empty task list before triage, found $task_count" >&2
    FAILURES=1
  fi

  CAPTURE_ID="$(qa_submit_capture "$raw_text")"
  if [[ -z "$CAPTURE_ID" ]]; then
    echo "FAIL: [$name] no capture row found for \"$raw_text\"" >&2
    FAILURES=1
    return 1
  fi

  if ! qa_capture_untriaged "$CAPTURE_ID"; then
    echo "FAIL: [$name] expected the capture to still be untriaged after setup" >&2
    FAILURES=1
    return 1
  fi
}

# Asserts durable state is unchanged after a rejected triage: the tasks
# table stays empty and CAPTURE_ID stays untriaged. Callers must have a
# FAILURES variable in scope.
qa_assert_durable_state_unchanged() {
  local name="$1" task_count
  task_count="$(qa_task_count)"
  if [[ "$task_count" != "0" ]]; then
    echo "FAIL: [$name] expected the task list to still be empty, found $task_count row(s)" >&2
    FAILURES=1
  fi
  if ! qa_capture_untriaged "$CAPTURE_ID"; then
    echo "FAIL: [$name] expected the capture to still be untriaged" >&2
    FAILURES=1
  fi
}

# Asserts a rejection after a qa_triage call: a client error (4xx) whose
# body names field_name under json_key ("invalid_field", "missing_field" or
# "unknown_kind"), plus qa_assert_durable_state_unchanged. Callers must have
# a FAILURES variable in scope.
qa_assert_rejected_naming() {
  local name="$1" json_key="$2" field_name="$3" named
  if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
    echo "FAIL: [$name] expected a client error, got status $STATUS" >&2
    FAILURES=1
  fi
  named="$(qa_json_field "$BODY" "$json_key")"
  if [[ "$named" != "$field_name" ]]; then
    echo "FAIL: [$name] expected the rejection to report $json_key=\"$field_name\", got \"$named\" (body: $BODY)" >&2
    FAILURES=1
  fi
  qa_assert_durable_state_unchanged "$name"
}

# The /life-areas page's own add-control endpoint, read from its markup
# rather than assumed. Shared by life_areas.sh and life_area_triage.sh.
qa_life_areas_add_endpoint() {
  local page="$1"
  python3 -c '
import re, sys
m = re.search(r"<form hx-post=\"([^\"]+)\" hx-target=\"#life-areas-list\"", sys.argv[1])
print(m.group(1) if m else "")
' "$page"
}

# Submits the add-life-area control (form-encoded) and sets STATUS and BODY.
# --data-urlencode (not a hand-built query string) so a name carrying spaces
# or markup round-trips exactly as typed.
qa_add_life_area() {
  local endpoint="$1" name="$2" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "name=$name")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}
