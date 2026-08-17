//! `POST /captures/{id}/dismiss`.
//!
//! The inbox's "no": the capture leaves the untriaged queue, no task is
//! created, and the row stays. Follows `T-forms-swap-one-fragment` exactly
//! as triage does — whatever happened, the response is the current `#lists`
//! fragment, `422` on rejection.

use crate::dismiss::store;
use crate::inbox::lists::{build_lists, ListsTemplate};
use crate::platform::clock::Clock;
use crate::platform::response::{render_template, write_failed};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

/// The same prose `triage::http` reports when a capture it was asked to
/// triage has already left the inbox -- one fact, worded once, told from
/// whichever side asked.
const CAPTURE_NOT_OPEN_MESSAGE: &str = "the capture is no longer in the inbox";

pub async fn dismiss_capture(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(capture_id): Path<i64>,
) -> Result<Response, StatusCode> {
    let open = store::capture_is_open(&pool, capture_id)
        .await
        .map_err(write_failed)?;
    let (status, error) = if open {
        store::mark_dismissed(&pool, capture_id, clock.now_ms())
            .await
            .map_err(write_failed)?;
        (StatusCode::OK, None)
    } else {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some((capture_id, CAPTURE_NOT_OPEN_MESSAGE.to_string())),
        )
    };
    let (captures, tasks, life_areas) = build_lists(&pool, error).await.map_err(write_failed)?;
    Ok(render_template(
        status,
        &ListsTemplate {
            captures,
            tasks,
            life_areas,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn insert_untriaged_capture(pool: &SqlitePool, raw_text: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, 'web', 0) RETURNING id",
        )
        .bind(raw_text)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn dismiss_response(pool: &SqlitePool, capture_id: i64) -> Response {
        let app = crate::platform::app::build_app(pool.clone(), Clock::system());
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/captures/{capture_id}/dismiss"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn body_string(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn dismissing_an_untriaged_capture_is_not_a_redirect_and_removes_it_from_the_list() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "asdfgh").await;

        let response = dismiss_response(&pool, capture_id).await;

        assert!(!response.status().is_redirection());
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(!body.contains("asdfgh"), "got:\n{body}");
    }

    #[tokio::test]
    async fn dismissing_a_capture_creates_no_task() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "asdfgh").await;

        dismiss_response(&pool, capture_id).await;

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 0);
    }

    #[tokio::test]
    async fn dismissing_a_capture_does_not_delete_its_row() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "asdfgh").await;

        dismiss_response(&pool, capture_id).await;

        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM captures WHERE id = ?")
            .bind(capture_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn dismissing_an_already_dismissed_capture_is_rejected() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "asdfgh").await;
        dismiss_response(&pool, capture_id).await;

        let response = dismiss_response(&pool, capture_id).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM captures WHERE id = ?")
            .bind(capture_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row_count, 1, "a rejected dismissal must not delete the row");
    }

    /// The rejection message names the reason (`CAPTURE_NOT_OPEN_MESSAGE`) --
    /// exercised directly rather than through a rendered response, because a
    /// capture already gone from the inbox has no row left in `#lists` to
    /// carry it, the same way a triage rejection's per-row message has
    /// nothing to attach to once its row is gone (`inbox::lists::build_lists`
    /// only attaches an error to a capture still in `list_untriaged`).
    #[test]
    fn the_not_open_message_names_why() {
        assert_eq!(
            CAPTURE_NOT_OPEN_MESSAGE,
            "the capture is no longer in the inbox"
        );
    }

    #[tokio::test]
    async fn dismissing_a_triaged_capture_is_rejected_and_leaves_the_task() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;
        crate::triage::store::insert_task(
            &pool,
            capture_id,
            &scheduler_core::task::TaskKind::Pool,
            None,
            0,
        )
        .await
        .unwrap();
        crate::triage::store::mark_triaged(&pool, capture_id, 0)
            .await
            .unwrap();

        let response = dismiss_response(&pool, capture_id).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 1);
    }

    #[tokio::test]
    async fn dismissing_a_capture_that_does_not_exist_is_rejected() {
        let (_dir, pool) = test_pool().await;

        let response = dismiss_response(&pool, 999).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn dismissing_hostile_capture_text_stays_escaped_in_the_response() {
        let (_dir, pool) = test_pool().await;
        insert_untriaged_capture(&pool, "<script>alert('boom')</script>").await;
        let asdfgh_id = insert_untriaged_capture(&pool, "asdfgh").await;

        let response = dismiss_response(&pool, asdfgh_id).await;

        let body = body_string(response).await;
        assert!(!body.contains("<script>"), "got:\n{body}");
        assert!(body.contains("boom"), "got:\n{body}");
    }
}
