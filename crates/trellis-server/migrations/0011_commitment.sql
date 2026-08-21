-- T-migrations-append-only: 0001-0010 are frozen; this is additive only.

-- `commitment` (#94, D-committed-is-at-or-by) replaces `deadline_type` on
-- the committed triage form: an explicit at/by choice rather than a
-- hard/soft one. `deadline_type` stays in the schema, unread -- dropping it
-- throws away what the owner already typed on every row triaged before this
-- migration, and a hard/soft judgement is not recoverable once dropped
-- (unlike a derivable number, which #88 dropped on exactly that distinction
-- for `scheduler_core::ratio`). New committed rows simply leave it NULL.
ALTER TABLE tasks ADD COLUMN commitment TEXT
    CHECK (commitment IS NULL OR commitment IN ('at', 'by'));
