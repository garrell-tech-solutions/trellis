#!/usr/bin/env bash
# Executable QA procedure: qa/quota_triage_validation.md (covers
# features/quota_triage_validation.feature, and the half of the name guard
# that moved house from qa/quota_screen.md). Drives the running server
# through its HTTP interface only, and inspects persisted state via a
# read-only sqlite3 query -- never through a project-internal API. The
# capture endpoint's response carries no identifier, so (as in
# capture_endpoint.sh) the capture row is located by its raw text.
#
# Creates quotas by triaging captures, never by inserting rows -- the whole
# point of this slice is that triage is the only door.
#
# MUST FAIL, NEVER SKIP, if Chrome or playwright-core is unavailable for the
# browser half -- the same rule qa/trip_controls.md and qa/phone_layout.md
# already carry.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-quota-triage-validation"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

setup_scenario() { qa_setup_scenario "$BIN" "$1" "$TMP_DIR" "go to the gym"; }

qa_task_count_quotas() {
  sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM quotas;'
}

# --- Procedure: a required field with no usable value ---
run_missing() {
  local field="$1" mode="$2" # mode: omitted | empty
  local name="missing-$field-$mode"
  setup_scenario "$name" || return
  local body
  if [[ "$mode" == "omitted" ]]; then
    if [[ "$field" == "name" ]]; then
      body='{"kind":"quota","hours":"4"}'
    else
      body='{"kind":"quota","name":"Piano"}'
    fi
  else
    if [[ "$field" == "name" ]]; then
      body='{"kind":"quota","name":"","hours":"4"}'
    else
      body='{"kind":"quota","name":"Piano","hours":""}'
    fi
  fi
  qa_triage "$CAPTURE_ID" "$body"
  qa_assert_rejected_naming "$name" missing_field "$field"
  qa_stop_server
}
run_missing name omitted
run_missing name empty
run_missing hours omitted
run_missing hours empty

# T-empty-equals-absent: omitted and empty must report the identical body
# for the same field -- a direct side-by-side comparison, run fresh rather
# than reconstructed from the calls above (whose FAILURES writes, made
# inside a plain function call, are not lost the way they would be through
# a $(...) capture, but whose response bodies were never kept).
compare_omitted_vs_empty() {
  local field="$1" other_field="$2" other_value="$3"
  local name="compare-$field"
  setup_scenario "$name-omitted" || return
  local omitted_body empty_body
  qa_triage "$CAPTURE_ID" "$(printf '{"kind":"quota","%s":"%s"}' "$other_field" "$other_value")"
  omitted_body="$BODY"
  qa_stop_server

  setup_scenario "$name-empty" || return
  qa_triage "$CAPTURE_ID" "$(printf '{"kind":"quota","%s":"","%s":"%s"}' "$field" "$other_field" "$other_value")"
  empty_body="$BODY"
  qa_stop_server

  if [[ "$omitted_body" != "$empty_body" ]]; then
    echo "FAIL: [$name] expected omitted and empty \"$field\" to reject with the identical body, got:
  omitted: $omitted_body
  empty:   $empty_body" >&2
    FAILURES=1
  fi
}
compare_omitted_vs_empty name hours 4
compare_omitted_vs_empty hours name Piano

# --- Procedure: an hour target that is not a positive number of minutes ---
run_invalid_hours() {
  local hours="$1"
  local name="invalid-hours-$hours"
  setup_scenario "$name" || return
  qa_triage "$CAPTURE_ID" "$(printf '{"kind":"quota","name":"Piano","hours":"%s"}' "$hours")"
  qa_assert_rejected_naming "$name" invalid_field hours
  qa_stop_server
}
run_invalid_hours "0"
run_invalid_hours "-2"
run_invalid_hours "four"
run_invalid_hours "0.004"

# --- Procedure: the name guard, over HTTP ---
name="the-name-guard-over-http"
exact_warning='“Piano” already exists at 4 h a week. Log your time against that one, or give this a different name.'
for variant in "piano" "PIANO" "Pi-ano" "pi ano"; do
  if setup_scenario "$name-$variant"; then
    qa_triage "$CAPTURE_ID" '{"kind":"quota","name":"Piano","hours":"4"}'
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] setup -- triaging Piano at 4 hours returned $STATUS: $BODY" >&2
      FAILURES=1
    fi
    CAPTURE_ID="$(qa_submit_capture "another capture $variant")"
    qa_triage "$CAPTURE_ID" "$(printf '{"kind":"quota","name":"%s","hours":"2"}' "$variant")"
    if [[ "$STATUS" != "422" ]]; then
      echo "FAIL: [$name] expected \"$variant\" refused with 422, got $STATUS" >&2
      FAILURES=1
    fi
    if [[ "$BODY" != *"$exact_warning"* ]]; then
      echo "FAIL: [$name] expected the exact-match warning for \"$variant\", got:
