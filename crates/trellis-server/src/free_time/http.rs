//! `GET /free-time`.
//!
//! Translation only, the same shape `stats::http` is: read the clock and
//! the owner's zone, ask `scheduler_core::free_time::free_intervals` what
//! each life area's guardrail projects to over the horizon, render what it
//! says. No arithmetic of its own beyond formatting -- summing durations
//! and reading civil fields off an already-resolved `Zoned` is not a rule
//! that survives changing HTTP, it is display.

use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use jiff::tz::TimeZone;
use jiff::Timestamp;
use scheduler_core::free_time::{free_intervals, Guardrail, Interval, Range};
use sqlx::SqlitePool;

/// The look-ahead this page reports over. Its own constant, not `stats::
/// WINDOW_MS`: the two fourteens are the same number by coincidence, not by
/// rule -- one is a retrospective instant window over past triage
/// timestamps, this is a prospective civil-date horizon over a guardrail,
/// and forcing them to share a constant would tie two unrelated views
/// together for no benefit (open question 2, `free-time` brief, #60).
const HORIZON_DAYS: i64 = 14;

struct FreeTimeArea {
    id: i64,
    name: String,
    total_hours: i64,
    intervals: Vec<String>,
}

#[derive(Template)]
#[template(path = "free_time.html")]
struct FreeTimeTemplate {
    life_areas: Vec<FreeTimeArea>,
    nav: Vec<NavLink>,
}

/// The civil date "now" falls on, in `tz` -- what "the next fourteen days"
/// starts counting from.
fn today_in(now_ms: i64, tz: &TimeZone) -> Result<jiff::civil::Date, StatusCode> {
    let now = Timestamp::from_millisecond(now_ms).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(now.to_zoned(tz.clone()).date())
}

/// One interval as the page shows it: the calendar date it falls on and its
/// wall-clock span in `tz` -- civil again, the way the owner authored the
/// band that produced it, not the UTC instant it is stored as.
fn format_interval(tz: &TimeZone, interval: Interval) -> Result<String, StatusCode> {
    let start = Timestamp::from_millisecond(interval.start_ms)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_zoned(tz.clone());
    let end = Timestamp::from_millisecond(interval.end_ms)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_zoned(tz.clone());
    Ok(format!(
        "{} {:02}:{:02}-{:02}:{:02}",
        start.date(),
        start.hour(),
        start.minute(),
        end.hour(),
        end.minute(),
    ))
}

fn free_time_area(
    id: i64,
    name: String,
    bands: &[scheduler_core::guardrail::Band],
    tz: &TimeZone,
    range: Range,
) -> Result<FreeTimeArea, StatusCode> {
    let guardrail = Guardrail {
        bands,
        timezone: tz,
    };
    let intervals = free_intervals(guardrail, range);
    let total_ms: i64 = intervals.iter().map(|i| i.duration_ms()).sum();
    let labels = intervals
        .iter()
        .map(|interval| format_interval(tz, *interval))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FreeTimeArea {
        id,
        name,
        total_hours: total_ms / 3_600_000,
        intervals: labels,
    })
}

/// The owner's zone and the civil-date horizon to project it over --
/// [`show_free_time`]'s setup half, ahead of the per-life-area projection.
async fn today_range(pool: &SqlitePool, clock: &Clock) -> Result<(TimeZone, Range), StatusCode> {
    let zone_name = crate::settings::current_timezone(pool)
        .await
        .map_err(write_failed)?;
    let tz = scheduler_core::timezone::resolve(&zone_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let today = today_in(clock.now_ms(), &tz)?;
    Ok((tz.clone(), Range::horizon(today, HORIZON_DAYS)))
}

async fn free_time_areas(
    pool: &SqlitePool,
    tz: &TimeZone,
    range: Range,
) -> Result<Vec<FreeTimeArea>, StatusCode> {
    let guardrails = crate::life_areas::guardrails(pool)
        .await
        .map_err(write_failed)?;
    let mut life_areas = Vec::with_capacity(guardrails.len());
    for area in guardrails {
        life_areas.push(free_time_area(area.id, area.name, &area.bands, tz, range)?);
    }
    Ok(life_areas)
}

pub async fn show_free_time(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
) -> Result<Response, StatusCode> {
    let (tz, range) = today_range(&pool, &clock).await?;
    let life_areas = free_time_areas(&pool, &tz, range).await?;

    Ok(render_template(
        StatusCode::OK,
        &FreeTimeTemplate {
            life_areas,
            nav: nav::links(Page::FreeTime),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{get_ok, test_pool};

    async fn get_free_time(pool: &SqlitePool) -> String {
        get_ok(pool, Clock::system(), "/free-time").await
    }

    #[tokio::test]
    async fn a_fresh_database_lists_every_seeded_life_area() {
        let (_dir, pool) = test_pool().await;

        let body = get_free_time(&pool).await;

        for expected in ["Work", "Fitness", "Learning", "Family", "Home"] {
            assert!(body.contains(expected), "expected {expected} in:\n{body}");
        }
    }

    #[tokio::test]
    async fn a_guardrail_band_reports_its_projected_free_time() {
        let (_dir, pool) = test_pool().await;
        let id = crate::life_areas::store::find_by_name(&pool, "Work")
            .await
            .unwrap()
            .unwrap()
            .id;
        // 2026-01-05 is a Monday, at midnight UTC -- "now" falls exactly on
        // the civil date the Monday band below projects into. The 14-day
        // horizon starting there covers two Mondays: 2026-01-05 and
        // 2026-01-12.
        crate::life_areas::store::insert_guardrail_band(&pool, id, "Mon", 540, 1020)
            .await
            .unwrap();

        let body = get_ok(&pool, Clock::pinned_at(1_767_571_200_000), "/free-time").await;

        assert!(body.contains("16h"), "got:\n{body}");
        assert!(body.contains("2026-01-05 09:00-17:00"), "got:\n{body}");
        assert!(body.contains("2026-01-12 09:00-17:00"), "got:\n{body}");
    }

    #[tokio::test]
    async fn a_life_area_with_no_guardrail_reports_zero_hours() {
        let (_dir, pool) = test_pool().await;

        let body = get_free_time(&pool).await;

        assert!(body.contains("Work") && body.contains("0h"), "got:\n{body}");
    }

    #[tokio::test]
    async fn hostile_text_in_a_life_area_name_is_escaped() {
        let (_dir, pool) = test_pool().await;
        crate::life_areas::store::insert(&pool, "<script>alert('boom')</script>")
            .await
            .unwrap();

        let body = get_free_time(&pool).await;

        assert!(!body.contains("<script>"), "got:\n{body}");
        assert!(body.contains("boom"), "got:\n{body}");
    }
}
