#!/usr/bin/env bash
# Executable QA procedure: qa/capture_endpoint.md (covers
# features/capture_endpoint.feature). Drives the running server through its
# HTTP interface only, and inspects persisted state via a read-only sqlite3
# query -- never through a project-internal API.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-capture-endpoint"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

SERVER_PID=""
trap qa_stop_server EXIT

# Setup: start the server against a fresh database (no prior schema, so the
# captures table starts empty).
if ! qa_start_server "$BIN" "$TMP_DIR/captures.sqlite" "$TMP_DIR/server.log"; then
  exit 1
fi

FAILURES=0

check_row() {
  local raw_text="$1" source="$2"
  local count
  count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM captures WHERE raw_text = '${raw_text}' AND source = '${source}';")"
  if [[ "$count" != "1" ]]; then
    echo "FAIL: expected exactly one row for raw_text=\"$raw_text\" source=\"$source\", found $count" >&2
    FAILURES=1
  fi
}

run_example() {
  local raw_text="$1" source="$2"
  local body result status time_total time_ms
  body="$(printf '{"raw_text":"%s","source":"%s"}' "$raw_text" "$source")"

  result="$(curl -s -o /dev/null -w '%{http_code} %{time_total}' \
    -X POST "http://$ADDR/captures" \
    -H 'content-type: application/json' \
    -d "$body" || true)"
  status="${result%% *}"
  time_total="${result#* }"

  if [[ "$status" != "201" ]]; then
    echo "FAIL: raw_text=\"$raw_text\" source=\"$source\" got status $status, expected 201" >&2
    FAILURES=1
  fi

  time_ms="$(awk -v t="$time_total" 'BEGIN { printf "%.0f", t * 1000 }')"
  if (( time_ms >= 50 )); then
    echo "FAIL: raw_text=\"$raw_text\" source=\"$source\" took ${time_ms}ms, expected under 50ms" >&2
    FAILURES=1
  fi

  check_row "$raw_text" "$source"
}

run_example "buy milk" "web"
run_example "call the dentist" "telegram"

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: capture_endpoint"
