#!/usr/bin/env bash
# Executable QA procedure: qa/committed_screen.md (covers
# features/committed_screen.feature). Drives the running server through
# its HTTP interface only -- GET /committed and the capture/triage
# endpoints used to set it up -- and inspects nothing beyond what the page
# itself renders.
#
# T-qa-binds-tolerantly-to-markup governs this file: every extraction binds
# to a class this template owns (committed-rows, committed-row,
# committed-date, committed-text, committed-tag, committed-past-badge,
# committed-meta), never to attribute order or adjacency.
#
# The clock is pinned for every procedure to 2026-08-24T09:00:00Z (a
# Monday), matching qa/committed_screen.md's own Setup section and
# features/committed_screen.feature's Background -- half of what this
# screen says (what is past, what a date cell reads) is relative to today.
#
# qa/committed_screen.md's by-hand walkthrough is NOT scripted here, for
# the usual reason (no browser automation, no phone) -- the doc says so of
# itself: "the third phone-first screen shipped without a phone."
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-committed-screen"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

NOW_ISO="2026-08-24T09:00:00Z"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_start_pinned() {
  qa_start_server "$BIN" "$1" "$2" "$NOW_ISO"
}

qa_get_committed() {
  curl -s "http://$ADDR/committed"
}

qa_get_inbox() {
  curl -s "http://$ADDR/"
}

# Creates one committed task via the JSON transport. tag, if given, is the
# context tag; omit it (or pass "") for none.
qa_committed_task() {
  local raw_text="$1" commitment="$2" deadline="$3" tag="${4:-}" capture_id body
  capture_id="$(qa_submit_capture "$raw_text")"
  if [[ -n "$tag" ]]; then
    body="$(python3 -c '
import json, sys
print(json.dumps({
    "kind": "committed", "deadline": sys.argv[1], "commitment": sys.argv[2],
    "priority": "P2", "estimated_minutes": 60, "context_tag": sys.argv[3],
}))
' "$deadline" "$commitment" "$tag")"
  else
    body="$(python3 -c '
import json, sys
print(json.dumps({
    "kind": "committed", "deadline": sys.argv[1], "commitment": sys.argv[2],
    "priority": "P2", "estimated_minutes": 60,
}))
' "$deadline" "$commitment")"
  fi
  qa_triage "$capture_id" "$body"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: setup -- triaging \"$raw_text\" as committed returned status $STATUS" >&2
    FAILURES=1
  fi
}

qa_committed_texts_in_order() {
  python3 -c '
import re, sys
for m in re.finditer(r"<div class=\"committed-text\">([^<]*)</div>", sys.argv[1]):
    print(m.group(1))
' "$1"
}

qa_header_section() {
  qa_between "$1" '<header>' '</header>'
}

# --- Procedure: date order, and what each row carries ---
name="date-order-and-what-each-row-carries"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_committed_task "Q3 planning doc" at "2026-08-27T17:00:00Z" "@desk"
  qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z" "@phone"
  qa_committed_task "Furnace service window" at "2026-08-28T13:00:00Z"

  page="$(qa_get_committed)"
  order="$(qa_committed_texts_in_order "$page")"
  expected="Book the dentist
Q3 planning doc
Furnace service window"
  if [[ "$order" != "$expected" ]]; then
    echo "FAIL: [$name] expected chronological order (Tue, Thu, Fri), got:
$order" >&2
    FAILURES=1
  fi

  dentist_row="$(qa_committed_row_for "$page" "Book the dentist")"
  dentist_tag="$(qa_between "$dentist_row" '<div class="committed-tag">' '</div>')"
  if [[ "$dentist_tag" != "@phone" ]]; then
    echo "FAIL: [$name] expected \"Book the dentist\" tagged @phone, got: $dentist_tag" >&2
    FAILURES=1
  fi

  furnace_row="$(qa_committed_row_for "$page" "Furnace service window")"
  if [[ "$furnace_row" == *"committed-tag"* ]]; then
    echo "FAIL: [$name] expected \"Furnace service window\" to render with no context, got: $furnace_row" >&2
    FAILURES=1
  fi

  meta="$(qa_between "$page" '<div class="committed-meta">' '</div>')"
  if [[ "$meta" != "3 dated" ]]; then
    echo "FAIL: [$name] expected the meta to read \"3 dated\", got: $meta" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: an at, and a by ---
