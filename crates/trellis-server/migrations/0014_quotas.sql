-- T-migrations-append-only: 0001-0013 are frozen; this is additive only.

-- `quotas` (#93, D-quotas-are-selected-not-typed): a named container with a
-- weekly hour target, created directly from the Quota screen without a
-- capture -- a different entity from `TaskKind::Quota`'s triaged capture
-- (`tasks` table, `kind = 'quota'`), which stays exactly as it is until
-- #138 closes the two into one. This table holds neither triaged rows nor
-- sessions logged against a quota; sessions are #93's second slice.
--
-- `name` carries `UNIQUE COLLATE NOCASE` as the backstop
-- (T-collation-enforces-name-identity): the richer rule -- punctuation
-- folded away and edit-distance similarity -- lives in
-- `scheduler_core::quota`, but a database constraint a write path cannot
-- forget to call is worth keeping under it regardless.
--
-- `weekly_target_minutes` is minutes, not a fractional hour column: the
-- canvas's own `step="0.5"` on the hours input always yields a whole
-- number of minutes, and an integer is exact where a stored float would
-- not be.
CREATE TABLE quotas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
        CHECK (length(name) > 0 AND name = TRIM(name)),
    weekly_target_minutes INTEGER NOT NULL CHECK (weekly_target_minutes > 0),
    created_at_ms INTEGER NOT NULL
);
