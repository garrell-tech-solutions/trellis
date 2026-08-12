ALTER TABLE captures ADD COLUMN triaged_at INTEGER;

-- deadline is stored as UTC epoch milliseconds (T3), parsed and validated at
-- the triage boundary. The CHECK constraints below are defence in depth
-- behind that boundary check, not a replacement for it (tasks has no
-- production data yet, so closing these domains here is a free change).
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    capture_id INTEGER NOT NULL REFERENCES captures(id),
    kind TEXT NOT NULL CHECK (kind IN ('pool', 'committed', 'quota')),
    deadline INTEGER,
    deadline_type TEXT CHECK (deadline_type IS NULL OR deadline_type IN ('hard', 'soft')),
    priority TEXT CHECK (priority IS NULL OR priority IN ('P1', 'P2', 'P3', 'P4')),
    target_count INTEGER,
    target_minutes_each INTEGER,
    period TEXT CHECK (period IS NULL OR period IN ('week', 'month')),
    archived_at INTEGER,
    created_at_ms INTEGER NOT NULL
);
