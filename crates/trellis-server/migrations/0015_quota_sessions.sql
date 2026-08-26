-- T-migrations-append-only: 0001-0014 are frozen; this is additive only.

-- `quota_sessions` (#93, D-logging-is-retrospective-and-separate): a single
-- logged session against a quota -- a durable consequence of a deliberate
-- act, and it earns its table (T-ephemeral-view-state-rides-the-request:
-- the test says when storage is legitimate, and a session is the clearest
-- yes this project has had).
--
-- `day_ms` is the *instant* of local midnight for the day the session
-- counts against, not a bare weekday name: "Mon" alone cannot survive a
-- week boundary (D-quota-no-rollover), and this project already resolves
-- the owner's timezone rather than reaching for UTC
-- (T-timezone-is-a-setting). "This week's sessions" is `day_ms BETWEEN`
-- the current week's own bounds, computed at read time from the current
-- instant and zone, never stored.
--
-- No `ON DELETE CASCADE` on `quota_id`: deleting a quota is not a
-- capability this project has built, so the constraint says nothing about
-- a case that cannot occur yet.
CREATE TABLE quota_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    quota_id INTEGER NOT NULL REFERENCES quotas(id),
    day_ms INTEGER NOT NULL,
    minutes INTEGER NOT NULL CHECK (minutes > 0),
    created_at_ms INTEGER NOT NULL
);
