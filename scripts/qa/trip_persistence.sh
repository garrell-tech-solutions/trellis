#!/usr/bin/env bash
# Executable QA procedure: qa/trip_persistence.md (covers
# features/trip_persistence.feature, the two changed trip_progress.feature
# scenarios, and the half of #129 no acceptance scenario can hold: that the
# rule survives a reload and a restart).
#
# Drives the running server through its HTTP interface only -- mark done
# (POST /pool/tasks/{id}/done), undo (.../undone), clear
# (POST /pool/trips/{tag}/clear) and complete-group
# (POST /pool/trips/{tag}/complete) -- and read-only sqlite3 for
# corroboration, never a fixture that writes cleared_at or a run marker
# directly (T-cross-capability-invariants-need-an-owner): this slice is
# entirely about what those routes leave behind.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-trip-persistence"
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

# Found by accessible name, never by glyph or position
# (T-qa-binds-tolerantly-to-markup) -- shares trip_progress.sh's own copy of
# this parser rather than a project-wide helper, per this project's
# established convention for small per-script markup parsers.
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

# Shares trip_controls.sh's own copy of this parser (the trip-complete
# control's own hx-post), per this project's established convention.
qa_complete_trip_endpoint() {
  local page="$1" tag="$2" trip
  trip="$(qa_trip_section "$page" "$tag")"
  python3 -c '
import re, sys
section = sys.argv[1]
m = re.search(r"<button type=\"button\" class=\"trip-complete\" hx-post=\"([^\"]+)\"", section)
print(m.group(1) if m else "")
' "$trip"
}

qa_complete_trip() {
  local page="$1" tag="$2" endpoint response
  endpoint="$(qa_complete_trip_endpoint "$page" "$tag")"
  if [[ -z "$endpoint" ]]; then
    echo "" >&2
    return 1
  fi
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

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

# --- Procedure: the walk the slice exists for ---
name="the-walk-the-slice-exists-for"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for i in $(seq 1 5); do
    qa_pool_task "homedepot errand $i" "@homedepot"
  done
  for i in 1 2 3; do
    qa_mark_pool_done "$(qa_task_id_for_text "homedepot errand $i")"
  done

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  qa_clear_done "$trip"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing done returned status $STATUS" >&2
    FAILURES=1
  fi

  # After the first clear: still a trip, 2 things, 2 open, nothing struck,
  # loose ends empty -- the defect this slice fixes.
  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected @homedepot to still be a trip after clearing three of five, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "2 things" ]]; then
    echo "FAIL: [$name] expected \"2 things\" after the first clear, got: $count_label" >&2
    FAILURES=1
  fi
  for i in 4 5; do
    row="$(qa_trip_item_row "$trip" "homedepot errand $i")"
    if [[ -z "$row" ]] || qa_is_struck "$row"; then
      echo "FAIL: [$name] expected \"homedepot errand $i\" open in the panel, got: $row" >&2
      FAILURES=1
    fi
  done
  loose="$(qa_loose_section "$page")"
  if [[ -n "$loose" ]]; then
    echo "FAIL: [$name] expected loose ends empty after the first clear, got:
$loose" >&2
    FAILURES=1
  fi

  # Mark the remaining two done: still a trip, 2 of 2 done, clear control
  # still offered -- the panel leaves on a tap, never on a tick.
  for i in 4 5; do
    qa_mark_pool_done "$(qa_task_id_for_text "homedepot errand $i")"
  done
  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected @homedepot to still be a trip once its last two are done, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "2 of 2 done" ]]; then
    echo "FAIL: [$name] expected \"2 of 2 done\", got: $count_label" >&2
    FAILURES=1
  fi
  if [[ -z "$(qa_clear_done_endpoint "$trip")" ]]; then
    echo "FAIL: [$name] expected the Clear done control still offered with nothing open, got:
$trip" >&2
    FAILURES=1
  fi

  # Clear done again: the tag leaves the screen entirely, no empty panel,
  # not mentioned in loose ends.
  qa_clear_done "$trip"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] the second clear returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_pool)"
  if [[ "$page" == *"@homedepot"* ]]; then
    echo "FAIL: [$name] expected @homedepot gone entirely once fully cleared, got:
