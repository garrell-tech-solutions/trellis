#!/usr/bin/env bash
# CI gate for decisions.md T-migrations-append-only (issue #32): migration
# files that already exist on the base branch are immutable. A branch may ADD
# migrations; it may never modify or delete one that is already on trunk.
#
# This is not a qa/*.md procedure -- it asserts a property of the git history,
# not of the running program -- so it lives under scripts/ci/ rather than
# scripts/qa/, and scripts/qa/run.sh does not pick it up.
#
# Usage: scripts/ci/migration_immutability.sh [base-ref]
#        base-ref defaults to $MIGRATION_BASE_REF, then to origin/trunk.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

BASE_REF="${1:-${MIGRATION_BASE_REF:-origin/trunk}}"
MIGRATIONS_DIR="crates/trellis-server/migrations"

if ! git rev-parse --verify --quiet "${BASE_REF}^{commit}" >/dev/null; then
  echo "FAIL: base ref \"$BASE_REF\" does not exist in this clone." >&2
  echo "      A shallow or single-branch checkout has no merge-base to diff against;" >&2
  echo "      fetch it first (actions/checkout needs fetch-depth: 0)." >&2
  exit 1
fi

HEAD_SHA="$(git rev-parse HEAD)"
if ! MERGE_BASE="$(git merge-base "$BASE_REF" HEAD)"; then
  echo "FAIL: no merge-base between \"$BASE_REF\" and HEAD." >&2
  echo "      The histories are unrelated or the clone is shallow (need fetch-depth: 0)." >&2
  exit 1
fi

# On the base branch itself HEAD *is* the base, so the diff is empty by
# construction and a green result would mean nothing. Say so out loud rather
# than reporting a pass that was never a test.
if [[ "$MERGE_BASE" == "$HEAD_SHA" ]]; then
  echo "SKIP: HEAD ($HEAD_SHA) is an ancestor of $BASE_REF -- no commits to check."
  echo "      This gate only has meaning on a branch ahead of $BASE_REF."
  exit 0
fi

# --no-renames: a rename is a delete plus an add, and renaming an applied
#   migration breaks the same databases that editing it breaks. Without this,
#   git's rename detection reports R, which --diff-filter=MD would wave through.
# T (typechange) is included for the same reason: swapping a migration for a
#   symlink changes what sqlx reads.
CHANGED="$(git diff --no-renames --diff-filter=MDT --name-status \
  "$MERGE_BASE" "$HEAD_SHA" -- "$MIGRATIONS_DIR")"

if [[ -z "$CHANGED" ]]; then
  echo "PASS: migration_immutability (no migration on $BASE_REF was modified or deleted)"
  exit 0
fi

cat >&2 <<EOF
FAIL: this branch modifies or deletes a migration that already exists on $BASE_REF.

$CHANGED

  (M = modified, D = deleted, T = type changed; compared against merge-base
   $MERGE_BASE)

WHY THIS IS BLOCKED

  sqlx records every migration it applies in the _sqlx_migrations table
  together with a checksum of the file's contents. Change a migration that a
  database has already run and that database refuses to start:

      run migrations: migration 2 was previously applied but has been modified

  There is no fallback and no remediation path -- the database is unbootable
  until the file is restored byte for byte. Your own checkout will not show
  you this, because a fresh database happily applies whatever the file says
  today. The database that breaks is someone else's, and it breaks after merge.

WHAT TO DO INSTEAD

  Leave the existing file untouched and add the next numbered migration that
  moves the schema forward, e.g.:

      $MIGRATIONS_DIR/0003_<what_it_does>.sql

  SQLite has no ALTER COLUMN, so a column type change is not a one-liner --
  it needs the table-rebuild pattern, all inside the new migration:

      CREATE TABLE tasks_new (
          -- the corrected schema, e.g. deadline INTEGER
      );
      INSERT INTO tasks_new (id, ..., deadline)
          SELECT id, ..., deadline FROM tasks;   -- cast/convert here
      DROP TABLE tasks;
      ALTER TABLE tasks_new RENAME TO tasks;
      -- recreate every index and trigger the old table had

  Editing the earlier migration is the path of least resistance and it is the
  one change that cannot be undone on another machine. See docs/decisions.md
  T-migrations-append-only and issue #32.
EOF
exit 1
