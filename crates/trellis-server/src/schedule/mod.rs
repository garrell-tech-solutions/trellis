//! **Schedule** -- `POST /schedule/generate`, `GET /schedule` (#75, M3
//! slice 1 of 5): places committed tasks into free time, least slack
//! first, and says why the rest did not fit. Forward pass only --
//! splitting, pins and the backward pass are later slices.
//!
//! The algorithm itself -- ordering, the four-reason infeasibility report,
//! the invariants -- lives in `scheduler_core::schedule`, the purest case
//! in the product of a rule that survives changing HTTP. This module is
//! translation either direction: every active committed task and its
//! life area's free time (read through [`crate::free_time::
//! free_time_by_life_area`], the same front door `/capacity` already uses,
//! `T-one-front-door-per-capability`) into `scheduler_core`'s own types on
//! the way in, and its answer into what [`store::replace_plan`] writes and
//! [`view`]'s rows show on the way out.
//!
//! **The plan is a record, not a view** (`R-incremental-patching`): `GET
//! /schedule` only ever reads what the last `POST /schedule/generate`
//! wrote. Nothing here recomputes on page load.

pub mod http;
pub mod store;
pub mod view;

use crate::platform::clock::Clock;
use scheduler_core::schedule::{schedule, LifeAreaWindow, ScheduleTask, UnplaceableReason};
use scheduler_core::task::{DeadlineType, Priority};
use sqlx::SqlitePool;

/// Every active life area's free time over the horizon, reshaped into what
/// `scheduler_core::schedule` asks for -- no projection of its own; that
/// stays `free_time`'s.
async fn scheduler_windows(
    pool: &SqlitePool,
    clock: &Clock,
) -> Result<Vec<LifeAreaWindow>, sqlx::Error> {
    let (areas, _tz) = crate::free_time::free_time_by_life_area(pool, clock).await?;
    Ok(areas
        .into_iter()
        .map(|area| LifeAreaWindow {
            life_area_id: area.id,
            intervals: area.intervals,
        })
        .collect())
}

fn to_schedule_task(row: &store::CommittedTaskRow) -> ScheduleTask {
    ScheduleTask {
        id: row.id,
        life_area_id: row.life_area_id,
        estimated_minutes: row.estimated_minutes,
        deadline_ms: row.deadline,
        deadline_type: DeadlineType::parse(&row.deadline_type)
            .expect("a stored deadline_type was validated by the triage boundary"),
        priority: Priority::parse(&row.priority)
            .expect("a stored priority was validated by the triage boundary"),
    }
}

/// Runs the forward pass over every active committed task and replaces the
/// stored plan wholesale.
///
/// `schedule()`'s answer reaches [`store::replace_plan`] as the very types
/// it returned. Flattening `PlacedBlock` into an `(i64, i64, i64)` on the
/// way was three fields whose names survived only in the reader's head --
/// two of them the same type, so transposing start and end still compiled
/// -- and it forced the reason enum open a layer earlier than the row that
/// stores it (`T-templates-take-view-models`' own reasoning, applied to a
/// store: a boundary carries the shape its far side means, not a tuple).
pub(crate) async fn generate(pool: &SqlitePool, clock: &Clock) -> Result<(), sqlx::Error> {
    let rows = store::committed_tasks(pool).await?;
    let tasks: Vec<ScheduleTask> = rows.iter().map(to_schedule_task).collect();
    let windows = scheduler_windows(pool, clock).await?;

    let result = schedule(&tasks, &[], &windows, &[], &[], &[], clock.now_ms());

    store::replace_plan(pool, &result.placed, &result.unplaceable).await
}

/// `ms` as the RFC 3339 instant every acceptance scenario names literally
/// -- `Timestamp`'s own `Display`, always UTC, since a block's stored
/// instant carries no zone of its own (`T-jiff-epoch-millis`).
fn format_instant(ms: i64) -> String {
    jiff::Timestamp::from_millisecond(ms)
        .expect("a stored block instant is representable")
        .to_string()
}

