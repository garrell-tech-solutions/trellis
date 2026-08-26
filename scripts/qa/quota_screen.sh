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

  qa_seed_quota "Running" "3"
  qa_seed_quota "Rust" "5"
  page="$(qa_get_quota)"
  names="$(qa_quota_names_in_order "$page")"
  expected=$'Piano\nRunning\nRust'
  if [[ "$names" != "$expected" ]]; then
    echo "FAIL: [$name] expected Piano, Running, Rust in triage order, got: $names" >&2
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

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: quota_screen"