$page" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a tag that never reached three ---
name="a-tag-that-never-reached-three"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for i in 1 2 3; do
    qa_pool_task "homedepot errand $i" "@homedepot"
  done
  qa_pool_task "milk" "@supermarket"
  qa_pool_task "coffee" "@supermarket"

  qa_mark_pool_done "$(qa_task_id_for_text "milk")"
  qa_mark_pool_done "$(qa_task_id_for_text "homedepot errand 1")"

  page="$(qa_get_pool)"
  qa_clear_done "$(qa_trip_section "$page" "@homedepot")"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing @homedepot returned status $STATUS" >&2
    FAILURES=1
  fi

  # @supermarket never formed a trip (only 2 tasks ever), so it has no
  # trip-panel clear control -- clear it by loose-end completion instead:
  # "milk" already left the screen the instant it was marked done (#122's
  # own rule for loose ends), so there is nothing further to clear there.

  page="$(qa_get_pool)"
  homedepot_trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$homedepot_trip" ]]; then
    echo "FAIL: [$name] expected @homedepot to hold as a trip at two items, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$homedepot_trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "2 things" ]]; then
    echo "FAIL: [$name] expected @homedepot to read \"2 things\", got: $count_label" >&2
    FAILURES=1
  fi

  if [[ -n "$(qa_trip_section "$page" "@supermarket")" ]]; then
    echo "FAIL: [$name] expected @supermarket to never form a trip, got:
$page" >&2
    FAILURES=1
  fi
  loose="$(qa_loose_section "$page")"
  row="$(qa_loose_row_containing "$loose" "coffee")"
  if [[ -z "$row" ]]; then
    echo "FAIL: [$name] expected \"coffee\" among loose ends, got:
$loose" >&2
    FAILURES=1
  elif [[ "$row" != *"@supermarket"* ]]; then
    echo "FAIL: [$name] expected \"coffee\" to keep its tag in loose ends, got: $row" >&2
    FAILURES=1
  fi
  if [[ "$page" == *"milk"* ]]; then
    echo "FAIL: [$name] expected \"milk\" gone -- a done loose end leaves at once, got:
$page" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a tag must re-earn its trip ---
name="a-tag-must-re-earn-its-trip"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for i in 1 2 3; do
    qa_pool_task "homedepot errand $i" "@homedepot"
    qa_mark_pool_done "$(qa_task_id_for_text "homedepot errand $i")"
  done
  page="$(qa_get_pool)"
  qa_clear_done "$(qa_trip_section "$page" "@homedepot")"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing done returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_pool)"
  if [[ "$page" == *"@homedepot"* ]]; then
    echo "FAIL: [$name] expected @homedepot gone entirely after its run ended, got:
$page" >&2
    FAILURES=1
  fi

  qa_pool_task "buy a hinge" "@homedepot"
  qa_pool_task "pick up trim" "@homedepot"
  page="$(qa_get_pool)"
  if [[ -n "$(qa_trip_section "$page" "@homedepot")" ]]; then
    echo "FAIL: [$name] expected no trip with only two captured after the run ended, got:
$page" >&2
    FAILURES=1
  fi
  loose="$(qa_loose_section "$page")"
  for text in "buy a hinge" "pick up trim"; do
    if [[ -z "$(qa_loose_row_containing "$loose" "$text")" ]]; then
      echo "FAIL: [$name] expected \"$text\" among loose ends, got:
$loose" >&2
      FAILURES=1
    fi
  done

  qa_pool_task "return the drill" "@homedepot"
  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected @homedepot to form a trip again at three, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "3 things" ]]; then
    echo "FAIL: [$name] expected \"3 things\" -- the cleared three do not count toward the new run, got: $count_label" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: it survives a reload, and a restart ---
