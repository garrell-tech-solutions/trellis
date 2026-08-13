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

# Starts the trellis server against a fresh database and waits until it is
# reachable. Sets DB_PATH, ADDR and SERVER_PID.
qa_start_server() {
  local bin="$1" db_path="$2" log_file="$3" port
  DB_PATH="$db_path"
  port="$(qa_free_port)"
  ADDR="127.0.0.1:$port"
  "$bin" serve --db "$DB_PATH" --addr "$ADDR" >"$log_file" 2>&1 &
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

qa_task_count() {
  sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM tasks;'
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
