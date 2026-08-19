-- T-migrations-append-only: 0001-0007 are frozen; this is additive only.

-- A committed task's own estimate (#62): capacity cannot report what a
-- fortnight of committed work asks for if committed tasks carry no minutes.
-- Nullable for schema reasons -- SQLite cannot add a NOT NULL column with
-- no default to a table that may already have rows -- the same pattern
-- T-quota-targets-required already set for the quota target columns and
-- migration 0004 set for life_area_id. Required at the triage boundary
-- instead. A NULL value means "written before the estimate was required",
-- never "takes no time": it is excluded from capacity's sum and surfaced as
-- a count on its life area's row, because counting it as zero would
-- under-report demand in the one direction that number must never be wrong.
ALTER TABLE tasks ADD COLUMN estimated_minutes INTEGER;
