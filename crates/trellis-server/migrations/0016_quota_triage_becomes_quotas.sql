-- T-migrations-append-only: 0001-0015 are frozen; this is additive only.

-- #138: a `kind='quota'` task, triaged before the quota screen existed,
-- becomes a `quotas` row -- closing the transitional state #93's cut
-- knowingly accepted (`quota/store.rs`'s own header: "a different table
-- from `tasks`, which `TaskKind::Quota` keeps using untouched"). This is
-- the owner's own complaint that opened #138: "it brings up the page but
-- it doesn't show any of the things that I've triaged as a quota."
--
-- THE CONVERSION IS A MULTIPLICATION, stated here as a decision rather
-- than arithmetic nobody signed: `weekly_target_minutes = target_count *
-- target_minutes_each`. It is the only reading that preserves the target
-- the owner actually set, and `D-quota-no-rollover`'s Monday reset makes
-- "per week" the only period the result could ever land in -- so `period`
-- is read nowhere here; every pre-#138 quota task was triaged against the
-- same three required fields, and `period` names no second conversion.
--
-- `captures.raw_text` becomes the quota's name. `quotas.name` carries
-- `UNIQUE COLLATE NOCASE` (T-collation-enforces-name-identity), so
-- `INSERT OR IGNORE` folds a case-only collision for free: a converted
-- quota never becomes a second counter beside one a screen already
-- defined under the same name (`quota-migration-no-second-counter-02`).
-- The existing screen-defined quota keeps its own target rather than the
-- converted one's, and rather than the two summed -- summing would invent
-- a number nobody chose, the same class of harm this scenario prevents in
-- the other direction. A migration that folds punctuation or spaces the
-- way `scheduler_core::quota::check_name` does is deliberately not
-- attempted here: that richer rule is Rust, this is SQL, and enshrining a
-- narrower fold as a migration would make a later, better one look like a
-- regression against data it never touched.
--
-- Guarded by `target_count`/`target_minutes_each IS NOT NULL` even though
-- every pre-#138 quota task satisfied both: a defensive floor against any
-- database this migration was never actually run against, not a case this
-- slice's own data exercises.
INSERT OR IGNORE INTO quotas (name, weekly_target_minutes, created_at_ms)
SELECT captures.raw_text, tasks.target_count * tasks.target_minutes_each, tasks.created_at_ms
FROM tasks
JOIN captures ON captures.id = tasks.capture_id
WHERE tasks.kind = 'quota'
  AND tasks.target_count IS NOT NULL
  AND tasks.target_minutes_each IS NOT NULL;
