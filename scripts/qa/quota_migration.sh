#!/usr/bin/env bash
# Executable QA procedure: qa/quota_migration.md (covers
# features/quota_migration.feature). Drives migration 0016 through the
# `trellis migrate` command and the Quota screen over HTTP; sqlite3 is used
# read-only for corroboration, and to seed the pre-#138 fixture rows the
# running product can no longer create through any live route (the old
# triage shape -- target_count/target_minutes_each/period -- was retired by
# this same slice). Never used to fake the migration's own effect.
#
# WHY A SECOND BINARY: `sqlx::migrate!("./migrations")` embeds the migration
# set at COMPILE TIME, so there is no runtime flag to stop it short of 0016.
# To reproduce "the owner's own upgrade path" -- fixture rows written before
# 0014/0015/0016 have ever run, then all three applied in one process, the
# same as the owner's actual pre-#147 binary meeting trunk today -- this
# script builds a second `trellis` binary with migration 0016 temporarily
# moved out of the crate, migrates a fresh database with THAT binary (so it
# stops at 0015), inserts the legacy fixture rows, then migrates the same
# database file with the normal (0016-carrying) binary built by every other
# QA script. A trap restores the moved file even on failure -- this must
# never leave the working tree missing a tracked migration.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"
cd "$ROOT_DIR"

TMP_DIR="./tmp/qa-quota-migration"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

MIGRATION_0016="crates/trellis-server/migrations/0016_quota_triage_becomes_quotas.sql"
MIGRATION_0016_STASH="$TMP_DIR/0016.sql.stash"

FAILURES=0
SERVER_PID=""

restore_migration_0016() {
  qa_stop_server
  if [[ -f "$MIGRATION_0016_STASH" && ! -f "$MIGRATION_0016" ]]; then
    mv "$MIGRATION_0016_STASH" "$MIGRATION_0016"
  fi
}
trap restore_migration_0016 EXIT

# Separate target directories AND a forced touch of platform/db.rs (the
# `sqlx::migrate!("./migrations")` call site) before each build: observed
# directly in this environment that the proc macro's own file-level
# rebuild-tracking only covers files it has already read. It does not
# notice a NEW file (0016) appearing, so a build taken right after
# restoring 0016 can still silently omit it unless something forces the
# call site itself to be seen as changed. Touching db.rs is what reliably
# does that; a fresh --target-dir per variant additionally guarantees no
# artifact from the other variant is ever reused.
force_rebuild_migrator() {
  touch crates/trellis-server/src/platform/db.rs
}

echo "building trellis (full, with migration 0016)..." >&2
force_rebuild_migrator
cargo build --quiet -p trellis-server --bin trellis --target-dir "$TMP_DIR/target-full"
BIN_FULL="$TMP_DIR/trellis-full"
cp "$TMP_DIR/target-full/debug/trellis" "$BIN_FULL"

echo "building trellis (pre-0016, migration 0016 removed)..." >&2
mv "$MIGRATION_0016" "$MIGRATION_0016_STASH"
force_rebuild_migrator
cargo build --quiet -p trellis-server --bin trellis --target-dir "$TMP_DIR/target-pre16"
BIN_PRE16="$TMP_DIR/trellis-pre16"
cp "$TMP_DIR/target-pre16/debug/trellis" "$BIN_PRE16"
mv "$MIGRATION_0016_STASH" "$MIGRATION_0016"
force_rebuild_migrator

# Inserts a pre-#138 quota task directly (captures + tasks rows), the shape
# a live pre-0016 binary wrote and no route in this build can write any
# more. left_inbox_at is set: this task was already triaged, not sitting in
# the inbox.
seed_legacy_quota_task() {
  local db="$1" raw_text="$2" target_count="$3" target_minutes_each="$4" sql_escaped
  sql_escaped="${raw_text//\'/\'\'}"
  sqlite3 "$db" "
INSERT INTO captures (raw_text, source, created_at_ms, left_inbox_at)
VALUES ('$sql_escaped', 'web', 0, 1);
INSERT INTO tasks (capture_id, kind, target_count, target_minutes_each, period, created_at_ms)
VALUES (last_insert_rowid(), 'quota', $target_count, $target_minutes_each, 'week', 0);
"
}

