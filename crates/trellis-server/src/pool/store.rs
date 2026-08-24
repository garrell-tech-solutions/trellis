//! What the pool screen reads: every pool task, with the text and context
//! tag of the capture it came from.
//!
//! `D-no-pool-on-calendar`: this is the *only* place pool work is offered,
//! so this query's `WHERE tasks.kind = 'pool'` is the whole of what keeps
//! committed and quota work off this screen (`pool-screen-only-pool-04`).
//!
//! `AND tasks.cleared_at IS NULL` (#122) is the only exclusion left: a
//! task marked done stays in this list -- struck through, not removed --
//! until its trip's "Clear done" control sweeps it out. `archived_at` alone
//! no longer keeps a row off this screen; see [`PoolTaskRow::done`].

use sqlx::SqlitePool;

/// A row of [`list_pool_tasks`].
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PoolTaskRow {
    pub task_id: i64,
    pub raw_text: String,
    pub context_tag: Option<String>,
    /// Struck-through (#122): `archived_at IS NOT NULL`. A row this query
    /// returns is never *cleared* -- that is what excludes it -- so `done`
    /// alone is enough for the view to decide open-vs-struck.
    pub done: bool,
}

/// Every pool task not yet cleared, alongside the text and context tag of
/// the capture it was triaged from. Unordered on purpose:
/// `scheduler_core::pool::group` establishes newest-first itself rather
/// than trusting a caller to have sorted already.
pub async fn list_pool_tasks(pool: &SqlitePool) -> Result<Vec<PoolTaskRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT tasks.id AS task_id, captures.raw_text, captures.context_tag, \
         (tasks.archived_at IS NOT NULL) AS done FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE tasks.kind = 'pool' AND tasks.cleared_at IS NULL",
    )
    .fetch_all(pool)
    .await
}

