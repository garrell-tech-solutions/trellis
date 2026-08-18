#!/usr/bin/env bash
# Executable QA procedure: qa/dismiss_capture.md (covers
# features/dismiss_capture.feature). Covers the automatable, curl-only
# procedures from qa/dismiss_capture.md. Drives the running server through
# its HTTP interface only -- the dismiss control's endpoint read from the
# page's own markup, never assumed -- and inspects persisted state via a
# read-only sqlite3 query, never a project-internal API.
#
# qa/dismiss_capture.md's "By-hand walkthrough" is deliberately NOT scripted
# here, for the same reason inbox_view.sh and triage_from_page.sh don't
# script theirs: this project's stack has no browser-automation tooling, and
# the document is explicit that the browser-visible half ("no full page
# reload", "no confirmation dialog") is a one-time human check curl cannot
# honestly replace. That walkthrough was performed manually this QA cycle
# and passed in full: asdfgh's row offered Dismiss alongside Pool, Committed
# and Quota; clicking it removed asdfgh from the inbox without a visible
# reload and with no confirmation dialog, and created no task; the inbox
# then read the ordinary empty-state message; buy milk, triaged as pool into
# Home, survived a server restart in the task list while asdfgh did not
# return; and no link, tab or list anywhere on the page showed dismissed
# captures or offered to undo one.
#
# The doc's "Procedure -- nothing else changed" is satisfied by
# scripts/qa/run.sh itself: it runs every script in this directory,
# including this one, alongside capture_endpoint.sh, inbox_view.sh,
# triage_from_page.sh, committed_triage_validation.sh,
# quota_triage_validation.sh, unknown_kind_rejection.sh, stats_ratio.sh,
# life_areas.sh and life_area_triage.sh, so it is not re-run here.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-dismiss-capture"
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

# The dismiss control's hx-post endpoint for capture_id, read from the
# page's own markup -- not assumed. Prints nothing if the row (or a dismiss
# control on it) cannot be found.
qa_dismiss_endpoint() {
  qa_row_control_endpoint "$1" "$2" ">Dismiss<"
}

# POSTs the dismiss control (no body) and sets STATUS and BODY.
qa_dismiss() {
  local endpoint="$1" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_capture_row_count() {
  sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM captures;'
}

# Asserts the captures table and the tasks table each hold the expected
# number of rows. Callers must have a FAILURES variable in scope.
qa_assert_row_and_task_counts() {
  local name="$1" expected_rows="$2" expected_tasks="$3" row_count task_count
  row_count="$(qa_capture_row_count)"
  if [[ "$row_count" != "$expected_rows" ]]; then
    echo "FAIL: [$name] expected $expected_rows capture row(s), found $row_count" >&2
    FAILURES=1
  fi
  task_count="$(qa_task_count)"
  if [[ "$task_count" != "$expected_tasks" ]]; then
    echo "FAIL: [$name] expected $expected_tasks task(s), found $task_count" >&2
    FAILURES=1
  fi
}

# Which door capture_id left the inbox by: "untriaged" (still open),
# "triaged" (left, and a tasks row references it) or "dismissed" (left, and
# none does). Reads the column that records leaving off the schema itself
# (PRAGMA table_info) rather than assuming its name, per
# qa/dismiss_capture.md's "read the column names off the schema" -- this
# procedure is agnostic to whether that is one column or the two-column
# shape the brief's open question 1 also allowed.
qa_capture_exit() {
  local capture_id="$1" left_col left_val task_count
  left_col="$(sqlite3 "$DB_PATH" "PRAGMA table_info(captures);" |
    awk -F'|' '$2 ~ /left_inbox|triaged/ {print $2; exit}')"
  if [[ -z "$left_col" ]]; then
    echo "unknown-schema"
    return
  fi
  left_val="$(sqlite3 "$DB_PATH" "SELECT IFNULL($left_col,'') FROM captures WHERE id = $capture_id;")"
  if [[ -z "$left_val" ]]; then
    echo "untriaged"
    return
  fi
  task_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM tasks WHERE capture_id = $capture_id;")"
  if [[ "$task_count" != "0" ]]; then
    echo "triaged"
  else
    echo "dismissed"
  fi
}

