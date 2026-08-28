//! **Mark done** — the pool and committed screens' other exit: a task you
//! did leaves the screen it lives on, permanently (#97, found while
//! specifying #94).
//!
//! **One column, no done-versus-killed discriminator, on purpose.**
//! Marking done writes `tasks.archived_at`. `T-capture-leaves-inbox-once`
//! managed a comparable exit with one column because the exit was
//! *derivable* — a triaged capture has a `tasks` row referencing it, a
//! dismissed one does not. Here nothing downstream distinguishes a task
//! you did from one you killed, so that discriminator would have to be
//! *stored* — and nothing in Trellis can kill a task today, so the column
//! could only ever hold one value. That is the speculative schema this
//! project refuses. When a kill control arrives it adds the discriminator
//! and backfills every existing row `done`, which is provably correct:
//! nothing else could have set the column.
//!
//! **Why this is its own capability rather than living in `pool` or
//! `committed`.** Both screens need to mark a task done, and the write
//! itself has nothing to do with either screen's own shape — the same
//! reason inbox membership is `inbox`'s and not `triage`'s or `dismiss`'s
//! copy of it (`T-inbox-owns-membership`). [`mark_task_done`] is the one
//! front door (`T-one-front-door-per-capability`); `pool::http` and
//! `committed::http` each own their own route and re-render their own
//! fragment, because only they know what "done" removed a row from.

mod store;

use sqlx::SqlitePool;

/// Marks `task_id` done at `done_at_ms`. Returns whether a row actually
/// changed — `false` when the task was already done or does not exist.
pub(crate) async fn mark_task_done(
    pool: &SqlitePool,
    task_id: i64,
    done_at_ms: i64,
) -> Result<bool, sqlx::Error> {
    store::mark_task_done(pool, task_id, done_at_ms).await
}

/// The direct inverse of [`mark_task_done`] (#122): unchecking a struck
/// item puts it back. Returns whether a row actually changed — `false`
/// when the task was already open, already cleared, or does not exist.
pub(crate) async fn unmark_task_done(pool: &SqlitePool, task_id: i64) -> Result<bool, sqlx::Error> {
    store::unmark_task_done(pool, task_id).await
}

/// What a "done" POST just archived: the task's id and the words it was
/// captured as (#111).
///
/// Not a view model -- it says what happened, not what to draw. Each screen
/// renders it behind its own undo route, which is markup, not data: that
/// difference is a `hx-post` attribute in `pool_body.html` and
/// `committed_body.html`, and never reached this far in the first place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JustArchived {
    pub id: i64,
    pub text: String,
}

/// [`JustArchived`] for `task_id`, if this capability has archived it, it has
/// not since been cleared, and it is of `kind` -- `None` otherwise.
///
/// **Why this is `mark_done`'s and not each screen's.** The same argument
/// this module makes for the write: both screens need it, and the fact it
/// rests on -- this task is archived and can still be put back -- has nothing
/// to do with either screen's own shape. Before this, `pool` scanned the row
/// off its own task list and `committed` had a query of its own for it,
/// because a done committed row leaves that list and a done pool row does
/// not. Two answers to one question, neither of them checking the thing the
/// offer actually depends on.
pub(crate) async fn just_archived(
    pool: &SqlitePool,
    task_id: i64,
    kind: &str,
) -> Result<Option<JustArchived>, sqlx::Error> {
    Ok(store::just_archived(pool, task_id, kind)
        .await?
        .map(|text| JustArchived { id: task_id, text }))
}

/// Marks every not-yet-done task of `kind` at `tag` done at `done_at_ms`, in
/// one statement (#125, `T-set-operations-execute-in-the-store`). Returns
/// how many rows actually changed. The only entry point a group completion
/// may use (`T-cross-capability-invariants-need-an-owner`: this capability
/// is the one place `archived_at` is written from).
pub(crate) async fn mark_group_done(
    pool: &SqlitePool,
    kind: &str,
    tag: &str,
    done_at_ms: i64,
) -> Result<u64, sqlx::Error> {
    store::mark_group_done(pool, kind, tag, done_at_ms).await
}