/// Sweeps every struck-through (`archived_at IS NOT NULL`), not-yet-cleared
/// pool task at `tag` off the screen (#122's "Clear done") -- scoped to the
/// tag rather than a caller-supplied id list, since the control acts on a
/// whole trip at once and the row never carries its own "still open" flag
/// for a caller to check first. Never touches an open task: the `WHERE`
/// makes clearing a no-op on anything not already done, the same shape the
/// mark-done write's own guard takes.
pub async fn clear_done(
    pool: &SqlitePool,
    tag: &str,
    cleared_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tasks SET cleared_at = ? \
         WHERE kind = 'pool' AND archived_at IS NOT NULL AND cleared_at IS NULL \
         AND capture_id IN (SELECT id FROM captures WHERE context_tag = ?)",
    )
    .bind(cleared_at_ms)
    .bind(tag)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{insert_capture, test_pool};
    use scheduler_core::task::TaskKind;

    fn committed_task_kind() -> TaskKind {
        TaskKind::Committed {
            deadline: 1787245200000,
            commitment: scheduler_core::task::Commitment::At,
            priority: scheduler_core::task::Priority::P1,
            estimated_minutes: 30,
        }
    }

    fn quota_task_kind() -> TaskKind {
        TaskKind::Quota {
            target_count: 3,
            target_minutes_each: 20,
            period: scheduler_core::task::Period::Week,
        }
    }

    async fn given_a_pool_task(
        pool: &SqlitePool,
        raw_text: &str,
        tag: Option<&str>,
    ) -> Vec<PoolTaskRow> {
        let capture_id = insert_capture(pool, raw_text, tag).await;
        crate::triage::store::insert_task(pool, capture_id, &TaskKind::Pool, 0)
            .await
            .unwrap();
        list_pool_tasks(pool).await.unwrap()
    }

    #[tokio::test]
    async fn list_pool_tasks_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_pool_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_pool_tasks_reports_a_pool_tasks_text_and_tag() {
        let (_dir, pool) = test_pool().await;

        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].raw_text, "buy screws");
        assert_eq!(tasks[0].context_tag.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn list_pool_tasks_reports_an_untagged_pool_task_with_no_tag() {
        let (_dir, pool) = test_pool().await;

        let tasks = given_a_pool_task(&pool, "fix the door latch", None).await;

        assert_eq!(tasks[0].context_tag, None);
    }

    #[tokio::test]
    async fn list_pool_tasks_excludes_committed_and_quota_tasks() {
        let (_dir, pool) = test_pool().await;
        let pool_capture = insert_capture(&pool, "buy screws", Some("@homedepot")).await;
        let committed_capture = insert_capture(&pool, "file the return", Some("@homedepot")).await;
        let quota_capture = insert_capture(&pool, "practise piano", Some("@homedepot")).await;
        crate::triage::store::insert_task(&pool, pool_capture, &TaskKind::Pool, 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(&pool, committed_capture, &committed_task_kind(), 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(&pool, quota_capture, &quota_task_kind(), 0)
            .await
            .unwrap();

        let tasks = list_pool_tasks(&pool).await.unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].raw_text, "buy screws");
    }

    #[tokio::test]
    async fn list_pool_tasks_excludes_an_untriaged_capture() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "buy screws", Some("@homedepot")).await;

        assert_eq!(list_pool_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_pool_tasks_still_reports_a_task_marked_done_but_not_cleared() {
        let (_dir, pool) = test_pool().await;
        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = tasks[0].task_id;
        sqlx::query("UPDATE tasks SET archived_at = 1 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        let tasks = list_pool_tasks(&pool).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].done);
    }

    #[tokio::test]
    async fn list_pool_tasks_reports_an_open_task_as_not_done() {
        let (_dir, pool) = test_pool().await;

        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;

        assert!(!tasks[0].done);
    }

    #[tokio::test]
    async fn list_pool_tasks_excludes_a_cleared_task() {
        let (_dir, pool) = test_pool().await;
        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = tasks[0].task_id;
        sqlx::query("UPDATE tasks SET archived_at = 1, cleared_at = 2 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(list_pool_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn clear_done_stamps_cleared_at_on_a_done_task_at_the_named_tag() {
        let (_dir, pool) = test_pool().await;
        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = tasks[0].task_id;
        sqlx::query("UPDATE tasks SET archived_at = 1 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        clear_done(&pool, "@homedepot", 42).await.unwrap();

        assert_eq!(list_pool_tasks(&pool).await.unwrap(), Vec::new());
        let cleared_at: Option<i64> =
            sqlx::query_scalar("SELECT cleared_at FROM tasks WHERE id = ?")
                .bind(task_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(cleared_at, Some(42));
    }

    #[tokio::test]
    async fn clear_done_leaves_an_open_task_at_the_same_tag_alone() {
        let (_dir, pool) = test_pool().await;
        let done_capture = insert_capture(&pool, "buy screws", Some("@homedepot")).await;
        crate::triage::store::insert_task(&pool, done_capture, &TaskKind::Pool, 0)
            .await
            .unwrap();
        let open_capture = insert_capture(&pool, "return the drill", Some("@homedepot")).await;
        crate::triage::store::insert_task(&pool, open_capture, &TaskKind::Pool, 0)
            .await
            .unwrap();
        let done_task_id: i64 = sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
            .bind(done_capture)
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE tasks SET archived_at = 1 WHERE id = ?")
            .bind(done_task_id)
            .execute(&pool)
            .await
            .unwrap();

        clear_done(&pool, "@homedepot", 42).await.unwrap();

        let remaining = list_pool_tasks(&pool).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].raw_text, "return the drill");
        assert!(!remaining[0].done);
    }

    #[tokio::test]
    async fn clear_done_leaves_a_done_task_at_a_different_tag_alone() {
        let (_dir, pool) = test_pool().await;
        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = tasks[0].task_id;
        sqlx::query("UPDATE tasks SET archived_at = 1 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        clear_done(&pool, "@supermarket", 42).await.unwrap();

        let remaining = list_pool_tasks(&pool).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert!(remaining[0].done);
    }
}
