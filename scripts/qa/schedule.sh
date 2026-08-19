#!/usr/bin/env bash
# Executable QA procedure: qa/schedule.md (covers features/schedule.feature).
# Covers the automatable, curl-only procedures from qa/schedule.md. Drives
# the running server through its HTTP interface only -- the schedule URL
# read from the header, guardrails set up through the life areas page's own
# controls, tasks triaged through the triage endpoint -- and inspects
# nothing beyond read-only sqlite3 checks the doc's own procedures ask for
# (the "whole or not at all" and "the plan does not move" procedures name
# database facts a page render cannot prove by itself).
#
# qa/schedule.md's "By-hand walkthrough" is NOT scripted here, for the same
# reason app_shell.sh's is not: this environment has no browser-automation
# tooling. Every fact it names is covered below over curl instead, pinning
# `now` with `trellis serve --now <RFC3339>` exactly as the doc's own setup
# instructs.
#
# Every scenario below pins `now` to 2026-08-17T09:00:00Z (a Monday), per
# the doc's own setup: "Today is Monday 17 August 2026, 09:00 UTC ... the
# first free interval starts at *now*."
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-schedule"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

NOW_ISO="2026-08-17T09:00:00Z"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_start_pinned() {
  qa_start_server "$BIN" "$1" "$2" "$NOW_ISO"
}

qa_get_life_areas() {
  curl -s "http://$ADDR/life-areas"
}

# The schedule page's own URL, read from the header of any page -- not
# assumed.
qa_schedule_url() {
  local page="$1"
  python3 -c '
import re, sys
page = sys.argv[1]
m = re.search(r"<a href=\"([^\"]*)\"[^>]*>Schedule</a>", page)
print(m.group(1) if m else "")
' "$page"
}

qa_get_schedule() {
  local url
  url="$(qa_schedule_url "$(curl -s "http://$ADDR/")")"
  curl -s "http://$ADDR$url"
}

# POSTs the page's own "Generate" control and sets STATUS and BODY -- BODY
# is the whole rendered page (hx-target="body"), not a fragment.
qa_generate_schedule() {
  local response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/schedule/generate")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_triage_committed() {
  local capture_id="$1" life_area="$2" estimated_minutes="$3" deadline="$4" deadline_type="$5"
  qa_triage "$capture_id" "$(python3 -c '
import json, sys
print(json.dumps({
    "kind": "committed",
    "deadline": sys.argv[3],
    "deadline_type": sys.argv[4],
    "priority": "P2",
    "estimated_minutes": int(sys.argv[2]),
    "life_area": sys.argv[1],
}))
' "$life_area" "$estimated_minutes" "$deadline" "$deadline_type")"
}

# The markup inside <ul id="$2">...</ul> from a rendered page, or "" if not
# found.
qa_schedule_section() {
  local page="$1" html_id="$2"
  python3 -c '
import re, sys
page, html_id = sys.argv[1], sys.argv[2]
m = re.search(r"<ul id=\"" + re.escape(html_id) + r"\">(.*?)</ul>", page, re.S)
print(m.group(1) if m else "")
' "$page" "$html_id"
}

qa_schedule_placed_section() { qa_schedule_section "$1" schedule-placed; }
qa_schedule_unplaceable_section() { qa_schedule_section "$1" schedule-unplaceable; }

# The first <li>...</li> in a section (as returned by qa_schedule_section)
# whose text contains needle, or "" if none matches.
qa_line_containing() {
  local section="$1" needle="$2"
  python3 -c '
import re, sys
section, needle = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<li>(.*?)</li>", section, re.S):
    if needle in m.group(1):
        print(m.group(1))
        sys.exit()
' "$section" "$needle"
}

qa_schedule_placed_line_for() {
  qa_line_containing "$(qa_schedule_placed_section "$1")" "$2"
}

qa_schedule_unplaceable_line_for() {
  qa_line_containing "$(qa_schedule_unplaceable_section "$1")" "$2"
}

# The number of <li> rows in a section.
qa_line_count() {
  local section="$1"
  python3 -c 'import re, sys; print(len(re.findall(r"<li>", sys.argv[1])))' "$section"
}

qa_task_id_for_text() {
  local text="$1" escaped
  escaped="${text//\'/\'\'}"
  sqlite3 "$DB_PATH" "SELECT tasks.id FROM tasks JOIN captures ON captures.id = tasks.capture_id WHERE captures.raw_text = '$escaped' AND tasks.kind = 'committed' ORDER BY tasks.id DESC LIMIT 1;"
}