name="an-at-and-a-by"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z"
  qa_committed_task "File the tax return" by "2026-08-27T17:00:00Z"
  qa_committed_task "Renew the passport" by "2026-08-28T17:00:00Z"

  page="$(qa_get_committed)"
  at_row="$(qa_committed_row_for "$page" "Book the dentist")"
  at_cell="$(qa_between "$at_row" '<div class="committed-date">' '</div>')"
  if [[ "$at_cell" != "TUE 8:30" ]]; then
    echo "FAIL: [$name] expected the at cell to read \"TUE 8:30\", got: $at_cell" >&2
    FAILURES=1
  fi

  by_row="$(qa_committed_row_for "$page" "File the tax return")"
  by_cell="$(qa_between "$by_row" '<div class="committed-date">' '</div>')"
  if [[ "$by_cell" != "BY THU" ]]; then
    echo "FAIL: [$name] expected the by cell to read \"BY THU\", got: $by_cell" >&2
    FAILURES=1
  fi

  # A by carrying a time is still a by, not silently an at.
  timed_by_row="$(qa_committed_row_for "$page" "Renew the passport")"
  timed_by_cell="$(qa_between "$timed_by_row" '<div class="committed-date">' '</div>')"
  if [[ "$timed_by_cell" != "BY FRI" ]]; then
    echo "FAIL: [$name] expected a by with a time to still read \"BY FRI\", got: $timed_by_cell" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a deadline that has passed ---
name="a-deadline-that-has-passed"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_committed_task "Renew the passport" by "2026-08-20T09:00:00Z"
  qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z"

  page="$(qa_get_committed)"
  order="$(qa_committed_texts_in_order "$page")"
  expected="Renew the passport
Book the dentist"
  if [[ "$order" != "$expected" ]]; then
    echo "FAIL: [$name] expected the past item first, got:
$order" >&2
    FAILURES=1
  fi

  past_row="$(qa_committed_row_for "$page" "Renew the passport")"
  if [[ "$past_row" != *"committed-past-badge"* ]]; then
    echo "FAIL: [$name] expected \"Renew the passport\" marked past, got: $past_row" >&2
    FAILURES=1
  fi
  future_row="$(qa_committed_row_for "$page" "Book the dentist")"
  if [[ "$future_row" == *"committed-past-badge"* ]]; then
    echo "FAIL: [$name] expected \"Book the dentist\" not marked past, got: $future_row" >&2
    FAILURES=1
  fi

  meta="$(qa_between "$page" '<div class="committed-meta">' '</div>')"
  if [[ "$meta" != "2 dated" ]]; then
    echo "FAIL: [$name] expected the count to include the past item, got: $meta" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: only committed work ---
name="only-committed-work"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_committed_task "Book the dentist" at "2026-08-25T08:30:00Z" "@desk"
  pool_id="$(qa_submit_capture "buy screws")"
  qa_triage "$pool_id" '{"kind":"pool","context_tag":"@desk"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup pool triage returned status $STATUS" >&2
    FAILURES=1
  fi
  quota_id="$(qa_submit_capture "practise piano")"
  qa_triage "$quota_id" '{"kind":"quota","target_count":3,"target_minutes_each":20,"period":"week","context_tag":"@desk"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup quota triage returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_committed)"
  if [[ "$page" == *"buy screws"* || "$page" == *"practise piano"* ]]; then
    echo "FAIL: [$name] expected only the committed task on the committed screen, got:
$page" >&2
    FAILURES=1
  fi
  meta="$(qa_between "$page" '<div class="committed-meta">' '</div>')"
  if [[ "$meta" != "1 dated" ]]; then
    echo "FAIL: [$name] expected the count to cover only the committed task, got: $meta" >&2
    FAILURES=1
  fi

  # This slice must not have moved anything off the pool screen.
  pool_page="$(curl -s "http://$ADDR/pool")"
  if [[ "$pool_page" != *"buy screws"* ]]; then
    echo "FAIL: [$name] expected the pool screen to still show \"buy screws\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: nothing dated ---
name="nothing-dated"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_committed)"
  if [[ "$page" != *"Nothing with a time on it. That is allowed."* ]]; then
    echo "FAIL: [$name] expected the exact empty-state message, got:
$page" >&2
    FAILURES=1
  fi
  if [[ "$page" != *'href="/"'* || "$page" != *"Go to Capture"* ]]; then
    echo "FAIL: [$name] expected a way back to Capture, got:
$page" >&2
    FAILURES=1
  fi
  meta="$(qa_between "$page" '<div class="committed-meta">' '</div>')"
  if [[ "$meta" != "nothing dated" ]]; then
    echo "FAIL: [$name] expected the meta to read \"nothing dated\" rather than a zero count, got: $meta" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: triage asks at or by, on both transports ---