# This is why the suite exists at this tier: every acceptance step is a
# fresh HTTP request, so the Gherkin already proves the rule is not
# per-render state, but it cannot see a real reload or a real restart.
name="it-survives-a-reload-and-a-restart"
DB_FOR_RESTART="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB_FOR_RESTART" "$TMP_DIR/$name.log"; then
  for i in $(seq 1 5); do
    qa_pool_task "homedepot errand $i" "@homedepot"
  done
  for i in 1 2 3; do
    qa_mark_pool_done "$(qa_task_id_for_text "homedepot errand $i")"
  done
  qa_clear_done "$(qa_trip_section "$(qa_get_pool)" "@homedepot")"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing done returned status $STATUS" >&2
    FAILURES=1
  fi

  baseline="$(qa_trip_section "$(qa_get_pool)" "@homedepot")"
  baseline_count="$(qa_between "$baseline" '<div class="trip-count">' '</div>')"
  if [[ "$baseline_count" != "2 things" ]]; then
    echo "FAIL: [$name] setup expected \"2 things\" before reload, got: $baseline_count" >&2
    FAILURES=1
  fi

  # A hard reload: a fresh GET, same as loading the URL again.
  reloaded="$(qa_trip_section "$(qa_get_pool)" "@homedepot")"
  reloaded_count="$(qa_between "$reloaded" '<div class="trip-count">' '</div>')"
  if [[ "$reloaded_count" != "$baseline_count" ]]; then
    echo "FAIL: [$name] reload changed the count: was \"$baseline_count\", now \"$reloaded_count\"" >&2
    FAILURES=1
  fi

  # A restart: stop the process, start a fresh one against the same
  # database file -- the only way to see run state held in process memory
  # rather than derived from what is durable.
  qa_stop_server
  if ! qa_start_server "$BIN" "$DB_FOR_RESTART" "$TMP_DIR/$name-restarted.log"; then
    echo "FAIL: [$name] server did not come back up after restart" >&2
    FAILURES=1
  else
    restarted="$(qa_trip_section "$(qa_get_pool)" "@homedepot")"
    restarted_count="$(qa_between "$restarted" '<div class="trip-count">' '</div>')"
    if [[ "$restarted_count" != "$baseline_count" ]]; then
      echo "FAIL: [$name] restart changed the count: was \"$baseline_count\", now \"$restarted_count\"" >&2
      FAILURES=1
    fi
    for i in 4 5; do
      row="$(qa_trip_item_row "$restarted" "homedepot errand $i")"
      if [[ -z "$row" ]] || qa_is_struck "$row"; then
        echo "FAIL: [$name] expected \"homedepot errand $i\" open after restart, got: $row" >&2
        FAILURES=1
      fi
    done
  fi

  # Same pair of checks at the other state the doc names: 2 of 2 done,
  # nothing open.
  for i in 4 5; do
    qa_mark_pool_done "$(qa_task_id_for_text "homedepot errand $i")"
  done
  baseline="$(qa_trip_section "$(qa_get_pool)" "@homedepot")"
  baseline_count="$(qa_between "$baseline" '<div class="trip-count">' '</div>')"
  if [[ "$baseline_count" != "2 of 2 done" ]]; then
    echo "FAIL: [$name] setup expected \"2 of 2 done\" before the second reload/restart, got: $baseline_count" >&2
    FAILURES=1
  fi

  reloaded_count="$(qa_between "$(qa_trip_section "$(qa_get_pool)" "@homedepot")" '<div class="trip-count">' '</div>')"
  if [[ "$reloaded_count" != "$baseline_count" ]]; then
    echo "FAIL: [$name] reload at 2-of-2-done changed the count: was \"$baseline_count\", now \"$reloaded_count\"" >&2
    FAILURES=1
  fi

  qa_stop_server
  if ! qa_start_server "$BIN" "$DB_FOR_RESTART" "$TMP_DIR/$name-restarted-2.log"; then
    echo "FAIL: [$name] server did not come back up after the second restart" >&2
    FAILURES=1
  else
    restarted_count="$(qa_between "$(qa_trip_section "$(qa_get_pool)" "@homedepot")" '<div class="trip-count">' '</div>')"
    if [[ "$restarted_count" != "$baseline_count" ]]; then
      echo "FAIL: [$name] restart at 2-of-2-done changed the count: was \"$baseline_count\", now \"$restarted_count\"" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the controls still agree with each other ---
name="the-controls-still-agree-with-each-other"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for i in $(seq 1 5); do
    qa_pool_task "homedepot errand $i" "@homedepot"
  done
  for i in 1 2 3; do
    qa_mark_pool_done "$(qa_task_id_for_text "homedepot errand $i")"
  done
  qa_clear_done "$(qa_trip_section "$(qa_get_pool)" "@homedepot")"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing done returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  complete_endpoint="$(qa_complete_trip_endpoint "$page" "@homedepot")"
  if [[ -z "$complete_endpoint" ]]; then
    echo "FAIL: [$name] expected a complete-group control for @homedepot mid-run, got:
$page" >&2
    FAILURES=1
  fi
  trip="$(qa_trip_section "$page" "@homedepot")"
  complete_label="$(python3 -c '
