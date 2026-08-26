//! What an accepted triage writes and reads: the new `tasks` row.
//!
//! Consuming the capture is deliberately *not* here. Triage does not stamp
//! the capture itself; it asks `inbox::close_capture` to take it
//! out, because "still in the inbox" is the inbox's fact and dismissal needs
//! the identical answer (`T-one-front-door-per-capability`). What is left is
//! what triage alone means.
//!
//! Everything here speaks `scheduler_core` types and `sqlx::Error` and must
//! not know an HTTP server exists (`T-module-boundary`, enforced by
//! `platform::boundary`).

use scheduler_core::task::TaskKind;
use sqlx::SqlitePool;

/// `tasks.life_area_id`, `tasks.deadline_type`, `tasks.target_count`,
/// `tasks.target_minutes_each` and `tasks.period` all stay in the schema
/// (#88, #94 and #138, `T-migrations-append-only`: dropping a column means
/// rebuilding the table for nothing) but nothing upstream of this function
/// can produce a value for any of them any more -- a quota's name and
/// weekly target land in `quotas` instead (#138) -- so this always writes
/// `NULL` rather than carrying a parameter every real caller would pass
/// `None` to.
pub async fn insert_task(
    pool: &SqlitePool,
    capture_id: i64,
    kind: &TaskKind,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    let attributes = kind.attributes();
    sqlx::query(
        "INSERT INTO tasks (capture_id, kind, deadline, deadline_type, commitment, priority, \
         estimated_minutes, target_count, target_minutes_each, period, life_area_id, \
         created_at_ms) \
         VALUES (?, ?, ?, NULL, ?, ?, ?, NULL, NULL, NULL, NULL, ?)",
    )
    .bind(capture_id)
    .bind(attributes.kind)
    .bind(attributes.deadline)
    .bind(attributes.commitment)
    .bind(attributes.priority)
    .bind(attributes.estimated_minutes)
    .bind(created_at_ms)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{insert_capture, test_pool};
    use scheduler_core::quota::WeeklyTarget;
    use scheduler_core::task::{Commitment, Priority};

    type StoredTask = (
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<String>,
    );

    async fn stored_task(pool: &SqlitePool) -> StoredTask {
        sqlx::query_as(
            "SELECT kind, deadline, commitment, priority, estimated_minutes, \
             target_count, target_minutes_each, period FROM tasks",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn committed() -> TaskKind {
        TaskKind::Committed {
            deadline: 1787245200000,
            commitment: Commitment::At,
            priority: Priority::P1,
            estimated_minutes: 180,
        }
    }

    fn quota() -> TaskKind {
        TaskKind::Quota {
            name: "Piano".to_string(),
            weekly_target: WeeklyTarget::from_minutes(135).unwrap(),
        }
    }

    #[tokio::test]
    async fn a_pool_task_stores_its_kind_and_leaves_every_other_attribute_null() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;

        insert_task(&pool, capture_id, &TaskKind::Pool, 7)
            .await
            .unwrap();

        assert_eq!(
            stored_task(&pool).await,
            ("pool".to_string(), None, None, None, None, None, None, None)
        );
    }

    #[tokio::test]
    async fn a_committed_task_stores_its_scheduling_metadata_and_no_quota_target() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;

        insert_task(&pool, capture_id, &committed(), 7)
            .await
            .unwrap();

        assert_eq!(
            stored_task(&pool).await,
            (
                "committed".to_string(),
                Some(1787245200000),
                Some("at".to_string()),
                Some("P1".to_string()),
                Some(180),
                None,
                None,
                None,
            )
        );
    }

    #[tokio::test]
    async fn a_committed_task_leaves_the_retired_deadline_type_column_null() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;

        insert_task(&pool, capture_id, &committed(), 7)
            .await
            .unwrap();

        let deadline_type: Option<String> = sqlx::query_scalar("SELECT deadline_type FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(deadline_type, None);
    }

    #[tokio::test]
    async fn a_quota_task_stores_no_deadline_and_no_legacy_target_columns() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;

        insert_task(&pool, capture_id, &quota(), 7).await.unwrap();

        assert_eq!(
            stored_task(&pool).await,
            (
                "quota".to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
                None
            )
        );
    }

    #[tokio::test]
    async fn the_task_records_the_capture_it_came_from_and_when_it_was_created() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;

        insert_task(&pool, capture_id, &TaskKind::Pool, 4242)
            .await
            .unwrap();

        let row: (i64, i64) = sqlx::query_as("SELECT capture_id, created_at_ms FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row, (capture_id, 4242));
    }

    #[tokio::test]
    async fn insert_task_reports_the_database_error_when_the_table_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::platform::db::connect(&dir.path().join("unmigrated.db"))
            .await
            .unwrap();

        assert!(insert_task(&pool, 1, &TaskKind::Pool, 0).await.is_err());
    }
}
