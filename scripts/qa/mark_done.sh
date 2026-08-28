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

# The way-back line's own text, or "" if none is present in fragment.
qa_way_back_text() {
  qa_between "$1" '<div class="way-back">' '</div>'
}

qa_way_back_name() {
  qa_between "$1" '<span class="way-back-name">' '</span>'
}

# The way-back-undo control's hx-post endpoint, or "" if absent.
qa_way_back_undo_endpoint() {
  local fragment="$1"
  python3 -c '
import re, sys
m = re.search(r"<button[^>]*class=\"way-back-undo\"[^>]*hx-post=\"([^\"]+)\"", sys.argv[1])
print(m.group(1) if m else "")
' "$fragment"
}

# --- Procedure: the way back ---
# Settled by the owner 2026-08-27. A completed task's row leaves the
# screen, and a line appears naming it with a way back -- lasting until
# the next action on that screen, gone on the next plain GET. Asserted
# against the tick's own response fragment (BODY), never a fresh
# navigation, per qa/mark_done.md's own "read the fragment the tick
# returned, not a fresh page".
name="the-way-back"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  qa_pool_task "return the drill" "@homedepot"
  screws_id="$(qa_task_id_for_text "buy screws")"
  drill_id="$(qa_task_id_for_text "return the drill")"

  before_loose_count="$(python3 -c '
import re, sys
print(len(re.findall(r"class=\"loose-item\"", sys.argv[1])))
' "$(qa_loose_section "$(qa_get_pool)")")"

  qa_mark_pool_done "$screws_id"
  way_back_name="$(qa_way_back_name "$BODY")"
  if [[ "$way_back_name" != "buy screws" ]]; then
    echo "FAIL: [$name] expected the way back to name \"buy screws\", got: \"$way_back_name\" (fragment: $BODY)" >&2
    FAILURES=1
  fi
  way_back_text="$(qa_way_back_text "$BODY")"
  if [[ "$way_back_text" != *"done"* ]]; then
    echo "FAIL: [$name] expected the way-back text to read the task done, got: $way_back_text" >&2
    FAILURES=1
  fi
  after_loose_count="$(python3 -c '
import re, sys
print(len(re.findall(r"class=\"loose-item\"", sys.argv[1])))
' "$(qa_loose_section "$BODY")")"
  if [[ "$after_loose_count" != "$((before_loose_count - 1))" ]]; then
    echo "FAIL: [$name] expected the loose-ends count to drop by one, before=$before_loose_count after=$after_loose_count" >&2
    FAILURES=1
  fi

  # Step 2: ticking the second names only the second, never a growing list.
  qa_mark_pool_done "$drill_id"
  way_back_name2="$(qa_way_back_name "$BODY")"
  if [[ "$way_back_name2" != "return the drill" ]]; then
    echo "FAIL: [$name] expected the way back to name only \"return the drill\", got: \"$way_back_name2\"" >&2
    FAILURES=1
  fi
  if [[ "$BODY" == *"buy screws"* ]]; then
    echo "FAIL: [$name] expected the first task not to still be named -- one task, never a growing list, got:
$BODY" >&2
    FAILURES=1
  fi

  # Step 3: a fresh GET carries no way back at all -- it rides the
  # request and nothing is stored.
  reloaded="$(qa_get_pool)"
  if [[ "$reloaded" == *"way-back"* ]]; then
    echo "FAIL: [$name] expected no way-back line on a fresh GET, got:
$reloaded" >&2
    FAILURES=1
  fi
  draft_columns="$(sqlite3 "$DB_PATH" "PRAGMA table_info(tasks);" | grep -ci "recent\|recently_done\|just_done" || true)"
  if [[ "$draft_columns" != "0" ]]; then
    echo "FAIL: [$name] found a column on tasks that looks like a recently-completed flag -- say that before anything else in the report (T-migrations-append-only means it can never be taken back)" >&2
    FAILURES=1
  fi

  # Step 4: the Committed screen carries the same shape.
  qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z"
  dentist_id="$(qa_task_id_for_text "Book the dentist")"
  qa_mark_committed_done "$dentist_id"
  committed_way_back_name="$(qa_way_back_name "$BODY")"
  if [[ "$committed_way_back_name" != "Book the dentist" ]]; then
    echo "FAIL: [$name] expected Committed's way back to name \"Book the dentist\", got: \"$committed_way_back_name\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: taking the way back ---
name="taking-the-way-back"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  qa_pool_task "return the drill" "@homedepot"
  qa_pool_task "pick up trim" "@homedepot"
  screws_id="$(qa_task_id_for_text "buy screws")"

  qa_mark_pool_done "$screws_id"
  undo_endpoint="$(qa_way_back_undo_endpoint "$BODY")"
  if [[ -z "$undo_endpoint" ]]; then
    echo "FAIL: [$name] could not find the way-back-undo control's endpoint in the tick's own response" >&2
    FAILURES=1
  else
    response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$undo_endpoint")"
    STATUS="${response##*$'\n'}"
    BODY="${response%$'\n'*}"
    if [[ "$STATUS" != "200" ]]; then
      echo "FAIL: [$name] taking the way back returned status $STATUS" >&2
      FAILURES=1
    fi
    if [[ "$BODY" != *"3 things"* ]]; then
      echo "FAIL: [$name] expected the trip to re-form (3 things), got:
$BODY" >&2
      FAILURES=1
    fi
    if [[ "$BODY" != *"buy screws"* ]]; then
      echo "FAIL: [$name] expected \"buy screws\" back among the open items" >&2
      FAILURES=1
    fi

    # Taking it again is a no-op: unmark_task_done's own already-open
    # guard, used rather than re-added.
    second_response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$undo_endpoint")"
    second_status="${second_response##*$'\n'}"
    second_body="${second_response%$'\n'*}"
    if [[ "$second_status" != "200" ]]; then
      echo "FAIL: [$name] taking the way back a second time returned status $second_status, expected a no-op 200" >&2
      FAILURES=1
    fi
    if [[ "$second_body" != *"3 things"* ]]; then
      echo "FAIL: [$name] expected the second take to change nothing (still 3 things), got:
$second_body" >&2
      FAILURES=1
    fi
  fi

  # Committed: take the way back, count reads 1 dated again.
  qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z"
  dentist_id="$(qa_task_id_for_text "Book the dentist")"
  qa_mark_committed_done "$dentist_id"
  committed_undo_endpoint="$(qa_way_back_undo_endpoint "$BODY")"
  if [[ -z "$committed_undo_endpoint" ]]; then
    echo "FAIL: [$name] could not find Committed's way-back-undo endpoint" >&2
    FAILURES=1
  else
    response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$committed_undo_endpoint")"
    STATUS="${response##*$'\n'}"
    BODY="${response%$'\n'*}"
    if [[ "$STATUS" != "200" || "$BODY" != *"Book the dentist"* ]]; then
      echo "FAIL: [$name] expected Committed's way back to restore \"Book the dentist\", got status $STATUS:
$BODY" >&2
      FAILURES=1
    fi
    committed_meta="$(qa_between "$(qa_get_committed)" '<div class="committed-meta">' '</div>')"
    if [[ "$committed_meta" != "1 dated" ]]; then
      echo "FAIL: [$name] expected the committed meta to read \"1 dated\" after undo, got: $committed_meta" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: what has no way back ---
name="what-has-no-way-back"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  qa_pool_task "return the drill" "@homedepot"
  qa_pool_task "pick up trim" "@homedepot"

  complete_response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/pool/trips/%40homedepot/complete")"
  complete_status="${complete_response##*$'\n'}"
  complete_body="${complete_response%$'\n'*}"
  if [[ "$complete_status" != "200" ]]; then
    echo "FAIL: [$name] group-complete returned status $complete_status" >&2
    FAILURES=1
  fi
  if [[ "$complete_body" == *"way-back"* ]]; then
    echo "FAIL: [$name] expected group completion's own response to offer no way back, got:
$complete_body" >&2
    FAILURES=1
  fi

  clear_response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/pool/trips/%40homedepot/clear")"
  clear_status="${clear_response##*$'\n'}"
  clear_body="${clear_response%$'\n'*}"
  if [[ "$clear_status" != "200" ]]; then
    echo "FAIL: [$name] clear-done returned status $clear_status" >&2
    FAILURES=1
  fi
  if [[ "$clear_body" == *"way-back"* ]]; then
    echo "FAIL: [$name] expected clearing done items' own response to offer no way back, got:
$clear_body" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the way back is at least 44px, in a browser ---
# qa/mark_done.md's own outcome for "the way back": a real tap target.
# The way back is transient -- it exists only in the tick's own response
# fragment -- so a static seed-and-navigate check cannot see it; this
# drives a real click in headless Chrome and measures what it produced.
# MUST FAIL, NEVER SKIP, if Chrome or playwright-core is unavailable --
# the same rule every other browser check in this project carries.
name="the-way-back-tap-target"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  if ! command -v node >/dev/null 2>&1; then
    echo "FAIL: node is not on PATH -- failing closed rather than skipping the way-back tap-target check" >&2
    FAILURES=1
  else
    NODE_MODULES_DIR="$(npm root -g 2>/dev/null || true)"
    if [[ -z "$NODE_MODULES_DIR" || ! -d "$NODE_MODULES_DIR/playwright-core" ]]; then
      echo "FAIL: playwright-core is not installed globally (npm install -g playwright-core) -- failing closed rather than skipping the way-back tap-target check" >&2
      FAILURES=1
    else
      qa_pool_task "buy screws" "@homedepot"
      qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z"
      pool_task_id="$(qa_task_id_for_text "buy screws")"
      committed_task_id="$(qa_task_id_for_text "Book the dentist")"
      if ! NODE_PATH="$NODE_MODULES_DIR" node "$SCRIPT_DIR/mark_done.cjs" "http://$ADDR" "$pool_task_id" "$committed_task_id"; then
        FAILURES=1
      fi
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: nothing lists completed work ---
# #111 gave Committed the same real, deliberate /undone route Pool already
# had (D-a-trip-survives-being-worked's shape, extended): both are now
# legitimate front doors to unmark_task_done, not guessed routes that
# happen to 404. What "nothing lists completed work" still means: no
# completed-work LIST or reorder control anywhere, ever, and no un-do
# affordance on a FRESH GET specifically (the way-back line is real but
# ephemeral -- qa/mark_done.md's own "the way back" procedure, scripted
# below, owns asserting it exists on the tick's own response).
name="nothing-lists-completed-work"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@homedepot"
  task_id="$(qa_task_id_for_text "buy screws")"
  qa_mark_pool_done "$task_id"

  # A fresh GET (not the tick's own response) must show no way back: it
  # rides the request and nothing is stored.
  page="$(qa_get_pool)"
  for needle in "way-back" "Undo" "Un-do" "Restore" "Un-archive" "Uncomplete"; do
    if [[ "$page" == *"$needle"* ]]; then
      echo "FAIL: [$name] expected no un-do control on a fresh GET, found \"$needle\"" >&2
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
