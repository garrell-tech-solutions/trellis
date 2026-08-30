-- T-migrations-append-only: 0001-0016 are frozen; this is additive only.

-- #148: a quota's own soft-delete, the same bargain `D-kill-means-archive`
-- already struck for tasks. `0015`'s `quota_sessions.quota_id REFERENCES
-- quotas(id)` carries no `ON DELETE`, and sqlx sets `PRAGMA foreign_keys =
-- ON` for every connection it opens -- a `DELETE FROM quotas` with sessions
-- attached is refused outright (SQLite error 787), not silently orphaned.
-- Archiving means that constraint never fires: nothing is deleted, so a
-- removed quota's own sessions are kept, with nowhere left to browse them
-- (`R-browsable-archive`).
ALTER TABLE quotas ADD COLUMN archived_at INTEGER;
