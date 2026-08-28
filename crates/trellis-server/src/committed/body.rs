//! The `#committed-body` fragment: the committed screen's own content,
//! swappable on its own -- the same shape `pool::body` and
//! `inbox::lists` take.

use crate::committed::view::{self, CommittedRowView, WayBackView};
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
    pub(super) way_back: Option<WayBackView>,
}

/// [`WayBackView`] for the task `just_done` names, if any -- looked up on
/// its own through [`super::store::task_text`] rather than scanned off
/// `rows`: unlike pool, [`super::store::list_committed_tasks`] excludes an
/// archived row outright, so the task just marked done is never in that list
/// by the time this runs.
async fn way_back(
    pool: &SqlitePool,
    just_done: Option<i64>,
) -> Result<Option<WayBackView>, sqlx::Error> {
    let Some(id) = just_done else {
        return Ok(None);
    };
    Ok(super::store::task_text(pool, id)
        .await?
        .map(|text| WayBackView { id, text }))
}

/// Fetches the current committed list and builds the `#committed-body`
/// fragment. Shared by the committed page and [`respond`]. `just_done`
/// names the task this one response just marked done, if any (#111) --
/// `None` for every caller but the "done" route itself.
pub(super) async fn build(
    pool: &SqlitePool,
    clock: Clock,
    just_done: Option<i64>,
) -> Result<CommittedBodyTemplate, sqlx::Error> {
    let rows = super::store::list_committed_tasks(pool).await?;
    let zone_name = crate::settings::current_timezone(pool).await?;
    let zone = scheduler_core::timezone::resolve(&zone_name)
        .expect("settings::set_timezone validates a zone before storing it");
    let way_back = way_back(pool, just_done).await?;
    let built = view::build(rows, clock.now_ms(), &zone);
    Ok(CommittedBodyTemplate {
        meta: built.meta,
        empty: built.empty,
        rows: built.rows,
        way_back,
    })
}

/// The `#committed-body` fragment, re-rendered from current state --
/// `T-forms-swap-one-fragment`'s response contract, for marking a task
/// done.
pub(super) async fn respond(
    pool: &SqlitePool,
    clock: Clock,
    just_done: Option<i64>,
) -> Result<Response, StatusCode> {
    let body = build(pool, clock, just_done).await.map_err(write_failed)?;
    Ok(render_template(StatusCode::OK, &body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{given_a_committed_task, test_pool};

    #[tokio::test]
    async fn just_done_none_carries_no_way_back() {
        let (_dir, pool) = test_pool().await;
        given_a_committed_task(&pool, "book the dentist").await;

        let built = build(&pool, Clock::pinned_at(0), None).await.unwrap();

        assert!(built.way_back.is_none());
    }

    #[tokio::test]
    async fn just_done_names_the_tasks_own_text() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_committed_task(&pool, "book the dentist").await;
        crate::mark_done::mark_task_done(&pool, task_id, 1)
            .await
            .unwrap();

        let built = build(&pool, Clock::pinned_at(0), Some(task_id))
            .await
            .unwrap();

        let way_back = built.way_back.unwrap();
        assert_eq!(way_back.id, task_id);
        assert_eq!(way_back.text, "book the dentist");
    }

    #[tokio::test]
    async fn an_unknown_just_done_id_carries_no_way_back() {
        let (_dir, pool) = test_pool().await;
        given_a_committed_task(&pool, "book the dentist").await;

        let built = build(&pool, Clock::pinned_at(0), Some(999)).await.unwrap();

        assert!(built.way_back.is_none());
    }
}