name="triage-asks-at-or-by-on-both-transports"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  CAPTURE_ID="$(qa_submit_capture "File the tax return")"
  qa_triage "$CAPTURE_ID" '{"kind":"committed","deadline":"2026-08-27T17:00:00Z","priority":"P1","estimated_minutes":60}'
  qa_assert_rejected_naming "$name-api" missing_field commitment

  page_id="$(qa_submit_capture "File the tax return (page)")"
  pool_row="$(qa_capture_row_block "$(qa_get_inbox)" "$page_id")"
  kind_endpoint="$(qa_block_control_endpoint "$pool_row" 'value="committed"')"
  qa_triage_form "$kind_endpoint" "kind=committed"
  committed_endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$page_id")" committed)"
  qa_triage_form "$committed_endpoint" "kind=committed&deadline_date=2026-08-27&priority=P1&estimated_minutes=60"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name-page] expected 422 with commitment omitted through the page, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"commitment is required"* ]]; then
    echo "FAIL: [$name-page] expected the rejection to name commitment, got:
$BODY" >&2
    FAILURES=1
  fi
  if ! qa_capture_untriaged "$page_id"; then
    echo "FAIL: [$name-page] expected the capture to still be untriaged" >&2
    FAILURES=1
  fi

  task_count="$(qa_task_count)"
  if [[ "$task_count" != "0" ]]; then
    echo "FAIL: [$name] expected no tasks created, found $task_count" >&2
    FAILURES=1
  fi

  # deadline_type is unread, not forbidden -- carrying it must not reject.
  dt_id="$(qa_submit_capture "renew the lease")"
  qa_triage "$dt_id" '{"kind":"committed","deadline":"2026-08-27T17:00:00Z","commitment":"at","deadline_type":"hard","priority":"P1","estimated_minutes":60}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a valid commitment alongside a stray deadline_type should still succeed, got status $STATUS" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile text stays escaped ---
name="hostile-text-stays-escaped"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  hostile="<script>alert('boom')</script>"
  qa_committed_task "$hostile" at "2026-08-25T08:30:00Z" "$hostile"

  page="$(qa_get_committed)"
  if [[ "$page" == *"<script>alert"* ]]; then
    echo "FAIL: [$name] the committed screen renders an unescaped <script> tag" >&2
    FAILURES=1
  fi
  row="$(qa_committed_row_for "$page" "boom")"
  text_cell="$(qa_between "$row" '<div class="committed-text">' '</div>')"
  tag_cell="$(qa_between "$row" '<div class="committed-tag">' '</div>')"
  if [[ "$text_cell" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive in the text cell, escaped rather than stripped, got: $text_cell" >&2
    FAILURES=1
  fi
  if [[ "$tag_cell" != *"boom"* ]]; then
    echo "FAIL: [$name] expected the word boom to survive in the context cell, escaped rather than stripped, got: $tag_cell" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the tab bar ---
# committed-screen-tabs-06 replaced pool_screen.feature's own narrower
# two-tab scenario under #94: it now asserts all three screens over all
# three tabs in one place, so this is the one QA script that owns the
# tab-bar check (scripts/qa/pool_screen.sh no longer does).
name="the-tab-bar"
if qa_start_pinned "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_assert_tab_bar() {
    local label="$1" header="$2" current="$3" labels current_needle current_count
    labels="$(python3 -c '
import re, sys
print(",".join(re.findall(r"<a href=\"[^\"]*\"[^>]*>([^<]*)</a>", sys.argv[1])))
' "$header")"
    if [[ "$labels" != "Capture,Pool,Committed" ]]; then
      echo "FAIL: [$name] expected exactly the tabs Capture,Pool,Committed in that order on the $label screen, got: $labels" >&2
      FAILURES=1
    fi
    current_needle="aria-current=\"page\">$current</a>"
    if [[ "$header" != *"$current_needle"* ]]; then
      echo "FAIL: [$name] expected $current marked current on the $label screen, got: $header" >&2
      FAILURES=1
    fi
    current_count="$(printf '%s' "$header" | grep -o 'aria-current="page"' | grep -c .)"
    if [[ "$current_count" != "1" ]]; then
      echo "FAIL: [$name] expected exactly one current tab on the $label screen, found $current_count" >&2
      FAILURES=1
    fi
  }

  qa_assert_tab_bar capture "$(qa_header_section "$(curl -s "http://$ADDR/")")" Capture
  qa_assert_tab_bar pool "$(qa_header_section "$(curl -s "http://$ADDR/pool")")" Pool
  qa_assert_tab_bar committed "$(qa_header_section "$(qa_get_committed)")" Committed
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: committed_screen"
