#!/usr/bin/env bash
# Executable QA procedure: qa/migrations.md (covers features/migrations.feature).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-migrations"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"
DB_PATH="$TMP_DIR/captures.sqlite"

echo "building trellis..." >&2
cargo build --quiet -p trellis-server --bin trellis
BIN="$ROOT_DIR/target/debug/trellis"

FAILURES=0

# Procedure -- empty database.
if ! "$BIN" migrate --db "$DB_PATH"; then
  echo "FAIL: migrate against empty database exited non-zero, expected 0" >&2
  FAILURES=1
fi

JOURNAL_MODE="$(sqlite3 "$DB_PATH" 'PRAGMA journal_mode;')"
if [[ "$JOURNAL_MODE" != "wal" ]]; then
  echo "FAIL: journal_mode is \"$JOURNAL_MODE\", expected \"wal\"" >&2
  FAILURES=1
fi

# Procedure -- re-run against an already-migrated database.
SCHEMA_BEFORE="$(sqlite3 "$DB_PATH" '.schema')"

if ! "$BIN" migrate --db "$DB_PATH"; then
  echo "FAIL: re-run migrate exited non-zero, expected 0" >&2
  FAILURES=1
fi

SCHEMA_AFTER="$(sqlite3 "$DB_PATH" '.schema')"
if [[ "$SCHEMA_BEFORE" != "$SCHEMA_AFTER" ]]; then
  echo "FAIL: schema changed on re-run" >&2
  diff <(echo "$SCHEMA_BEFORE") <(echo "$SCHEMA_AFTER") >&2 || true
  FAILURES=1
fi

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: migrations"
