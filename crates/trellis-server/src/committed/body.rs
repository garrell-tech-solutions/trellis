//! The `#committed-body` fragment: the committed screen's own content,
//! swappable on its own -- the same shape `pool::body` and
//! `inbox::lists` take.

use crate::committed::view::{self, CommittedRowView};
use crate::platform::clock::Clock;
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

/// The `#committed-body` fragment on its own -- what marking a task done
/// swaps in.
#[derive(Template)]
#[template(path = "committed_body.html")]
pub(super) struct CommittedBodyTemplate {
    pub(super) meta: String,
    pub(super) empty: bool,
    pub(super) rows: Vec<CommittedRowView>,
}

/// Fetches the current committed list and builds the `#committed-body`
/// fragment. Shared by the committed page and [`respond`].
pub(super) async fn build(
    pool: &SqlitePool,
    clock: Clock,
) -> Result<CommittedBodyTemplate, sqlx::Error> {
    let rows = super::store::list_committed_tasks(pool).await?;
    let built = view::build(rows, clock.now_ms());
    Ok(CommittedBodyTemplate {
        meta: built.meta,
        empty: built.empty,
        rows: built.rows,
    })
}

/// The `#committed-body` fragment, re-rendered from current state --
/// `T-forms-swap-one-fragment`'s response contract, for marking a task
/// done.
pub(super) async fn respond(pool: &SqlitePool, clock: Clock) -> Result<Response, StatusCode> {
    let body = build(pool, clock).await.map_err(write_failed)?;
    Ok(render_template(StatusCode::OK, &body))
}