$BODY" >&2
      FAILURES=1
    fi
    count="$(qa_task_count_quotas)"
    if [[ "$count" != "1" ]]; then
      echo "FAIL: [$name] expected exactly one quota after refusing \"$variant\", got $count" >&2
      FAILURES=1
    fi
    if ! qa_capture_untriaged "$CAPTURE_ID"; then
      echo "FAIL: [$name] expected the capture naming \"$variant\" to still be untriaged after a refused triage" >&2
      FAILURES=1
    fi
  fi
  qa_stop_server
done

# Pianoo: similar, warned, still possible; then Guitar, unrelated, created
# immediately with no warning.
name="similar-and-unrelated"
if setup_scenario "$name-setup"; then
  qa_triage "$CAPTURE_ID" '{"kind":"quota","name":"Piano","hours":"4"}'

  pianoo_id="$(qa_submit_capture "learn more piano")"
  qa_triage "$pianoo_id" '{"kind":"quota","name":"Pianoo","hours":"2"}'
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] expected \"Pianoo\" refused (warned) with 422 on its first submit, got $STATUS" >&2
    FAILURES=1
  fi
  similar_warning='That reads a lot like “Piano” (4 h a week). Same thing?'
  if [[ "$BODY" != *"$similar_warning"* ]]; then
    echo "FAIL: [$name] expected the similar-match warning for \"Pianoo\", got:
$BODY" >&2
    FAILURES=1
  fi
  confirm_control="$(qa_json_field "$BODY" confirm_control)"
  if [[ "$confirm_control" != "Create anyway" ]]; then
    echo "FAIL: [$name] expected confirm_control \"Create anyway\", got: $confirm_control" >&2
    FAILURES=1
  fi
  if ! qa_capture_untriaged "$pianoo_id"; then
    echo "FAIL: [$name] a warned (not yet confirmed) triage must not consume the capture" >&2
    FAILURES=1
  fi

  qa_triage "$pianoo_id" '{"kind":"quota","name":"Pianoo","hours":"2","confirmed":"Pianoo"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] expected \"Pianoo\" created on its confirmed resubmission, got $STATUS:
$BODY" >&2
    FAILURES=1
  fi
  count="$(qa_task_count_quotas)"
  if [[ "$count" != "2" ]]; then
    echo "FAIL: [$name] expected two quotas after confirming Pianoo, got $count" >&2
    FAILURES=1
  fi

  guitar_id="$(qa_submit_capture "learn guitar")"
  qa_triage "$guitar_id" '{"kind":"quota","name":"Guitar","hours":"3"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] expected \"Guitar\" created immediately, got $STATUS:
$BODY" >&2
    FAILURES=1
  fi

  # Corroborate the backstop: the database itself, not just the Rust check,
  # refuses a case-variant name -- T-collation-enforces-name-identity says a
  # constraint the write path forgot to call cannot be relied on. A scratch
  # copy via sqlite3's own .backup (a plain file cp misses the WAL).
  sqlite3 "$DB_PATH" ".backup '$TMP_DIR/$name-scratch.sqlite'"
  insert_error="$(sqlite3 "$TMP_DIR/$name-scratch.sqlite" \
    "INSERT INTO quotas (name, weekly_target_minutes, created_at_ms) VALUES ('piano', 60, 0);" 2>&1 || true)"
  if [[ "$insert_error" != *"UNIQUE constraint failed"* ]]; then
    echo "FAIL: [$name] expected the database itself to refuse a direct case-variant insert with a UNIQUE constraint error, got: $insert_error -- if only the Rust check refuses this, T-collation-enforces-name-identity's backstop is missing" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the name box is the escape hatch, in a browser ---
name="the-name-box-in-a-browser"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  if ! command -v node >/dev/null 2>&1; then
    echo "FAIL: node is not on PATH -- failing closed rather than skipping the quota-triage-validation browser check" >&2
    FAILURES=1
  else
    NODE_MODULES_DIR="$(npm root -g 2>/dev/null || true)"
    if [[ -z "$NODE_MODULES_DIR" || ! -d "$NODE_MODULES_DIR/playwright-core" ]]; then
      echo "FAIL: playwright-core is not installed globally (npm install -g playwright-core) -- failing closed rather than skipping the quota-triage-validation browser check" >&2
      FAILURES=1
    else
      finish_id="$(qa_submit_capture "finish chapter 3")"
      workout_id="$(qa_submit_capture "workout")"
      reload_id="$(qa_submit_capture "practise piano")"
      if ! NODE_PATH="$NODE_MODULES_DIR" node "$SCRIPT_DIR/quota_triage_validation.cjs" "http://$ADDR" "$finish_id" "$workout_id" "$reload_id"; then
        FAILURES=1
      fi
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: quota_triage_validation"
