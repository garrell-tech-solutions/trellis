//! The `#quota-body` fragment: the quota screen's own content, swappable on
//! its own -- the same shape `pool::body` and `committed::body` take.

use crate::platform::clock::Clock;
use crate::platform::response::{render_template, write_failed};
use crate::quota::view::{self, QuotaRowView};
use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;
use std::collections::HashSet;

/// The `#quota-body` fragment on its own -- what every session write swaps
/// in (#138: defining a quota is not a route this screen offers any more).
#[derive(Template)]
#[template(path = "quota_body.html")]
pub(super) struct QuotaBodyTemplate {
    pub(super) meta: String,
    pub(super) empty: bool,
    pub(super) quotas: Vec<QuotaRowView>,
    pub(super) day_options: Vec<String>,
    pub(super) today: String,
}

/// The owner's current week, resolved the way `committed/body.rs` resolves
/// "now" (`T-timezone-is-a-setting`): never UTC, never a second notion of
/// the zone. Shared by [`build`] and every session route in
/// `crate::quota::http`, since logging, correcting and deleting a session
/// all need to know which days this week allows
/// (`quota-sessions-only-days-that-have-happened-03`).
pub(super) async fn current_week(
    pool: &SqlitePool,
    clock: Clock,
) -> Result<(scheduler_core::quota::Week, jiff::tz::TimeZone), sqlx::Error> {
    let zone_name = crate::settings::current_timezone(pool).await?;
    let zone = scheduler_core::timezone::resolve(&zone_name)
        .expect("settings::set_timezone validates a zone before storing it");
    let week = scheduler_core::quota::Week::of(clock.now_ms(), &zone);
    Ok((week, zone))
}

/// Fetches the current quota list, this week's own sessions and builds the
/// `#quota-body` fragment. Shared by the quota page and [`respond`] (a
/// session write, accepted or rejected).
pub(super) async fn build(
    pool: &SqlitePool,
    clock: Clock,
    expanded_ids: &HashSet<i64>,
) -> Result<QuotaBodyTemplate, sqlx::Error> {
    let rows = super::store::list_quotas(pool).await?;
    let (week, zone) = current_week(pool, clock).await?;
    let (week_start_ms, week_end_ms) = week.bounds_ms(&zone);
    let sessions = super::store::week_sessions(pool, week_start_ms, week_end_ms).await?;
    let built = view::build(rows, sessions, &week, &zone, expanded_ids);
    Ok(QuotaBodyTemplate {
        meta: built.meta,
        empty: built.empty,
        quotas: built.quotas,
        day_options: built.day_options,
        today: built.today,
    })
}

/// The `#quota-body` fragment, re-rendered from current state --
/// `T-forms-swap-one-fragment`'s response contract -- at `status`, which is
/// `200`/`201` for an accepted session write and `422`
/// (`T-422-is-product-wide`) for a rejected one.
pub(super) async fn respond(
    pool: &SqlitePool,
    clock: Clock,
    status: StatusCode,
    expanded_ids: &HashSet<i64>,
) -> Result<Response, StatusCode> {
    let body = build(pool, clock, expanded_ids)
        .await
        .map_err(write_failed)?;
    Ok(render_template(status, &body))
}
