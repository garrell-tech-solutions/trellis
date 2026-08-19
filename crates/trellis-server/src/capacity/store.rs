//! This capability's own read of `tasks`: what committed and quota work
//! asks for, per life area (`T-capability-owns-its-queries`) -- a different
//! question of the same table `triage::store` writes and `stats::store`
//! counts, so it gets its own query rather than reaching into either.
//! Excludes archived tasks (`T-archived-at-only`); nothing writes
//! `archived_at` before M8, so that half of every query here is untested by
//! any affordance, only by a fixture that sets the column directly.

use sqlx::SqlitePool;

/// Every active committed task's estimate in one life area, `None` for one
/// that predates `estimated_minutes` becoming required at triage.
pub async fn committed_estimates(
    pool: &SqlitePool,
    life_area_id: i64,
) -> Result<Vec<Option<i64>>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT estimated_minutes FROM tasks \
         WHERE life_area_id = ? AND kind = 'committed' AND archived_at IS NULL",
    )
    .bind(life_area_id)
    .fetch_all(pool)
    .await
}

/// One active quota task's own recurring target, as stored.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct QuotaTargetRow {
    pub target_count: i64,
    pub target_minutes_each: i64,
    pub period: String,
}

/// Every active quota task's target in one life area.
pub async fn quota_targets(
    pool: &SqlitePool,
    life_area_id: i64,
) -> Result<Vec<QuotaTargetRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT target_count, target_minutes_each, period FROM tasks \
         WHERE life_area_id = ? AND kind = 'quota' AND archived_at IS NULL",
    )
    .bind(life_area_id)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{seeded_life_area_id, test_pool};
    use scheduler_core::task::{DeadlineType, Period, Priority, TaskKind};

    async fn given_a_capture(pool: &SqlitePool) -> i64 {
        crate::capture::store::insert(pool, "buy milk", "web", 0)
            .await
            .unwrap()
    }

    async fn given_a_task(pool: &SqlitePool, life_area_id: i64, kind: &TaskKind) -> i64 {
        let capture_id = given_a_capture(pool).await;
        crate::triage::store::insert_task(pool, capture_id, kind, Some(life_area_id), 0)
            .await
            .unwrap();
        sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn committed(estimated_minutes: i64) -> TaskKind {
        TaskKind::Committed {
            deadline: 1787245200000,
            deadline_type: DeadlineType::Hard,
            priority: Priority::P1,
            estimated_minutes,
        }
    }

    fn quota(target_count: i64, target_minutes_each: i64, period: Period) -> TaskKind {
        TaskKind::Quota {
            target_count,
            target_minutes_each,
            period,
        }
    }

    #[tokio::test]
    async fn committed_estimates_is_empty_for_a_life_area_with_no_committed_tasks() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;

        assert_eq!(committed_estimates(&pool, work).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn committed_estimates_reports_every_committed_tasks_own_estimate() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        given_a_task(&pool, work, &committed(180)).await;
        given_a_task(&pool, work, &committed(120)).await;

        let mut estimates = committed_estimates(&pool, work).await.unwrap();
        estimates.sort();

        assert_eq!(estimates, vec![Some(120), Some(180)]);
    }

    #[tokio::test]
    async fn committed_estimates_excludes_another_life_areas_task() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let fitness = seeded_life_area_id(&pool, "Fitness").await;
        given_a_task(&pool, fitness, &committed(180)).await;

        assert_eq!(committed_estimates(&pool, work).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn committed_estimates_excludes_a_pool_or_quota_task() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        given_a_task(&pool, work, &TaskKind::Pool).await;
        given_a_task(&pool, work, &quota(3, 40, Period::Week)).await;

        assert_eq!(committed_estimates(&pool, work).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn committed_estimates_excludes_an_archived_task() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let task_id = given_a_task(&pool, work, &committed(180)).await;
        sqlx::query("UPDATE tasks SET archived_at = 1000 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(committed_estimates(&pool, work).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn committed_estimates_reports_none_for_a_stored_task_predating_the_column() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let capture_id = given_a_capture(&pool).await;
        // Bypasses the triage boundary on purpose: this is the only way a
        // committed task with no estimate can exist, since triage now
        // requires the field.
        sqlx::query(
            "INSERT INTO tasks (capture_id, kind, deadline, deadline_type, priority, \
             life_area_id, created_at_ms) VALUES (?, 'committed', 1787245200000, 'hard', 'P1', ?, 0)",
        )
        .bind(capture_id)
        .bind(work)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(committed_estimates(&pool, work).await.unwrap(), vec![None]);
    }

    #[tokio::test]
    async fn quota_targets_reports_every_quota_tasks_own_target() {
        let (_dir, pool) = test_pool().await;
        let learning = seeded_life_area_id(&pool, "Learning").await;
        given_a_task(&pool, learning, &quota(3, 40, Period::Week)).await;

        let targets = quota_targets(&pool, learning).await.unwrap();

        assert_eq!(
            targets,
            vec![QuotaTargetRow {
                target_count: 3,
                target_minutes_each: 40,
                period: "week".to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn quota_targets_excludes_a_committed_or_pool_task() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        given_a_task(&pool, work, &committed(180)).await;
        given_a_task(&pool, work, &TaskKind::Pool).await;

        assert_eq!(quota_targets(&pool, work).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn quota_targets_excludes_an_archived_task() {
        let (_dir, pool) = test_pool().await;
        let learning = seeded_life_area_id(&pool, "Learning").await;
        let task_id = given_a_task(&pool, learning, &quota(3, 40, Period::Week)).await;
        sqlx::query("UPDATE tasks SET archived_at = 1000 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(quota_targets(&pool, learning).await.unwrap(), Vec::new());
    }
}
