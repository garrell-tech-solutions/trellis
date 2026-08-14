//! The `tasks` table.

use scheduler_core::task::TaskKind;
use sqlx::SqlitePool;

#[cfg(test)]
use scheduler_core::task::{DeadlineType, Period, Priority};

pub async fn insert(
    pool: &SqlitePool,
    capture_id: i64,
    kind: &TaskKind,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    let attributes = kind.attributes();
    sqlx::query(
        "INSERT INTO tasks (capture_id, kind, deadline, deadline_type, priority, \
         target_count, target_minutes_each, period, created_at_ms) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(capture_id)
    .bind(attributes.kind)
    .bind(attributes.deadline)
    .bind(attributes.deadline_type)
    .bind(attributes.priority)
    .bind(attributes.target_count)
    .bind(attributes.target_minutes_each)
    .bind(attributes.period)
    .bind(created_at_ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// A row of [`list_all`]: a task, alongside the text of the capture it was
/// triaged from — the task list's own contents have no text of their own to
/// show, so the join is the store's business, not the page's.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct TaskWithCaptureText {
    pub kind: String,
    pub raw_text: String,
}

/// Every task, newest first.
pub async fn list_all(pool: &SqlitePool) -> Result<Vec<TaskWithCaptureText>, sqlx::Error> {
    sqlx::query_as(
        "SELECT tasks.kind, captures.raw_text FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         ORDER BY tasks.id DESC",
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_pool;

    type StoredTask = (
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<String>,
    );

    async fn stored_task(pool: &SqlitePool) -> StoredTask {
        sqlx::query_as(
            "SELECT kind, deadline, deadline_type, priority, target_count, \
             target_minutes_each, period FROM tasks",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn committed() -> TaskKind {
        TaskKind::Committed {
            deadline: 1787245200000,
            deadline_type: DeadlineType::Hard,
            priority: Priority::P1,
        }
    }

    fn quota() -> TaskKind {
        TaskKind::Quota {
            target_count: 3,
            target_minutes_each: 45,
            period: Period::Week,
        }
    }

    async fn insert_capture(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO captures (raw_text, source, created_at_ms) VALUES ('buy milk', 'web', 0) RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_pool_task_stores_its_kind_and_leaves_every_other_attribute_null() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool).await;

        insert(&pool, capture_id, &TaskKind::Pool, 7).await.unwrap();

        assert_eq!(
            stored_task(&pool).await,
            ("pool".to_string(), None, None, None, None, None, None)
        );
    }

    #[tokio::test]
    async fn a_committed_task_stores_its_scheduling_metadata_and_no_quota_target() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool).await;

        insert(&pool, capture_id, &committed(), 7).await.unwrap();

        assert_eq!(
            stored_task(&pool).await,
            (
                "committed".to_string(),
                Some(1787245200000),
                Some("hard".to_string()),
                Some("P1".to_string()),
                None,
                None,
                None,
            )
        );
    }

    #[tokio::test]
    async fn a_quota_task_stores_its_target_and_no_deadline() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool).await;

        insert(&pool, capture_id, &quota(), 7).await.unwrap();

        assert_eq!(
            stored_task(&pool).await,
            (
                "quota".to_string(),
                None,
                None,
                None,
                Some(3),
                Some(45),
                Some("week".to_string()),
            )
        );
    }

    #[tokio::test]
    async fn the_task_records_the_capture_it_came_from_and_when_it_was_created() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool).await;

        insert(&pool, capture_id, &TaskKind::Pool, 4242)
            .await
            .unwrap();

        let row: (i64, i64) = sqlx::query_as("SELECT capture_id, created_at_ms FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row, (capture_id, 4242));
    }

    #[tokio::test]
    async fn insert_reports_the_database_error_when_the_table_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::connect(&dir.path().join("unmigrated.db"))
            .await
            .unwrap();

        assert!(insert(&pool, 1, &TaskKind::Pool, 0).await.is_err());
    }

    #[tokio::test]
    async fn list_all_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_all(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_all_reports_each_tasks_kind_and_the_text_of_the_capture_it_came_from() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool).await;

        insert(&pool, capture_id, &TaskKind::Pool, 7).await.unwrap();

        assert_eq!(
            list_all(&pool).await.unwrap(),
            vec![TaskWithCaptureText {
                kind: "pool".to_string(),
                raw_text: "buy milk".to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn list_all_lists_tasks_newest_first() {
        let (_dir, pool) = test_pool().await;
        let first_capture = insert_capture(&pool).await;
        let second_capture = sqlx::query_scalar(
            "INSERT INTO captures (raw_text, source, created_at_ms) \
             VALUES ('call the dentist', 'web', 0) RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        insert(&pool, first_capture, &TaskKind::Pool, 1)
            .await
            .unwrap();
        insert(&pool, second_capture, &TaskKind::Pool, 2)
            .await
            .unwrap();

        let tasks = list_all(&pool).await.unwrap();
        assert_eq!(
            tasks
                .iter()
                .map(|t| t.raw_text.as_str())
                .collect::<Vec<_>>(),
            vec!["call the dentist", "buy milk"]
        );
    }
}
