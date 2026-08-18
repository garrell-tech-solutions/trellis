#!/usr/bin/env bash
# Executable QA procedure: qa/guardrails.md (covers features/guardrails.feature).
# Covers the automatable, curl-only procedures from qa/guardrails.md. Drives
# the running server through its HTTP interface only -- the life areas
# page's own guardrail controls, endpoints read from the page's own markup,
# never assumed -- and inspects persisted state via a read-only sqlite3
# query, never a project-internal API.
#
# qa/guardrails.md's "By-hand walkthrough" is NOT scripted here: this
# environment has no browser-automation tooling, so the runtime, visual
# facts it names (a band appearing on the page after Save, a restart with
# eyes on the screen) are not claimed as manually verified. Every fact the
# walkthrough exists to show is covered below over curl instead -- including
# the restart and archive-keeps-the-guardrail step, which the by-hand
# walkthrough and the acceptance suite both cannot show (the page filters
# archived life areas out, and acceptance runs in-process) and which is why
# it is a QA procedure rather than a Gherkin scenario, per qa/guardrails.md
# itself.
#
# qa/guardrails.md's own "nothing else changed" procedure is satisfied by
# scripts/qa/run.sh running every script in this directory together, so it
# is not re-run here.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-guardrails"
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

# The state text a life area's own row shows: "no guardrail",
# "never scheduled - menu only", or "" if it carries bands instead (a row
# with bands shows the bands themselves, not a state sentence).
qa_life_area_state() {
  local page="$1" name="$2" block
  block="$(qa_life_area_row_block "$page" "$name")"
  python3 -c '
import re, sys
block = sys.argv[1]
m = re.search(r"<span>([^<]*)</span>", block)
print(m.group(1) if m else "")
' "$block"
}

# The guardrail bands listed on name's row, one label per line, in document
# order -- "" (no lines) if the row carries none.
qa_guardrail_band_labels() {
  local page="$1" name="$2" block
  block="$(qa_life_area_row_block "$page" "$name")"
  python3 -c '
import re, sys
block = sys.argv[1]
for m in re.finditer(r"<div>([^<]*)\n<form", block):
    print(m.group(1).strip())
' "$block"
}

# POSTs name's guardrail band form (weekday checkboxes on "<days>", a
# comma-separated list of Mon/Tue/.../Sun) and sets STATUS and BODY.
qa_save_guardrail_band() {
  local page="$1" name="$2" days="$3" start="$4" end="$5" endpoint data response
  endpoint="$(qa_life_area_control_endpoint "$page" "$name" '>Add band<')"
  data="start=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' "$start")"
  data="$data&end=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' "$end")"
  IFS=',' read -ra day_list <<< "$days"
  for day in "${day_list[@]}"; do
    day="$(echo "$day" | tr -d ' ' | tr '[:upper:]' '[:lower:]')"
    [[ -z "$day" ]] && continue
    data="$data&$day=on"
  done
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "$data")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# POSTs name's pool-only form and sets STATUS and BODY.
qa_save_pool_only() {
  local page="$1" name="$2" endpoint response
  endpoint="$(qa_life_area_control_endpoint "$page" "$name" '>Save<')"
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d "pool_only=on")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

# POSTs name's guardrail form with neither a day/time nor pool_only, and
# sets STATUS and BODY -- what Save sends with nothing filled in.
qa_save_guardrail_empty() {
  local page="$1" name="$2" endpoint response
  endpoint="$(qa_life_area_control_endpoint "$page" "$name" '>Add band<')"
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' -d '')"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_guardrail_band_row_count() {
  sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM guardrail_bands;'
}

