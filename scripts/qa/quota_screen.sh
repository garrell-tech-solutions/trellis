#!/usr/bin/env bash
# Executable QA procedure: qa/quota_screen.md (covers
# features/quota_screen.feature, the edited committed-screen-tabs-06, and
# the browser-only half of the name guard: tap-target size and whether
# typing costs a network request, via scripts/qa/quota_screen.cjs).
#
# T-qa-binds-tolerantly-to-markup: every extraction binds to a class or
# element this template owns (quota-row, quota-name, quota-readout,
# quota-note, quota-meta, quota-empty, quota-define-form,
# quota-define-submit, quota-warning), never to colour, font or attribute
# order -- #137 just re-did the whole palette and typeface.
#
# Defines every quota through the route (POST /quota, form-encoded), never
# by inserting rows: the whole slice is about what that path refuses.
#
# MUST FAIL, NEVER SKIP, if Chrome or playwright-core is unavailable --
# the same rule qa/trip_controls.md and qa/phone_layout.md already carry.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-quota-screen"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

if ! command -v node >/dev/null 2>&1; then
  echo "FAIL: node is not on PATH -- failing closed rather than skipping the quota-screen browser check" >&2
  exit 1
fi

NODE_MODULES_DIR="$(npm root -g 2>/dev/null || true)"
if [[ -z "$NODE_MODULES_DIR" || ! -d "$NODE_MODULES_DIR/playwright-core" ]]; then
  echo "FAIL: playwright-core is not installed globally (npm install -g playwright-core) -- failing closed rather than skipping the quota-screen browser check" >&2
  exit 1
fi

qa_get_quota() {
  curl -s "http://$ADDR/quota"
}

# POSTs the define-quota form. confirmed, if given, is the name a prior
# "Create anyway" warning was shown for.
qa_define_quota() {
  local name="$1" hours="$2" confirmed="${3:-}" data response
  data="name=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' "$name")"
  data="$data&hours=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' "$hours")"
  if [[ -n "$confirmed" ]]; then
    data="$data&confirmed=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' "$confirmed")"
  fi
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/quota" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "$data")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# Defines a quota, failing the setup loudly if the route rejects it.
qa_seed_quota() {
  local name="$1" hours="$2"
  qa_define_quota "$name" "$hours"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: setup -- defining \"$name\" at $hours hours returned status $STATUS:
$BODY" >&2
    FAILURES=1
  fi
}

# Triages a capture as the OLD TaskKind::Quota (a triaged capture, not this
# screen's entity) -- quota-screen-triaged-quotas-are-elsewhere-09's own
# setup, going through the route rather than a fixture.
qa_quota_triage_task() {
  local raw_text="$1" tag="${2:-}" capture_id body
  capture_id="$(qa_submit_capture "$raw_text")"
  if [[ -n "$tag" ]]; then
    body="$(python3 -c 'import json,sys; print(json.dumps({"kind":"quota","target_count":3,"target_minutes_each":45,"period":"week","context_tag":sys.argv[1]}))' "$tag")"
  else
    body='{"kind":"quota","target_count":3,"target_minutes_each":45,"period":"week"}'
  fi
  qa_triage "$capture_id" "$body"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: setup -- triaging \"$raw_text\" as a quota task returned status $STATUS" >&2
    FAILURES=1
  fi
}

qa_quota_row_block() {
  local page="$1" name="$2"
  python3 -c '
import re, sys
page, name = sys.argv[1], sys.argv[2]
# quota-sessions (#93) changed the row from <li> to <div>, so a session
# list nested inside can use its own <ul><li>: scanning to the next
# quota-row marker (or end of string) rather than a matching </li>, the
# same technique quota_sessions.sh already uses for this shape.
for m in re.finditer(r"<div class=\"quota-row\"[^>]*>(?:(?!<div class=\"quota-row\").)*", page, re.S):
    block = m.group(0)
    nm = re.search(r"<div class=\"quota-name\">([^<]*)</div>", block)
    if nm and nm.group(1) == name:
        print(block)
        sys.exit()
' "$page" "$name"
}

qa_task_count_quotas() {
  sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM quotas;'
}

qa_quota_names_in_order() {
  python3 -c '
import re, sys
for m in re.finditer(r"<div class=\"quota-name\">([^<]*)</div>", sys.argv[1]):
    print(m.group(1))
' "$1"
}

qa_quota_meta() {
  qa_between "$1" '<div class="quota-meta">' '</div>'
}

qa_quota_empty_note() {
  qa_between "$1" '<div class="quota-empty">
<p>' '</p>'
}

qa_define_control_label() {
  qa_between "$1" '<button type="submit" class="quota-define-submit">' '</button>'
}

qa_define_warning() {
  local page="$1" warning
  warning="$(qa_between "$page" '<p class="quota-warning">' '</p>')"
  echo "$warning"
}

# --- Procedure: the fourth tab ---
# committed_screen.sh already owns the full four-screen tab-bar assertion
# (committed-screen-tabs-06); this is the narrower "no dead link" half
# specific to the Quota route itself.
name="the-fourth-tab"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  status="$(curl -s -o /dev/null -w '%{http_code}' "http://$ADDR/quota")"
  if [[ "$status" != "200" ]]; then
    echo "FAIL: [$name] GET /quota returned status $status -- a live tab must not 404" >&2
    FAILURES=1
  fi
  page="$(qa_get_quota)"
  header="$(qa_between "$page" '<header>' '</header>')"
  if [[ "$header" != *'aria-current="page">Quota</a>'* ]]; then
    echo "FAIL: [$name] expected the Quota tab to mark itself current, got:
$header" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: defining a quota ---
name="defining-a-quota"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_quota)"
  meta="$(qa_quota_meta "$page")"
  if [[ "$meta" != "none yet" ]]; then
    echo "FAIL: [$name] expected meta \"none yet\" on an empty screen, got: $meta" >&2
    FAILURES=1
  fi
  note="$(qa_quota_empty_note "$page")"
  if [[ "$note" != "A quota is a weekly hour target you keep — practice, study, running. Define one below." ]]; then
    echo "FAIL: [$name] expected the empty-state note, got: $note" >&2
    FAILURES=1
  fi
  if [[ "$(qa_define_control_label "$page")" != "+ Define a new quota" ]]; then
    echo "FAIL: [$name] expected the define control labelled \"+ Define a new quota\", got:
