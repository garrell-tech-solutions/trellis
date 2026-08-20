//! What an accepted triage writes and reads: the new `tasks` row, and the
//! life-area name it must resolve before writing one.
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

/// `life_area_id` is nullable at storage for schema reasons (the same
/// `T-quota-targets-required` pattern the quota target columns already use)
/// even though the triage boundary requires one for every kind; callers that
/// have already resolved a submission's life area pass `Some`, and only a
/// fixture inserting a row directly (bypassing the boundary) would pass
/// `None`.
pub async fn insert_task(
    pool: &SqlitePool,
    capture_id: i64,
    kind: &TaskKind,
    life_area_id: Option<i64>,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    let attributes = kind.attributes();
    sqlx::query(
        "INSERT INTO tasks (capture_id, kind, deadline, deadline_type, priority, \
         estimated_minutes, target_count, target_minutes_each, period, life_area_id, \
         created_at_ms) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(capture_id)
    .bind(attributes.kind)
    .bind(attributes.deadline)
    .bind(attributes.deadline_type)
    .bind(attributes.priority)
    .bind(attributes.estimated_minutes)
    .bind(attributes.target_count)
    .bind(attributes.target_minutes_each)
    .bind(attributes.period)
    .bind(life_area_id)
    .bind(created_at_ms)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use scheduler_core::task::{DeadlineType, Period, Priority};

    /// Setting up "a capture exists" by calling the capture domain's own
    /// writer rather than retyping its `INSERT` here: a fixture that spells
    /// out another module's SQL is a second copy of that schema.
    async fn given_a_capture(pool: &SqlitePool, raw_text: &str) -> i64 {
        crate::capture::store::insert(pool, raw_text, "web", None, 0)
            .await
            .unwrap()
    }

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
            "SELECT kind, deadline, deadline_type, priority, estimated_minutes, \
             target_count, target_minutes_each, period FROM tasks",
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
            estimated_minutes: 180,
        }
    }

    fn quota() -> TaskKind {
        TaskKind::Quota {
            target_count: 3,
            target_minutes_each: 45,
            period: Period::Week,
        }
    }

    #[tokio::test]
    async fn a_pool_task_stores_its_kind_and_leaves_every_other_attribute_null() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;

        insert_task(&pool, capture_id, &TaskKind::Pool, None, 7)
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
        let capture_id = given_a_capture(&pool, "buy milk").await;

        insert_task(&pool, capture_id, &committed(), None, 7)
            .await
            .unwrap();

        assert_eq!(
            stored_task(&pool).await,
            (
                "committed".to_string(),
                Some(1787245200000),
                Some("hard".to_string()),
                Some("P1".to_string()),
                Some(180),
                None,
                None,
                None,
            )
        );
    }

    #[tokio::test]
    async fn a_quota_task_stores_its_target_and_no_deadline() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;

        insert_task(&pool, capture_id, &quota(), None, 7)
            .await
            .unwrap();

        assert_eq!(
            stored_task(&pool).await,
            (
                "quota".to_string(),
                None,
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
        let capture_id = given_a_capture(&pool, "buy milk").await;

        insert_task(&pool, capture_id, &TaskKind::Pool, None, 4242)
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

        assert!(insert_task(&pool, 1, &TaskKind::Pool, None, 0)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn a_task_stores_the_resolved_life_area_id() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;

        insert_task(&pool, capture_id, &TaskKind::Pool, Some(3), 7)
            .await
            .unwrap();

        let life_area_id: Option<i64> = sqlx::query_scalar("SELECT life_area_id FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(life_area_id, Some(3));
    }
}
