#!/usr/bin/env bash
# Executable QA procedure: qa/trip_controls.md (covers
# features/trip_controls.feature, and the browser-only half of #120 no
# acceptance scenario can hold -- expanding and collapsing). Two halves:
# scripts/qa/trip_controls.cjs driving a real Chrome for the expand
# control and the complete-group control's placement, and an HTTP-level
# timing procedure for T-set-operations-execute-in-the-store.
#
# T-cross-capability-invariants-need-an-owner: every completion below goes
# through the production route (POST /pool/trips/{tag}/complete), never a
# fixture that archives rows directly -- qa/trip_controls.md's own warning
# that a shortcut fixture proves nothing about the control.
#
# MUST FAIL, NEVER SKIP, if Chrome or playwright-core is unavailable --
# the same rule qa/phone_layout.md and qa/colour.md already carry.
#
# GATED IN GITHUB ACTIONS: .github/workflows/ci.yml's `gate` job already
# resolves PHONE_LAYOUT_CHROME for phone_layout.sh and colour.sh; this
# script reads the same variable and gets its own `run:` line.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-trip-controls"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0
SERVER_PID=""
trap qa_stop_server EXIT

if ! command -v node >/dev/null 2>&1; then
  echo "FAIL: node is not on PATH -- failing closed rather than skipping the trip-controls check" >&2
  exit 1
fi

NODE_MODULES_DIR="$(npm root -g 2>/dev/null || true)"
if [[ -z "$NODE_MODULES_DIR" || ! -d "$NODE_MODULES_DIR/playwright-core" ]]; then
  echo "FAIL: playwright-core is not installed globally (npm install -g playwright-core) -- failing closed rather than skipping the trip-controls check" >&2
  exit 1
fi

qa_get_pool() {
  curl -s "http://$ADDR/pool"
}

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

if qa_start_server "$BIN" "$TMP_DIR/trip-controls.sqlite" "$TMP_DIR/trip-controls.log"; then
  # Seed a trip of eight and a trip of five at different tags, per
  # qa/trip_controls.md's own instruction, so independence is observable.
  for i in $(seq 1 8); do
    qa_pool_task "homedepot item $i" "@homedepot"
  done
  for i in $(seq 1 5); do
    qa_pool_task "supermarket item $i" "@supermarket"
  done

  if ! NODE_PATH="$NODE_MODULES_DIR" node "$SCRIPT_DIR/trip_controls.cjs" "http://$ADDR"; then
    FAILURES=1
  fi
else
  FAILURES=1
fi
qa_stop_server

# --- Procedure: one statement, not a loop ---
# T-set-operations-execute-in-the-store / T-latency-is-a-qa-assertion's own
# condition (a quiet machine): completing twenty is one round trip through
# mark_done's front door, not twenty. A proxy, not a proof -- it cannot see
# a loop in a handler directly, but a loop of twenty statements through
# sqlx is visible in wall clock, and this is the only view QA has of it.
name="one-statement-not-a-loop"
if qa_start_server "$BIN" "$TMP_DIR/$name.sqlite" "$TMP_DIR/$name.log"; then
  for i in $(seq 1 20); do
    qa_pool_task "twenty item $i" "@bigtrip"
  done
  page="$(qa_get_pool)"
  endpoint_twenty="$(qa_complete_trip_endpoint "$page" "@bigtrip")"
  if [[ -z "$endpoint_twenty" ]]; then
    echo "FAIL: [$name] could not find the complete-group control for @bigtrip" >&2
    FAILURES=1
  else
    twenty_start_ns="$(date +%s%N)"
    curl -s -o /dev/null -X POST "http://$ADDR$endpoint_twenty"
    twenty_end_ns="$(date +%s%N)"
    twenty_ms="$(( (twenty_end_ns - twenty_start_ns) / 1000000 ))"

    qa_pool_task "one item"
    one_id="$(sqlite3 "$DB_PATH" "SELECT tasks.id FROM tasks JOIN captures ON captures.id = tasks.capture_id WHERE captures.raw_text = 'one item' ORDER BY tasks.id DESC LIMIT 1;")"
    one_start_ns="$(date +%s%N)"
    curl -s -o /dev/null -X POST "http://$ADDR/pool/tasks/$one_id/done"
    one_end_ns="$(date +%s%N)"
    one_ms="$(( (one_end_ns - one_start_ns) / 1000000 ))"

    echo "[$name] completing 20 took ${twenty_ms}ms; completing 1 took ${one_ms}ms" >&2
    # A generous bound, not a tight one: a per-row loop through sqlx would
    # cost roughly 20x, not merely "a bit more" -- 8x leaves wide margin
    # for scheduling noise on a shared machine while still catching an
    # actual per-row loop, which this project's own capture-latency-budget
    # check has already found flaky under load (one_screen.sh).
    bound_ms="$(( one_ms * 8 > 5 ? one_ms * 8 : 5 ))"
    if [[ "$twenty_ms" -gt "$bound_ms" ]]; then
      echo "FAIL: [$name] completing 20 took ${twenty_ms}ms, more than 8x completing 1 (${one_ms}ms, bound ${bound_ms}ms) -- looks like a per-row loop, not one statement" >&2
      FAILURES=1
    fi

    completed_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM tasks JOIN captures ON captures.id = tasks.capture_id WHERE captures.context_tag = '@bigtrip' AND tasks.archived_at IS NOT NULL;")"
    if [[ "$completed_count" != "20" ]]; then
      echo "FAIL: [$name] expected all 20 tasks archived, found $completed_count" >&2
      FAILURES=1
    fi
    other_archived="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM tasks JOIN captures ON captures.id = tasks.capture_id WHERE captures.context_tag != '@bigtrip' AND tasks.archived_at IS NOT NULL;")"
    if [[ "$other_archived" != "0" ]]; then
      echo "FAIL: [$name] expected nothing outside @bigtrip archived, found $other_archived" >&2
      FAILURES=1
    fi
  fi
else
  FAILURES=1
fi
qa_stop_server

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: trip_controls"
