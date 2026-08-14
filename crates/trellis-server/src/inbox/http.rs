//! `GET /`: the untriaged capture queue and task list (D-visible-slices'
//! first two slices, issues #30 and #33).

use crate::inbox::lists::build_lists;
use crate::inbox::view::{CaptureRow, TaskRow};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

#[derive(Template)]
#[template(path = "inbox.html")]
struct InboxTemplate {
    captures: Vec<CaptureRow>,
    tasks: Vec<TaskRow>,
}

pub async fn show_inbox(State(pool): State<SqlitePool>) -> Result<Response, StatusCode> {
    let (captures, tasks) = build_lists(&pool, None).await.map_err(write_failed)?;
    Ok(render_template(
        StatusCode::OK,
        &InboxTemplate { captures, tasks },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::to_bytes;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn get_inbox(pool: &SqlitePool) -> String {
        let app = crate::platform::app::build_app(pool.clone());
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    async fn insert_untriaged_capture(pool: &SqlitePool, raw_text: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, 'web', 0) RETURNING id",
        )
        .bind(raw_text)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn the_inbox_shows_an_empty_state_message_when_there_is_nothing_to_triage() {
        let (_dir, pool) = test_pool().await;

        let body = get_inbox(&pool).await;

        assert!(
            body.contains("Nothing to triage"),
            "expected an empty-state message, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn the_inbox_lists_untriaged_captures_newest_first() {
        let (_dir, pool) = test_pool().await;
        insert_untriaged_capture(&pool, "call the dentist").await;
        insert_untriaged_capture(&pool, "buy milk").await;

        let body = get_inbox(&pool).await;

        let newer = body.find("buy milk").expect("buy milk should be listed");
        let older = body
            .find("call the dentist")
            .expect("call the dentist should be listed");
        assert!(newer < older, "expected buy milk (newest) to list first");
    }

    #[tokio::test]
    async fn the_inbox_excludes_a_triaged_capture() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;
        sqlx::query("UPDATE captures SET triaged_at = 1 WHERE id = ?")
            .bind(capture_id)
            .execute(&pool)
            .await
            .unwrap();

        let body = get_inbox(&pool).await;

        assert!(
            !body.contains("call the dentist"),
            "a triaged capture must not appear in the inbox, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn the_inbox_escapes_hostile_capture_text() {
        let (_dir, pool) = test_pool().await;
        insert_untriaged_capture(&pool, "<script>alert('boom')</script>").await;

        let body = get_inbox(&pool).await;

        assert!(
            !body.contains("<script>"),
            "an unescaped <script> tag must not appear in the response, got:\n{body}"
        );
        assert!(
            body.contains("boom"),
            "the capture's text must survive, escaped, got:\n{body}"
        );
    }
}
