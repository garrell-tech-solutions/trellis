//! `/stats`'s own query: task counts by kind inside the rolling window.
//!
//! Counting tasks for `/stats` is `/stats`'s query, in its own `store.rs` --
//! not a function added to `triage::store`, which already touches `tasks`
//! for an unrelated reason (`T-capability-owns-its-queries`). Windowed by
//! `tasks.created_at_ms`, which is triage time (R2: "instrument the ratio at
//! triage") -- not `captures.created_at_ms`, which is capture time and
//! diverges from it whenever a capture sits in the inbox for days.

use sqlx::SqlitePool;

/// Fourteen days, in milliseconds: "14 x 24h back from now -- instant
/// arithmetic on epoch millis, no timezone" (the handoff brief's settled
/// answer to the window's exact definition).
pub const WINDOW_MS: i64 = 14 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowCounts {
    pub committed: i64,
    pub pool: i64,
    pub quota: i64,
}

/// Task counts by kind, over `[window_start_ms, now_ms]` inclusive.
pub async fn counts_in_window(
    pool: &SqlitePool,
    window_start_ms: i64,
    now_ms: i64,
) -> Result<WindowCounts, sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT kind, COUNT(*) FROM tasks \
         WHERE created_at_ms >= ? AND created_at_ms <= ? \
         GROUP BY kind",
    )
    .bind(window_start_ms)
    .bind(now_ms)
    .fetch_all(pool)
    .await?;

    let mut counts = WindowCounts::default();
    for (kind, count) in rows {
        add_kind_count(&mut counts, &kind, count);
    }
    Ok(counts)
}

fn add_kind_count(counts: &mut WindowCounts, kind: &str, count: i64) {
    match kind {
        "committed" => counts.committed = count,
        "pool" => counts.pool = count,
        "quota" => counts.quota = count,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use scheduler_core::task::TaskKind;

    async fn given_a_capture(pool: &SqlitePool) -> i64 {
        crate::capture::store::insert(pool, "buy milk", "web", 0)
            .await
            .unwrap()
    }

    async fn given_a_task(pool: &SqlitePool, kind: &TaskKind, created_at_ms: i64) {
        let capture_id = given_a_capture(pool).await;
        crate::triage::store::insert_task(pool, capture_id, kind, created_at_ms)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn counts_in_window_is_zero_against_an_empty_database() {
        let (_dir, pool) = test_pool().await;

        let counts = counts_in_window(&pool, 0, 1_000_000).await.unwrap();

        assert_eq!(counts, WindowCounts::default());
    }

    #[tokio::test]
    async fn counts_in_window_tallies_each_kind_separately() {
        let (_dir, pool) = test_pool().await;
        given_a_task(&pool, &TaskKind::Pool, 100).await;
        given_a_task(&pool, &TaskKind::Pool, 100).await;
        given_a_task(
            &pool,
            &TaskKind::Committed {
                deadline: 1787245200000,
                deadline_type: scheduler_core::task::DeadlineType::Hard,
                priority: scheduler_core::task::Priority::P1,
            },
            100,
        )
        .await;
        given_a_task(
            &pool,
            &TaskKind::Quota {
                target_count: 3,
                target_minutes_each: 45,
                period: scheduler_core::task::Period::Week,
            },
            100,
        )
        .await;

        let counts = counts_in_window(&pool, 0, 1_000).await.unwrap();

        assert_eq!(
            counts,
            WindowCounts {
                committed: 1,
                pool: 2,
                quota: 1,
            }
        );
    }

    #[tokio::test]
    async fn counts_in_window_excludes_a_task_created_before_the_window_start() {
        let (_dir, pool) = test_pool().await;
        given_a_task(&pool, &TaskKind::Pool, 50).await;
        given_a_task(&pool, &TaskKind::Pool, 150).await;

        let counts = counts_in_window(&pool, 100, 1_000).await.unwrap();

        assert_eq!(counts.pool, 1);
    }

    #[tokio::test]
    async fn counts_in_window_excludes_a_task_created_after_now() {
        let (_dir, pool) = test_pool().await;
        given_a_task(&pool, &TaskKind::Pool, 500).await;
        given_a_task(&pool, &TaskKind::Pool, 2_000).await;

        let counts = counts_in_window(&pool, 0, 1_000).await.unwrap();

        assert_eq!(counts.pool, 1);
    }

    #[tokio::test]
    async fn counts_in_window_includes_the_window_boundaries() {
        let (_dir, pool) = test_pool().await;
        given_a_task(&pool, &TaskKind::Pool, 100).await;
        given_a_task(&pool, &TaskKind::Pool, 1_000).await;

        let counts = counts_in_window(&pool, 100, 1_000).await.unwrap();

        assert_eq!(counts.pool, 2);
    }

    #[tokio::test]
    async fn counts_in_window_reports_the_database_error_when_the_table_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::platform::db::connect(&dir.path().join("unmigrated.db"))
            .await
            .unwrap();

        assert!(counts_in_window(&pool, 0, 1_000).await.is_err());
    }
}
