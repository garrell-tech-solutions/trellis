#!/usr/bin/env bash
# Executable QA procedure: qa/life_areas.md (covers features/life_areas.feature).
# Drives the running server through its HTTP interface only -- the page at
# GET /life-areas, the page at GET /, and whatever endpoints their own
# controls submit to (read from the page's own markup, not assumed, per the
# procedure's "Independent of Implementation" note) -- and inspects persisted
# state via a read-only sqlite3 query, never a project-internal API.
#
# qa/life_areas.md's "By-hand walkthrough" is deliberately NOT scripted here,
# for the same reason inbox_view.sh and triage_from_page.sh don't script
# theirs: this project's stack has no browser-automation tooling, and the
# walkthrough exists to check what a browser renders. That walkthrough covers
# both this slice and life_area_triage's (qa/life_areas.md says so directly),
# so qa/life_area_triage.md does not repeat it and neither does
# life_area_triage.sh. It was performed manually this QA cycle, over the
# identical HTTP surface a browser would drive, and passed in full: a fresh
# database listed Work/Fitness/Learning/Family/Home; adding "Side project"
# appeared in the list without a restart; triaging "sketch the landing page"
# into it tagged the task list row with that name; and both the new life area
# and the tag survived a server restart.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-life-areas"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

qa_get_life_areas() {
  curl -s "http://$ADDR/life-areas"
}

# The ordered list of names in the #life-areas-list fragment's <ul
# id="life-areas">, from each row's <span class="life-area-name"> -- for
# membership/order/count assertions. Exact whitespace is checked separately
# (qa_raw_life_area_row), because askama never puts stray whitespace inside
# that span, so this helper needs no trimming of its own to hide.
qa_life_area_names() {
  local page="$1" section
  section="$(qa_html_section "$page" life-areas)"
  python3 -c '
import re, sys
section = sys.argv[1]
for m in re.finditer(r"<span class=\"life-area-name\">([^<]*)</span>", section):
    print(m.group(1))
' "$section"
}

# The exact text a life area named (or containing) needle renders with in
# its <span class="life-area-name">, leading/trailing whitespace preserved
# -- "" if no row matches.
qa_raw_life_area_row() {
  local page="$1" needle="$2"
  python3 -c '
import re, sys
page, needle = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<span class=\"life-area-name\">([^<]*)</span>", page):
    if needle in m.group(1):
        print(m.group(1))
        break
' "$page" "$needle"
}

# The archive form's hx-post endpoint for the row whose name is exactly
# `name`, read from that row's own markup.
qa_life_area_archive_endpoint() {
  qa_life_area_control_endpoint "$1" "$2" ">Archive<"
}

qa_life_area_row_count() {
  sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM life_areas;'
}

SEEDED_NAMES=$'Work\nFitness\nLearning\nFamily\nHome'

# --- Procedure: a fresh database is seeded with five life areas ---
name="seeded-five"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  names="$(qa_life_area_names "$(qa_get_life_areas)")"
  if [[ "$names" != "$SEEDED_NAMES" ]]; then
    echo "FAIL: [$name] expected exactly Work,Fitness,Learning,Family,Home in order, got:
$names" >&2
    FAILURES=1
  fi
  row_count="$(qa_life_area_row_count)"
  if [[ "$row_count" != "5" ]]; then
    echo "FAIL: [$name] expected 5 rows in life_areas, found $row_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: adding a life area, with no restart ---
# A capture is submitted first even though the procedure text doesn't say
# to: the triage picker this procedure checks on / is rendered per capture
# row (capture_row.html), not as a page-level control, so there is nothing
# to read without one -- the archived-06 procedure below needs the same
# capture for the same reason.
name="add-no-restart"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_submit_capture "sketch the landing page" >/dev/null
  endpoint="$(qa_life_areas_add_endpoint "$(qa_get_life_areas)")"
  if [[ -z "$endpoint" ]]; then
    echo "FAIL: [$name] could not find the add-life-area control" >&2
    FAILURES=1
  else
    qa_add_life_area "$endpoint" "Side project"
    if [[ "$STATUS" -ge 300 && "$STATUS" -lt 400 ]]; then
      echo "FAIL: [$name] add responded with a redirect (status $STATUS)" >&2
      FAILURES=1
    fi
    names="$(qa_life_area_names "$(qa_get_life_areas)")"
    if [[ "$(echo "$names" | tail -1)" != "Side project" || "$(echo "$names" | wc -l | tr -d ' ')" != "6" ]]; then
      echo "FAIL: [$name] expected \"Side project\" listed sixth (six total), got:
$names" >&2
      FAILURES=1
    fi
    picker="$(curl -s "http://$ADDR/")"
    if [[ "$picker" != *"Side project"* ]]; then
      echo "FAIL: [$name] the triage picker on / does not offer \"Side project\" without a restart" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a life area cannot be added twice ---
name="no-duplicates"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  endpoint="$(qa_life_areas_add_endpoint "$(qa_get_life_areas)")"
  for variant in Work work WORK; do
    qa_add_life_area "$endpoint" "$variant"
    if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
      echo "FAIL: [$name] adding \"$variant\" should be rejected, got status $STATUS" >&2
      FAILURES=1
    fi
    if [[ "$BODY" != *"already a life area"* ]]; then
      echo "FAIL: [$name] expected the rejection for \"$variant\" to say it is already a life area, got:
$BODY" >&2
      FAILURES=1
    fi
  done
  names="$(qa_life_area_names "$(qa_get_life_areas)")"
  count="$(echo "$names" | grep -c .)"
  if [[ "$count" != "5" ]]; then
    echo "FAIL: [$name] expected exactly 5 listed after three duplicate attempts, found $count:
