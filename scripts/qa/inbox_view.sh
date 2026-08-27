#!/usr/bin/env bash
# Executable QA procedure: qa/inbox_view.md (covers features/inbox_view.feature).
# Covers the automatable, curl-only procedures from qa/inbox_view.md: list
# order, quick-add without a full reload, empty state, a triaged row
# staying (restyled) instead of vanishing, the three-most-recently-triaged
# cap, and hostile text escaping. Drives the running server through its
# HTTP interface only, and inspects persisted state via a read-only sqlite3
# query -- never through a project-internal API.
#
# #140 retired the flat Tasks list. There is no `<ul id="tasks">` any more:
# every capture -- untriaged, or one of the three most recently triaged --
# renders into the single `<ul id="captures">` this script reads.
#
# qa/inbox_view.md's "By-hand walkthrough" is deliberately NOT scripted here:
# this project's stack has no browser-automation tooling, and the document
# is explicit that the browser-visible half of "no full page reload" (watch
# for a refresh flicker) is a one-time human check curl cannot honestly
# replace. That walkthrough was performed manually this QA cycle and passed
# in full: the capture page is a box and Recent with no wall of every task
# ever triaged, a triaged row stays restyled without a visible reload, and
# it is still there after a server restart.
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

qa_inbox_section() { qa_html_section "$1" captures; }

# Counts occurrences of needle in the raw HTML -- the "exactly once" check
# `inbox-view-triaged-row-stays-04` names, and what breakage 5 (restoring
# the old Tasks list) would move to two.
qa_count_occurrences() {
  python3 -c 'import sys; print(sys.argv[1].count(sys.argv[2]))' "$1" "$2"
}

# --- Procedure: list order ---
name="list-order"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_submit_capture "call the dentist" >/dev/null
  qa_submit_capture "buy milk" >/dev/null
  inbox="$(qa_inbox_section "$(qa_get_inbox)")"
  if [[ "$inbox" != *"call the dentist"* || "$inbox" != *"buy milk"* ]]; then
    echo "FAIL: [$name] expected both captures listed, got:
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
    echo "FAIL: [$name] expected no capture rows rendered, found at least one <li>" >&2
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

# --- Procedure: a triaged row stays, and reads what it became ---
name="triaged-row-stays"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  screws_id="$(qa_submit_capture "buy screws")"
  qa_triage "$screws_id" '{"kind":"pool","context_tag":"@homedepot"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup pool+tag triage returned status $STATUS" >&2
    FAILURES=1
  fi
  untagged_id="$(qa_submit_capture "return the drill")"
  qa_triage "$untagged_id" '{"kind":"pool"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup untagged pool triage returned status $STATUS" >&2
    FAILURES=1
  fi
  committed_id="$(qa_submit_capture "call the dentist")"
  qa_triage "$committed_id" '{"kind":"committed","deadline":"2026-09-01T09:00:00Z","commitment":"at","priority":"P1","estimated_minutes":30,"context_tag":"@desk"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup committed triage returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_inbox)"
  inbox="$(qa_inbox_section "$page")"

  screws_row="$(qa_capture_row_block "$page" "$screws_id")"
  if [[ "$screws_row" != *"Pool · @homedepot"* ]]; then
    echo "FAIL: [$name] expected \"buy screws\" to read \"Pool · @homedepot\", got: $screws_row" >&2
    FAILURES=1
  fi
  if [[ "$screws_row" == *"<button type=\"submit\">Pool</button>"* ]]; then
    echo "FAIL: [$name] expected no kind buttons on the triaged \"buy screws\" row, got: $screws_row" >&2
    FAILURES=1
  fi

  drill_row="$(qa_capture_row_block "$page" "$untagged_id")"
  if [[ "$drill_row" != *"Pool · no context"* ]]; then
    echo "FAIL: [$name] expected \"return the drill\" to read \"Pool · no context\", got: $drill_row" >&2
    FAILURES=1
  fi

  dentist_row="$(qa_capture_row_block "$page" "$committed_id")"
  if [[ "$dentist_row" != *"Committed · @desk"* ]]; then
    echo "FAIL: [$name] expected \"call the dentist\" to read \"Committed · @desk\", got: $dentist_row" >&2
    FAILURES=1
  fi

  count="$(qa_count_occurrences "$page" "buy screws")"
  if [[ "$count" != "1" ]]; then
    echo "FAIL: [$name] expected \"buy screws\" to appear exactly once on the whole page, found $count -- a second occurrence means the old Tasks list survived" >&2
    FAILURES=1
  fi

  qa_stop_server
  if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name-restart.log"; then
    reloaded="$(qa_get_inbox)"
    if [[ "$reloaded" != *"buy screws"* || "$reloaded" != *"Pool · @homedepot"* ]]; then
      echo "FAIL: [$name] expected \"buy screws\" still restyled after a restart" >&2
      FAILURES=1
    fi
    draft_columns="$(sqlite3 "$DB_PATH" "PRAGMA table_info(captures);" | grep -ci "recent\|recently_triaged" || true)"
    if [[ "$draft_columns" != "0" ]]; then
      echo "FAIL: [$name] found a column on captures that looks like a recently-triaged flag or timestamp -- say that before anything else (T-ephemeral-view-state-rides-the-request; append-only migrations mean it can never be taken back)" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: only the three most recently triaged stay ---
name="three-most-recently-triaged"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for word in one two three four; do
    cid="$(qa_submit_capture "$word")"
    qa_triage "$cid" '{"kind":"pool"}'
    if [[ "$STATUS" != "201" ]]; then
      echo "FAIL: [$name] setup -- triaging \"$word\" returned status $STATUS" >&2
      FAILURES=1
    fi
  done

  inbox="$(qa_inbox_section "$(qa_get_inbox)")"
  for word in four three two; do
    if [[ "$inbox" != *">$word<"* && "$inbox" != *"$word"* ]]; then
      echo "FAIL: [$name] expected \"$word\" still shown among the three most recently triaged, got:
$inbox" >&2
      FAILURES=1
    fi
  done
  if [[ "$inbox" == *">one<"* ]]; then
    echo "FAIL: [$name] expected \"one\" (the fourth-oldest triaged) to have dropped off, got:
$inbox" >&2
    FAILURES=1
  fi

  qa_submit_capture "still waiting" >/dev/null
  inbox2="$(qa_inbox_section "$(qa_get_inbox)")"
  if [[ "$inbox2" != *"still waiting"* ]]; then
    echo "FAIL: [$name] expected the untriaged \"still waiting\" capture to be listed, got:
$inbox2" >&2
    FAILURES=1
  fi
  for word in four three two; do
    if [[ "$inbox2" != *"$word"* ]]; then
      echo "FAIL: [$name] expected \"$word\" not to be pushed off by an untriaged capture, got:
$inbox2" >&2
      FAILURES=1
    fi
  done
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
