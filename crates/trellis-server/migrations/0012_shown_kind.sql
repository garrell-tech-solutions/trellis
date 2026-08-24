-- T-migrations-append-only: 0001-0011 are frozen; this is additive only.

-- `shown_kind` (#119): which kind's fields panel the inbox is currently
-- showing for a still-untriaged capture -- a display preference, not a
-- triage decision (pool never sets it; it files on one tap and leaves the
-- inbox before this column matters). NULL means no panel is open, the
-- state every capture starts in and the one a triaged capture leaves
-- behind unread. Closed to the two kinds that have a panel at all
-- (D-pool-is-default keeps pool out of this column's domain entirely).
ALTER TABLE captures ADD COLUMN shown_kind TEXT
    CHECK (shown_kind IS NULL OR shown_kind IN ('committed', 'quota'));
