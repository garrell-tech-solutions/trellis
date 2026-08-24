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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{archived_at, insert_capture, test_pool};
    use scheduler_core::task::TaskKind;

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
}
