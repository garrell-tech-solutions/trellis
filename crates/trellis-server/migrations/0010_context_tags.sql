-- T-migrations-append-only: 0001-0009 are frozen; this is additive only.

-- A context tag (#82, D-context-tags-are-the-taxonomy): free text, optional,
-- disposable -- the product's only taxonomy now. Lives on the capture, not
-- the task: a task reads it through the capture it came from
-- (features/context_tags.feature's own header argues the "one fact, one
-- row" call, T-archived-at-only's shape applied to this fact). A dismissed
-- capture keeps a tag nothing reads, which costs nothing --
-- D-kill-means-archive keeps that row regardless.
--
-- COLLATE NOCASE, the same call life_areas.name already made
-- (T-collation-enforces-name-identity), applied to something typed rather
-- than picked: "@HomeDepot" and "@homedepot" are one tag, and the slip is
-- likelier here since nothing is chosen from a list. Unlike a life area's
-- name this column carries no UNIQUE constraint -- many captures share one
-- tag by design -- so the collation only settles how two spellings compare
-- (GROUP BY, DISTINCT, ORDER BY, `=`), never which one is stored; canonicalizing
-- a freshly submitted spelling to the one first used is the write path's own
-- job (`context-tags-case-is-one-tag-06`: shown as first typed), in
-- trellis-server, because it needs a lookup against every tag stored before
-- and this crate would need a database to make it (T-capability-owns-its-queries).
--
-- The CHECK is defence in depth behind the same boundary check every other
-- optional free-text field in this schema gets: NULL means no tag, never an
-- empty or whitespace-only string (T-empty-equals-absent).
ALTER TABLE captures ADD COLUMN context_tag TEXT COLLATE NOCASE
    CHECK (context_tag IS NULL OR (length(context_tag) > 0 AND context_tag = TRIM(context_tag)));