# Inserts a screen-defined quota directly into the (post-0014) quotas table
# -- the shape #147's now-retired define form wrote.
seed_screen_quota() {
  local db="$1" name="$2" weekly_target_minutes="$3" sql_escaped
  sql_escaped="${name//\'/\'\'}"
  sqlite3 "$db" "INSERT INTO quotas (name, weekly_target_minutes, created_at_ms) VALUES ('$sql_escaped', $weekly_target_minutes, 0);"
}

qa_get_quota() {
  curl -s "http://$ADDR/quota"
}

qa_quota_meta() {
  qa_between "$1" '<div class="quota-meta">' '</div>'
}

qa_quota_names_in_order() {
  python3 -c '
import re, sys
for m in re.finditer(r"<div class=\"quota-name\">([^<]*)</div>", sys.argv[1]):
    print(m.group(1))
' "$1"
}

qa_quota_row_block() {
  local page="$1" name="$2"
  python3 -c '
import re, sys
page, name = sys.argv[1], sys.argv[2]
for m in re.finditer(r"<div class=\"quota-row\"[^>]*>(?:(?!<div class=\"quota-row\").)*", page, re.S):
    block = m.group(0)
    nm = re.search(r"<div class=\"quota-name\">([^<]*)</div>", block)
    if nm and nm.group(1) == name:
        print(block)
        sys.exit()
' "$page" "$name"
}

qa_quota_readout() {
  qa_between "$1" '<div class="quota-readout">' '</div>'
}

# --- Procedure: the owner's own upgrade path ---
name="owners-own-upgrade-path"
DB="$TMP_DIR/$name.sqlite"
if "$BIN_PRE16" migrate --db "$DB"; then
  # quotas is migration 0014's table, so the pre-0016 binary still creates
  # it -- what must be true here is that it is EMPTY: 0016 (the conversion)
  # has not run yet, only 0014 (the table) and 0015 (sessions) have.
  quota_rows_before="$(sqlite3 "$DB" "SELECT COUNT(*) FROM quotas;")"
  if [[ "$quota_rows_before" != "0" ]]; then
    echo "FAIL: [$name] setup -- expected an empty quotas table before 0016 has run, found $quota_rows_before row(s)" >&2
    FAILURES=1
  fi
  seed_legacy_quota_task "$DB" "workout" 3 45
  seed_legacy_quota_task "$DB" "learning with lev" 1 30
  pre_pool_like_count="$(sqlite3 "$DB" "SELECT COUNT(*) FROM tasks WHERE kind = 'quota';")"
  if [[ "$pre_pool_like_count" != "2" ]]; then
    echo "FAIL: [$name] setup -- expected two legacy quota tasks seeded, found $pre_pool_like_count" >&2
    FAILURES=1
  fi

  if ! "$BIN_FULL" migrate --db "$DB"; then
    echo "FAIL: [$name] migrate (0016 included) exited non-zero against the owner's upgrade path" >&2
    FAILURES=1
  fi
  journal_mode="$(sqlite3 "$DB" 'PRAGMA journal_mode;')"
  if [[ "$journal_mode" != "wal" ]]; then
    echo "FAIL: [$name] journal_mode is \"$journal_mode\", expected \"wal\"" >&2
    FAILURES=1
  fi

  if qa_start_server "$BIN_FULL" "$DB" "$TMP_DIR/$name.log"; then
    page="$(qa_get_quota)"
    meta="$(qa_quota_meta "$page")"
    if [[ "$meta" != "2 quotas" ]]; then
      echo "FAIL: [$name] expected meta \"2 quotas\", got: $meta" >&2
      FAILURES=1
    fi
    workout_readout="$(qa_quota_readout "$(qa_quota_row_block "$page" "workout")")"
    if [[ "$workout_readout" != "0m / 2h 15m" ]]; then
      echo "FAIL: [$name] expected workout's readout \"0m / 2h 15m\" (3 x 45 = 135), got: $workout_readout" >&2
      FAILURES=1
    fi
    lev_readout="$(qa_quota_readout "$(qa_quota_row_block "$page" "learning with lev")")"
    if [[ "$lev_readout" != "0m / 30m" ]]; then
      echo "FAIL: [$name] expected \"learning with lev\"'s readout \"0m / 30m\" (1 x 30 = 30), got: $lev_readout" >&2
      FAILURES=1
    fi
    db_quota_count="$(sqlite3 "$DB" "SELECT COUNT(*) FROM quotas;")"
    if [[ "$db_quota_count" != "2" ]]; then
      echo "FAIL: [$name] expected exactly two rows in quotas, found $db_quota_count" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
  qa_stop_server

  # Re-run: a no-op. sqlx tracks 0016 as applied and will not run it again,
  # so this also corroborates that guarantee rather than only the INSERT OR
  # IGNORE that would make a second literal run safe.
  if ! "$BIN_FULL" migrate --db "$DB"; then
    echo "FAIL: [$name] re-running migrate exited non-zero" >&2
    FAILURES=1
  fi
  if qa_start_server "$BIN_FULL" "$DB" "$TMP_DIR/$name-rerun.log"; then
    meta="$(qa_quota_meta "$(qa_get_quota)")"
    if [[ "$meta" != "2 quotas" ]]; then
      echo "FAIL: [$name] re-run -- expected meta still \"2 quotas\" (no duplication), got: $meta" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
  qa_stop_server