# --- Procedure: a fresh database has no guardrails ---
name="fresh-database-no-guardrails"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_life_areas)"
  for la in Work Fitness Learning Family Home; do
    state="$(qa_life_area_state "$page" "$la")"
    if [[ "$state" != "no guardrail" ]]; then
      echo "FAIL: [$name] expected $la to show \"no guardrail\", got \"$state\"" >&2
      FAILURES=1
    fi
  done
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a band is stored as authored and read back the same way ---
name="band-authored-and-read-back"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work "Mon,Tue,Wed,Thu,Fri" "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Work's band returned status $STATUS" >&2
    FAILURES=1
  fi
  labels="$(qa_guardrail_band_labels "$(qa_get_life_areas)" Work)"
  if [[ "$labels" != "Mon, Tue, Wed, Thu, Fri 09:00-17:00" ]]; then
    echo "FAIL: [$name] expected the band listed as \"Mon, Tue, Wed, Thu, Fri 09:00-17:00\", got: $labels" >&2
    FAILURES=1
  fi
  work_id="$(sqlite3 "$DB_PATH" "SELECT id FROM life_areas WHERE name = 'Work';")"
  stored="$(sqlite3 "$DB_PATH" "SELECT weekday, start_minutes, end_minutes FROM guardrail_bands WHERE life_area_id = $work_id AND weekday = 'Mon';")"
  if [[ "$stored" != "Mon|540|1020" ]]; then
    echo "FAIL: [$name] expected Mon stored as start 540 (09:00) end 1020 (17:00), got: $stored" >&2
    FAILURES=1
  fi
  column_type="$(sqlite3 "$DB_PATH" "SELECT type FROM pragma_table_info('guardrail_bands') WHERE name = 'start_minutes';")"
  if [[ "$column_type" != "INTEGER" ]]; then
    echo "FAIL: [$name] expected start_minutes to be an integer minutes-of-day column, got type $column_type" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: saving with neither a band nor the never-scheduled mark is refused ---
name="neither-band-nor-pool-only-refused"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_empty "$(qa_get_life_areas)" Work
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] expected 422, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *'id="life-areas-list"'* ]]; then
    echo "FAIL: [$name] expected the re-rendered #life-areas-list fragment, got:
$BODY" >&2
    FAILURES=1
  fi
  block="$(qa_life_area_row_block "$BODY" Work)"
  if [[ "$block" != *"guardrail-error"* ]]; then
    echo "FAIL: [$name] expected the rejection message on Work's own row, got:
$block" >&2
    FAILURES=1
  fi
  if [[ "$block" != *"guardrail band"* || "$block" != *"never scheduled"* ]]; then
    echo "FAIL: [$name] expected the message to name both a guardrail band and never scheduled, got:
$block" >&2
    FAILURES=1
  fi
  state="$(qa_life_area_state "$(qa_get_life_areas)" Work)"
  if [[ "$state" != "no guardrail" ]]; then
    echo "FAIL: [$name] expected Work to still show \"no guardrail\", got \"$state\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a life area's own bands may not overlap ---
name="own-bands-may-not-overlap"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "12:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] the first band should be accepted, got status $STATUS" >&2
    FAILURES=1
  fi

  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "11:00" "17:00"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] an overlapping band should be rejected, got status $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"overlaps"* ]]; then
    echo "FAIL: [$name] expected the rejection to say the band overlaps, got:
$BODY" >&2
    FAILURES=1
  fi
  labels="$(qa_guardrail_band_labels "$(qa_get_life_areas)" Work)"
  band_count="$(printf '%s\n' "$labels" | grep -c .)"
  if [[ "$band_count" != "1" ]]; then
    echo "FAIL: [$name] expected Work to still have exactly 1 band after the rejection, found $band_count" >&2
    FAILURES=1
  fi

  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "12:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] a touching-but-not-overlapping band should be accepted, got status $STATUS" >&2
    FAILURES=1
  fi
  labels="$(qa_guardrail_band_labels "$(qa_get_life_areas)" Work)"
  band_count="$(printf '%s\n' "$labels" | grep -c .)"
  if [[ "$band_count" != "2" ]]; then
    echo "FAIL: [$name] expected Work to have 2 bands after a touching band was added, found $band_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: two life areas may claim the same hours ---
name="cross-life-area-overlap-allowed"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] Work's band should be accepted, got status $STATUS" >&2
    FAILURES=1
  fi
  qa_save_guardrail_band "$(qa_get_life_areas)" Learning Mon "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] Learning's identical band should also be accepted, got status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_life_areas)"
  if [[ "$(qa_guardrail_band_labels "$page" Work)" != "Mon 09:00-17:00" ]]; then
    echo "FAIL: [$name] expected Work to list Mon 09:00-17:00" >&2
    FAILURES=1
  fi
  if [[ "$(qa_guardrail_band_labels "$page" Learning)" != "Mon 09:00-17:00" ]]; then
    echo "FAIL: [$name] expected Learning to also list Mon 09:00-17:00" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a never-scheduled life area is well-formed with no hours ---
