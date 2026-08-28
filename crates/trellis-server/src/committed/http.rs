//! `GET /committed`. `POST /committed/tasks/{id}/done`: marks a committed
//! task done (#97) and swaps in the `#committed-body` fragment.

use super::body;
use crate::committed::view::{CommittedRowView, WayBackView};
use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

/// The full page is the only thing that is not the `#committed-body`
/// fragment, so it is the only caller that takes `body::build`'s fields
/// apart instead of going through `body::respond` -- the same shape
/// `pool::http::PoolTemplate` takes.
#[derive(Template)]
#[template(path = "committed.html")]
struct CommittedTemplate {
    meta: String,
    empty: bool,
    rows: Vec<CommittedRowView>,
    /// Always `None` -- see `pool::http::PoolTemplate`'s identical field
    /// (#111): a fresh `GET /committed` never just did anything.
    way_back: Option<WayBackView>,
    nav: Vec<NavLink>,
}

pub async fn show_committed(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
) -> Result<Response, StatusCode> {
    let built = body::build(&pool, clock, None)
        .await
        .map_err(write_failed)?;
    Ok(render_template(
        StatusCode::OK,
        &CommittedTemplate {
            meta: built.meta,
            empty: built.empty,
            rows: built.rows,
            way_back: built.way_back,
            nav: nav::links(Page::Committed),
        },
    ))
}

/// Marks `task_id` done and swaps in the current `#committed-body`
/// fragment, regardless of whether it had already been marked -- see
/// `pool::http::mark_pool_task_done`'s identical reasoning, `task_id` as
/// this response's own `just_done` included (#111).
pub async fn mark_committed_task_done(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(task_id): Path<i64>,
) -> Result<Response, StatusCode> {
    crate::mark_done::mark_task_done(&pool, task_id, clock.now_ms())
        .await
        .map_err(write_failed)?;
    body::respond(&pool, clock, Some(task_id)).await
}

/// Unchecks `task_id` (#111, the committed row's own counterpart to
/// `pool::http::unmark_pool_task_done`, previously missing entirely -- see
/// that function's identical reasoning) and swaps in the current
/// `#committed-body` fragment. Carries no way back of its own: an undo is
/// not itself something to undo.
pub async fn unmark_committed_task_done(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(task_id): Path<i64>,
) -> Result<Response, StatusCode> {
    crate::mark_done::unmark_task_done(&pool, task_id)
        .await
        .map_err(write_failed)?;
    body::respond(&pool, clock, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{
        archived_at, given_a_committed_task, http_request, test_pool,
    };

    async fn get_committed(pool: &SqlitePool) -> (StatusCode, String) {
        http_request(pool, Clock::pinned_at(1787562000000), "GET", "/committed").await
    }

    #[tokio::test]
    async fn the_committed_screen_is_reachable_and_shows_the_empty_state_when_nothing_is_dated() {
        let (_dir, pool) = test_pool().await;

        let (status, body) = get_committed(&pool).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains("Nothing with a time on it. That is allowed."),
            "got:\n{body}"
        );
    }

    #[tokio::test]
    async fn the_committed_screen_marks_committed_as_the_current_tab() {
        let (_dir, pool) = test_pool().await;

        let (_, body) = get_committed(&pool).await;

        assert!(
            body.contains(r#"aria-current="page">Committed</a>"#),
            "got:\n{body}"
        );
    }

    #[tokio::test]
    async fn the_committed_screen_shows_a_row_for_a_committed_task() {
        let (_dir, pool) = test_pool().await;
        given_a_committed_task(&pool, "book the dentist").await;

        let (_, body) = get_committed(&pool).await;

        assert!(body.contains("book the dentist"), "got:\n{body}");
    }

    async fn post_mark_done(pool: &SqlitePool, task_id: i64) -> (StatusCode, String) {
        http_request(
            pool,
            Clock::pinned_at(1787562000000),
            "POST",
            &format!("/committed/tasks/{task_id}/done"),
        )
        .await
    }

    #[tokio::test]
    async fn marking_a_committed_task_done_removes_it_from_the_next_render() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_committed_task(&pool, "book the dentist").await;

        let (status, body) = post_mark_done(&pool, task_id).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            !body.contains(r#"<div class="committed-text">book the dentist</div>"#),
            "expected the row gone, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn marking_a_committed_task_done_names_it_in_the_way_back() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_committed_task(&pool, "book the dentist").await;

        let (_, body) = post_mark_done(&pool, task_id).await;

        assert!(
            body.contains(r#"<span class="way-back-name">book the dentist</span>"#),
            "got:\n{body}"
        );
    }

    #[tokio::test]
    async fn marking_a_committed_task_done_still_renders_a_different_remaining_task() {
        let (_dir, pool) = test_pool().await;
        let done_id = given_a_committed_task(&pool, "book the dentist").await;
        given_a_committed_task(&pool, "renew the passport").await;

        let (status, body) = post_mark_done(&pool, done_id).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("renew the passport"), "got:\n{body}");
    }

    #[tokio::test]
    async fn marking_a_committed_task_done_stamps_the_row_rather_than_deleting_it() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_committed_task(&pool, "book the dentist").await;

        post_mark_done(&pool, task_id).await;

        assert!(
            archived_at(&pool, task_id).await.is_some(),
            "expected archived_at to be stamped, got None"
        );
    }

    async fn post_undone(pool: &SqlitePool, task_id: i64) -> (StatusCode, String) {
        http_request(
            pool,
            Clock::pinned_at(1787562000000),
            "POST",
            &format!("/committed/tasks/{task_id}/undone"),
        )
        .await
    }

    #[tokio::test]
    async fn taking_the_way_back_returns_the_row_to_the_next_render() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_committed_task(&pool, "book the dentist").await;
        post_mark_done(&pool, task_id).await;

        let (status, body) = post_undone(&pool, task_id).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains(r#"<div class="committed-text">book the dentist</div>"#),
            "got:\n{body}"
        );
    }

    #[tokio::test]
    async fn taking_the_way_back_clears_archived_at() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_committed_task(&pool, "book the dentist").await;
        post_mark_done(&pool, task_id).await;

        post_undone(&pool, task_id).await;

        assert_eq!(archived_at(&pool, task_id).await, None);
    }

    #[tokio::test]
    async fn taking_the_way_back_carries_no_way_back_of_its_own() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_committed_task(&pool, "book the dentist").await;
        post_mark_done(&pool, task_id).await;

        let (_, body) = post_undone(&pool, task_id).await;

        assert!(
            !body.contains("way-back-name"),
            "an undo is not itself something to undo, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn taking_the_way_back_twice_changes_nothing_the_second_time() {
        let (_dir, pool) = test_pool().await;
        let task_id = given_a_committed_task(&pool, "book the dentist").await;
        post_mark_done(&pool, task_id).await;
        post_undone(&pool, task_id).await;

        let (status, _) = post_undone(&pool, task_id).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(archived_at(&pool, task_id).await, None);
    }
}
