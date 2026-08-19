//! **Capacity** -- `GET /capacity` (#62, M2's closing slice): what each
//! active life area needs against what it has, over the free-time horizon.
//! Supply is [`crate::free_time::free_time_by_life_area`]'s own answer --
//! guardrail bands minus exceptions -- read through that front door rather
//! than re-derived, so this page and `/free-time` can never disagree about
//! the same fortnight (`T-one-front-door-per-capability`). Demand is this
//! capability's own query over `tasks` ([`store`]), because counting what
//! committed and quota work ask for is a question nothing else already
//! answers (`T-capability-owns-its-queries`).
//!
//! The arithmetic itself -- what counts as demand, how quota prorates,
//! where the warning fires -- lives in `scheduler_core::capacity`, the same
//! call `scheduler_core::ratio` already made for `/stats`' rules.

pub mod http;
pub mod store;
pub mod view;

use crate::platform::clock::Clock;
use scheduler_core::capacity::{capacity_for, committed_demand, quota_demand_minutes};
use scheduler_core::task::Period;
use sqlx::SqlitePool;
use view::CapacityRow;

/// One active life area's demand: committed and quota work summed into
/// minutes, and how many committed tasks carried no estimate
/// (`store::committed_estimates`' own contract -- a stored task predating
/// the column, never counted as zero).
async fn demand_minutes(pool: &SqlitePool, life_area_id: i64) -> Result<(i64, usize), sqlx::Error> {
    let estimates = store::committed_estimates(pool, life_area_id).await?;
    let committed = committed_demand(&estimates);

    let targets = store::quota_targets(pool, life_area_id).await?;
    let quota_minutes: i64 = targets
        .iter()
        .map(|target| {
            let period = Period::parse(&target.period).expect(
                "a stored period was validated by T-period-closed-set before it was written",
            );
            quota_demand_minutes(
                target.target_count,
                target.target_minutes_each,
                period,
                crate::free_time::HORIZON_DAYS,
            )
        })
        .sum();

    Ok((committed.minutes + quota_minutes, committed.unestimated))
}

fn never_scheduled_row(id: i64, name: String) -> CapacityRow {
    CapacityRow {
        id,
        name,
        never_scheduled: true,
        needed_hours: 0.0,
        available_hours: 0.0,
        percent_used: 0,
        over_hours: None,
        unestimated_committed: 0,
    }
}

fn measured_row(
    id: i64,
    name: String,
    needed_minutes: i64,
    available_minutes: i64,
    unestimated_committed: usize,
) -> CapacityRow {
    let capacity = capacity_for(needed_minutes, available_minutes);
    CapacityRow {
        id,
        name,
        never_scheduled: false,
        needed_hours: capacity.needed_minutes as f64 / 60.0,
        available_hours: capacity.available_minutes as f64 / 60.0,
        percent_used: capacity.percent_used,
        over_hours: capacity.over_minutes.map(|minutes| minutes as f64 / 60.0),
        unestimated_committed,
    }
}

