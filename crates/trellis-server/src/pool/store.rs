//! What the pool screen reads: every pool task, with the text and context
//! tag of the capture it came from.
//!
//! `D-no-pool-on-calendar`: this is the *only* place pool work is offered,
//! so this query's `WHERE tasks.kind = 'pool'` is the whole of what keeps
//! committed and quota work off this screen (`pool-screen-only-pool-04`).

use sqlx::SqlitePool;

/// A row of [`list_pool_tasks`].
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PoolTaskRow {
    pub task_id: i64,
    pub raw_text: String,
    pub context_tag: Option<String>,
}

/// Every pool task, alongside the text and context tag of the capture it
/// was triaged from. Unordered on purpose: `scheduler_core::pool::group`
/// establishes newest-first itself rather than trusting a caller to have
/// sorted already.
pub async fn list_pool_tasks(pool: &SqlitePool) -> Result<Vec<PoolTaskRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT tasks.id AS task_id, captures.raw_text, captures.context_tag FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE tasks.kind = 'pool'",
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use scheduler_core::task::TaskKind;

    async fn given_a_capture(pool: &SqlitePool, raw_text: &str, tag: Option<&str>) -> i64 {
        crate::capture::store::insert(pool, raw_text, "web", tag, 0)
            .await
            .unwrap()
    }

    async fn given_a_pool_task(
        pool: &SqlitePool,
        raw_text: &str,
        tag: Option<&str>,
    ) -> Vec<PoolTaskRow> {
        let capture_id = given_a_capture(pool, raw_text, tag).await;
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
        let pool_capture = given_a_capture(&pool, "buy screws", Some("@homedepot")).await;
        let committed_capture = given_a_capture(&pool, "file the return", Some("@homedepot")).await;
        let quota_capture = given_a_capture(&pool, "practise piano", Some("@homedepot")).await;
        crate::triage::store::insert_task(&pool, pool_capture, &TaskKind::Pool, 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            &pool,
            committed_capture,
            &TaskKind::Committed {
                deadline: 1787245200000,
                commitment: scheduler_core::task::Commitment::At,
                priority: scheduler_core::task::Priority::P1,
                estimated_minutes: 30,
            },
            0,
        )
        .await
        .unwrap();
        crate::triage::store::insert_task(
            &pool,
            quota_capture,
            &TaskKind::Quota {
                target_count: 3,
                target_minutes_each: 20,
                period: scheduler_core::task::Period::Week,
            },
            0,
        )
        .await
        .unwrap();

        let tasks = list_pool_tasks(&pool).await.unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].raw_text, "buy screws");
    }

    #[tokio::test]
    async fn list_pool_tasks_excludes_an_untriaged_capture() {
        let (_dir, pool) = test_pool().await;
        given_a_capture(&pool, "buy screws", Some("@homedepot")).await;

        assert_eq!(list_pool_tasks(&pool).await.unwrap(), Vec::new());
    }
}