# One "start_ms|end_ms" line per proposed block belonging to the committed
# task whose capture carries raw_text, earliest first -- "" if the task has
# none (unplaceable, or not found).
qa_block_rows_for_text() {
  local text="$1" task_id
  task_id="$(qa_task_id_for_text "$text")"
  if [[ -z "$task_id" ]]; then
    echo ""
    return
  fi
  sqlite3 "$DB_PATH" "SELECT start_ms || '|' || end_ms FROM block WHERE task_id = $task_id AND state = 'proposed' ORDER BY start_ms ASC;"
}

qa_block_count_for_text() {
  local rows
  rows="$(qa_block_rows_for_text "$1")"
  if [[ -z "$rows" ]]; then
    echo 0
  else
    printf '%s\n' "$rows" | grep -c .
  fi
}

# Total minutes across every proposed block for the task, 0 if none.
qa_block_minutes_for_text() {
  local text="$1" task_id
  task_id="$(qa_task_id_for_text "$text")"
  if [[ -z "$task_id" ]]; then
    echo 0
    return
  fi
  sqlite3 "$DB_PATH" "SELECT IFNULL(SUM(end_ms - start_ms), 0) / 60000 FROM block WHERE task_id = $task_id AND state = 'proposed';"
}

qa_block_states_for_text() {
  local text="$1" task_id
  task_id="$(qa_task_id_for_text "$text")"
  if [[ -z "$task_id" ]]; then
    echo ""
    return
  fi
  sqlite3 "$DB_PATH" "SELECT DISTINCT state FROM block WHERE task_id = $task_id;"
}

# --- Procedure: a task is placed inside its own life area's hours ---
name="placed-inside-own-life-areas-hours"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work "Mon,Tue,Wed,Thu,Fri" "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Work's band returned status $STATUS" >&2
    FAILURES=1
  fi
  cid="$(qa_submit_capture "write the Q3 deck")"
  qa_triage_committed "$cid" Work 120 "2026-08-21T17:00:00Z" hard
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the committed task returned status $STATUS" >&2
    FAILURES=1
  fi

  qa_generate_schedule
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] generating the schedule returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_schedule)"
  line="$(qa_schedule_placed_line_for "$page" "write the Q3 deck")"
  if [[ "$line" != *"(Work)"* || "$line" != *"2026-08-17T09:00:00Z to 2026-08-17T11:00:00Z"* ]]; then
    echo "FAIL: [$name] expected a Work block Mon 09:00-11:00, got: $line" >&2
    FAILURES=1
  fi
  if [[ -n "$(qa_schedule_unplaceable_line_for "$page" "write the Q3 deck")" ]]; then
    echo "FAIL: [$name] the task should not also appear as unplaceable" >&2
    FAILURES=1
  fi

  states="$(qa_block_states_for_text "write the Q3 deck")"
  if [[ "$states" != "proposed" ]]; then
    echo "FAIL: [$name] expected the stored block's state to be exactly proposed, got: $states" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the tighter deadline goes first, and soft may be overrun ---
name="tighter-deadline-first-soft-may-overrun"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "11:00"
  cid_soon="$(qa_submit_capture "file the return")"
  qa_triage_committed "$cid_soon" Work 120 "2026-08-17T11:00:00Z" soft
  cid_later="$(qa_submit_capture "renew the passport")"
  qa_triage_committed "$cid_later" Work 120 "2026-08-21T17:00:00Z" soft

  qa_generate_schedule
  page="$(qa_get_schedule)"

  soon_line="$(qa_schedule_placed_line_for "$page" "file the return")"
  later_line="$(qa_schedule_placed_line_for "$page" "renew the passport")"
  if [[ "$soon_line" != *"2026-08-17T09:00:00Z to 2026-08-17T11:00:00Z"* ]]; then
    echo "FAIL: [$name] expected the Monday-deadline task on 2026-08-17, got: $soon_line" >&2
    FAILURES=1
  fi
  if [[ "$later_line" != *"2026-08-24T09:00:00Z to 2026-08-24T11:00:00Z"* ]]; then
    echo "FAIL: [$name] expected the Friday-deadline task pushed to 2026-08-24, got: $later_line" >&2
    FAILURES=1
  fi
  if [[ "$later_line" != *"finishes after its deadline"* ]]; then
    echo "FAIL: [$name] expected the pushed task to report its overrun, got: $later_line" >&2
    FAILURES=1
  fi
  if [[ "$soon_line" == *"finishes after its deadline"* ]]; then
    echo "FAIL: [$name] the on-time task should not report an overrun, got: $soon_line" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: reversing which task has the tighter deadline reverses the order ---
