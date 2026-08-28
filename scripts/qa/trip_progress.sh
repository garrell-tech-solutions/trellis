#!/usr/bin/env bash
# Executable QA procedure: qa/trip_progress.md (covers
# features/trip_progress.feature). Drives the running server through its
# HTTP interface only -- the Pool screen's own checkboxes and its "Clear
# done" control -- and read-only sqlite3, never a fixture that writes
# archived_at/cleared_at directly. T-cross-capability-invariants-need-an-owner
# bites here: this slice is entirely about what POST /pool/tasks/{id}/done
# and its two new siblings leave behind, so a shortcut fixture proves nothing.
#
# T-qa-binds-tolerantly-to-markup: the "Clear done" control is found by its
# accessible name ("Clear done"), never by its glyph (bare "x") or its
# position, per the doc's own warning that the glyph is the thing most
# likely to change.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-trip-progress"
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

qa_task_id_for_text() {
  local text="$1" escaped
  escaped="${text//\'/\'\'}"
  sqlite3 "$DB_PATH" "SELECT tasks.id FROM tasks JOIN captures ON captures.id = tasks.capture_id WHERE captures.raw_text = '$escaped' ORDER BY tasks.id DESC LIMIT 1;"
}

qa_mark_pool_done() {
  local task_id="$1" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/pool/tasks/$task_id/done")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_mark_pool_undone() {
  local task_id="$1" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR/pool/tasks/$task_id/undone")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# The "Clear done" control's own hx-post endpoint within trip_section,
# found by its accessible name (aria-label="Clear done"), never by its
# glyph or position -- the doc's own explicit warning, since the glyph
# (a bare x) is the thing most likely to change. A standalone <button>,
# not wrapped in a <form> (htmx posts straight from the button's own
# hx-post), so qa_block_control_endpoint's <form>-scoped search does not
# apply here.
qa_clear_done_endpoint() {
  local section="$1"
  python3 -c '
import re, sys
section = sys.argv[1]
m = re.search(r"<button\b[^>]*aria-label=\"Clear done\"[^>]*>", section)
if not m:
    print("")
    sys.exit()
hx = re.search(r"hx-post=\"([^\"]+)\"", m.group(0))
print(hx.group(1) if hx else "")
' "$section"
}

qa_clear_done() {
  local trip_section="$1" endpoint response
  endpoint="$(qa_clear_done_endpoint "$trip_section")"
  if [[ -z "$endpoint" ]]; then
    echo "" >&2
    return 1
  fi
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# The <li ...>...</li> block for item_text within trip_section, or "" if
# none matches -- carries its own class="done" (or not) and its checkbox.
qa_trip_item_row() {
  local trip_section="$1" item_text="$2"
  python3 -c '
import re, sys
section, needle = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<li\b[^>]*>(?:(?!</li>).)*?</li>", section, re.S):
    if needle in m.group(0):
        print(m.group(0))
        sys.exit()
' "$trip_section" "$item_text"
}

qa_is_struck() {
  [[ "$1" == '<li class="done">'* ]]
}

# --- Procedure: struck in place, and the label ---
name="struck-in-place-and-the-label"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for text in "buy screws" "return the drill" "pick up trim" "grab a tarp" "sand the deck"; do
    qa_pool_task "$text" "@homedepot"
  done
  for text in "buy screws" "return the drill" "pick up trim"; do
    qa_mark_pool_done "$(qa_task_id_for_text "$text")"
  done

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected @homedepot to still be a trip after three of five were marked done" >&2
    FAILURES=1
  fi

  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "3 of 5 done" ]]; then
    echo "FAIL: [$name] expected the label \"3 of 5 done\", got: $count_label" >&2
    FAILURES=1
  fi

  for text in "buy screws" "return the drill" "pick up trim"; do
    row="$(qa_trip_item_row "$trip" "$text")"
    if ! qa_is_struck "$row"; then
      echo "FAIL: [$name] expected \"$text\" struck through, got: $row" >&2
      FAILURES=1
    fi
  done
  for text in "grab a tarp" "sand the deck"; do
    row="$(qa_trip_item_row "$trip" "$text")"
    if qa_is_struck "$row"; then
      echo "FAIL: [$name] expected \"$text\" still open, got: $row" >&2
      FAILURES=1
    fi
  done

  loose="$(qa_loose_section "$page")"
  if [[ -n "$loose" ]]; then
    echo "FAIL: [$name] expected no loose ends -- the two open items are still in the trip, not scattered:
