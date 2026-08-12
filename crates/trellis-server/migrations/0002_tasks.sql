ALTER TABLE captures ADD COLUMN triaged_at INTEGER;

CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    capture_id INTEGER NOT NULL REFERENCES captures(id),
    kind TEXT NOT NULL,
    deadline TEXT,
    deadline_type TEXT,
    priority TEXT,
    target_count INTEGER,
    target_minutes_each INTEGER,
    period TEXT,
    archived_at INTEGER,
    created_at_ms INTEGER NOT NULL
);