name="reversing-deadlines-reverses-order"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "11:00"
  # This time "file the return" carries the loose deadline and "renew the
  # passport" the tight one -- the exact swap qa/schedule.md's own doc asks
  # for.
  cid_a="$(qa_submit_capture "file the return")"
  qa_triage_committed "$cid_a" Work 120 "2026-08-21T17:00:00Z" soft
  cid_b="$(qa_submit_capture "renew the passport")"
  qa_triage_committed "$cid_b" Work 120 "2026-08-17T11:00:00Z" soft

  qa_generate_schedule
  page="$(qa_get_schedule)"

  a_line="$(qa_schedule_placed_line_for "$page" "file the return")"
  b_line="$(qa_schedule_placed_line_for "$page" "renew the passport")"
  if [[ "$b_line" != *"2026-08-17T09:00:00Z to 2026-08-17T11:00:00Z"* ]]; then
    echo "FAIL: [$name] expected the now-tight-deadline task first on 2026-08-17, got: $b_line" >&2
    FAILURES=1
  fi
  if [[ "$a_line" != *"2026-08-24T09:00:00Z to 2026-08-24T11:00:00Z"* ]]; then
    echo "FAIL: [$name] expected the now-loose-deadline task pushed to 2026-08-24, got: $a_line" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: whole or not at all ---
name="whole-or-not-at-all"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "11:00"
  cid="$(qa_submit_capture "rebuild the deck")"
  qa_triage_committed "$cid" Work 180 "2026-08-27T17:00:00Z" soft

  qa_generate_schedule
  page="$(qa_get_schedule)"

  if [[ -n "$(qa_schedule_placed_line_for "$page" "rebuild the deck")" ]]; then
    echo "FAIL: [$name] a 180-minute task should never be placed against a 2-hour interval" >&2
    FAILURES=1
  fi
  unplaceable_line="$(qa_schedule_unplaceable_line_for "$page" "rebuild the deck")"
  if [[ "$unplaceable_line" != *"chunk_policy_unsatisfiable"* ]]; then
    echo "FAIL: [$name] expected chunk_policy_unsatisfiable, got: $unplaceable_line" >&2
    FAILURES=1
  fi
  if [[ "$(qa_block_count_for_text "rebuild the deck")" != "0" ]]; then
    echo "FAIL: [$name] expected zero stored blocks for an unplaceable task" >&2
    FAILURES=1
  fi
  if [[ "$(qa_block_minutes_for_text "rebuild the deck")" != "0" ]]; then
    echo "FAIL: [$name] expected zero total placed minutes; a half-placed task would still sum to 120 or 180" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a hard deadline that cannot be met ---
name="hard-deadline-unreachable"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work "Mon,Tue,Wed,Thu,Fri" "09:00" "17:00"
  cid="$(qa_submit_capture "close the books")"
  qa_triage_committed "$cid" Work 120 "2026-08-17T10:00:00Z" hard

  qa_generate_schedule
  page="$(qa_get_schedule)"

  if [[ -n "$(qa_schedule_placed_line_for "$page" "close the books")" ]]; then
    echo "FAIL: [$name] an unreachable hard deadline must not be placed" >&2
    FAILURES=1
  fi
  unplaceable_line="$(qa_schedule_unplaceable_line_for "$page" "close the books")"
  if [[ "$unplaceable_line" != *"deadline_unreachable"* ]]; then
    echo "FAIL: [$name] expected deadline_unreachable, got: $unplaceable_line" >&2
    FAILURES=1
  fi
  if [[ "$(qa_block_count_for_text "close the books")" != "0" ]]; then
    echo "FAIL: [$name] expected zero stored blocks for a deadline-unreachable task" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the same unreachable deadline, placed as soft instead ---
name="soft-deadline-placed-and-overruns"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work "Mon,Tue,Wed,Thu,Fri" "09:00" "17:00"
  cid="$(qa_submit_capture "close the books")"
  qa_triage_committed "$cid" Work 120 "2026-08-17T10:00:00Z" soft

  qa_generate_schedule
  page="$(qa_get_schedule)"

  line="$(qa_schedule_placed_line_for "$page" "close the books")"
  if [[ "$line" != *"2026-08-17T09:00:00Z to 2026-08-17T11:00:00Z"* ]]; then
    echo "FAIL: [$name] expected the soft task placed at 09:00-11:00, got: $line" >&2
    FAILURES=1
  fi
  if [[ "$line" != *"finishes after its deadline"* ]]; then
    echo "FAIL: [$name] expected the overrun to be reported, got: $line" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: when the hours run out ---