$names" >&2
    FAILURES=1
  fi
  row_count="$(qa_life_area_row_count)"
  if [[ "$row_count" != "5" ]]; then
    echo "FAIL: [$name] expected exactly 5 rows after three duplicate attempts, found $row_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: names are trimmed, and a blank name is refused ---
name="trim-and-blank"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  endpoint="$(qa_life_areas_add_endpoint "$(qa_get_life_areas)")"
  qa_add_life_area "$endpoint" "  Side project  "
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] adding \"  Side project  \" should succeed, got status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_life_areas)"
  raw_row="$(qa_raw_life_area_row "$page" "Side project")"
  if [[ "$raw_row" != "Side project" ]]; then
    echo "FAIL: [$name] expected the rendered name trimmed to exactly \"Side project\", got: $(printf '%q' "$raw_row")" >&2
    FAILURES=1
  fi
  stored="$(sqlite3 "$DB_PATH" "SELECT name FROM life_areas WHERE name LIKE '%Side project%';")"
  if [[ "$stored" != "Side project" ]]; then
    echo "FAIL: [$name] expected the stored name trimmed to exactly \"Side project\", got: $(printf '%q' "$stored")" >&2
    FAILURES=1
  fi

  qa_add_life_area "$endpoint" "   "
  if [[ "$STATUS" -lt 400 || "$STATUS" -ge 500 ]]; then
    echo "FAIL: [$name] a whitespace-only name should be rejected, got status $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"name"* ]]; then
    echo "FAIL: [$name] expected the rejection to name the name field, got:
$BODY" >&2
    FAILURES=1
  fi
  names="$(qa_life_area_names "$(qa_get_life_areas)")"
  count="$(echo "$names" | grep -c .)"
  if [[ "$count" != "6" ]]; then
    echo "FAIL: [$name] expected still 6 listed after the blank-name rejection, found $count:
$names" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: archiving takes a life area out of circulation without erasing it ---
name="archive-not-erase"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  capture_id="$(qa_submit_capture "sketch the landing page")"
  qa_triage "$capture_id" '{"kind":"pool","life_area":"Learning"}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] setup triage into Learning returned status $STATUS, expected 201" >&2
    FAILURES=1
  fi
  archive_endpoint="$(qa_life_area_archive_endpoint "$(qa_get_life_areas)" "Learning")"
  if [[ -z "$archive_endpoint" ]]; then
    echo "FAIL: [$name] could not find Learning's archive control" >&2
    FAILURES=1
  else
    response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$archive_endpoint")"
    archive_status="${response##*$'\n'}"
    if [[ "$archive_status" -lt 200 || "$archive_status" -ge 300 ]]; then
      echo "FAIL: [$name] archiving Learning returned status $archive_status" >&2
      FAILURES=1
    fi
    life_areas_page="$(qa_get_life_areas)"
    if [[ "$life_areas_page" == *">Learning<"* || "$(qa_life_area_names "$life_areas_page")" == *Learning* ]]; then
      echo "FAIL: [$name] Learning still appears on /life-areas after archiving" >&2
      FAILURES=1
    fi
    home_page="$(curl -s "http://$ADDR/")"
    if [[ "$home_page" == *"<option value=\"Learning\">"* ]]; then
      echo "FAIL: [$name] the triage picker on / still offers Learning after archiving" >&2
      FAILURES=1
    fi
    tasks="$(qa_html_section "$home_page" tasks)"
    if [[ "$tasks" != *"Learning"* ]]; then
      echo "FAIL: [$name] the already-triaged task no longer shows tagged Learning after archiving" >&2
      FAILURES=1
    fi
    archived_at="$(sqlite3 "$DB_PATH" "SELECT archived_at FROM life_areas WHERE name = 'Learning';")"
    if [[ -z "$archived_at" ]]; then
      echo "FAIL: [$name] expected Learning's row to still exist with archived_at set, found none" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile text in a life area name stays escaped ---
name="hostile-text"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_submit_capture "sketch the landing page" >/dev/null
  endpoint="$(qa_life_areas_add_endpoint "$(qa_get_life_areas)")"
  qa_add_life_area "$endpoint" "<script>alert('boom')</script>"
  life_areas_page="$(qa_get_life_areas)"
  if [[ "$life_areas_page" == *"<script>"* ]]; then
    echo "FAIL: [$name] /life-areas contains an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$life_areas_page" != *"boom"* ]]; then
    echo "FAIL: [$name] /life-areas does not contain the word \"boom\" -- content may have been stripped instead of escaped" >&2
    FAILURES=1
  fi
  picker_page="$(curl -s "http://$ADDR/")"
  if [[ "$picker_page" == *"<script>"* ]]; then
    echo "FAIL: [$name] the triage picker on / contains an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$picker_page" != *"boom"* ]]; then
    echo "FAIL: [$name] the triage picker on / does not contain the word \"boom\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: life areas survive a restart ---
name="survives-restart"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-1.log"; then
  endpoint="$(qa_life_areas_add_endpoint "$(qa_get_life_areas)")"
  qa_add_life_area "$endpoint" "Side project"
  archive_endpoint="$(qa_life_area_archive_endpoint "$(qa_get_life_areas)" "Learning")"
  curl -s -o /dev/null -X POST "http://$ADDR$archive_endpoint"
  qa_stop_server
  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log"; then
    names="$(qa_life_area_names "$(qa_get_life_areas)")"
    if [[ "$names" != *"Side project"* ]]; then
      echo "FAIL: [$name] \"Side project\" is missing after a server restart" >&2
      FAILURES=1
    fi
    if [[ "$names" == *Learning* ]]; then
      echo "FAIL: [$name] \"Learning\" still lists as active after a server restart" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: life_areas"
