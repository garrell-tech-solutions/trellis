#!/usr/bin/env bash
# Executable QA procedure: qa/quota_screen.md (covers
# features/quota_screen.feature and the edited committed-screen-tabs-06).
#
# T-qa-binds-tolerantly-to-markup: every extraction binds to a class or
# element this template owns (quota-row, quota-name, quota-readout,
# quota-note, quota-meta, quota-empty), never to colour, font or attribute
# order -- #137 just re-did the whole palette and typeface.
#
# Gets quotas onto this screen by triaging captures, never by inserting
# rows: the whole point of this slice is that triage is the only door.
# #138 retired the quota screen's own define form (`+ Define a new quota`,
# `POST /quota`) entirely -- there is no browser procedure left in
# qa/quota_screen.md (the old tap-target/as-you-type checks belonged to
# that form and moved, in spirit, to qa/quota_triage_validation.md's
# triage-panel procedure), so this script is curl/sqlite3 only.
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

qa_get_quota() {
  curl -s "http://$ADDR/quota"
}

# Triages a capture as a quota through the one door this slice leaves open:
# POST /captures/{id}/triage with a name and an hour target.
qa_triage_quota() {
  local raw_text="$1" name="$2" hours="$3" capture_id body
  capture_id="$(qa_submit_capture "$raw_text")"
  body="$(python3 -c 'import json,sys; print(json.dumps({"kind":"quota","name":sys.argv[1],"hours":sys.argv[2]}))' "$name" "$hours")"
  qa_triage "$capture_id" "$body"
}

qa_seed_quota() {
  local name="$1" hours="$2"
  qa_triage_quota "$name" "$name" "$hours"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: setup -- triaging \"$name\" as a quota at $hours hours returned status $STATUS:
$BODY" >&2
    FAILURES=1
  fi
}

qa_quota_row_block() {
  local page="$1" name="$2"
  python3 -c '
import re, sys
page, name = sys.argv[1], sys.argv[2]
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

qa_quota_note() {
  qa_between "$1" '<div class="quota-note">' '</div>'
}

qa_quota_readout() {
  qa_between "$1" '<div class="quota-readout">' '</div>'
}

# Logs a session against quota_id (through the route, never sqlite3) and
# sets STATUS/BODY.
qa_log_session() {
  local quota_id="$1" day="$2" minutes="$3" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/quota/$quota_id/sessions" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "day=$day&minutes=$minutes")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_quota_id_for_name() {
  sqlite3 "$DB_PATH" "SELECT id FROM quotas WHERE name = '${1//\'/\'\'}';"
}

# The quota-change-form's own hx-post endpoint within a row block, read
# from the page's own markup -- not assumed.
qa_change_endpoint() {
  local row="$1"
  python3 -c '
import re, sys
m = re.search(r"<form class=\"quota-change-form\"[^>]*hx-post=\"([^\"]+)\"", sys.argv[1])
print(m.group(1) if m else "")
' "$row"
}

# POSTs a rename/retarget through the row's own change form and sets
# STATUS/BODY.
qa_change_quota() {
  local endpoint="$1" name="$2" hours="$3" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "name=$name" --data-urlencode "hours=$hours")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_quota_change_error() {
  qa_between "$1" '<p class="quota-change-error">' '</p>'
}

# POSTs the row's own remove control and sets STATUS/BODY.
qa_remove_quota() {
  local quota_id="$1" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/quota/$quota_id/remove")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# --- Procedure: the fourth tab ---
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

# --- Procedure: an empty screen that is not a dead end ---
name="empty-screen-not-a-dead-end"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_quota)"
  meta="$(qa_quota_meta "$page")"
  if [[ "$meta" != "none yet" ]]; then
    echo "FAIL: [$name] expected meta \"none yet\" on an empty screen, got: $meta" >&2
    FAILURES=1
  fi
  note="$(qa_between "$page" '<div class="quota-empty">
<p>' '</p>')"
  if [[ "$note" != "A quota is a weekly hour target you keep — practice, study, running. Capture one and triage it." ]]; then
    echo "FAIL: [$name] expected the new empty-state note pointing at Capture, got: $note" >&2
    FAILURES=1
  fi
  if [[ "$page" == *"quota-define-form"* || "$page" == *"Define a new quota"* ]]; then
    echo "FAIL: [$name] expected no way to define a quota on this screen, but found define markup" >&2
    FAILURES=1
  fi
  if [[ "$page" != *'<a href="/" class="quota-go-capture">Go to Capture'* ]]; then
    echo "FAIL: [$name] expected a way back to Capture on the empty screen, got:
$page" >&2
    FAILURES=1
  fi

  # The door the owner asked closed must be closed all the way: a live
  # POST /quota (curled directly, bypassing whatever button is or is not
  # drawn) must not create a quota either.
  status="$(curl -s -o /dev/null -w '%{http_code}' -X POST "http://$ADDR/quota" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d 'name=Piano&hours=4')"
  if [[ "$status" != "404" && "$status" != "405" ]]; then
    echo "FAIL: [$name] expected POST /quota to answer 404/405 (no route), got $status -- a live create endpoint with no link is the same hole in the other direction" >&2
    FAILURES=1
  fi
  if [[ "$(qa_task_count_quotas)" != "0" ]]; then
    echo "FAIL: [$name] expected POST /quota to have created nothing" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: reading a quota ---
name="reading-a-quota"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_seed_quota "Piano" "4"
  if [[ "$STATUS" != "201" ]]; then
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

  # Piano, Running, Rust happens to be alphabetical too -- qa/quota_screen.md's
  # own named trap ("breakage 8") -- so Rust and Running are seeded here in
  # an order that disagrees with alphabetical, or a query sorted by name
  # instead of id would pass this check and prove nothing.
  qa_seed_quota "Rust" "5"
  qa_seed_quota "Running" "3"
  page="$(qa_get_quota)"
  names="$(qa_quota_names_in_order "$page")"
  expected=$'Piano\nRust\nRunning'
  if [[ "$names" != "$expected" ]]; then
    echo "FAIL: [$name] expected Piano, Rust, Running in triage order, got: $names" >&2
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

# --- Procedure: changing a quota ---
name="changing-a-quota"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_seed_quota "Piano" "4"
  piano_id="$(qa_quota_id_for_name "Piano")"
  qa_log_session "$piano_id" "Mon" "30"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup -- logging 30m against Piano returned status $STATUS" >&2
    FAILURES=1
  fi

  # Step 1: retarget to 2 hours. The logged 30 minutes must survive.
  row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
  endpoint="$(qa_change_endpoint "$row")"
  qa_change_quota "$endpoint" "Piano" "2"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] retargeting Piano returned status $STATUS:
$BODY" >&2
    FAILURES=1
  fi
  readout="$(qa_quota_readout "$(qa_quota_row_block "$BODY" "Piano")")"
  if [[ "$readout" != "30m / 2h" ]]; then
    echo "FAIL: [$name] expected \"30m / 2h\" after retargeting (logged time survives), got: $readout" >&2
    FAILURES=1
  fi
  note="$(qa_quota_note "$(qa_quota_row_block "$BODY" "Piano")")"
  if [[ "$note" != "1h 30m left this week · 25%" ]]; then
    echo "FAIL: [$name] expected the note \"1h 30m left this week · 25%\", got: $note" >&2
    FAILURES=1
  fi

  # Step 2: rename. The sessions come with the name.
  row="$(qa_quota_row_block "$(qa_get_quota)" "Piano")"
  endpoint="$(qa_change_endpoint "$row")"
  qa_change_quota "$endpoint" "Piano theory" "2"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] renaming Piano returned status $STATUS:
$BODY" >&2
    FAILURES=1
  fi
  # Retargeted to 2h above, then renamed at 2h again -- readout should
  # read against 2h (0m/2h logged 30m => "30m / 2h").
  readout="$(qa_quota_readout "$(qa_quota_row_block "$BODY" "Piano theory")")"
  if [[ "$readout" != "30m / 2h" ]]; then
    echo "FAIL: [$name] expected the renamed row \"Piano theory\" to still read \"30m / 2h\", got: $readout" >&2
    FAILURES=1
  fi

  # Step 3: rename Running onto Piano theory's name, and onto a
  # case/punctuation respelling of it -- both refused.
  qa_seed_quota "Running" "3"
  running_id="$(qa_quota_id_for_name "Running")"
  for target in "piano theory" "Piano-Theory"; do
    row="$(qa_quota_row_block "$(qa_get_quota)" "Running")"
    endpoint="$(qa_change_endpoint "$row")"
    qa_change_quota "$endpoint" "$target" "3"
    if [[ "$STATUS" != "422" ]]; then
      echo "FAIL: [$name] expected renaming Running to \"$target\" refused with 422, got $STATUS" >&2
      FAILURES=1
    fi
    error="$(qa_quota_change_error "$BODY")"
    if [[ "$error" != *"already exists"* ]]; then
      echo "FAIL: [$name] expected the refusal to name what already exists, got: $error" >&2
      FAILURES=1
    fi
  done
  names="$(qa_quota_names_in_order "$(qa_get_quota)")"
  expected=$'Piano theory\nRunning'
  if [[ "$names" != "$expected" ]]; then
    echo "FAIL: [$name] expected the listing unchanged by the refused renames, got: $names" >&2
    FAILURES=1
  fi
  running_name_after="$(sqlite3 "$DB_PATH" "SELECT name FROM quotas WHERE id = $running_id;")"
  if [[ "$running_name_after" != "Running" ]]; then
    echo "FAIL: [$name] expected Running's own row unchanged by the refused rename, got: $running_name_after" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: removing a quota ---
