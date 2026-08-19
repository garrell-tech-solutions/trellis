//! This capability's own read of `tasks` (`T-capability-owns-its-queries`):
//! every active committed task with an estimate and a resolved life area,
//! alongside the text of the capture it came from. And its own two tables:
//! `block`, the Plan-layer placements `schedule()` writes, and
//! `schedule_unplaceable`, the infeasibility report alongside them
//! (`T-fact-plan-line`).

use scheduler_core::schedule::{PlacedBlock, Unplaceable};
use sqlx::SqlitePool;

/// A committed task as the forward pass needs it, plus its own text --
/// nothing else renders a task's text into the schedule page, so this
/// capability reads it directly rather than asking another one for it
/// (`T-capability-owns-its-queries`).
///
/// `T-capacity-never-under-reports-demand`'s reasoning applies here too: a
/// stored task with no estimate or no resolved life area predates a
/// boundary that now requires both, or bypassed it. Excluded outright by
/// this query's own `WHERE`, rather than reaching `scheduler_core::schedule`
/// as a task it cannot place -- the reason enum does not grow for a state
/// that cannot occur through triage.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct CommittedTaskRow {
    pub id: i64,
    pub life_area_id: i64,
    pub estimated_minutes: i64,
    pub deadline: i64,
    pub deadline_type: String,
    pub priority: String,
    pub raw_text: String,
}

/// Every active committed task the forward pass may place.
pub async fn committed_tasks(pool: &SqlitePool) -> Result<Vec<CommittedTaskRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT tasks.id, tasks.life_area_id, tasks.estimated_minutes, tasks.deadline, \
         tasks.deadline_type, tasks.priority, captures.raw_text FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE tasks.kind = 'committed' AND tasks.archived_at IS NULL \
         AND tasks.life_area_id IS NOT NULL AND tasks.estimated_minutes IS NOT NULL",
    )
    .fetch_all(pool)
    .await
}

/// One currently placed block, joined back to the text, life area and
/// deadline a reader needs to render it -- everything [`super::view::
/// PlacedRow`] is built from.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PlacedBlockRow {
    pub start_ms: i64,
    pub end_ms: i64,
    pub raw_text: String,
    pub life_area_name: Option<String>,
    pub deadline: i64,
}

/// Every currently proposed block, earliest first -- the plan as it was
/// last written, read back exactly (`R-incremental-patching`).
pub async fn placed_blocks(pool: &SqlitePool) -> Result<Vec<PlacedBlockRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT block.start_ms, block.end_ms, captures.raw_text, \
         life_areas.name AS life_area_name, tasks.deadline FROM block \
         JOIN tasks ON tasks.id = block.task_id \
         JOIN captures ON captures.id = tasks.capture_id \
         LEFT JOIN life_areas ON life_areas.id = tasks.life_area_id \
         WHERE block.state = 'proposed' \
         ORDER BY block.start_ms ASC",
    )
    .fetch_all(pool)
    .await
}

/// One currently unplaceable task, joined back to its text.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct UnplaceableTaskRow {
    pub raw_text: String,
    pub reason: String,
}

/// Every task the last `schedule()` run could not place, and why.
pub async fn unplaceable_tasks(pool: &SqlitePool) -> Result<Vec<UnplaceableTaskRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT captures.raw_text, schedule_unplaceable.reason FROM schedule_unplaceable \
         JOIN tasks ON tasks.id = schedule_unplaceable.task_id \
         JOIN captures ON captures.id = tasks.capture_id \
         ORDER BY schedule_unplaceable.id ASC",
    )
    .fetch_all(pool)
    .await
}

