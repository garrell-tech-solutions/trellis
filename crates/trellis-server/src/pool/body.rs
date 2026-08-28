//! The `#pool-body` fragment: the pool screen's own content, swappable on
//! its own.
//!
//! Two endpoints render it -- `GET /pool` wraps it in the full page, and
//! marking a task done swaps it in on its own, the same shape
//! `inbox::lists` established for `#lists`.

use crate::platform::response::{render_template, write_failed};
use crate::pool::store;
use crate::pool::view::{self, LooseItemView, TripView, WayBackView};
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
    pub(super) way_back: Option<WayBackView>,
}

/// Fetches the current pool and builds the `#pool-body` fragment. Shared by
/// the pool page and [`respond`]: both need the identical view, built the
/// identical way. `expanded_tags` names which trips render already expanded
/// (#120) -- ridden along on this one request, never stored. `just_done`
/// names the task this one response just marked done, if any (#111) -- `None`
/// for every caller but the "done" route itself, so a fresh `GET /pool` or an
/// undo never manufactures a way back for something this request did not do.
///
/// The text comes off `rows`, not a second query: a task just marked done is
/// still in this list -- only a *cleared* one leaves it -- so the row
/// carrying `just_done`'s id already has the capture text this line needs.
pub(super) async fn build(
    pool: &SqlitePool,
    expanded_tags: &HashSet<String>,
    just_done: Option<i64>,
) -> Result<PoolBodyTemplate, sqlx::Error> {
    let rows = store::list_pool_tasks(pool).await?;
    let run_sizes = store::run_member_counts(pool).await?;
    let way_back = just_done.and_then(|id| {
        rows.iter()
            .find(|row| row.task_id == id)
            .map(|row| WayBackView {
                id,
                text: row.raw_text.clone(),
            })
    });
    let built = view::build(rows, run_sizes, expanded_tags);
    Ok(PoolBodyTemplate {
        meta: built.meta,
        empty: built.empty,
        trips: built.trips,
        loose: built.loose,
        way_back,
    })
}

/// The `#pool-body` fragment, re-rendered from current state --
/// `T-forms-swap-one-fragment`'s response contract, for marking a task
/// done.
pub(super) async fn respond(
    pool: &SqlitePool,
    expanded_tags: &HashSet<String>,
    just_done: Option<i64>,
) -> Result<Response, StatusCode> {
    let body = build(pool, expanded_tags, just_done)
        .await
        .map_err(write_failed)?;
    Ok(render_template(StatusCode::OK, &body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    async fn given_a_pool_task(pool: &SqlitePool, raw_text: &str) -> i64 {
        let capture_id = crate::platform::test_support::insert_capture(pool, raw_text, None).await;
        crate::triage::store::insert_task(
            pool,
            capture_id,
            &scheduler_core::task::TaskKind::Pool,
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

    #[tokio::test]
    async fn just_done_none_carries_no_way_back() {
        let (_dir, pool) = test_pool().await;
        given_a_pool_task(&pool, "buy screws").await;

        let built = build(&pool, &HashSet::new(), None).await.unwrap();

        assert!(built.way_back.is_none());
    }

    #[tokio::test]
    async fn just_done_names_the_tasks_own_text() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_pool_task(&pool, "buy screws").await;
        crate::mark_done::mark_task_done(&pool, task_id, 1)
            .await
            .unwrap();

        let built = build(&pool, &HashSet::new(), Some(task_id)).await.unwrap();

        let way_back = built.way_back.unwrap();
        assert_eq!(way_back.id, task_id);
        assert_eq!(way_back.text, "buy screws");
    }

    #[tokio::test]
    async fn an_unknown_just_done_id_carries_no_way_back() {
        let (_dir, pool) = test_pool().await;
        given_a_pool_task(&pool, "buy screws").await;

        let built = build(&pool, &HashSet::new(), Some(999)).await.unwrap();

        assert!(built.way_back.is_none());
    }
}
