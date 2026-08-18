-- T-migrations-append-only: 0001-0005 are frozen; this is additive only.

-- One row per weekday a band spans -- "Mon-Fri 09:00-17:00" is five rows
-- sharing the same start/end. Storage stays one row per weekday
-- (T-jiff-epoch-millis: civil wall-clock, never an instant); the form that
-- authors a band spares the owner five repeats of the same two times, not
-- this table.
CREATE TABLE guardrail_bands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    life_area_id INTEGER NOT NULL REFERENCES life_areas(id),
    weekday TEXT NOT NULL CHECK (weekday IN ('Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun')),
    start_minutes INTEGER NOT NULL CHECK (start_minutes >= 0 AND start_minutes < 1440),
    end_minutes INTEGER NOT NULL CHECK (end_minutes > start_minutes AND end_minutes <= 1440)
);

-- pool_only is a column, not merely the absence of any band. Read alongside
-- T-archived-at-only and T-capture-leaves-inbox-once, and settled the other
-- way from both: those each collapsed two fields that spelled one state
-- into a single signal. "No guardrail yet" and "deliberately pool-only" are
-- two different states a life area can be in, not one state spelled twice --
-- a column says the second out loud and survives a life area that is still
-- being set up looking identical to one the owner decided never gets
-- scheduled.
--
-- Nullable-by-default's usual reason does not apply here: NOT NULL DEFAULT 0
-- is fine because every existing row's honest state is "not pool-only",
-- unlike a column whose correct value for existing rows is unknowable
-- (T-quota-targets-required's case, which is why that one stayed nullable).
ALTER TABLE life_areas ADD COLUMN pool_only INTEGER NOT NULL DEFAULT 0;

-- The owner's timezone: one row, one value, for the whole product
-- (D-single-user -- one zone, not one per guardrail). The CHECK pins it to
-- exactly one row rather than trusting every future writer to remember
-- WHERE id = 1; UTC is the only default that cannot silently mean the wrong
-- hour (D-manual-triage-until-llm's reasoning, applied to time zones instead
-- of life areas: a guessed zone is a wrong answer nobody chose).
CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    timezone TEXT NOT NULL DEFAULT 'UTC'
);
INSERT INTO settings (id, timezone) VALUES (1, 'UTC');