name="hours-run-out"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "11:00"
  # Slack (deadline - now - estimate): "prep the offsite" 0, "audit the
  # ledger" ~198h, "annual review" ~298h -- the same shape
  # schedule-capacity-exceeded-05 tests in scheduler-core directly, driven
  # here over HTTP against a two-Monday, four-hour horizon.
  cid_a="$(qa_submit_capture "prep the offsite")"
  qa_triage_committed "$cid_a" Work 120 "2026-08-17T11:00:00Z" soft
  cid_b="$(qa_submit_capture "audit the ledger")"
  qa_triage_committed "$cid_b" Work 120 "2026-08-25T17:00:00Z" soft
  cid_c="$(qa_submit_capture "annual review")"
  qa_triage_committed "$cid_c" Work 120 "2026-08-29T21:00:00Z" soft

  qa_generate_schedule
  page="$(qa_get_schedule)"

  placed_count="$(qa_line_count "$(qa_schedule_placed_section "$page")")"
  unplaceable_count="$(qa_line_count "$(qa_schedule_unplaceable_section "$page")")"
  if [[ "$placed_count" != "2" ]]; then
    echo "FAIL: [$name] expected exactly 2 placed tasks, got $placed_count" >&2
    FAILURES=1
  fi
  if [[ "$unplaceable_count" != "1" ]]; then
    echo "FAIL: [$name] expected exactly 1 unplaceable task, got $unplaceable_count" >&2
    FAILURES=1
  fi
  if [[ -z "$(qa_schedule_placed_line_for "$page" "prep the offsite")" ]]; then
    echo "FAIL: [$name] expected the zero-slack task to be placed" >&2
    FAILURES=1
  fi
  if [[ -z "$(qa_schedule_placed_line_for "$page" "audit the ledger")" ]]; then
    echo "FAIL: [$name] expected the middle-slack task to be placed" >&2
    FAILURES=1
  fi
  reason="$(qa_schedule_unplaceable_line_for "$page" "annual review")"
  if [[ "$reason" != *"capacity_exceeded"* ]]; then
    echo "FAIL: [$name] expected the most-slack task to be refused capacity_exceeded, got: $reason" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a life area with no hours ---
name="life-area-with-no-hours"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_pool_only "$(qa_get_life_areas)" Fitness
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking Fitness never-scheduled returned status $STATUS" >&2
    FAILURES=1
  fi
  cid="$(qa_submit_capture "run a 5k")"
  qa_triage_committed "$cid" Fitness 60 "2026-08-27T17:00:00Z" soft

  qa_generate_schedule
  page="$(qa_get_schedule)"

  if [[ -n "$(qa_schedule_placed_line_for "$page" "run a 5k")" ]]; then
    echo "FAIL: [$name] a never-scheduled life area's task must not be placed" >&2
    FAILURES=1
  fi
  reason="$(qa_schedule_unplaceable_line_for "$page" "run a 5k")"
  if [[ "$reason" != *"no_window"* ]]; then
    echo "FAIL: [$name] expected no_window, got: $reason" >&2
    FAILURES=1
  fi
  if [[ "$reason" == *"capacity_exceeded"* ]]; then
    echo "FAIL: [$name] opting out is not the same as being full: $reason" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: pool and quota are not scheduled ---
name="pool-and-quota-not-scheduled"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work "Mon,Tue,Wed,Thu,Fri" "09:00" "17:00"
  qa_triage "$(qa_submit_capture "read the spec")" '{"kind":"pool","life_area":"Work"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the pool task returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_triage "$(qa_submit_capture "read every day")" '{"kind":"quota","target_count":3,"target_minutes_each":40,"period":"week","life_area":"Work"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] triaging the quota task returned status $STATUS" >&2
    FAILURES=1
  fi

  qa_generate_schedule
  page="$(qa_get_schedule)"

  if [[ "$page" != *"Nothing to schedule"* ]]; then
    echo "FAIL: [$name] pool and quota are not inputs; the page should show the empty state" >&2
    FAILURES=1
  fi
  for text in "read the spec" "read every day"; do
    if [[ -n "$(qa_schedule_placed_line_for "$page" "$text")" ]]; then
      echo "FAIL: [$name] \"$text\" should never appear as placed" >&2
      FAILURES=1
    fi
    if [[ -n "$(qa_schedule_unplaceable_line_for "$page" "$text")" ]]; then
      echo "FAIL: [$name] \"$text\" should never appear as unplaceable either" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the plan does not move until you ask ---
