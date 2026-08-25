//! The `#pool-body` fragment: the pool screen's own content, swappable on
//! its own.
//!
//! Two endpoints render it -- `GET /pool` wraps it in the full page, and
//! marking a task done swaps it in on its own, the same shape
//! `inbox::lists` established for `#lists`.

use crate::platform::response::{render_template, write_failed};
use crate::pool::store;
use crate::pool::view::{self, LooseItemView, TripView};
use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;
use std::collections::HashSet;

/// The `#pool-body` fragment on its own -- what marking a task done swaps
/// in. Kept separate from the full-page template for the same reason
/// `inbox::lists::ListsTemplate` is: the response to a mark-done POST is
/// not a page.
#[derive(Template)]
#[template(path = "pool_body.html")]
pub(super) struct PoolBodyTemplate {
    pub(super) meta: String,
    pub(super) empty: bool,
    pub(super) trips: Vec<TripView>,
    pub(super) loose: Vec<LooseItemView>,
}

/// Fetches the current pool and builds the `#pool-body` fragment. Shared by
/// the pool page and [`respond`]: both need the identical view, built the
/// identical way. `expanded_tags` names which trips render already expanded
/// (#120) -- ridden along on this one request, never stored.
pub(super) async fn build(
    pool: &SqlitePool,
    expanded_tags: &HashSet<String>,
) -> Result<PoolBodyTemplate, sqlx::Error> {
    let rows = store::list_pool_tasks(pool).await?;
    let run_sizes = store::run_member_counts(pool).await?;
    let built = view::build(rows, run_sizes, expanded_tags);
    Ok(PoolBodyTemplate {
        meta: built.meta,
        empty: built.empty,
        trips: built.trips,
        loose: built.loose,
    })
}

/// The `#pool-body` fragment, re-rendered from current state --
/// `T-forms-swap-one-fragment`'s response contract, for marking a task
/// done.
pub(super) async fn respond(
    pool: &SqlitePool,
    expanded_tags: &HashSet<String>,
) -> Result<Response, StatusCode> {
    let body = build(pool, expanded_tags).await.map_err(write_failed)?;
    Ok(render_template(StatusCode::OK, &body))
}