$page" >&2
    FAILURES=1
  fi

  # No name.
  qa_define_quota "" "4"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] expected 422 with no name, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$(qa_task_count_quotas)" != "0" ]]; then
    echo "FAIL: [$name] expected zero quotas after rejecting a nameless submission" >&2
    FAILURES=1
  fi

  # No hours.
  qa_define_quota "Piano" ""
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] expected 422 with no hours, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *'value="Piano"'* ]]; then
    echo "FAIL: [$name] expected the rejected form to still hold the typed name, got:
$BODY" >&2
    FAILURES=1
  fi

  # Zero hours.
  qa_define_quota "Piano" "0"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] expected 422 with zero hours, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$(qa_task_count_quotas)" != "0" ]]; then
    echo "FAIL: [$name] expected zero quotas after every rejection, got $(qa_task_count_quotas)" >&2
    FAILURES=1
  fi

  # A real definition.
  qa_define_quota "Piano" "4"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] expected 201 defining Piano at 4 hours, got $STATUS:
$BODY" >&2
    FAILURES=1
  fi
  page="$(qa_get_quota)"
  row="$(qa_quota_row_block "$page" "Piano")"
  if [[ -z "$row" ]]; then
    echo "FAIL: [$name] expected a Piano row, got:
$page" >&2
    FAILURES=1
  fi
  readout="$(qa_between "$row" '<div class="quota-readout">' '</div>')"
  if [[ "$readout" != "0m / 4h" ]]; then
    echo "FAIL: [$name] expected the readout \"0m / 4h\", got: $readout" >&2
    FAILURES=1
  fi
  row_note="$(qa_between "$row" '<div class="quota-note">' '</div>')"
  if [[ "$row_note" != "4h left this week · 0%" ]]; then
    echo "FAIL: [$name] expected the note \"4h left this week · 0%\", got: $row_note" >&2
    FAILURES=1
  fi
  meta="$(qa_quota_meta "$(qa_get_quota)")"
  if [[ "$meta" != "1 quota" ]]; then
    echo "FAIL: [$name] expected meta \"1 quota\", got: $meta" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the name guard, over HTTP ---
name="the-name-guard-over-http"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_seed_quota "Piano" "4"

  exact_warning='“Piano” already exists at 4 h a week. File it there instead of making a second one.'
  for variant in "piano" "PIANO" "Pi-ano" "pi ano"; do
    qa_define_quota "$variant" "2"
    if [[ "$STATUS" != "422" ]]; then
      echo "FAIL: [$name] expected \"$variant\" refused with 422, got $STATUS" >&2
      FAILURES=1
    fi
    if [[ "$BODY" != *"$exact_warning"* ]]; then
      echo "FAIL: [$name] expected the exact-match warning for \"$variant\", got:
$BODY" >&2
      FAILURES=1
    fi
  done
  count="$(qa_task_count_quotas)"
  if [[ "$count" != "1" ]]; then
    echo "FAIL: [$name] expected exactly one quota after four refused exact-match variants, got $count" >&2
    FAILURES=1
  fi

  # Pianoo: similar, warned, still possible.
  qa_define_quota "Pianoo" "2"
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
  create_label="$(qa_define_control_label "$BODY")"
  if [[ "$create_label" != "Create anyway" ]]; then
    echo "FAIL: [$name] expected the control to read \"Create anyway\" after a similar-match warning, got: $create_label" >&2
    FAILURES=1
  fi
  confirmed_field="$(python3 -c '
import re, sys
m = re.search(r"<input type=\"hidden\" name=\"confirmed\" value=\"([^\"]*)\">", sys.argv[1])
print(m.group(1) if m else "")
' "$BODY")"
  if [[ "$confirmed_field" != "Pianoo" ]]; then
    echo "FAIL: [$name] expected the resubmission to carry confirmed=Pianoo, got: $confirmed_field" >&2
    FAILURES=1
  fi

  qa_define_quota "Pianoo" "2" "Pianoo"
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

  # Guitar: unrelated, created immediately with no warning.
  qa_define_quota "Guitar" "3"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] expected \"Guitar\" created immediately, got $STATUS:
$BODY" >&2
    FAILURES=1
  fi

  # Corroborate the backstop: the database itself, not just the Rust
  # check, refuses a case-variant name -- T-collation-enforces-name-identity
  # says a constraint the write path forgot to call cannot be relied on.
  # A scratch copy, never the live database this server is using -- via
  # sqlite3's own .backup, since a plain file cp of the main db file
  # misses whatever is still sitting in the WAL and copies an empty
  # schema.
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

# --- Procedure: quotas are listed in the order they were defined ---
name="defined-order"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_seed_quota "Piano" "4"
  qa_seed_quota "Running" "3"
  qa_seed_quota "Rust" "5"

  page="$(qa_get_quota)"
  names="$(qa_quota_names_in_order "$page")"
  expected=$'Piano\nRunning\nRust'
  if [[ "$names" != "$expected" ]]; then
    echo "FAIL: [$name] expected Piano, Running, Rust in definition order, got: $names" >&2
    FAILURES=1
  fi
  meta="$(qa_quota_meta "$page")"
  if [[ "$meta" != "3 quotas" ]]; then
    echo "FAIL: [$name] expected meta \"3 quotas\", got: $meta" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a quota is not a task ---
# quota-screen-triaged-quotas-are-elsewhere-09: a triaged TaskKind::Quota
# capture must not appear on this screen, must not be migrated onto it, and
# must not alter its counts.
name="a-quota-is-not-a-task"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_quota_triage_task "practise piano" "@home"
  qa_seed_quota "Piano" "4"

  page="$(qa_get_quota)"
  if [[ "$page" == *"practise piano"* ]]; then
    echo "FAIL: [$name] expected the triaged quota task to be absent from this screen, got:
$page" >&2
    FAILURES=1
  fi
  names="$(qa_quota_names_in_order "$page")"
  if [[ "$names" != "Piano" ]]; then
    echo "FAIL: [$name] expected only the entity-quota \"Piano\", got: $names" >&2
    FAILURES=1
  fi
  meta="$(qa_quota_meta "$page")"
  if [[ "$meta" != "1 quota" ]]; then
    echo "FAIL: [$name] expected meta \"1 quota\" -- the triaged task must not count, got: $meta" >&2
    FAILURES=1
  fi

  # Corroborate: the old triaged row is untouched, still carrying its own
  # target_count/target_minutes_each/period, and no row exists in the new
  # quotas table for it.
  old_row="$(sqlite3 "$DB_PATH" "SELECT target_count, target_minutes_each, period FROM tasks WHERE kind = 'quota';")"
  if [[ "$old_row" != "3|45|week" ]]; then
    echo "FAIL: [$name] expected the old triaged quota row untouched (3|45|week), got: $old_row" >&2
    FAILURES=1
  fi
  new_table_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM quotas WHERE name = 'practise piano';")"
  if [[ "$new_table_count" != "0" ]]; then
    echo "FAIL: [$name] expected no row in the new quotas table for the triaged task, found $new_table_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the warning as you type, in a browser ---
name="the-warning-as-you-type-in-a-browser"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_seed_quota "Piano" "4"

  if ! NODE_PATH="$NODE_MODULES_DIR" node "$SCRIPT_DIR/quota_screen.cjs" "http://$ADDR"; then
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: quota_screen"
