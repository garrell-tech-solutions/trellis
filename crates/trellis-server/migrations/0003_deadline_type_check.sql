-- T-migrations-append-only: 0002 is frozen, so this closes a gap folded in
-- from the PR #31 review rather than editing it. `deadline` had no CHECK
-- while kind/deadline_type/priority/period all did — SQLite's INTEGER
-- *affinity* does not reject text, so `INSERT ... deadline='banana'`
-- succeeded through direct SQL despite the triage boundary's own validation.
--
-- SQLite has no ALTER COLUMN, so adding a CHECK needs the table-rebuild
-- pattern even though the column type itself is unchanged.
CREATE TABLE tasks_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    capture_id INTEGER NOT NULL REFERENCES captures(id),
    kind TEXT NOT NULL CHECK (kind IN ('pool', 'committed', 'quota')),
    deadline INTEGER CHECK (deadline IS NULL OR typeof(deadline) = 'integer'),
    deadline_type TEXT CHECK (deadline_type IS NULL OR deadline_type IN ('hard', 'soft')),
    priority TEXT CHECK (priority IS NULL OR priority IN ('P1', 'P2', 'P3', 'P4')),
    target_count INTEGER,
    target_minutes_each INTEGER,
    period TEXT CHECK (period IS NULL OR period IN ('week', 'month')),
    archived_at INTEGER,
    created_at_ms INTEGER NOT NULL
);

INSERT INTO tasks_new (
    id, capture_id, kind, deadline, deadline_type, priority,
    target_count, target_minutes_each, period, archived_at, created_at_ms
)
SELECT
    id, capture_id, kind, deadline, deadline_type, priority,
    target_count, target_minutes_each, period, archived_at, created_at_ms
FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;
