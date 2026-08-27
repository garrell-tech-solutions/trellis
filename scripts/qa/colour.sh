#!/usr/bin/env bash
# Executable QA procedure: qa/colour.md (covers features/colour.feature and
# the browser-only half nothing in the acceptance runtime can see). Two
# halves: plain HTTP for the metadata (color-scheme, theme-color, the
# manifest), and scripts/qa/colour.cjs driving a real Chrome for contrast,
# the closed palette, literal white, and the two schemes actually
# differing -- the same real-browser requirement scripts/qa/phone_layout.sh
# established for #101, extended here for #124/#128.
#
# MUST FAIL, NEVER SKIP, if Chrome or playwright-core is unavailable --
# qa/colour.md's own explicit warning, harder here than for phone_layout:
# the failure this slice fixes shipped, was measured, was written down in
# an issue, and stayed shipped. A check that goes quiet when its browser
# is missing reproduces that one layer down.
#
# GATED IN GITHUB ACTIONS: .github/workflows/ci.yml's `gate` job already
# resolves a Chrome binary into PHONE_LAYOUT_CHROME for phone_layout.sh;
# this script reads the same variable and gets its own `run:` line right
# after it, per qa/colour.md's own instruction that it needs no second
# resolution step.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-colour"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

if ! command -v node >/dev/null 2>&1; then
  echo "FAIL: node is not on PATH -- failing closed rather than skipping the colour check" >&2
  exit 1
fi

NODE_MODULES_DIR="$(npm root -g 2>/dev/null || true)"
if [[ -z "$NODE_MODULES_DIR" || ! -d "$NODE_MODULES_DIR/playwright-core" ]]; then
  echo "FAIL: playwright-core is not installed globally (npm install -g playwright-core) -- failing closed rather than skipping the colour check" >&2
  exit 1
fi

qa_get_screen() {
  curl -s "http://$ADDR$1"
}

# Every value a <meta name="color-scheme"> or <meta name="theme-color">
# carries, in document order.
qa_meta_values() {
  local body="$1" name="$2"
  python3 -c '
import re, sys
body, name = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<meta name=\"" + re.escape(name) + r"\"([^>]*)>", body):
    attrs = m.group(1)
    content_m = re.search(r"content=\"([^\"]*)\"", attrs)
    media_m = re.search(r"media=\"([^\"]*)\"", attrs)
    print((content_m.group(1) if content_m else "") + "\t" + (media_m.group(1) if media_m else ""))
' "$body" "$name"
}

