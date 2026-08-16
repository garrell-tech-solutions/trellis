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

# True (exit 0) if capture_id is still present and untriaged.
qa_capture_untriaged() {
  local capture_id="$1"
  [[ -z "$(sqlite3 "$DB_PATH" "SELECT IFNULL(triaged_at,'') FROM captures WHERE id = $capture_id;")" ]]
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
