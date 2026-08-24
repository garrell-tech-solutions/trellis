-- T-migrations-append-only: 0001-0012 are frozen; this is additive only.

-- `cleared_at` (#122, D-a-trip-survives-being-worked): when a struck-through
-- pool item was explicitly swept off the screen by the "Clear done" control.
-- A third state `archived_at` alone cannot express: `archived_at IS NOT
-- NULL AND cleared_at IS NULL` is struck-and-still-displayed, `cleared_at
-- IS NOT NULL` is struck-and-no-longer-displayed. The row is never
-- deleted (`D-kill-means-archive`) -- it still feeds the reckoning, it
-- simply stops being read back onto the Pool screen. Never set without
-- `archived_at` also being set -- you cannot clear what was never marked
-- done -- enforced by the CHECK rather than trusted to every caller.
ALTER TABLE tasks ADD COLUMN cleared_at INTEGER
    CHECK (cleared_at IS NULL OR archived_at IS NOT NULL);