if qa_start_server "$BIN" "$TMP_DIR/colour.sqlite" "$TMP_DIR/colour.log" "2026-08-24T09:00:00Z"; then
  # --- Seed so that every component this doc names actually appears ---
  # (qa/colour.md: "an element that does not render cannot be measured").
  seed_report=()

  # Several captures: rows, meta lines, the quick-add box and its
  # placeholders render on every /  response regardless of content.
  qa_submit_capture "buy milk" >/dev/null
  qa_submit_capture "call the dentist" >/dev/null
  seed_report+=("2 plain captures (rows, meta lines)")

  # One capture with its committed panel open: the chosen kind chip, the
  # open commitment summary, the field panel and its labels.
  open_id="$(qa_submit_capture "book the dentist")"
  kind_endpoint="$(qa_row_control_endpoint "$(qa_get_screen /)" "$open_id" 'value="committed"')"
  qa_triage_form "$kind_endpoint" "kind=committed"
  if [[ "$STATUS" != "200" ]]; then
    echo "FAIL: setup -- opening the committed panel returned status $STATUS" >&2
    FAILURES=1
  fi
  seed_report+=("1 capture with its committed panel open (chosen chip, open summary, field panel)")

  # One triage rejection: submit the open committed panel's own form with
  # a required field missing, rendering the warning panel (.triage-error).
  reject_endpoint="$(qa_open_panel_endpoint "$(qa_capture_row_block "$BODY" "$open_id")" committed)"
  qa_triage_form "$reject_endpoint" "kind=committed&commitment=by&priority=P1&estimated_minutes=30"
  if [[ "$STATUS" != "422" ]]; then
    echo "FAIL: setup -- the deliberate rejection returned status $STATUS, expected 422" >&2
    FAILURES=1
  fi
  seed_report+=("1 triage rejection (warning panel, .triage-error)")

  # A trip: three pool tasks at one tag, one marked done (trip panel,
  # progress label, clear control, struck item).
  for text in "buy screws" "return the drill" "pick up trim"; do
    qa_pool_task "$text" "@homedepot"
  done
  screws_id="$(sqlite3 "$DB_PATH" "SELECT tasks.id FROM tasks JOIN captures ON captures.id = tasks.capture_id WHERE captures.raw_text = 'buy screws' ORDER BY tasks.id DESC LIMIT 1;")"
  curl -s -o /dev/null -X POST "http://$ADDR/pool/tasks/$screws_id/done"
  seed_report+=("1 trip (@homedepot, 3 tasks, 1 struck: progress label, clear control)")

  # A loose end: a tagged pool task below the trip threshold.
  qa_pool_task "fix the door latch" "@garage"
  seed_report+=("1 loose end (@garage, below threshold, keeps its tag)")

  # Two quotas: one mid-week (name, readout, a part-filled bar, the note,
  # both quick-log chips and the two disclosure summaries) and one with
  # nothing logged (the empty-week message and "nothing logged"). TWO, not
  # three -- /quota is declared `expectOverflow: false`, and a third row
  # pushes the screen past 844px and fails that assertion rather than any
  # colour one. Server pinned to a Monday above, so Mon is the only day the
  # picker offers and the only day a session can be logged against.
  qa_quota "Piano" "4" "Mon:60 Mon:30" >/dev/null
  qa_quota "Spanish" "3" "" >/dev/null
  seed_report+=("2 quotas (part-filled bar + sessions, empty week)")

  # A committed task with a past date: the past date cell and the PAST
  # badge. Server pinned to 2026-08-24T09:00:00Z above.
  past_id="$(qa_submit_capture "renew the passport")"
  qa_triage "$past_id" '{"kind":"committed","deadline":"2026-08-20T17:00:00Z","commitment":"by","priority":"P1","estimated_minutes":30}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: setup -- the past-dated committed task returned status $STATUS" >&2
    FAILURES=1
  fi
  seed_report+=("1 past-dated committed task (past date cell, PAST badge)")

  echo "seed inventory:" >&2
  for line in "${seed_report[@]}"; do
    echo "  - $line" >&2
  done

  # --- Procedure: the metadata, over HTTP ---
  for screen in / /pool /committed; do
    body="$(qa_get_screen "$screen")"
    scheme_values="$(qa_meta_values "$body" "color-scheme")"
    scheme_content="$(echo "$scheme_values" | head -1 | cut -f1)"
    if [[ "$scheme_content" != "light dark" ]]; then
      echo "FAIL: [metadata] $screen declares color-scheme \"$scheme_content\", expected \"light dark\"" >&2
      FAILURES=1
    fi

    theme_values="$(qa_meta_values "$body" "theme-color")"
    theme_count="$(echo "$theme_values" | grep -c . || true)"
    if [[ "$theme_count" != "2" ]]; then
      echo "FAIL: [metadata] $screen carries $theme_count theme-color meta tag(s), expected 2" >&2
      FAILURES=1
    fi
    light_line="$(echo "$theme_values" | sed -n '1p')"
    dark_line="$(echo "$theme_values" | sed -n '2p')"
    light_content="$(echo "$light_line" | cut -f1)"
    light_media="$(echo "$light_line" | cut -f2)"
    dark_content="$(echo "$dark_line" | cut -f1)"
    dark_media="$(echo "$dark_line" | cut -f2)"
    if [[ -n "$light_media" ]]; then
      echo "FAIL: [metadata] $screen's first theme-color carries a media attribute (\"$light_media\") -- the unmediated one must be first and stay light" >&2
      FAILURES=1
    fi
    if [[ "$light_content" != "#fafdfe" ]]; then
      echo "FAIL: [metadata] $screen's light theme-color is \"$light_content\", expected #fafdfe" >&2
      FAILURES=1
    fi
    if [[ "$dark_media" != "(prefers-color-scheme: dark)" ]]; then
      echo "FAIL: [metadata] $screen's second theme-color's media is \"$dark_media\", expected \"(prefers-color-scheme: dark)\"" >&2
      FAILURES=1
    fi
    if [[ "$dark_content" != "#091014" ]]; then
      echo "FAIL: [metadata] $screen's dark theme-color is \"$dark_content\", expected #091014" >&2
      FAILURES=1
    fi
  done

  manifest_body="$(curl -s "http://$ADDR/manifest.webmanifest")"
  for pair in "theme_color=#fafdfe" "background_color=#fafdfe"; do
    member="${pair%%=*}"
    expected="${pair#*=}"
    actual="$(python3 -c '
import json, sys
print(json.loads(sys.argv[1]).get(sys.argv[2], ""))
' "$manifest_body" "$member")"
    if [[ "$actual" != "$expected" ]]; then
      echo "FAIL: [metadata] manifest member \"$member\" is \"$actual\", expected \"$expected\"" >&2
      FAILURES=1
    fi
  done

  # --- Procedure: the automated check, in both schemes (browser) ---
  if ! NODE_PATH="$NODE_MODULES_DIR" node "$SCRIPT_DIR/colour.cjs" "http://$ADDR"; then
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: colour"