# --- Procedure: every untriaged capture offers a dismiss action ---
name="every-row-offers-dismiss"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  asdfgh_id="$(qa_submit_capture "asdfgh")"
  milk_id="$(qa_submit_capture "buy milk")"
  page="$(qa_get_inbox)"
  for cid in "$asdfgh_id" "$milk_id"; do
    endpoint="$(qa_dismiss_endpoint "$page" "$cid")"
    if [[ -z "$endpoint" ]]; then
      echo "FAIL: [$name] no dismiss control found for capture $cid" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: dismissing removes it from the inbox and creates nothing ---
name="dismiss-leaves-inbox-creates-nothing"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  cid="$(qa_submit_capture "asdfgh")"
  endpoint="$(qa_dismiss_endpoint "$(qa_get_inbox)" "$cid")"
  qa_dismiss "$endpoint"
  if [[ "$STATUS" -ge 300 && "$STATUS" -lt 400 ]]; then
    echo "FAIL: [$name] dismissal redirected the browser (status $STATUS)" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *'id="lists"'* ]]; then
    echo "FAIL: [$name] expected the response to carry the #lists fragment, got:
$BODY" >&2
    FAILURES=1
  fi
  inbox="$(qa_html_section "$(qa_get_inbox)" captures)"
  if [[ "$inbox" == *"asdfgh"* ]]; then
    echo "FAIL: [$name] asdfgh still appears in the inbox after dismissal" >&2
    FAILURES=1
  fi
  task_count="$(qa_task_count)"
  if [[ "$task_count" != "0" ]]; then
    echo "FAIL: [$name] expected the task list to still be empty, found $task_count row(s)" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the row is kept, whichever way the capture leaves ---
name="row-kept-whichever-door"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  asdfgh_id="$(qa_submit_capture "asdfgh")"
  milk_id="$(qa_submit_capture "buy milk")"
  qa_triage "$milk_id" '{"kind":"pool","life_area":"Home"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup triage of buy milk returned status $STATUS" >&2
    FAILURES=1
  fi
  endpoint="$(qa_dismiss_endpoint "$(qa_get_inbox)" "$asdfgh_id")"
  qa_dismiss "$endpoint"
  if [[ "$STATUS" -ge 300 && "$STATUS" -lt 400 ]]; then
    echo "FAIL: [$name] dismissal redirected the browser (status $STATUS)" >&2
    FAILURES=1
  fi
  inbox="$(qa_html_section "$(qa_get_inbox)" captures)"
  if [[ -n "$inbox" ]]; then
    echo "FAIL: [$name] expected the inbox to list no captures, got:
$inbox" >&2
    FAILURES=1
  fi
  qa_assert_row_and_task_counts "$name" 2 1
  if [[ "$(qa_capture_exit "$milk_id")" != "triaged" ]]; then
    echo "FAIL: [$name] expected buy milk's row to record a triage exit" >&2
    FAILURES=1
  fi
  if [[ "$(qa_capture_exit "$asdfgh_id")" != "dismissed" ]]; then
    echo "FAIL: [$name] expected asdfgh's row to record a dismissal exit" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a capture leaves the inbox exactly once ---