name="plan-does-not-move-until-you-ask"
db_path="$TMP_DIR/$name.sqlite"
if qa_start_pinned "$db_path" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work "Mon,Tue,Wed,Thu,Fri" "09:00" "17:00"
  cid_first="$(qa_submit_capture "write the Q3 deck")"
  qa_triage_committed "$cid_first" Work 120 "2026-08-21T17:00:00Z" hard
  qa_generate_schedule
  first_page="$(qa_get_schedule)"
  first_line="$(qa_schedule_placed_line_for "$first_page" "write the Q3 deck")"
  if [[ -z "$first_line" ]]; then
    echo "FAIL: [$name] setup expected the first task to be placed" >&2
    FAILURES=1
  fi
  blocks_before_restart="$(qa_block_rows_for_text "write the Q3 deck")"

  cid_second="$(qa_submit_capture "book the venue")"
  qa_triage_committed "$cid_second" Work 60 "2026-08-21T17:00:00Z" hard

  reload_page="$(qa_get_schedule)"
  reload_line="$(qa_schedule_placed_line_for "$reload_page" "write the Q3 deck")"
  if [[ "$reload_line" != "$first_line" ]]; then
    echo "FAIL: [$name] a reload without Generate must not move the existing block" >&2
    echo "  before: $first_line" >&2
    echo "  after:  $reload_line" >&2
    FAILURES=1
  fi
  if [[ -n "$(qa_schedule_placed_line_for "$reload_page" "book the venue")" ]]; then
    echo "FAIL: [$name] the second task must not appear before Generate is asked for again" >&2
    FAILURES=1
  fi

  qa_generate_schedule
  generated_page="$(qa_get_schedule)"
  if [[ -z "$(qa_schedule_placed_line_for "$generated_page" "write the Q3 deck")" || -z "$(qa_schedule_placed_line_for "$generated_page" "book the venue")" ]]; then
    echo "FAIL: [$name] expected both tasks placed after Generate" >&2
    FAILURES=1
  fi
  qa_stop_server

  if ! qa_start_pinned "$db_path" "$TMP_DIR/$name-restart.log"; then
    echo "FAIL: [$name] the server did not come back up against the same database" >&2
    FAILURES=1
  else
    restarted_page="$(qa_get_schedule)"
    if [[ "$restarted_page" != "$generated_page" ]]; then
      echo "FAIL: [$name] a restart without Generate must read the byte-identical plan, not recompute it" >&2
      FAILURES=1
    fi
    blocks_after_restart="$(qa_block_rows_for_text "write the Q3 deck")"
    if [[ "$blocks_after_restart" != "$blocks_before_restart" ]]; then
      echo "FAIL: [$name] the stored block for the first task changed across restart" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: nothing to schedule ---
name="nothing-to-schedule"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work "Mon,Tue,Wed,Thu,Fri" "09:00" "17:00"
  page="$(qa_get_schedule)"
  if [[ "$page" != *"Nothing to schedule"* ]]; then
    echo "FAIL: [$name] a fresh board with nothing triaged should show the true empty-state message" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

name="everything-refused-is-a-different-emptiness"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "11:00"
  cid="$(qa_submit_capture "rebuild the deck")"
  qa_triage_committed "$cid" Work 180 "2026-08-27T17:00:00Z" soft
  qa_generate_schedule
  page="$(qa_get_schedule)"
  if [[ "$page" == *"Nothing to schedule"* ]]; then
    echo "FAIL: [$name] a full won't-fit list must not show the empty-state message" >&2
    FAILURES=1
  fi
  if [[ -z "$(qa_schedule_unplaceable_line_for "$page" "rebuild the deck")" ]]; then
    echo "FAIL: [$name] expected the refused task to be listed under won't fit" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile text stays escaped ---
name="hostile-text-stays-escaped"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work "Mon,Tue,Wed,Thu,Fri" "09:00" "17:00"
  cid="$(qa_submit_capture "<script>alert('boom')</script>")"
  qa_triage_committed "$cid" Work 120 "2026-08-21T17:00:00Z" hard

  qa_generate_schedule
  page="$(qa_get_schedule)"

  if [[ "$page" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the schedule page renders an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$page" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive, escaped rather than stripped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: schedule"
