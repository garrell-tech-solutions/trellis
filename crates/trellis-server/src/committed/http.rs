//! `GET /committed`. `POST /committed/tasks/{id}/done`: marks a committed
//! task done (#97) and swaps in the `#committed-body` fragment.

use super::body;
use crate::committed::view::CommittedRowView;
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
    nav: Vec<NavLink>,
}

pub async fn show_committed(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
) -> Result<Response, StatusCode> {
    let built = body::build(&pool, clock).await.map_err(write_failed)?;
    Ok(render_template(
        StatusCode::OK,
        &CommittedTemplate {
            meta: built.meta,
            empty: built.empty,
            rows: built.rows,
            nav: nav::links(Page::Committed),
        },
    ))
}

/// Marks `task_id` done and swaps in the current `#committed-body`
/// fragment, regardless of whether it had already been marked -- see
/// `pool::http::mark_pool_task_done`'s identical reasoning.
pub async fn mark_committed_task_done(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(task_id): Path<i64>,
) -> Result<Response, StatusCode> {
    crate::mark_done::mark_task_done(&pool, task_id, clock.now_ms())
        .await
        .map_err(write_failed)?;
    body::respond(&pool, clock).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{archived_at, http_request, test_pool};

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

    async fn given_a_committed_task(pool: &SqlitePool, raw_text: &str) -> i64 {
        let (capture_id, _) = crate::capture::create(pool, raw_text, "web", None, 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            pool,
            capture_id,
            &scheduler_core::task::TaskKind::Committed {
                deadline: 1787646600000,
                commitment: scheduler_core::task::Commitment::At,
                priority: scheduler_core::task::Priority::P1,
                estimated_minutes: 30,
            },
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
        assert!(!body.contains("book the dentist"), "got:\n{body}");
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
}
