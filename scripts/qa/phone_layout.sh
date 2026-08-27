#!/usr/bin/env bash
# Executable QA procedure: qa/phone_layout.md. No Gherkin covers this --
# see the doc's own "Why there is no Gherkin for this": every acceptance
# test and every other QA script in this project asserts over HTTP against
# markup, and none can observe a computed style, a scroll height, or a
# gesture. This one drives a real, headless Chrome via playwright-core
# against a running server at a phone viewport (390x844) and asserts
# geometry instead.
#
# MUST FAIL, NEVER SKIP, if Chrome or playwright-core is unavailable --
# qa/phone_layout.md's own explicit warning. This bug (#101) shipped in
# #92, #94 and #96, was named in all three pull-request bodies, and no
# gate in this repo could see it; a check that silently skips when its own
# dependency is missing reproduces that exact blind spot one layer down.
#
# GATED IN GITHUB ACTIONS as of #101: `.github/workflows/ci.yml` runs this
# script in the `gate` job, after the core-purity check and before the musl
# release build. The job resolves a Chrome binary into PHONE_LAYOUT_CHROME
# and fails with one clear ::error:: line if it finds none -- it does not
# skip, which is the whole point.
#
# This file previously said the opposite, correctly, for the length of one
# commit: the check shipped working but ungated, and the wiring was added at
# the owner's direction rather than by the pipeline. Recorded because a
# comment that describes CI is a comment that goes stale the moment CI
# changes, and this one already has once.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-phone-layout"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

if ! command -v node >/dev/null 2>&1; then
  echo "FAIL: node is not on PATH -- failing closed rather than skipping the phone-layout check" >&2
  exit 1
fi

NODE_MODULES_DIR="$(npm root -g 2>/dev/null || true)"
if [[ -z "$NODE_MODULES_DIR" || ! -d "$NODE_MODULES_DIR/playwright-core" ]]; then
  echo "FAIL: playwright-core is not installed globally (npm install -g playwright-core) -- failing closed rather than skipping the phone-layout check" >&2
  exit 1
fi

if qa_start_server "$BIN" "$TMP_DIR/phone-layout.sqlite" "$TMP_DIR/phone-layout.log" "2026-08-24T09:00:00Z"; then
  # Seed enough captures that Capture overflows 844px, rather than assuming
  # which screen is long -- qa/phone_layout.md's own instruction.
  for i in $(seq 1 40); do
    curl -s -o /dev/null -X POST "http://$ADDR/captures" \
      -H 'content-type: application/json' \
      -d "$(python3 -c 'import json,sys; print(json.dumps({"raw_text": f"errand number {sys.argv[1]}", "source": "web"}))' "$i")"
  done

  # #119's own "nothing else changed": a row's fields-panel makes that row
  # taller, and the document must still not scroll (qa/disclosures.md:
  # "phone_layout still passes: the row is taller with a panel open").
  # Opens one seeded capture's committed panel so the assertions below run
  # against a row that actually has one open, not just the pre-#119 shape.
  disclosures_capture_id="$(qa_submit_capture "open a panel for the phone check")"
  curl -s -o /dev/null -X POST "http://$ADDR/captures/$disclosures_capture_id/kind" \
    -H 'content-type: application/x-www-form-urlencoded' \
    -d 'kind=committed'

  # qa/committed_date.md's own "the cell does not clip, on a phone"
  # procedure: seed a committed task dated far enough out that
  # committed_screen::date_cell uses its long form ("BY 17 SEP" rather than
  # "BY THU"), the longest thing the 66px date cell must hold, and let
  # phone_layout.cjs assert none of the rendered date cells overflow.
  far_capture_id="$(qa_submit_capture "Renew the passport")"
  qa_triage "$far_capture_id" '{"kind":"committed","deadline":"2026-09-17T17:00:00Z","commitment":"by","priority":"P1","estimated_minutes":30}'
  if [[ "$STATUS" != "201" ]]; then
    echo "FAIL: setup -- triaging the far-out committed task returned status $STATUS" >&2
    FAILURES=1
  fi

  # The Quota screen owns controls no other screen has -- the quick-log
  # chips, two nested disclosures and a session row's correction form -- and
  # every one of them is a tap target this check exists to measure. An
  # unseeded /quota renders none of them, so it is seeded here for the same
  # reason the committed task above is: a screen that renders nothing cannot
  # fail a layout assertion. Server pinned to a Monday, so Mon is the only
  # day the picker offers.
  qa_quota "Piano" "4" "Mon:60 Mon:30" >/dev/null
  qa_quota "Spanish" "3" "" >/dev/null

  if ! NODE_PATH="$NODE_MODULES_DIR" node "$SCRIPT_DIR/phone_layout.cjs" "http://$ADDR"; then
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: phone_layout"
