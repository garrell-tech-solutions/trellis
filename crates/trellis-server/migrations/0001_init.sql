CREATE TABLE captures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    raw_text TEXT NOT NULL,
    source TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);
