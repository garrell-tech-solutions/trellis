-- T-migrations-append-only: 0001-0009 are frozen; this is additive only.

-- `COLLATE NOCASE` makes `@HomeDepot` and `@homedepot` compare equal for
-- every ordinary lookup (`=`, `GROUP BY`, `DISTINCT`) without rewriting what
-- is stored -- the same argument `life_areas.name` made before #88, applied
-- here to something typed rather than picked, where the slip is likelier
-- (`context-tags-case-is-one-tag-06`). This is comparison only: the stored
-- text is whatever spelling was written, and choosing which spelling wins on
-- a second use is `capture::resolve_tag`'s job, not the schema's -- a lookup
-- against every prior tag needs a query this column cannot express alone.
--
-- No UNIQUE: a tag is not a managed row a second capture "already has" the
-- way a life area name was. Many captures share one tag by design.
--
-- The CHECK mirrors `scheduler_core::context_tag::normalize`'s own rule
-- (trimmed, non-empty, or NULL) at the boundary the application code cannot
-- bypass: NULL is "no tag", and nothing else may be blank or padded.
ALTER TABLE captures ADD COLUMN context_tag TEXT COLLATE NOCASE
    CHECK (context_tag IS NULL OR (length(context_tag) > 0 AND context_tag = TRIM(context_tag)));
