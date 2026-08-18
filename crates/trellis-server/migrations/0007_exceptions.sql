-- T-migrations-append-only: 0001-0006 are frozen; this is additive only.

-- A dated exception: how the owner narrows a life area's guardrail, or
-- every life area's, for specific civil dates (#61). `life_area_id NULL`
-- means global -- every life area's hours are reduced, not one; scope is
-- read explicitly, never implied (`exceptions-scope-is-visible-03`).
--
-- Dates are stored as ISO 8601 civil dates, not instants
-- (T-jiff-epoch-millis: "20 August" is a civil date in the owner's zone,
-- the same reasoning `guardrail_bands` already applies to a band's
-- minutes-of-day). `end_date >= start_date` is the one well-formedness rule
-- an exception has (scheduler_core::exception::well_formed_range); the
-- application refuses a backwards range with a named reason before this
-- constraint would ever fire.
--
-- Configuration, not a work item (D-kill-means-archive): removing an
-- exception deletes the row outright, the call `#59` already made for a
-- guardrail band. A past exception is not cleaned up either -- the
-- fourteen-day horizon simply stops reaching it, so nothing here needs an
-- expiry job or a purge.
CREATE TABLE exceptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    life_area_id INTEGER REFERENCES life_areas(id),
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL CHECK (end_date >= start_date),
    label TEXT NOT NULL DEFAULT ''
);
