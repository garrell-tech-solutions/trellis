//! The one write this capability owns: stamping `tasks.archived_at`.
//!
//! `tasks.archived_at` has existed since migration `0002` and nothing had
//! ever written it before #97. `T-archived-at-only` stays literally true —
//! it remains the single archive signal, not a done/killed pair, because
//! nothing in Trellis can kill a task today (see `mod.rs`).

use sqlx::SqlitePool;

/// Stamps `task_id` done at `done_at_ms`, unless it already carries a
/// timestamp. Returns whether a row actually changed, so a caller can tell
/// "already done" from "just done" without a second query — not that any
/// caller currently needs to (`D-inaction-archives`: there is no un-do to
/// report failing).
pub(super) async fn mark_task_done(
    pool: &SqlitePool,
    task_id: i64,
    done_at_ms: i64,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("UPDATE tasks SET archived_at = ? WHERE id = ? AND archived_at IS NULL")
            .bind(done_at_ms)
            .bind(task_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

/// The direct inverse of [`mark_task_done`] (#122,
/// `D-a-trip-survives-being-worked`: unchecking a struck item puts it
/// back). Guarded by `cleared_at IS NULL` -- a cleared task has no control
/// on screen to trigger this from, but the guard keeps the invariant true
/// in the database regardless of what a caller might attempt.
pub(super) async fn unmark_task_done(pool: &SqlitePool, task_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE tasks SET archived_at = NULL \
         WHERE id = ? AND archived_at IS NOT NULL AND cleared_at IS NULL",
    )
    .bind(task_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Marks every not-yet-done task of `kind` at `tag` done at `done_at_ms`, in
/// one statement (`T-set-operations-execute-in-the-store`: completing N
/// tasks is one round trip through `sqlx`, not N). Returns how many rows
/// actually changed. `kind` keeps this generic rather than pool-specific --
/// the tag-and-kind join is the same shape the pool capability's own
/// tag-scoped sweep already uses elsewhere, just writing `archived_at`
/// instead of a different column.
pub(super) async fn mark_group_done(
    pool: &SqlitePool,
    kind: &str,
    tag: &str,
    done_at_ms: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE tasks SET archived_at = ? \
         WHERE kind = ? AND archived_at IS NULL \
         AND capture_id IN (SELECT id FROM captures WHERE context_tag = ?)",
    )
    .bind(done_at_ms)
    .bind(kind)
    .bind(tag)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// The task `task_id` names, and the capture text it was triaged from --
/// but only while unmarking it would actually do something.
///
/// **The predicate is [`unmark_task_done`]'s own**, deliberately: `archived_at
/// IS NOT NULL AND cleared_at IS NULL`, plus the `kind` of the screen asking.
/// A way back is an offer to run that statement, so the offer exists exactly
/// when the statement would change a row. Written once here rather than
/// approximated a second time by whatever list each screen happens to have
/// in hand.
pub(super) async fn just_archived(
    pool: &SqlitePool,
    task_id: i64,
    kind: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT captures.raw_text FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE tasks.id = ? AND tasks.kind = ? \
         AND tasks.archived_at IS NOT NULL AND tasks.cleared_at IS NULL",
    )
    .bind(task_id)
    .bind(kind)
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{archived_at, insert_capture, test_pool};
    use scheduler_core::task::TaskKind;

    const TASK_POOL: &str = scheduler_core::task::POOL;

    async fn given_a_pool_task(pool: &SqlitePool, raw_text: &str) -> i64 {
        let capture_id = insert_capture(pool, raw_text, None).await;
        crate::triage::store::insert_task(pool, capture_id, &TaskKind::Pool, 0)
            .await
            .unwrap();
        sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    // --- just_archived: the offer exists exactly when the undo would work ---

    #[tokio::test]
    async fn just_archived_names_a_task_this_capability_has_archived() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;
        mark_task_done(&pool, task_id, 4242).await.unwrap();

        assert_eq!(
            just_archived(&pool, task_id, TASK_POOL).await.unwrap(),
            Some("buy screws".to_string())
        );
    }

    /// The half neither screen checked before: a task nobody has marked done
    /// is not something to offer a way back from.
    #[tokio::test]
    async fn just_archived_is_none_for_a_task_that_is_not_done() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;

        assert_eq!(
            just_archived(&pool, task_id, TASK_POOL).await.unwrap(),
            None
        );
    }

    /// And the other half: `unmark_task_done` refuses a cleared task, so the
    /// offer to run it must refuse the same one -- a control that does
    /// nothing is worse than no control.
    #[tokio::test]
    async fn just_archived_is_none_once_the_task_has_been_cleared() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;
        mark_task_done(&pool, task_id, 4242).await.unwrap();
        sqlx::query("UPDATE tasks SET cleared_at = 9999 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            just_archived(&pool, task_id, TASK_POOL).await.unwrap(),
            None
        );
        assert!(!unmark_task_done(&pool, task_id).await.unwrap());
    }

    #[tokio::test]
    async fn just_archived_is_none_for_a_task_of_another_kind() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;
        mark_task_done(&pool, task_id, 4242).await.unwrap();

        assert_eq!(
            just_archived(&pool, task_id, scheduler_core::task::COMMITTED)
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn just_archived_is_none_for_an_id_naming_no_task() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(just_archived(&pool, 999, TASK_POOL).await.unwrap(), None);
    }

    #[tokio::test]
    async fn mark_task_done_stamps_archived_at() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;

        let changed = mark_task_done(&pool, task_id, 4242).await.unwrap();

        assert!(changed);
        assert_eq!(archived_at(&pool, task_id).await, Some(4242));
    }

    #[tokio::test]
    async fn mark_task_done_is_a_no_op_the_second_time() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;
        mark_task_done(&pool, task_id, 1).await.unwrap();

        let changed = mark_task_done(&pool, task_id, 2).await.unwrap();

        assert!(!changed);
        assert_eq!(
            archived_at(&pool, task_id).await,
            Some(1),
            "the first stamp must not be overwritten"
        );
    }

    #[tokio::test]
    async fn mark_task_done_reports_no_change_for_an_unknown_id() {
        let (_dir, pool) = test_pool().await;

        let changed = mark_task_done(&pool, 999, 1).await.unwrap();

        assert!(!changed);
    }

    #[tokio::test]
    async fn unmark_task_done_clears_archived_at() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;
        mark_task_done(&pool, task_id, 4242).await.unwrap();

        let changed = unmark_task_done(&pool, task_id).await.unwrap();

        assert!(changed);
        assert_eq!(archived_at(&pool, task_id).await, None);
    }

    #[tokio::test]
    async fn unmark_task_done_is_a_no_op_on_a_task_that_was_never_done() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;

        let changed = unmark_task_done(&pool, task_id).await.unwrap();

        assert!(!changed);
    }

    #[tokio::test]
    async fn unmark_task_done_reports_no_change_for_an_unknown_id() {
        let (_dir, pool) = test_pool().await;

        let changed = unmark_task_done(&pool, 999).await.unwrap();

        assert!(!changed);
    }

    #[tokio::test]
    async fn unmark_task_done_leaves_a_cleared_task_alone() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;
        mark_task_done(&pool, task_id, 1).await.unwrap();
        sqlx::query("UPDATE tasks SET cleared_at = 2 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        let changed = unmark_task_done(&pool, task_id).await.unwrap();

        assert!(!changed);
        assert_eq!(archived_at(&pool, task_id).await, Some(1));
    }

    // --- #125: a group completion is one statement ----------------------

    async fn given_a_tagged_pool_task(pool: &SqlitePool, raw_text: &str, tag: &str) -> i64 {
        let capture_id = insert_capture(pool, raw_text, Some(tag)).await;
        crate::triage::store::insert_task(pool, capture_id, &TaskKind::Pool, 0)
            .await
            .unwrap();
        sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn mark_group_done_stamps_every_open_task_at_the_tag() {
        let (_dir, pool) = test_pool().await;
        let a = given_a_tagged_pool_task(&pool, "buy screws", "@homedepot").await;
        let b = given_a_tagged_pool_task(&pool, "return the drill", "@homedepot").await;

        let changed = mark_group_done(&pool, "pool", "@homedepot", 4242)
            .await
            .unwrap();

        assert_eq!(changed, 2);
        assert_eq!(archived_at(&pool, a).await, Some(4242));
        assert_eq!(archived_at(&pool, b).await, Some(4242));
    }

    #[tokio::test]
    async fn mark_group_done_leaves_a_different_tag_alone() {
        let (_dir, pool) = test_pool().await;
        let homedepot = given_a_tagged_pool_task(&pool, "buy screws", "@homedepot").await;
        let supermarket = given_a_tagged_pool_task(&pool, "milk", "@supermarket").await;

        mark_group_done(&pool, "pool", "@homedepot", 4242)
            .await
            .unwrap();

        assert_eq!(archived_at(&pool, homedepot).await, Some(4242));
        assert_eq!(archived_at(&pool, supermarket).await, None);
    }

    #[tokio::test]
    async fn mark_group_done_leaves_an_already_done_task_alone() {
        let (_dir, pool) = test_pool().await;
        let already_done = given_a_tagged_pool_task(&pool, "buy screws", "@homedepot").await;
        mark_task_done(&pool, already_done, 1).await.unwrap();
        let still_open = given_a_tagged_pool_task(&pool, "return the drill", "@homedepot").await;

        let changed = mark_group_done(&pool, "pool", "@homedepot", 4242)
            .await
            .unwrap();

        assert_eq!(changed, 1, "only the still-open task should have changed");
        assert_eq!(
            archived_at(&pool, already_done).await,
            Some(1),
            "the earlier stamp must not be overwritten"
        );
        assert_eq!(archived_at(&pool, still_open).await, Some(4242));
    }

    #[tokio::test]
    async fn mark_group_done_leaves_a_different_kind_alone() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "renew the passport", Some("@homedepot")).await;
        crate::triage::store::insert_task(
            &pool,
            capture_id,
            &TaskKind::Committed {
                deadline: 1,
                commitment: scheduler_core::task::Commitment::At,
                priority: scheduler_core::task::Priority::P1,
                estimated_minutes: 30,
            },
            0,
        )
        .await
        .unwrap();
        let committed_task_id: i64 =
            sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let changed = mark_group_done(&pool, "pool", "@homedepot", 4242)
            .await
            .unwrap();

        assert_eq!(changed, 0);
        assert_eq!(archived_at(&pool, committed_task_id).await, None);
    }

    #[tokio::test]
    async fn mark_group_done_reports_zero_for_an_unknown_tag() {
        let (_dir, pool) = test_pool().await;

        let changed = mark_group_done(&pool, "pool", "@nowhere", 4242)
            .await
            .unwrap();

        assert_eq!(changed, 0);
    }
}