name="removing-a-quota"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name.log"; then
  qa_seed_quota "Piano" "4"
  qa_seed_quota "Running" "3"
  piano_id="$(qa_quota_id_for_name "Piano")"
  qa_log_session "$piano_id" "Mon" "30"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup -- logging 30m against Piano returned status $STATUS" >&2
    FAILURES=1
  fi

  qa_remove_quota "$piano_id"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] removing Piano returned status $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" == *">Piano<"* ]]; then
    echo "FAIL: [$name] expected Piano gone from the response, got:
$BODY" >&2
    FAILURES=1
  fi
  meta="$(qa_quota_meta "$BODY")"
  if [[ "$meta" != "1 quota" ]]; then
    echo "FAIL: [$name] expected meta \"1 quota\" after removal, got: $meta" >&2
    FAILURES=1
  fi

  # Reload: still gone.
  page="$(qa_get_quota)"
  if [[ "$page" == *">Piano<"* ]]; then
    echo "FAIL: [$name] expected Piano still gone after a reload, got:
$page" >&2
    FAILURES=1
  fi
  if [[ "$page" != *"Running"* ]]; then
    echo "FAIL: [$name] expected Running untouched, got:
$page" >&2
    FAILURES=1
  fi

  # Nothing was deleted: corroborate on a scratch copy (never the live db).
  sqlite3 "$DB" ".backup '$TMP_DIR/$name-scratch.sqlite'"
  quota_row_count="$(sqlite3 "$TMP_DIR/$name-scratch.sqlite" "SELECT COUNT(*) FROM quotas WHERE id = $piano_id;")"
  if [[ "$quota_row_count" != "1" ]]; then
    echo "FAIL: [$name] expected the quotas row for Piano to still exist (removal archives, not deletes), found $quota_row_count" >&2
    FAILURES=1
  fi
  session_count="$(sqlite3 "$TMP_DIR/$name-scratch.sqlite" "SELECT COUNT(*) FROM quota_sessions WHERE quota_id = $piano_id;")"
  if [[ "$session_count" != "1" ]]; then
    echo "FAIL: [$name] expected Piano's logged session to still exist, found $session_count" >&2
    FAILURES=1
  fi
  archived_at="$(sqlite3 "$TMP_DIR/$name-scratch.sqlite" "SELECT archived_at FROM quotas WHERE id = $piano_id;")"
  if [[ -z "$archived_at" ]]; then
    echo "FAIL: [$name] expected archived_at stamped on the removed row" >&2
    FAILURES=1
  fi

  # Step 3: triaging "Piano" again revives it, with its own sessions, at
  # the newly typed target -- one quota, not two.
  # A distinct raw_text (not "Piano", which the removed quota's own seed
  # capture already used) so qa_submit_capture's raw_text lookup stays
  # unambiguous -- the quota's own name is set independently via the
  # triage body's "name" field, which is what actually decides revival.
  new_capture_id="$(qa_submit_capture "practise Piano again")"
  qa_triage "$new_capture_id" '{"kind":"quota","name":"Piano","hours":"2"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] re-triaging \"Piano\" returned status $STATUS:
$BODY" >&2
    FAILURES=1
  fi
  page="$(qa_get_quota)"
  meta="$(qa_quota_meta "$page")"
  if [[ "$meta" != "2 quotas" ]]; then
    echo "FAIL: [$name] expected meta \"2 quotas\" after reviving Piano (one quota, not two), got: $meta" >&2
    FAILURES=1
  fi
  readout="$(qa_quota_readout "$(qa_quota_row_block "$page" "Piano")")"
  if [[ "$readout" != "30m / 2h" ]]; then
    echo "FAIL: [$name] expected the revived Piano to read \"30m / 2h\" (its own sessions, the new target), got: $readout" >&2
    FAILURES=1
  fi
  revived_id="$(qa_quota_id_for_name "Piano")"
  if [[ "$revived_id" != "$piano_id" ]]; then
    echo "FAIL: [$name] expected the revived quota to be the same row (id $piano_id), got id $revived_id" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the foreign key, and why removal is not a delete ---
name="the-foreign-key-and-why-removal-is-not-a-delete"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name.log"; then
  qa_seed_quota "Piano" "4"
  piano_id="$(qa_quota_id_for_name "Piano")"
  qa_log_session "$piano_id" "Mon" "30"
  qa_stop_server

  # sqlite3's own CLI connection does not set foreign_keys=ON by default
  # either (SQLite's own default, same as any bare connection) -- PRAGMA
  # it explicitly to reproduce what sqlx's connect() does for every
  # connection the app opens, per the specifier's own proof.
  sqlite3 "$DB" ".backup '$TMP_DIR/$name-scratch.sqlite'"
  delete_error="$(sqlite3 "$TMP_DIR/$name-scratch.sqlite" "PRAGMA foreign_keys = ON; DELETE FROM quotas WHERE id = $piano_id;" 2>&1 || true)"
  if [[ "$delete_error" != *"FOREIGN KEY constraint failed"* ]]; then
    echo "FAIL: [$name] expected DELETE FROM quotas to be refused with a FOREIGN KEY constraint error (sqlx sets foreign_keys=ON), got: $delete_error" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: quota_screen"
