#!/usr/bin/env bash
# Executable QA procedure: qa/inbox_view.md (covers features/inbox_view.feature).
# Covers the automatable, curl-only procedures from qa/inbox_view.md: list
# order, quick-add without a full reload, empty state, triaged captures
# excluded, and hostile text escaping. Drives the running server through its
# HTTP interface only, and inspects persisted state via a read-only sqlite3
# query -- never through a project-internal API.
#
# qa/inbox_view.md's "By-hand walkthrough" is deliberately NOT scripted here:
# this project's stack has no browser-automation tooling, and the document
# is explicit that the browser-visible half of "no full page reload" (watch
# for a refresh flicker) is a one-time human check curl cannot honestly
# replace. That walkthrough was performed manually this QA cycle and passed
# in full: empty state renders a message, quick-add's response is a 201
# whose body is the new row's markup (not a redirect), the row appears in a
# fresh GET / immediately after, and the capture survives a server restart.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-inbox-view"
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

qa_index_of() {
  python3 -c 'import sys; print(sys.argv[1].find(sys.argv[2]))' "$1" "$2"
}

# The page renders the inbox and the task list together (triage-from-page),
# so assertions below scope to the inbox's own <ul id="captures"> section --
# a capture's text legitimately appears in the task list too once triaged.
qa_inbox_section() { qa_html_section "$1" captures; }

# --- Procedure: list order ---
name="list-order"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_submit_capture "call the dentist" >/dev/null
  qa_submit_capture "buy milk" >/dev/null
  inbox="$(qa_inbox_section "$(qa_get_inbox)")"
  if [[ "$inbox" != *"call the dentist"* || "$inbox" != *"buy milk"* ]]; then
    echo "FAIL: [$name] expected both captures listed in the inbox, got:
$inbox" >&2
    FAILURES=1
  else
    newer_pos="$(qa_index_of "$inbox" "buy milk")"
    older_pos="$(qa_index_of "$inbox" "call the dentist")"
    if (( newer_pos >= older_pos )); then
      echo "FAIL: [$name] expected \"buy milk\" (newest) before \"call the dentist\" (oldest), found at positions $newer_pos and $older_pos" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: quick-add without a full reload ---
# Reads the quick-add endpoint from the page's own markup rather than
# assuming POST /captures, per the procedure's "Independent of
# Implementation" note.
name="quick-add"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_inbox)"
  endpoint="$(echo "$page" | grep -oE 'hx-post="[^"]+"' | head -1 | sed -E 's/hx-post="([^"]+)"/\1/')"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the quick-add form's hx-post endpoint in the page" >&2
    FAILURES=1
  else
    response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
      -H 'content-type: application/x-www-form-urlencoded' \
      -H 'HX-Request: true' \
      -d 'raw_text=buy+milk&source=web')"
    status="${response##*$'\n'}"
    body="${response%$'\n'*}"
    if [[ "$status" -ge 300 && "$status" -lt 400 ]]; then
      echo "FAIL: [$name] quick-add submission redirected the browser (status $status)" >&2
      FAILURES=1
    fi
    if [[ "$body" != *"buy milk"* ]]; then
      echo "FAIL: [$name] quick-add response body did not include \"buy milk\": $body" >&2
      FAILURES=1
    fi
    page2="$(qa_get_inbox)"
    if [[ "$page2" != *"buy milk"* ]]; then
      echo "FAIL: [$name] a fresh GET / did not list \"buy milk\" after the quick-add submission" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: empty state ---
name="empty-state"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_inbox)"
  inbox="$(qa_inbox_section "$page")"
  if [[ "$inbox" == *"<li"* ]]; then
    echo "FAIL: [$name] expected no capture rows rendered, found at least one <li> in the inbox" >&2
    FAILURES=1
  fi
  if [[ "$page" != *"Nothing to triage"* ]]; then
    echo "FAIL: [$name] expected an empty-state message, got:
$page" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: triaged captures are excluded ---
name="triaged-excluded"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "call the dentist")"
  qa_triage "$capture_id" '{"kind":"pool"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup triage returned status $STATUS, expected 201" >&2
    FAILURES=1
  fi
  inbox="$(qa_inbox_section "$(qa_get_inbox)")"
  if [[ "$inbox" == *"call the dentist"* ]]; then
    echo "FAIL: [$name] a triaged capture still appears in the inbox section" >&2
    FAILURES=1
  fi
  if qa_capture_untriaged "$capture_id"; then
    echo "FAIL: [$name] expected the capture to be triaged (triaged_at set) in the database" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile capture text is escaped ---
name="hostile-text"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_submit_capture "<script>alert('boom')</script>" >/dev/null
  inbox="$(qa_inbox_section "$(qa_get_inbox)")"
  if [[ "$inbox" == *"<script>"* ]]; then
    echo "FAIL: [$name] response contains an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$inbox" != *"boom"* ]]; then
    echo "FAIL: [$name] response does not contain the word \"boom\" -- content may have been stripped instead of escaped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: inbox_view"