/// Every active life area's row, in the order [`crate::free_time::
/// free_time_by_life_area`] lists them. A never-scheduled life area
/// (`T-guardrail-well-formedness`) is reported as opted out, never as
/// zero-available-and-over -- it never reaches [`store`] at all, since
/// there is nothing to measure it against.
pub(crate) async fn rows(
    pool: &SqlitePool,
    clock: &Clock,
) -> Result<Vec<CapacityRow>, sqlx::Error> {
    let (areas, _tz) = crate::free_time::free_time_by_life_area(pool, clock).await?;
    let mut rows = Vec::with_capacity(areas.len());
    for area in areas {
        if area.pool_only {
            rows.push(never_scheduled_row(area.id, area.name));
            continue;
        }
        let available_minutes: i64 = area
            .intervals
            .iter()
            .map(|interval| interval.duration_ms() / 60_000)
            .sum();
        let (needed_minutes, unestimated_committed) = demand_minutes(pool, area.id).await?;
        rows.push(measured_row(
            area.id,
            area.name,
            needed_minutes,
            available_minutes,
            unestimated_committed,
        ));
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{seeded_life_area_id, test_pool};
    use scheduler_core::task::{DeadlineType, Priority, TaskKind};

    async fn given_a_committed_task(pool: &SqlitePool, life_area_id: i64, estimated_minutes: i64) {
        let capture_id = crate::capture::store::insert(pool, "buy milk", "web", 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            pool,
            capture_id,
            &TaskKind::Committed {
                deadline: 1787245200000,
                deadline_type: DeadlineType::Hard,
                priority: Priority::P1,
                estimated_minutes,
            },
            Some(life_area_id),
            0,
        )
        .await
        .unwrap();
    }

    /// Fitness, walled to two hours every Saturday -- 4h available over the
    /// 14-day horizon (two Saturdays), the fixture every demand-side test
    /// below measures against.
    async fn fitness_with_a_saturday_band(pool: &SqlitePool) -> i64 {
        let fitness = seeded_life_area_id(pool, "Fitness").await;
        crate::life_areas::store::insert_guardrail_band(pool, fitness, "Sat", 540, 660)
            .await
            .unwrap();
        fitness
    }

    async fn row_for(pool: &SqlitePool, name: &str) -> CapacityRow {
        rows(pool, &Clock::system())
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.name == name)
            .unwrap()
    }

    #[tokio::test]
    async fn a_life_area_with_no_guardrail_and_no_tasks_reports_zero_and_zero() {
        let (_dir, pool) = test_pool().await;

        let row = row_for(&pool, "Fitness").await;

        assert!(!row.never_scheduled);
        assert_eq!(row.needed_hours, 0.0);
        assert_eq!(row.available_hours, 0.0);
    }

    #[tokio::test]
    async fn a_committed_tasks_estimate_becomes_hours_of_demand() {
        let (_dir, pool) = test_pool().await;
        let fitness = fitness_with_a_saturday_band(&pool).await;
        given_a_committed_task(&pool, fitness, 180).await;

        let row = row_for(&pool, "Fitness").await;

        assert_eq!(row.needed_hours, 3.0);
        assert_eq!(row.available_hours, 4.0);
        assert_eq!(row.percent_used, 75);
        assert_eq!(row.over_hours, None);
    }

    #[tokio::test]
    async fn committed_and_quota_demand_are_summed_not_subtracted() {
        let (_dir, pool) = test_pool().await;
        let fitness = fitness_with_a_saturday_band(&pool).await;
        given_a_committed_task(&pool, fitness, 180).await;
        let capture_id = crate::capture::store::insert(&pool, "run", "web", 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            &pool,
            capture_id,
            &TaskKind::Quota {
                target_count: 1,
                target_minutes_each: 60,
                period: scheduler_core::task::Period::Week,
            },
            Some(fitness),
            0,
        )
        .await
        .unwrap();

        let row = row_for(&pool, "Fitness").await;

        // 180 minutes committed + 120 minutes quota (1/week x 60min over a
        // 14-day horizon) = 300 minutes = 5.0h. A demand_minutes that
        // subtracted instead of summed would report 1.0h.
        assert_eq!(row.needed_hours, 5.0);
    }

    #[tokio::test]
    async fn over_hours_is_computed_by_dividing_minutes_not_by_a_different_operator() {
        let (_dir, pool) = test_pool().await;
        let fitness = fitness_with_a_saturday_band(&pool).await;
        given_a_committed_task(&pool, fitness, 360).await;

        let row = row_for(&pool, "Fitness").await;

        // available_hours is 4.0 (two Saturdays x 2h, per the sibling test
        // above); needed_hours is 6.0, so over is 2 hours = 120 minutes.
        // 120 / 60.0 = 2.0; 120 % 60.0 = 0.0; 120 * 60.0 = 7200.0 -- the
        // three candidate mutations of the division all disagree with 2.0.
        assert_eq!(row.over_hours, Some(2.0));
    }

    #[tokio::test]
    async fn a_never_scheduled_life_area_reports_no_capacity_even_with_tasks() {
        let (_dir, pool) = test_pool().await;
        let fitness = seeded_life_area_id(&pool, "Fitness").await;
        crate::life_areas::store::set_pool_only(&pool, fitness, true)
            .await
            .unwrap();
        given_a_committed_task(&pool, fitness, 180).await;

        let row = row_for(&pool, "Fitness").await;

        assert!(row.never_scheduled);
        assert_eq!(row.needed_hours, 0.0);
        assert_eq!(row.available_hours, 0.0);
    }

    #[tokio::test]
    async fn an_unestimated_committed_task_is_not_counted_as_zero_but_is_surfaced() {
        let (_dir, pool) = test_pool().await;
        let fitness = fitness_with_a_saturday_band(&pool).await;
        let capture_id = crate::capture::store::insert(&pool, "buy milk", "web", 0)
            .await
            .unwrap();
        // Bypasses the triage boundary: the only way an unestimated
        // committed task can exist, since triage now requires the field.
        sqlx::query(
            "INSERT INTO tasks (capture_id, kind, deadline, deadline_type, priority, \
             life_area_id, created_at_ms) VALUES (?, 'committed', 1787245200000, 'hard', 'P1', ?, 0)",
        )
        .bind(capture_id)
        .bind(fitness)
        .execute(&pool)
        .await
        .unwrap();

        let row = row_for(&pool, "Fitness").await;

        assert_eq!(
            row.needed_hours, 0.0,
            "an unestimated task must not be counted as zero-cost demand \
             (it is excluded, not zeroed)"
        );
        assert_eq!(row.unestimated_committed, 1);
    }
}