name="never-scheduled-well-formed-no-hours"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_pool_only "$(qa_get_life_areas)" Work
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking Work never-scheduled returned status $STATUS" >&2
    FAILURES=1
  fi
  page="$(qa_get_life_areas)"
  state="$(qa_life_area_state "$page" Work)"
  if [[ "$state" != "never scheduled - menu only" ]]; then
    echo "FAIL: [$name] expected Work to show \"never scheduled - menu only\", got \"$state\"" >&2
    FAILURES=1
  fi
  labels="$(qa_guardrail_band_labels "$page" Work)"
  if [[ -n "$labels" ]]; then
    echo "FAIL: [$name] expected Work to carry zero bands, got: $labels" >&2
    FAILURES=1
  fi
  work_id="$(sqlite3 "$DB_PATH" "SELECT id FROM life_areas WHERE name = 'Work';")"
  pool_only="$(sqlite3 "$DB_PATH" "SELECT pool_only FROM life_areas WHERE id = $work_id;")"
  if [[ "$pool_only" != "1" ]]; then
    echo "FAIL: [$name] expected pool_only to be stored as its own explicit signal, got $pool_only" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a band can be removed ---
name="band-can-be-removed"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving the band returned status $STATUS" >&2
    FAILURES=1
  fi
  remove_endpoint="$(qa_life_area_control_endpoint "$(qa_get_life_areas)" Work ">Remove<")"
  if [[ -z "$remove_endpoint" ]]; then
    echo "FAIL: [$name] could not find the band's Remove control" >&2
    FAILURES=1
  else
    curl -s -o /dev/null -X POST "http://$ADDR$remove_endpoint"
    state="$(qa_life_area_state "$(qa_get_life_areas)" Work)"
    if [[ "$state" != "no guardrail" ]]; then
      echo "FAIL: [$name] expected Work to show \"no guardrail\" again after removing its only band, got \"$state\"" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: guardrails survive a restart, and archiving keeps them ---
name="survives-restart-archive-keeps-guardrail"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-1.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "09:00" "17:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Work's band returned status $STATUS" >&2
    FAILURES=1
  fi
  qa_save_guardrail_band "$(qa_get_life_areas)" Fitness Mon "06:00" "07:00"
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: [$name] saving Fitness's band returned status $STATUS" >&2
    FAILURES=1
  fi
  add_endpoint="$(qa_life_areas_add_endpoint "$(qa_get_life_areas)")"
  qa_add_life_area "$add_endpoint" "Side project"
  qa_save_pool_only "$(qa_get_life_areas)" "Side project"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] marking Side project never-scheduled returned status $STATUS" >&2
    FAILURES=1
  fi

  archive_endpoint="$(qa_life_area_control_endpoint "$(qa_get_life_areas)" Fitness ">Archive<")"
  curl -s -o /dev/null -X POST "http://$ADDR$archive_endpoint"
  fitness_id="$(sqlite3 "$DB" "SELECT id FROM life_areas WHERE name = 'Fitness';")"

  qa_stop_server
  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log"; then
    page="$(qa_get_life_areas)"
    if [[ "$(qa_guardrail_band_labels "$page" Work)" != "Mon 09:00-17:00" ]]; then
      echo "FAIL: [$name] expected Work's band to survive the restart" >&2
      FAILURES=1
    fi
    if [[ "$(qa_life_area_state "$page" "Side project")" != "never scheduled - menu only" ]]; then
      echo "FAIL: [$name] expected Side project's never-scheduled mark to survive the restart" >&2
      FAILURES=1
    fi
    if [[ "$page" == *">Fitness<"* ]]; then
      echo "FAIL: [$name] expected archived Fitness to no longer appear on the page" >&2
      FAILURES=1
    fi
    fitness_bands="$(sqlite3 "$DB" "SELECT COUNT(*) FROM guardrail_bands WHERE life_area_id = $fitness_id;")"
    if [[ "$fitness_bands" != "1" ]]; then
      echo "FAIL: [$name] expected Fitness's band row to still exist after archiving, found $fitness_bands" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile text stays escaped ---
name="hostile-text-escaped"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  qa_save_guardrail_band "$(qa_get_life_areas)" Work Mon "<script>alert('boom')</script>" "17:00"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] a hostile start time should be rejected, got status $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" == *"<script>"* ]]; then
    echo "FAIL: [$name] the response contains an unescaped <script> tag" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: guardrails"
