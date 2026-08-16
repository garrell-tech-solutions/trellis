-- Life areas: user-managed rows, not a closed enum (T-life-areas-are-data).
-- Shape settled here per docs/design/architecture.md's placeholder
-- (`{ name, archived_at }`). COLLATE NOCASE makes "Work" and "work" the same
-- row at the schema level, backing the application's own case-insensitive
-- duplicate check with a constraint rather than leaving it a convention.
--
-- T-migrations-append-only: 0001-0003 are frozen, so this is additive only.
CREATE TABLE life_areas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    archived_at INTEGER
);

INSERT INTO life_areas (name) VALUES ('Work'), ('Fitness'), ('Learning'), ('Family'), ('Home');

-- Nullable for schema reasons -- SQLite cannot add a NOT NULL column with no
-- default to a table that may already have rows. Required at the triage
-- boundary instead, the same pattern T-quota-targets-required already set for
-- the quota target fields: nullable column, required application rule.
ALTER TABLE tasks ADD COLUMN life_area_id INTEGER REFERENCES life_areas(id);