import re, sys
m = re.search(r"<button type=\"button\" class=\"trip-complete\"[^>]*>([^<]*)</button>", sys.argv[1])
print(m.group(1) if m else "")
' "$trip")"
  if [[ "$complete_label" != "Complete all 2" ]]; then
    echo "FAIL: [$name] expected the complete-group control to read \"Complete all 2\", got: $complete_label" >&2
    FAILURES=1
  fi

  qa_complete_trip "$page" "@homedepot"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] completing the group returned status $STATUS" >&2
    FAILURES=1
  fi

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected the panel to hold once the group was completed, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "2 of 2 done" ]]; then
    echo "FAIL: [$name] expected \"2 of 2 done\" after completing the group, got: $count_label" >&2
    FAILURES=1
  fi
  if [[ -n "$(qa_complete_trip_endpoint "$page" "@homedepot")" ]]; then
    echo "FAIL: [$name] expected no complete-group control once nothing is left open" >&2
    FAILURES=1
  fi

  # Untick one: the item comes back, the label reads 1 of 2, and the trip
  # holds without re-earning three -- the run never ended.
  qa_mark_pool_undone "$(qa_task_id_for_text "homedepot errand 4")"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] unchecking returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected the trip to still be there after unchecking, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "1 of 2 done" ]]; then
    echo "FAIL: [$name] expected \"1 of 2 done\" after unchecking one, got: $count_label" >&2
    FAILURES=1
  fi

  # qa/trip_persistence.md's own by-hand walkthrough originally claimed
  # this clear left @homedepot "gone entirely" -- verified by hand against
  # a running server and found inconsistent with the rule stated three
  # lines above it in the same document ("a run ends when the LAST thing
  # waiting there is cleared away"): only the struck item (errand 5) is
  # cleared by this route, and the unticked one (errand 4) is still open
  # -- still waiting -- when the clear runs. Flagged to the specifier
  # rather than scripted as written; the specifier corrected the doc
  # (step 5 now reads "1 things", step 6 added) and confirmed this
  # scripted behaviour.
  qa_clear_done "$trip"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] step 5's clear returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected @homedepot to persist with its still-open item after clearing only the struck one, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "1 things" ]]; then
    echo "FAIL: [$name] expected \"1 things\" -- the one item that was never struck is still waiting, got: $count_label" >&2
    FAILURES=1
  fi

  # Step 6: tick the last item, then tap Clear done again -- this time the
  # clear leaves nothing waiting, so the run ends and the tag leaves
  # entirely.
  qa_mark_pool_done "$(qa_task_id_for_text "homedepot errand 4")"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] step 6's mark-done returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@homedepot")"
  qa_clear_done "$trip"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] step 6's clear returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_pool)"
  if [[ "$page" == *"@homedepot"* ]]; then
    echo "FAIL: [$name] expected @homedepot gone entirely once the last waiting item was cleared, got:
$page" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: cross-capability -- a run is identified by a tag, so it
# inherits that tag's canonicalisation ---
# pool-screen-case-folded-grouping-03 asserts the grouping half; nothing
# asserted the run half before this. capture::resolve_tag folds case on the
# way in, keeping the earlier spelling -- pool::group buckets by plain
# string equality, correct only because that already happened
# (T-cross-capability-invariants-need-an-owner).
name="a-run-is-identified-by-a-tag-so-it-inherits-case-folding"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_pool_task "buy screws" "@HomeDepot"
  qa_pool_task "return the drill" "@HomeDepot"
  qa_pool_task "pick up trim" "@HomeDepot"

  page="$(qa_get_pool)"
  if [[ -z "$(qa_trip_section "$page" "@HomeDepot")" ]]; then
    echo "FAIL: [$name] setup expected @HomeDepot (the first-seen spelling) to form a trip, got:
$page" >&2
    FAILURES=1
  fi

  qa_mark_pool_done "$(qa_task_id_for_text "buy screws")"
  qa_mark_pool_done "$(qa_task_id_for_text "return the drill")"
  qa_clear_done "$(qa_trip_section "$(qa_get_pool)" "@HomeDepot")"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] clearing done returned status $STATUS" >&2
    FAILURES=1
  fi

  # Capture one more at the SHOUTING-case variant -- resolve_tag folds it
  # to the earlier spelling, so this must land in the same run as the
  # three above rather than starting a fresh, below-threshold one.
  qa_pool_task "buy a hinge" "@HOMEDEPOT"

  page="$(qa_get_pool)"
  trip="$(qa_trip_section "$page" "@HomeDepot")"
  if [[ -z "$trip" ]]; then
    echo "FAIL: [$name] expected one persisted trip under the earlier spelling, got:
$page" >&2
    FAILURES=1
  fi
  count_label="$(qa_between "$trip" '<div class="trip-count">' '</div>')"
  if [[ "$count_label" != "2 things" ]]; then
    echo "FAIL: [$name] expected \"2 things\" -- one run of two open items, not two runs each below threshold, got: $count_label" >&2
    FAILURES=1
  fi
  if [[ "$page" == *"trip-tag\">@HOMEDEPOT"* || "$page" == *"trip-tag\">@homedepot"* ]]; then
    echo "FAIL: [$name] expected no second panel under a differently-cased spelling, got:
$page" >&2
    FAILURES=1
  fi
  loose="$(qa_loose_section "$page")"
  if [[ -n "$(qa_loose_row_containing "$loose" "buy a hinge")" ]]; then
    echo "FAIL: [$name] expected \"buy a hinge\" inside the persisted trip, not loose ends, got:
$loose" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: trip_persistence"
