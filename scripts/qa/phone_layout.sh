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
# NOT WIRED INTO GITHUB ACTIONS CI. .github/workflows/ci.yml invokes two
# scripts/qa/*.sh scripts by explicit name (scheduler_core_purity.sh,
# release_binary.sh); it does not glob this directory, and this script was
# not added to it. It runs locally via this file and via
# scripts/qa/run.sh, which is what a QA cycle runs -- but nothing gates a
# GitHub Actions run on it today. Whoever owns .github/workflows/ci.yml
# should decide whether a browser dependency belongs in that job; this is
# reported rather than decided here, per the brief's own instruction to
# say plainly when a check is not CI-gated rather than pretend otherwise.
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

if qa_start_server "$BIN" "$TMP_DIR/phone-layout.sqlite" "$TMP_DIR/phone-layout.log"; then
  # Seed enough captures that Capture overflows 844px, rather than assuming
  # which screen is long -- qa/phone_layout.md's own instruction.
  for i in $(seq 1 40); do
    curl -s -o /dev/null -X POST "http://$ADDR/captures" \
      -H 'content-type: application/json' \
      -d "$(python3 -c 'import json,sys; print(json.dumps({"raw_text": f"errand number {sys.argv[1]}", "source": "web"}))' "$i")"
  done

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