else
  echo "FAIL: [$name] setup -- pre-0016 migrate exited non-zero against a fresh database" >&2
  FAILURES=1
fi

# --- Procedure: a name that is already taken ---
run_collision() {
  local variant="$1"
  local name="collision-$variant"
  local DB="$TMP_DIR/$name.sqlite"
  "$BIN_PRE16" migrate --db "$DB" >/dev/null
  seed_screen_quota "$DB" "Piano" 240
  seed_legacy_quota_task "$DB" "$variant" 2 30

  if ! "$BIN_FULL" migrate --db "$DB"; then
    echo "FAIL: [$name] expected migrate to exit successfully on a name collision (UNIQUE COLLATE NOCASE, INSERT OR IGNORE), it did not" >&2
    FAILURES=1
    return
  fi

  if qa_start_server "$BIN_FULL" "$DB" "$TMP_DIR/$name.log"; then
    page="$(qa_get_quota)"
    meta="$(qa_quota_meta "$page")"
    if [[ "$meta" != "1 quota" ]]; then
      echo "FAIL: [$name] expected meta \"1 quota\" (no second counter for \"$variant\"), got: $meta" >&2
      FAILURES=1
    fi
    names="$(qa_quota_names_in_order "$page")"
    if [[ "$names" != "Piano" ]]; then
      echo "FAIL: [$name] expected only \"Piano\" to survive, got: $names" >&2
      FAILURES=1
    fi
    readout="$(qa_quota_readout "$(qa_quota_row_block "$page" "Piano")")"
    if [[ "$readout" != "0m / 4h" ]]; then
      echo "FAIL: [$name] expected Piano's own target to survive unchanged (\"0m / 4h\"), not overwritten and not summed, got: $readout" >&2
      FAILURES=1
    fi
  else
    FAILURES=1
  fi
  qa_stop_server
}
run_collision "piano"
run_collision "PIANO"

# --- Procedure: the limit that is deliberately not asserted ---
# "Pi-ano" folds case (COLLATE NOCASE) but not punctuation, so this is
# reported as a FINDING -- both one-quota and two-quota outcomes are
# acceptable per qa/quota_migration.md; only a migrate failure is a defect.
name="deliberately-unasserted-punctuation-limit"
DB="$TMP_DIR/$name.sqlite"
"$BIN_PRE16" migrate --db "$DB" >/dev/null
seed_screen_quota "$DB" "Piano" 240
seed_legacy_quota_task "$DB" "Pi-ano" 2 30
if ! "$BIN_FULL" migrate --db "$DB"; then
  echo "FAIL: [$name] expected migrate to exit successfully even here" >&2
  FAILURES=1
elif qa_start_server "$BIN_FULL" "$DB" "$TMP_DIR/$name.log"; then
  meta="$(qa_quota_meta "$(qa_get_quota)")"
  echo "FINDING: [$name] \"Pi-ano\" against an existing \"Piano\": meta reads \"$meta\" -- one quota means punctuation happened to fold too (deeper than the spec requires, not a regression); two means the documented, deliberately-unasserted gap. Either is acceptable; only a migrate failure would not be."
  qa_stop_server
else
  FAILURES=1
fi

if [[ "$FAILURES" -ne 0 ]]; then
  exit 1
fi
echo "PASS: quota_migration"