async fn clear_plan(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM block WHERE state IN ('proposed', 'published')")
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM schedule_unplaceable")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn insert_placed_blocks(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    placed: &[PlacedBlock],
) -> Result<(), sqlx::Error> {
    for block in placed {
        sqlx::query(
            "INSERT INTO block (task_id, start_ms, end_ms, state) VALUES (?, ?, ?, 'proposed')",
        )
        .bind(block.task_id)
        .bind(block.start_ms)
        .bind(block.end_ms)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Each reason as `UnplaceableReason::as_str` writes it -- the core owns
/// the word, this owns the row. Migration `0009`'s `CHECK` names the same
/// four and is the backstop, not the definition.
async fn insert_unplaceable_reasons(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    unplaceable: &[Unplaceable],
) -> Result<(), sqlx::Error> {
    for task in unplaceable {
        sqlx::query("INSERT INTO schedule_unplaceable (task_id, reason) VALUES (?, ?)")
            .bind(task.task_id)
            .bind(task.reason.as_str())
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// Replaces the whole stored plan: every placed block and every
/// unplaceable reason, wholesale (`R-incremental-patching`: recomputed
/// from scratch, never patched). One transaction, so a reader never sees a
/// moment with the old plan's blocks deleted but its unplaceable rows
/// still standing, or the reverse.
pub async fn replace_plan(
    pool: &SqlitePool,
    placed: &[PlacedBlock],
    unplaceable: &[Unplaceable],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    clear_plan(&mut tx).await?;
    insert_placed_blocks(&mut tx, placed).await?;
    insert_unplaceable_reasons(&mut tx, unplaceable).await?;
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{seeded_life_area_id, test_pool};
    use scheduler_core::schedule::UnplaceableReason;
    use scheduler_core::task::{DeadlineType, Priority, TaskKind};

    async fn given_a_committed_task(pool: &SqlitePool, life_area_id: i64, raw_text: &str) -> i64 {
        let capture_id = crate::capture::store::insert(pool, raw_text, "web", 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            pool,
            capture_id,
            &TaskKind::Committed {
                deadline: 1_787_245_200_000,
                deadline_type: DeadlineType::Hard,
                priority: Priority::P1,
                estimated_minutes: 120,
            },
            Some(life_area_id),
            0,
        )
        .await
        .unwrap();
        sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn block(task_id: i64, start_ms: i64, end_ms: i64) -> PlacedBlock {
        PlacedBlock {
            task_id,
            start_ms,
            end_ms,
        }
    }

    fn refused(task_id: i64, reason: UnplaceableReason) -> Unplaceable {
        Unplaceable { task_id, reason }
    }

    #[tokio::test]
    async fn committed_tasks_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;
        assert_eq!(committed_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn committed_tasks_reports_an_active_committed_tasks_own_fields() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        given_a_committed_task(&pool, work, "write the Q3 deck").await;

        let rows = committed_tasks(&pool).await.unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].life_area_id, work);
        assert_eq!(rows[0].estimated_minutes, 120);
        assert_eq!(rows[0].raw_text, "write the Q3 deck");
        assert_eq!(rows[0].deadline_type, "hard");
        assert_eq!(rows[0].priority, "P1");
    }

    #[tokio::test]
    async fn committed_tasks_excludes_a_pool_or_quota_task() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let capture_id = crate::capture::store::insert(&pool, "read the spec", "web", 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(&pool, capture_id, &TaskKind::Pool, Some(work), 0)
            .await
            .unwrap();

        assert_eq!(committed_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn committed_tasks_excludes_an_archived_task() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let task_id = given_a_committed_task(&pool, work, "write the Q3 deck").await;
        sqlx::query("UPDATE tasks SET archived_at = 1000 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(committed_tasks(&pool).await.unwrap(), Vec::new());
    }

    /// The state schedule's own reason enum refuses to grow for
    /// (`T-capacity-never-under-reports-demand`'s reasoning, applied here):
    /// bypassing the triage boundary is the only way to produce it.
    #[tokio::test]
    async fn committed_tasks_excludes_a_task_with_no_estimate() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let capture_id = crate::capture::store::insert(&pool, "buy milk", "web", 0)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO tasks (capture_id, kind, deadline, deadline_type, priority, \
             life_area_id, created_at_ms) VALUES (?, 'committed', 1787245200000, 'hard', 'P1', ?, 0)",
        )
        .bind(capture_id)
        .bind(work)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(committed_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn replace_plan_stores_every_placed_block() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let task_id = given_a_committed_task(&pool, work, "write the Q3 deck").await;

        replace_plan(&pool, &[block(task_id, 1_000, 2_000)], &[])
            .await
            .unwrap();

        let placed = placed_blocks(&pool).await.unwrap();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].start_ms, 1_000);
        assert_eq!(placed[0].end_ms, 2_000);
        assert_eq!(placed[0].raw_text, "write the Q3 deck");
        assert_eq!(placed[0].life_area_name.as_deref(), Some("Work"));
    }

    #[tokio::test]
    async fn replace_plan_stores_every_unplaceable_task_and_its_reason() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let task_id = given_a_committed_task(&pool, work, "rebuild the deck").await;

        replace_plan(
            &pool,
            &[],
            &[refused(
                task_id,
                UnplaceableReason::ChunkPolicyUnsatisfiable,
            )],
        )
        .await
        .unwrap();

        let unplaceable = unplaceable_tasks(&pool).await.unwrap();
        assert_eq!(
            unplaceable,
            vec![UnplaceableTaskRow {
                raw_text: "rebuild the deck".to_string(),
                reason: "chunk_policy_unsatisfiable".to_string(),
            }]
        );
    }

    /// **The schema and the core agree about the four reasons, and this is
    /// what makes them agree.** Migration `0009`'s `CHECK (reason IN ...)`
    /// and `UnplaceableReason::as_str` are two independent statements of one
    /// closed vocabulary, in two languages, with nothing tying them -- the
    /// same shape `removing_a_band_removes_exactly_the_rows_that_band
    /// _displayed` exists for over in guardrails. A fifth reason added to
    /// the enum and not to a migration fails here rather than at the first
    /// run that produces it.
    ///
    /// It walks `ALL` rather than listing the four, so the fifth reason is
    /// covered by existing, and it is exhaustive rather than sampled
    /// because the set is closed and has four members -- a property test
    /// over it would be strictly weaker.
    #[tokio::test]
    async fn every_reason_the_core_can_produce_is_a_reason_the_schema_accepts() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let task_id = given_a_committed_task(&pool, work, "rebuild the deck").await;

        for reason in UnplaceableReason::ALL {
            replace_plan(&pool, &[], &[refused(task_id, reason)])
                .await
                .unwrap_or_else(|error| panic!("the schema refused {reason:?}: {error}"));

            let stored = unplaceable_tasks(&pool).await.unwrap();
            assert_eq!(
                stored
                    .iter()
                    .map(|row| row.reason.as_str())
                    .collect::<Vec<_>>(),
                vec![reason.as_str()],
                "{reason:?} did not survive the round trip through the row"
            );
            assert_eq!(UnplaceableReason::parse(&stored[0].reason), Some(reason));
        }
    }

    #[tokio::test]
    async fn replace_plan_wholly_replaces_the_previous_plan() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let first = given_a_committed_task(&pool, work, "first").await;
        let second = given_a_committed_task(&pool, work, "second").await;

        replace_plan(&pool, &[block(first, 1_000, 2_000)], &[])
            .await
            .unwrap();
        replace_plan(&pool, &[block(second, 3_000, 4_000)], &[])
            .await
            .unwrap();

        let placed = placed_blocks(&pool).await.unwrap();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].raw_text, "second");
    }

    #[tokio::test]
    async fn placed_blocks_is_ordered_by_start_time() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        let first = given_a_committed_task(&pool, work, "first").await;
        let second = given_a_committed_task(&pool, work, "second").await;

        replace_plan(
            &pool,
            &[block(second, 3_000, 4_000), block(first, 1_000, 2_000)],
            &[],
        )
        .await
        .unwrap();

        let placed = placed_blocks(&pool).await.unwrap();
        assert_eq!(
            placed
                .iter()
                .map(|p| p.raw_text.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
    }
}
