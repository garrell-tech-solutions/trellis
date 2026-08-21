#!/usr/bin/env bash
# Backs up the Trellis SQLite database to a timestamped file.
#
# T-sqlite-sqlx: the database runs in WAL mode. Copying the file (`cp`) while
# a WAL-mode database is open can copy the main file and the -wal/-shm files
# out of sync with each other, producing a file that looks fine and is
# corrupt -- and the corruption is only discovered when the backup is
# actually needed. `sqlite3 ... ".backup"` takes a consistent snapshot
# through SQLite's own backup API instead, which is safe to run against a
# live, open database. issue #98.
set -euo pipefail

DB_PATH="${1:-$HOME/.local/share/trellis/trellis.db}"
BACKUP_DIR="${TRELLIS_BACKUP_DIR:-$HOME/.local/share/trellis/backups}"

if [[ ! -f "$DB_PATH" ]]; then
  echo "no database at $DB_PATH" >&2
  exit 1
fi

mkdir -p "$BACKUP_DIR"

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DEST="$BACKUP_DIR/trellis-$STAMP.db"

sqlite3 "$DB_PATH" ".backup '$DEST'"

# Verify the backup is a file sqlite3 can actually open and read, not just a
# file that exists. A backup nobody ever verified is a hope, not a backup.
INTEGRITY="$(sqlite3 "$DEST" "PRAGMA integrity_check;")"
if [[ "$INTEGRITY" != "ok" ]]; then
  echo "backup at $DEST failed integrity check: $INTEGRITY" >&2
  exit 1
fi

echo "$DEST"
