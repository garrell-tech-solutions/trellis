#!/usr/bin/env bash
# Executable QA procedure: qa/timezone_setting.md (covers
# features/timezone_setting.feature). Covers the automatable, curl-only
# procedures from qa/timezone_setting.md. Drives the running server through
# its HTTP interface only -- the timezone control's endpoint read from the
# life areas page's own markup -- and inspects persisted state via a
# read-only sqlite3 query, never a project-internal API.
#
# qa/timezone_setting.md's "By-hand walkthrough" is NOT scripted here: this
# environment has no browser-automation tooling, so it is not claimed as
# manually verified. Every fact it names is covered below over curl instead,
# including the restart step -- the walkthrough's step 5 is exactly what
# "the zone can be changed, and it sticks" verifies.
#
# qa/timezone_setting.md's own "nothing else changed" procedure names the
# life_areas, life_area_triage and app_shell QA suites; scripts/qa/run.sh
# already runs every script in this directory together, so it is not
# re-run here.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-timezone-setting"
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

# The timezone the #timezone fragment reports, or "" if not found.
qa_reported_timezone() {
  local page="$1"
  python3 -c '
import re, sys
page = sys.argv[1]
m = re.search(r"Timezone: ([^<]*)<", page)
print(m.group(1).strip() if m else "")
' "$page"
}

# The #timezone fragment's own hx-post endpoint, read from the page's
# markup -- not assumed.
qa_timezone_endpoint() {
  local page="$1"
  python3 -c '
import re, sys
page = sys.argv[1]
m = re.search(r"<div id=\"timezone\">.*?<form hx-post=\"([^\"]+)\"", page, re.S)
print(m.group(1) if m else "")
' "$page"
}

# POSTs the timezone control with zone and sets STATUS and BODY.
qa_set_timezone() {
  local endpoint="$1" zone="$2" response
  response="$(curl -s -w '\n%{http_code}' -X POST "http://$ADDR$endpoint" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "zone=$zone")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

qa_settings_row_count() {
  sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM settings;'
}

# --- Procedure: the default is UTC ---
name="default-is-utc"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  page="$(qa_get_life_areas)"
  reported="$(qa_reported_timezone "$page")"
  if [[ "$reported" != "UTC" ]]; then
    echo "FAIL: [$name] expected the page to report UTC, got \"$reported\"" >&2
    FAILURES=1
  fi
  stored="$(sqlite3 "$DB_PATH" "SELECT timezone FROM settings WHERE id = 1;")"
  if [[ "$stored" != "UTC" ]]; then
    echo "FAIL: [$name] expected the stored timezone to be UTC, got \"$stored\"" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: the zone can be changed, and it sticks ---
name="zone-can-be-changed-and-sticks"
DB="$TMP_DIR/$name.sqlite"
if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-1.log"; then
  endpoint="$(qa_timezone_endpoint "$(qa_get_life_areas)")"
  qa_set_timezone "$endpoint" "Europe/London"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: [$name] setting Europe/London returned status $STATUS" >&2
    FAILURES=1
  fi
  reported="$(qa_reported_timezone "$(qa_get_life_areas)")"
  if [[ "$reported" != "Europe/London" ]]; then
    echo "FAIL: [$name] expected the page to report Europe/London, got \"$reported\"" >&2
    FAILURES=1
  fi
  qa_stop_server

  if qa_start_server "$BIN" "$DB" "$TMP_DIR/$name-2.log"; then
    reported="$(qa_reported_timezone "$(qa_get_life_areas)")"
    if [[ "$reported" != "Europe/London" ]]; then
      echo "FAIL: [$name] expected Europe/London to survive a restart, got \"$reported\"" >&2
      FAILURES=1
    fi
    endpoint="$(qa_timezone_endpoint "$(qa_get_life_areas)")"
    qa_set_timezone "$endpoint" "America/Denver"
    if [[ "$STATUS" != "200" ]]; then
      echo "FAIL: [$name] setting America/Denver returned status $STATUS" >&2
      FAILURES=1
    fi
    reported="$(qa_reported_timezone "$(qa_get_life_areas)")"
    if [[ "$reported" != "America/Denver" ]]; then
      echo "FAIL: [$name] expected the page to report America/Denver, got \"$reported\"" >&2
      FAILURES=1
    fi
    row_count="$(qa_settings_row_count)"
    if [[ "$row_count" != "1" ]]; then
      echo "FAIL: [$name] expected exactly one settings row, found $row_count" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: a name that is not a timezone is refused ---
name="unknown-zone-refused"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  endpoint="$(qa_timezone_endpoint "$(qa_get_life_areas)")"

  qa_set_timezone "$endpoint" "Mars/Olympus"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name-mars] expected 422, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"Mars/Olympus"* ]]; then
    echo "FAIL: [$name-mars] expected the rejection to echo the submitted value, got:
$BODY" >&2
    FAILURES=1
  fi

  qa_set_timezone "$endpoint" "Europe/Londonn"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name-typo] expected 422, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"Europe/Londonn"* ]]; then
    echo "FAIL: [$name-typo] expected the rejection to echo the submitted value, got:
$BODY" >&2
    FAILURES=1
  fi

  reported="$(qa_reported_timezone "$(qa_get_life_areas)")"
  if [[ "$reported" != "UTC" ]]; then
    echo "FAIL: [$name] expected the stored zone to remain UTC after two rejections, got \"$reported\"" >&2
    FAILURES=1
  fi
  row_count="$(qa_settings_row_count)"
  if [[ "$row_count" != "1" ]]; then
    echo "FAIL: [$name] expected exactly one settings row after two rejections, found $row_count" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: hostile text stays escaped ---
name="hostile-text-escaped"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  endpoint="$(qa_timezone_endpoint "$(qa_get_life_areas)")"
  qa_set_timezone "$endpoint" "<script>alert('boom')</script>"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: [$name] expected 422, got $STATUS" >&2
    FAILURES=1
  fi
  if [[ "$BODY" == *"<script>"* ]]; then
    echo "FAIL: [$name] the response contains an unescaped <script> tag" >&2
    FAILURES=1
  fi
  if [[ "$BODY" != *"boom"* ]]; then
    echo "FAIL: [$name] the response does not contain the word boom -- content may have been stripped instead of escaped" >&2
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: timezone_setting"
