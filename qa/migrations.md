# QA Procedure: Database migrations are idempotent and enable WAL mode

Covers: `features/migrations.feature`

## Procedure — empty database
1. Create a fresh, empty trellis database file (no prior schema).
2. Run the project's migration command against it.
3. Observe the migration command's exit code.
4. Inspect the database file's journal mode (e.g. `PRAGMA journal_mode;` via
   the project's own database inspection affordance).

### Expected Observable Outcomes
- Migration command exits `0`.
- Journal mode reads `wal`.

## Procedure — re-run against an already-migrated database
1. Starting from the already-migrated database file from the prior
   procedure, record the current schema (e.g. `sqlite3 <db> .schema`).
2. Run the project's migration command again against the same file.
3. Observe the migration command's exit code.
4. Record the schema again and compare it to the pre-run snapshot.

### Expected Observable Outcomes
- Migration command exits `0` on the re-run.
- Schema snapshot before and after the re-run is identical.

## Independent of Implementation
This procedure only depends on the migration command's exit code and the
resulting on-disk schema/journal mode — not on the migration tool's
internals or the number/order of migration files.