$loose" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: unchecking puts it back ---
name="unchecking-puts-it-back"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for text in "buy screws" "return the drill" "pick up trim"; do
    qa_pool_task "$text" "@homedepot"
  done
  screws_id="$(qa_task_id_for_text "buy screws")"
  qa_mark_pool_done "$screws_id"

  trip="$(qa_trip_section "$(qa_get_pool)" "@homedepot")"
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "1 of 3 done" ]]; then
    echo "FAIL: [$name] expected \"1 of 3 done\" before unchecking, got: $count_label" >&2
    FAILURES=1
  fi

  qa_mark_pool_undone "$screws_id"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] unchecking returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "3 things" ]]; then
    echo "FAIL: [$name] expected \"3 things\" after unchecking the only struck item, got: $count_label" >&2
    FAILURES=1
  fi
  row="$(qa_trip_item_row "$trip" "buy screws")"
  if qa_is_struck "$row"; then
    echo "FAIL: [$name] expected \"buy screws\" back to open, got: $row" >&2
    FAILURES=1
  fi
  archived_at="$(sqlite3 "$DB_PATH" "SELECT archived_at FROM tasks WHERE id = $screws_id;")"
  if [[ -n "$archived_at" ]]; then
    echo "FAIL: [$name] expected archived_at cleared back to NULL, got: $archived_at" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: clearing, and what it can take with it ---
# qa/trip_progress.md's own text says "five tasks, three done, clear ->
# two remain, still a trip" -- arithmetically impossible against
# TRIP_THRESHOLD=3 (5 - 3 = 2 < 3, so scheduler_core::pool::group would
# drop it to loose ends, confirmed empirically and by the coder's own
# clearing_done_removes_only_the_struck_items test comment, which says the
# five-item fixture ALSO reaches the below-threshold case). The doc's own
# next line -- "the interesting [below-threshold] case ... a five-item
# fixture cannot reach" -- only makes sense if the five-item case survives
# clearing at or above threshold, i.e. two done, three remaining, not
# three done. Tests the arithmetically consistent, empirically confirmed
# behavior (two done leaves three -- exactly TRIP_THRESHOLD -- still a
# trip); flagged the doc's own inconsistency to the specifier rather than
# scripting what it literally says.
name="clearing-and-what-it-can-take-with-it"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for text in "buy screws" "return the drill" "pick up trim" "grab a tarp" "sand the deck"; do
    qa_pool_task "$text" "@homedepot"
  done
  for text in "buy screws" "return the drill"; do
    qa_mark_pool_done "$(qa_task_id_for_text "$text")"
  done

  trip="$(qa_trip_section "$(qa_get_pool)" "@homedepot")"
  qa_clear_done "$trip"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing done returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected @homedepot to still be a trip (3 survivors, at threshold), got:
$page" >&2
    FAILURES=1
  fi
  for text in "buy screws" "return the drill"; do
    if [[ "$trip" == *"$text"* ]]; then
      echo "FAIL: [$name] expected \"$text\" gone after clearing, got:
$trip" >&2
      FAILURES=1
    fi
  done
  for text in "pick up trim" "grab a tarp" "sand the deck"; do
    row="$(qa_trip_item_row "$trip" "$text")"
    if [[ -z "$row" ]]; then
      echo "FAIL: [$name] expected \"$text\" to survive clearing, got:
$trip" >&2
      FAILURES=1
    elif qa_is_struck "$row"; then
      echo "FAIL: [$name] expected \"$text\" to remain open (never struck), got: $row" >&2
      FAILURES=1
    fi
  done
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "3 things" ]]; then
    echo "FAIL: [$name] expected \"3 things\" -- nothing left struck after clearing, got: $count_label" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: clearing, and what it does not take with it ---
# #129 (qa/trip_persistence.md) extended D-a-trip-survives-being-worked:
# clearing tidies a panel, it does not dissolve it, even when the clear
# drops the concurrently-waiting count below TRIP_THRESHOLD. This reverses
# trip-progress-clearing-can-drop-a-group-05, deliberately and in the open --
# the prior version of this script asserted the defect itself.
name="clearing-does-not-drop-a-formed-trip-below-threshold"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for text in "buy screws" "return the drill" "pick up trim" "grab a tarp" "sand the deck"; do
    qa_pool_task "$text" "@homedepot"
  done
  for text in "buy screws" "return the drill" "pick up trim"; do
    qa_mark_pool_done "$(qa_task_id_for_text "$text")"
  done

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] setup expected @homedepot to still be a trip with three struck" >&2
    FAILURES=1
  fi

  qa_clear_done "$trip"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing done returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected @homedepot to stay a trip once cleared to two open items, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "2 things" ]]; then
    echo "FAIL: [$name] expected \"2 things\" after clearing, got: $count_label" >&2
    FAILURES=1
  fi
  for text in "grab a tarp" "sand the deck"; do
    row="$(qa_trip_item_row "$trip" "$text")"
    if [[ -z "$row" ]]; then
      echo "FAIL: [$name] expected \"$text\" to survive in the panel, got:
$trip" >&2
      FAILURES=1
    elif qa_is_struck "$row"; then
      echo "FAIL: [$name] expected \"$text\" to remain open, got: $row" >&2
      FAILURES=1
    fi
  done
  loose="$(qa_loose_section "$page")"
  if [[ -n "$loose" ]]; then
    echo "FAIL: [$name] expected no loose ends -- survivors of a formed trip stay in the panel, got:
$loose" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# The interesting case qa/trip_progress.md names explicitly: a three-item
# fixture with one cleared, which a five-item fixture cannot reach --
# formation (3 waiting) and persistence (a run that once reached 3) agree
# here only because the run has already formed before the clear.
name="a-three-item-trip-also-holds-below-threshold-after-clearing"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for text in "buy screws" "return the drill" "pick up trim"; do
    qa_pool_task "$text" "@homedepot"
  done
  qa_mark_pool_done "$(qa_task_id_for_text "buy screws")"

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] setup expected @homedepot to still be a trip with one struck" >&2
    FAILURES=1
  fi

  qa_clear_done "$trip"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing done returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected @homedepot to stay a trip once cleared to two open items, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "2 things" ]]; then
    echo "FAIL: [$name] expected \"2 things\" after clearing, got: $count_label" >&2
    FAILURES=1
  fi
  for text in "return the drill" "pick up trim"; do
    row="$(qa_trip_item_row "$trip" "$text")"
    if [[ -z "$row" ]]; then
      echo "FAIL: [$name] expected \"$text\" to survive in the panel, got:
$trip" >&2
      FAILURES=1
    elif qa_is_struck "$row"; then
      echo "FAIL: [$name] expected \"$text\" to remain open, got: $row" >&2
      FAILURES=1
    fi
  done
  loose="$(qa_loose_section "$page")"
  if [[ -n "$loose" ]]; then
    echo "FAIL: [$name] expected no loose ends -- the interesting case is that this stays a trip, got:
$loose" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a group with nothing open left ---
name="a-group-with-nothing-open-left"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for text in "buy screws" "return the drill" "pick up trim"; do
    qa_pool_task "$text" "@homedepot"
    qa_mark_pool_done "$(qa_task_id_for_text "$text")"
  done

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected the panel to still be there with nothing open, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "3 of 3 done" ]]; then
    echo "FAIL: [$name] expected \"3 of 3 done\", got: $count_label" >&2
    FAILURES=1
  fi

  qa_clear_done "$trip"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing done returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  if [[ "$page" == *"@homedepot"* ]]; then
    echo "FAIL: [$name] expected the group gone entirely once fully cleared, got:
$page" >&2
    FAILURES=1
  fi
  if [[ "$page" != *"Nothing in the pool"* ]]; then
    echo "FAIL: [$name] expected the empty state, no empty panel left behind, got:
$page" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: loose ends are unchanged ---
name="loose-ends-are-unchanged"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "fix the door latch"
  qa_pool_task "call the dentist"
  latch_id="$(qa_task_id_for_text "fix the door latch")"

  qa_mark_pool_done "$latch_id"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking done returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  if [[ "$page" == *"fix the door latch"* ]]; then
    echo "FAIL: [$name] expected a done loose end to leave the screen at once, got:
$page" >&2
    FAILURES=1
  fi
  loose="$(qa_loose_section "$page")"
  if [[ -z "$(qa_loose_row_containing "$loose" "call the dentist")" ]]; then
    echo "FAIL: [$name] expected the untouched loose end to remain, got:
$loose" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a completed loose end, after #111 ---
# trip-progress-loose-ends-unchanged-08 was narrowed by undo-a-completion,
# not deleted: the row leaves the LIST, and is named exactly once, in the
# way-back line (qa/mark_done.md owns the way back itself; this only
# guards that the narrowing did not become a reversal).
name="a-completed-loose-end-after-111"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "fix the door latch"
  qa_pool_task "call the dentist"
  latch_id="$(qa_task_id_for_text "fix the door latch")"

  qa_mark_pool_done "$latch_id"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking done returned status $STATUS" >&2
    FAILURES=1
  fi

  loose="$(qa_loose_section "$BODY")"
  if [[ -n "$(qa_loose_row_containing "$loose" "fix the door latch")" ]]; then
    echo "FAIL: [$name] expected the completed loose end gone from the list, got:
$loose" >&2
    FAILURES=1
  fi
  if [[ -z "$(qa_loose_row_containing "$loose" "call the dentist")" ]]; then
    echo "FAIL: [$name] expected the untouched loose end to remain in the list, got:
$loose" >&2
    FAILURES=1
  fi
  occurrences="$(python3 -c 'import sys; print(sys.argv[1].count(sys.argv[2]))' "$BODY" "fix the door latch")"
  if [[ "$occurrences" != "1" ]]; then
    echo "FAIL: [$name] expected \"fix the door latch\" named exactly once (in the way back), found $occurrences" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"way-back"* ]]; then
    echo "FAIL: [$name] expected the way-back line in the tick's own response, got:
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
echo "PASS: trip_progress"
