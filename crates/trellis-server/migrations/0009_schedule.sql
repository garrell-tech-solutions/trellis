-- T-migrations-append-only: 0001-0008 are frozen; this is additive only.

-- The Plan/Constraints line runs inside this table, by state
-- (`T-fact-plan-line`). `proposed` and `published` are Plan-layer --
-- disposable, engine-written, deleted wholesale and regenerated on every
-- `schedule()` run (`R-incremental-patching`). `in_progress`, `completed`
-- and `missed` are Constraints-layer -- immutable facts this slice reads
-- and never writes. M3 slice 1 writes `proposed` only; the other four
-- states are declared now because the table fixes `Block`'s shape, and a
-- CHECK naming only what one slice writes would have to be widened by
-- every slice after it.
--
-- No `life_area_id` column: a block's life area is derivable through the
-- task it belongs to, and this slice never borrows another life area's
-- hours (`D-life-area-owns-its-time`), so the two are always the same.
CREATE TABLE block (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL REFERENCES tasks(id),
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL CHECK (end_ms > start_ms),
    state TEXT NOT NULL DEFAULT 'proposed'
        CHECK (state IN ('proposed', 'published', 'in_progress', 'completed', 'missed'))
);

-- A task the forward pass could not place, and why -- the infeasibility
-- report's own record. Plan-layer in the same sense `block` rows above
-- are: written wholesale on every `schedule()` run, disposable, and
-- deleted before that run's own fresh rows are written. Recorded so a
-- reload between one generation and the next shows the same "won't fit"
-- list rather than one recomputed live against whichever tasks happen to
-- exist at view time (`R-incremental-patching`'s "recomputed from scratch,
-- never patched" -- scratch means the last `schedule()` run, not the
-- current instant).
CREATE TABLE schedule_unplaceable (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL REFERENCES tasks(id),
    reason TEXT NOT NULL
        CHECK (reason IN ('no_window', 'deadline_unreachable', 'capacity_exceeded', 'chunk_policy_unsatisfiable'))
);
