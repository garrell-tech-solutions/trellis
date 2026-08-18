//! `GET /free-time`.
//!
//! Translation only, the same shape `stats::http` is: ask [`super::
//! free_time_by_life_area`] what each life area's guardrail projects to
//! over the horizon minus whatever exceptions apply to it, render what it
//! says. No arithmetic of its own beyond formatting -- summing durations
//! and reading civil fields off an already-resolved `Zoned` is not a rule
//! that survives changing HTTP, it is display.
//!
//! Also renders the exceptions list and its add form (#61): the
//! specifier's call was that the exception controls live here rather than
//! on a page of their own, since this is the only page whose numbers they
//! change. `exceptions::http` owns the writes; this handler only reads
//! `exceptions::list` back (`T-one-front-door-per-capability`).

use crate::exceptions::view::ExceptionListItem;
use crate::life_areas::view::LifeAreaOption;
use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use jiff::tz::TimeZone;
use jiff::Timestamp;
use scheduler_core::free_time::Interval;
use sqlx::SqlitePool;

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
    exceptions: Vec<ExceptionListItem>,
    exception_error: Option<String>,
    life_area_options: Vec<LifeAreaOption>,
    nav: Vec<NavLink>,
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
    area: super::LifeAreaFreeTime,
    tz: &TimeZone,
) -> Result<FreeTimeArea, StatusCode> {
    let total_ms: i64 = area.intervals.iter().map(|i| i.duration_ms()).sum();
    let labels = area
        .intervals
        .iter()
        .map(|interval| format_interval(tz, *interval))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FreeTimeArea {
        id: area.id,
        name: area.name,
        total_hours: total_ms / 3_600_000,
        intervals: labels,
    })
}

pub async fn show_free_time(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
) -> Result<Response, StatusCode> {
    let (areas, tz) = super::free_time_by_life_area(&pool, &clock)
        .await
        .map_err(write_failed)?;
    let life_areas = areas
        .into_iter()
        .map(|area| free_time_area(area, &tz))
        .collect::<Result<Vec<_>, _>>()?;
    let exceptions = crate::exceptions::list(&pool).await.map_err(write_failed)?;
    let life_area_options = crate::life_areas::active_options(&pool)
        .await
        .map_err(write_failed)?;

    Ok(render_template(
        StatusCode::OK,
        &FreeTimeTemplate {
            life_areas,
            exceptions,
            exception_error: None,
            life_area_options,
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