/// A stored reason, back as the closed thing it was written from.
///
/// The `expect` states an invariant two independent gates already hold:
/// nothing but [`store::replace_plan`] writes this column, and it writes
/// `UnplaceableReason::as_str`; migration `0009`'s own `CHECK` refuses any
/// other word. Re-closing it here is what stops the page rendering whatever
/// text happened to be in the row -- the same move `to_schedule_task` makes
/// for `deadline_type` and `priority`.
fn parse_reason(stored: &str) -> UnplaceableReason {
    UnplaceableReason::parse(stored)
        .expect("a stored reason was written from UnplaceableReason::as_str")
}

/// Every currently placed block, earliest first.
pub(crate) async fn placed_rows(pool: &SqlitePool) -> Result<Vec<view::PlacedRow>, sqlx::Error> {
    let rows = store::placed_blocks(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| view::PlacedRow {
            text: row.raw_text,
            life_area_name: row.life_area_name,
            start: format_instant(row.start_ms),
            end: format_instant(row.end_ms),
            overruns_deadline: row.end_ms > row.deadline,
        })
        .collect())
}

/// Every task the last generation could not place.
pub(crate) async fn unplaceable_rows(
    pool: &SqlitePool,
) -> Result<Vec<view::UnplaceableRow>, sqlx::Error> {
    let rows = store::unplaceable_tasks(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| view::UnplaceableRow {
            text: row.raw_text,
            reason: parse_reason(&row.reason).as_str(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{seeded_life_area_id, test_pool};
    use scheduler_core::task::TaskKind;

    async fn given_a_committed_task(
        pool: &SqlitePool,
        life_area_id: i64,
        raw_text: &str,
        estimated_minutes: i64,
        deadline_ms: i64,
        deadline_type: DeadlineType,
    ) {
        let capture_id = crate::capture::store::insert(pool, raw_text, "web", 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            pool,
            capture_id,
            &TaskKind::Committed {
                deadline: deadline_ms,
                deadline_type,
                priority: Priority::P2,
                estimated_minutes,
            },
            Some(life_area_id),
            0,
        )
        .await
        .unwrap();
    }

    /// Monday 2026-08-17T09:00:00Z, and a Mon-Fri 09:00-17:00 Work
    /// guardrail -- the fixture every test below places against.
    async fn work_with_a_weekday_guardrail(pool: &SqlitePool) -> i64 {
        let work = seeded_life_area_id(pool, "Work").await;
        for day in ["Mon", "Tue", "Wed", "Thu", "Fri"] {
            crate::life_areas::store::insert_guardrail_band(pool, work, day, 540, 1020)
                .await
                .unwrap();
        }
        work
    }

    /// The narrower fixture a test needs when the free window itself must
    /// be exactly the two hours 09:00-11:00 this Monday, not the whole
    /// working week [`work_with_a_weekday_guardrail`] opens up.
    async fn work_with_a_monday_morning_guardrail(pool: &SqlitePool) -> i64 {
        let work = seeded_life_area_id(pool, "Work").await;
        crate::life_areas::store::insert_guardrail_band(pool, work, "Mon", 540, 660)
            .await
            .unwrap();
        work
    }

    /// "write the Q3 deck", 120 minutes, due four days out with a hard
    /// deadline -- the placed-task fixture two tests below both start from.
    async fn given_the_q3_deck_task(pool: &SqlitePool, life_area_id: i64) {
        given_a_committed_task(
            pool,
            life_area_id,
            "write the Q3 deck",
            120,
            1_786_957_200_000 + 4 * 24 * 3_600_000,
            DeadlineType::Hard,
        )
        .await;
    }

    fn pinned_monday_nine() -> Clock {
        Clock::pinned_at(1_786_957_200_000) // 2026-08-17T09:00:00Z
    }

    #[tokio::test]
    async fn generate_places_a_committed_task_inside_its_life_areas_hours() {
        let (_dir, pool) = test_pool().await;
        let work = work_with_a_weekday_guardrail(&pool).await;
        given_the_q3_deck_task(&pool, work).await;

        generate(&pool, &pinned_monday_nine()).await.unwrap();

        let placed = placed_rows(&pool).await.unwrap();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].text, "write the Q3 deck");
        assert_eq!(placed[0].life_area_name.as_deref(), Some("Work"));
        assert_eq!(placed[0].start, "2026-08-17T09:00:00Z");
        assert_eq!(placed[0].end, "2026-08-17T11:00:00Z");
        assert!(!placed[0].overruns_deadline);
    }

    #[tokio::test]
    async fn generate_reports_an_unplaceable_task_with_its_reason() {
        let (_dir, pool) = test_pool().await;
        let work = work_with_a_monday_morning_guardrail(&pool).await;
        given_a_committed_task(
            &pool,
            work,
            "rebuild the deck",
            180,
            1_786_957_200_000 + 10 * 24 * 3_600_000,
            DeadlineType::Soft,
        )
        .await;

        generate(&pool, &pinned_monday_nine()).await.unwrap();

        let unplaceable = unplaceable_rows(&pool).await.unwrap();
        assert_eq!(unplaceable.len(), 1);
        assert_eq!(unplaceable[0].text, "rebuild the deck");
        assert_eq!(unplaceable[0].reason, "chunk_policy_unsatisfiable");
        assert!(placed_rows(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn generate_never_places_a_pool_or_quota_task() {
        let (_dir, pool) = test_pool().await;
        let work = work_with_a_weekday_guardrail(&pool).await;
        let capture_id = crate::capture::store::insert(&pool, "read the spec", "web", 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(&pool, capture_id, &TaskKind::Pool, Some(work), 0)
            .await
            .unwrap();

        generate(&pool, &pinned_monday_nine()).await.unwrap();

        assert!(placed_rows(&pool).await.unwrap().is_empty());
        assert!(unplaceable_rows(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn generate_replaces_the_previous_plan_rather_than_appending_to_it() {
        let (_dir, pool) = test_pool().await;
        let work = work_with_a_weekday_guardrail(&pool).await;
        given_the_q3_deck_task(&pool, work).await;
        generate(&pool, &pinned_monday_nine()).await.unwrap();
        assert_eq!(placed_rows(&pool).await.unwrap().len(), 1);

        given_a_committed_task(
            &pool,
            work,
            "book the venue",
            60,
            1_786_957_200_000 + 4 * 24 * 3_600_000,
            DeadlineType::Hard,
        )
        .await;

        // No regenerate yet -- the second task must not appear.
        assert_eq!(placed_rows(&pool).await.unwrap().len(), 1);

        generate(&pool, &pinned_monday_nine()).await.unwrap();
        assert_eq!(placed_rows(&pool).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_soft_task_placed_past_its_deadline_reports_the_overrun() {
        let (_dir, pool) = test_pool().await;
        let work = work_with_a_monday_morning_guardrail(&pool).await;
        // The only free interval is 09:00-11:00 this Monday; the deadline
        // is inside it, but a soft task placed there still finishes at
        // 11:00, after a deadline set to 10:00.
        given_a_committed_task(
            &pool,
            work,
            "file the return",
            120,
            1_786_957_200_000 + 3_600_000,
            DeadlineType::Soft,
        )
        .await;

        generate(&pool, &pinned_monday_nine()).await.unwrap();

        let placed = placed_rows(&pool).await.unwrap();
        assert_eq!(placed.len(), 1);
        assert!(placed[0].overruns_deadline);
    }

    #[tokio::test]
    async fn the_tighter_deadline_is_placed_first_and_the_looser_one_overruns() {
        let (_dir, pool) = test_pool().await;
        let work = work_with_a_monday_morning_guardrail(&pool).await;
        given_a_committed_task(
            &pool,
            work,
            "renew the passport",
            120,
            1_786_957_200_000 + 4 * 24 * 3_600_000, // 2026-08-21T17:00:00Z
            DeadlineType::Soft,
        )
        .await;
        given_a_committed_task(
            &pool,
            work,
            "file the return",
            120,
            1_786_957_200_000 + 2 * 3_600_000, // 2026-08-17T11:00:00Z
            DeadlineType::Soft,
        )
        .await;

        generate(&pool, &pinned_monday_nine()).await.unwrap();

        let placed = placed_rows(&pool).await.unwrap();
        let row_for = |text: &str| placed.iter().find(|r| r.text == text).unwrap();
        assert_eq!(row_for("file the return").start, "2026-08-17T09:00:00Z");
        assert_eq!(row_for("renew the passport").start, "2026-08-24T09:00:00Z");
        assert!(row_for("renew the passport").overruns_deadline);
        assert!(!row_for("file the return").overruns_deadline);
    }
}