name="leaves-inbox-exactly-once"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  asdfgh_id="$(qa_submit_capture "asdfgh")"
  # Read the dismiss endpoint while the row is still in the inbox -- once
  # triaged, its row (and the control on it) is gone from #lists, per
  # T-forms-swap-one-fragment, so this is the only point this procedure can
  # honestly observe it from markup rather than assume it.
  asdfgh_dismiss_endpoint="$(qa_dismiss_endpoint "$(qa_get_inbox)" "$asdfgh_id")"
  qa_triage "$asdfgh_id" '{"kind":"pool","life_area":"Home"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup triage of asdfgh returned status $STATUS" >&2
    FAILURES=1
  fi

  qa_dismiss "$asdfgh_dismiss_endpoint"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] dismissing a triaged capture should be rejected with 422, got $STATUS" >&2
    FAILURES=1
  fi

  milk_id="$(qa_submit_capture "buy milk")"
  milk_dismiss_endpoint="$(qa_dismiss_endpoint "$(qa_get_inbox)" "$milk_id")"
  qa_dismiss "$milk_dismiss_endpoint"
  if [[ "$STATUS" -ge 300 && "$STATUS" -lt 400 ]]; then
    echo "FAIL: [$name] dismissing buy milk redirected the browser (status $STATUS)" >&2
    FAILURES=1
  fi

  qa_triage "$milk_id" '{"kind":"pool","life_area":"Home"}'
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] triaging a dismissed capture should be rejected with 422, got $STATUS" >&2
    FAILURES=1
  fi
  named="$(qa_json_field "$BODY" capture_not_open)"
  if [[ "$named" != "the capture is no longer in the inbox" ]]; then
    echo "FAIL: [$name] expected the triage rejection to name capture_not_open, got body: $BODY" >&2
    FAILURES=1
  fi

  qa_dismiss "$milk_dismiss_endpoint"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] dismissing an already-dismissed capture should be rejected with 422, got $STATUS" >&2
    FAILURES=1
  fi

  qa_assert_row_and_task_counts "$name" 2 1
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a dismissed capture stays gone across a restart ---
name="dismissed-stays-gone-across-restart"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-1.log"; then
  asdfgh_id="$(qa_submit_capture "asdfgh")"
  milk_id="$(qa_submit_capture "buy milk")"
  endpoint="$(qa_dismiss_endpoint "$(qa_get_inbox)" "$asdfgh_id")"
  qa_dismiss "$endpoint"
  qa_triage "$milk_id" '{"kind":"pool","life_area":"Home"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup triage of buy milk returned status $STATUS" >&2
    FAILURES=1
  fi
  inbox="$(qa_html_section "$(qa_get_inbox)" captures)"
  if [[ -n "$inbox" ]]; then
    echo "FAIL: [$name] expected the inbox to be empty before restart, got:
$inbox" >&2
    FAILURES=1
  fi
  qa_stop_server

  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log"; then
    page="$(qa_get_inbox)"
    inbox="$(qa_html_section "$page" captures)"
    tasks="$(qa_html_section "$page" tasks)"
    if [[ -n "$inbox" ]]; then
      echo "FAIL: [$name] expected the inbox to still be empty after restart, got:
$inbox" >&2
      FAILURES=1
    fi
    if [[ "$tasks" != *"buy milk"* ]]; then
      echo "FAIL: [$name] expected buy milk to still be in the task list after restart" >&2
      FAILURES=1
    fi
    row_count="$(qa_capture_row_count)"
    if [[ "$row_count" != "2" ]]; then
      echo "FAIL: [$name] expected 2 capture rows after restart, found $row_count" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the inbox says the same thing when everything is dismissed ---
name="empty-state-after-dismissal"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  cid="$(qa_submit_capture "asdfgh")"
  endpoint="$(qa_dismiss_endpoint "$(qa_get_inbox)" "$cid")"
  qa_dismiss "$endpoint"
  page="$(qa_get_inbox)"
  if [[ "$page" != *"Nothing to triage. Add a capture above to get started."* ]]; then
    echo "FAIL: [$name] expected the ordinary empty-state message, got:
$page" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile capture text stays escaped in what a dismissal returns ---
name="hostile-text-escaped-in-dismissal-response"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_submit_capture "<script>alert('boom')</script>" >/dev/null
  asdfgh_id="$(qa_submit_capture "asdfgh")"
  endpoint="$(qa_dismiss_endpoint "$(qa_get_inbox)" "$asdfgh_id")"
  qa_dismiss "$endpoint"
  if [[ "$BODY" == *"<script>"* ]]; then
    echo "FAIL: [$name] dismissal response contains an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"boom"* ]]; then
    echo "FAIL: [$name] dismissal response does not contain the word boom -- content may have been stripped instead of escaped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: dismiss_capture"
