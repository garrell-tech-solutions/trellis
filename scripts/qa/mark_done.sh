#!/usr/bin/env bash
# Executable QA procedure: qa/mark_done.md (covers features/mark_done.feature).
# Drives the running server through its HTTP interface only -- the Pool and
# Committed screens' own done checkbox, POST /pool/tasks/{id}/done and
# POST /committed/tasks/{id}/done -- and inspects nothing beyond what the
# fragment renders and a read-only sqlite3 query.
#
# T-cross-capability-invariants-need-an-owner bites here, per the doc's own
# warning: every task below is marked done through the production route,
# never by writing tasks.archived_at directly, or every procedure would
# pass against a control that does nothing.
#
# T-qa-binds-tolerantly-to-markup governs this file: bound to classes these
# templates own (done-check, trip panel, loose, committed-row, ...), never
# to attribute order or adjacency.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-mark-done"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_get_pool() {
  curl -s "http://$ADDR/pool"
}

qa_get_committed() {
  curl -s "http://$ADDR/committed"
}

qa_task_id_for_text() {
  local text="$1" escaped
  escaped="${text//\'/\'\'}"
  sqlite3 "$DB_PATH" "SELECT tasks.id FROM tasks JOIN captures ON captures.id = tasks.capture_id WHERE captures.raw_text = '$escaped' ORDER BY tasks.id DESC LIMIT 1;"
}

qa_committed_task() {
  local raw_text="$1" commitment="$2" deadline="$3" capture_id body
  capture_id="$(qa_submit_capture "$raw_text")"
  body="$(python3 -c '
import json, sys
print(json.dumps({
    "kind": "committed", "deadline": sys.argv[1], "commitment": sys.argv[2],
    "priority": "P2", "estimated_minutes": 60,
}))
' "$deadline" "$commitment")"
  qa_triage "$capture_id" "$body"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: setup -- triaging \"$raw_text\" as committed returned status $STATUS" >&2
    FAILURES=1
  fi
}

# Marks task_id done through the pool screen's own front door and sets
# STATUS/BODY -- the "through the checkbox" route qa/mark_done.md insists
# on, never a direct write to tasks.archived_at.
qa_mark_pool_done() {
  local task_id="$1" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/pool/tasks/$task_id/done")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_mark_committed_done() {
  local task_id="$1" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/committed/tasks/$task_id/done")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# --- Procedure: a pool task leaves the screen ---
name="a-pool-task-leaves-the-screen"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  qa_pool_task "fix the door latch"
  task_id="$(qa_task_id_for_text "buy screws")"

  qa_mark_pool_done "$task_id"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking done returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  if [[ "$page" == *"buy screws"* ]]; then
    echo "FAIL: [$name] expected \"buy screws\" gone from the pool screen, got:
$page" >&2
    FAILURES=1
  fi
  loose="$(qa_loose_section "$page")"
  if [[ -z "$(qa_loose_row_containing "$loose" "fix the door latch")" ]]; then
    echo "FAIL: [$name] expected \"fix the door latch\" to remain among the loose ends" >&2
    FAILURES=1
  fi

  row_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM tasks WHERE id = $task_id;")"
  if [[ "$row_count" != "1" ]]; then
    echo "FAIL: [$name] expected the row to still exist (kept, not deleted), found $row_count" >&2
    FAILURES=1
  fi
  archived_at="$(sqlite3 "$DB_PATH" "SELECT archived_at FROM tasks WHERE id = $task_id;")"
  if [[ -z "$archived_at" ]]; then
    echo "FAIL: [$name] expected archived_at to be stamped" >&2
    FAILURES=1
  fi
  tag="$(sqlite3 "$DB_PATH" "SELECT context_tag FROM captures WHERE id = (SELECT capture_id FROM tasks WHERE id = $task_id);")"
  if [[ "$tag" != "@homedepot" ]]; then
    echo "FAIL: [$name] expected the tag unchanged at @homedepot, got: $tag" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# "The trip that falls apart" (marking 2 of 3 same-tagged tasks done used to
# dissolve the trip into loose ends) is REMOVED, not rewritten, per #122
# (D-a-trip-survives-being-worked): persistence now counts everything
# displayed, so working a trip never dissolves it -- the exact opposite of
# what this procedure asserted. features/mark_done.feature's own
# mark-done-trip-drops-below-three-02 was removed the same way, superseded
# by trip_progress.feature's trip-progress-panel-holds-02 (holds while
# worked) and trip-progress-clearing-can-drop-a-group-05 (drops only once
# explicitly cleared) -- both now scripts/qa/trip_progress.sh's, not this
# file's. qa/mark_done.md's own "Procedure -- the trip that falls apart"
# still describes the old rule verbatim; flagged to the specifier as stale
# rather than silently left unscripted.

# --- Procedure: a committed task leaves the screen ---
name="a-committed-task-leaves-the-screen"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z"
  qa_committed_task "File the tax return" by "2026-08-27T17:00:00Z"
  dentist_id="$(qa_task_id_for_text "Book the dentist")"

  qa_mark_committed_done "$dentist_id"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking done returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_committed)"
  if [[ "$page" == *"Book the dentist"* ]]; then
    echo "FAIL: [$name] expected \"Book the dentist\" gone, got:
$page" >&2
    FAILURES=1
  fi
  if [[ "$page" != *"File the tax return"* || "$page" != *"BY THU"* ]]; then
    echo "FAIL: [$name] expected the remaining item to still render its own date cell, got:
$page" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the counts ---
name="the-counts"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  qa_pool_task "fix the door latch"
  qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z"

  qa_mark_pool_done "$(qa_task_id_for_text "buy screws")"
  qa_mark_committed_done "$(qa_task_id_for_text "Book the dentist")"

  pool_meta="$(qa_between "$(qa_get_pool)" '<div class="pool-meta">' '</div>')"
  if [[ "$pool_meta" != "1 waiting" ]]; then
    echo "FAIL: [$name] expected the pool meta to read \"1 waiting\", got: $pool_meta" >&2
    FAILURES=1
  fi
  committed_meta="$(qa_between "$(qa_get_committed)" '<div class="committed-meta">' '</div>')"
  if [[ "$committed_meta" != "nothing dated" ]]; then
    echo "FAIL: [$name] expected the committed meta to read \"nothing dated\", got: $committed_meta" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: quota work is untouched ---
name="quota-work-is-untouched"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  quota_id="$(qa_submit_capture "practise piano")"
  qa_triage "$quota_id" '{"kind":"quota","name":"practise piano","hours":"1"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup quota triage returned status $STATUS" >&2
    FAILURES=1
  fi

  # No done control exists anywhere quota work could appear, because
  # quota work does not appear on either screen at all -- the same
  # exclusivity D-no-pool-on-calendar draws for the calendar, applied here
  # to Pool and Committed.
  pool_page="$(qa_get_pool)"
  committed_page="$(qa_get_committed)"
  if [[ "$pool_page" == *"practise piano"* || "$committed_page" == *"practise piano"* ]]; then
    echo "FAIL: [$name] expected the quota task to appear on neither screen" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: nothing lists completed work, and no un-archive route exists for Committed ---
# qa/mark_done.md's own text still reads "Try any plausible un-archive
# route by hand. It should refuse or not exist" unqualified -- true for
# Committed, which #122 never touched, but no longer true for Pool:
# POST /pool/tasks/{id}/undone is now a real, documented, deliberate route
# (D-a-trip-survives-being-worked's own "unchecking a struck item puts it
# back"), not a guessed one that happens to 404. Testing it here as "must
# fail" would assert the opposite of what qa/trip_progress.md's own
# procedures now verify it does; flagged to the specifier as the second
# stale spot in this doc (see the removed "trip that falls apart" note
# above), and this procedure checks what remains true: no un-do label
# anywhere, no completed list, no reorder control, and Committed still has
# no un-archive route at all.
name="nothing-lists-completed-work"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  task_id="$(qa_task_id_for_text "buy screws")"
  qa_mark_pool_done "$task_id"

  page="$(qa_get_pool)"
  for needle in "Undo" "Un-do" "Restore" "Un-archive" "Uncomplete"; do
    if [[ "$page" == *"$needle"* ]]; then
      echo "FAIL: [$name] expected no un-do control, found \"$needle\"" >&2
      FAILURES=1
    fi
  done
  for needle in "Completed" "Done items" "completed-list"; do
    if [[ "$page" == *"$needle"* ]]; then
      echo "FAIL: [$name] expected no list of completed work, found \"$needle\"" >&2
      FAILURES=1
    fi
  done
  for needle in "Raise priority" "Lower priority" "&#9650;" "&#9660;"; do
    if [[ "$page" == *"$needle"* ]]; then
      echo "FAIL: [$name] expected no reorder control, found \"$needle\"" >&2
      FAILURES=1
    fi
  done

  # Committed has no #122 equivalent -- a guessed un-archive route there
  # should still refuse or not exist.
  qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z"
  committed_task_id="$(qa_task_id_for_text "Book the dentist")"
  qa_mark_committed_done "$committed_task_id"
  committed_undo_status="$(curl -s -o /dev/null -w '%{http_code}' -X POST "http://$ADDR/committed/tasks/$committed_task_id/undone")"
  if [[ "$committed_undo_status" -ge 200 && "$committed_undo_status" -lt 300 ]]; then
    echo "FAIL: [$name] a guessed Committed un-archive route succeeded with status $committed_undo_status" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile text stays escaped ---
name="hostile-text-stays-escaped"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  qa_pool_task "<script>alert('boom')</script>"
  screws_id="$(qa_task_id_for_text "buy screws")"

  qa_mark_pool_done "$screws_id"
  if [[ "$BODY" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the mark-done response renders an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive in the mark-done response (the surviving hostile row), escaped rather than stripped, got:
$BODY" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: mark_done"
