//! `/stats`'s own query: task counts by kind between two instants.
//!
//! Counting tasks for `/stats` is `/stats`'s query, in its own `store.rs` --
//! not a function added to `triage::store`, which already touches `tasks`
//! for an unrelated reason (`T-capability-owns-its-queries`). Windowed by
//! `tasks.created_at_ms`, which is triage time (R2: "instrument the ratio at
//! triage") -- not `captures.created_at_ms`, which is capture time and
//! diverges from it whenever a capture sits in the inbox for days.
//!
//! Which instants those are is not decided here. This module knows how to
//! count a range; how long the range is, and what the counts mean, are
//! `scheduler_core::ratio`'s -- so a change to the window's length never
//! reaches the SQL, and the SQL never has an opinion about the ratio.

use scheduler_core::ratio::KindCounts;
use sqlx::SqlitePool;

/// Task counts by kind, over `[window_start_ms, now_ms]` inclusive.
pub async fn counts_in_window(
    pool: &SqlitePool,
    window_start_ms: i64,
    now_ms: i64,
) -> Result<KindCounts, sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT kind, COUNT(*) FROM tasks \
         WHERE created_at_ms >= ? AND created_at_ms <= ? \
         GROUP BY kind",
    )
    .bind(window_start_ms)
    .bind(now_ms)
    .fetch_all(pool)
    .await?;

    let mut counts = KindCounts::default();
    for (kind, count) in rows {
        counts.record(&kind, count);
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use proptest::prelude::*;
    use scheduler_core::task::TaskKind;

    async fn given_a_capture(pool: &SqlitePool) -> i64 {
        crate::capture::store::insert(pool, "buy milk", "web", 0)
            .await
            .unwrap()
    }

    async fn given_a_task(pool: &SqlitePool, kind: &TaskKind, created_at_ms: i64) {
        let capture_id = given_a_capture(pool).await;
        crate::triage::store::insert_task(pool, capture_id, kind, None, created_at_ms)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn counts_in_window_is_zero_against_an_empty_database() {
        let (_dir, pool) = test_pool().await;

        let counts = counts_in_window(&pool, 0, 1_000_000).await.unwrap();

        assert_eq!(counts, KindCounts::default());
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
                estimated_minutes: 180,
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
            KindCounts {
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

    /// The three kinds, indexed so a property can draw one.
    fn nth_kind(index: usize) -> TaskKind {
        match index {
            0 => TaskKind::Pool,
            1 => TaskKind::Committed {
                deadline: 1787245200000,
                deadline_type: scheduler_core::task::DeadlineType::Hard,
                priority: scheduler_core::task::Priority::P1,
                estimated_minutes: 180,
            },
            _ => TaskKind::Quota {
                target_count: 3,
                target_minutes_each: 45,
                period: scheduler_core::task::Period::Week,
            },
        }
    }

    fn tallied_in_memory(tasks: &[(usize, i64)], window: (i64, i64)) -> KindCounts {
        let mut counts = KindCounts::default();
        for (kind, created_at_ms) in tasks {
            if !(window.0..=window.1).contains(created_at_ms) {
                continue;
            }
            match kind {
                0 => counts.pool += 1,
                1 => counts.committed += 1,
                _ => counts.quota += 1,
            }
        }
        counts
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

        /// The query is a *filter and a tally*: exactly the tasks stamped
        /// inside the window, each counted against its own kind. The example
        /// tests above sample one task either side of each bound; this pins
        /// the whole `WHERE` and the `GROUP BY` against any mix of kinds and
        /// any spread of instants across and around the window, which is the
        /// shape a fortnight of real triage actually has.
        #[test]
        #[ignore]
        fn counts_in_window_tallies_exactly_the_tasks_stamped_inside_it(
            tasks in prop::collection::vec((0usize..3, 900i64..2_101), 0..12),
        ) {
            const WINDOW: (i64, i64) = (1_000, 2_000);

            let rt = tokio::runtime::Runtime::new().unwrap();
            let counted = rt.block_on(async {
                let (_dir, pool) = test_pool().await;
                for (kind, created_at_ms) in &tasks {
                    given_a_task(&pool, &nth_kind(*kind), *created_at_ms).await;
                }
                counts_in_window(&pool, WINDOW.0, WINDOW.1).await.unwrap()
            });

            prop_assert_eq!(counted, tallied_in_memory(&tasks, WINDOW));
        }
    }
}
