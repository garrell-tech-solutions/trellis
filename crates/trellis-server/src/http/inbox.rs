//! `GET /`: the untriaged capture queue and task list (D-visible-slices'
//! first two slices, issues #30 and #33).

use crate::http::view::{CaptureRow, TaskRow};
use crate::http::{render_template, write_failed};
use crate::store;
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

/// The `#lists` fragment on its own — what a page-originated triage response
/// swaps in. Kept separate from [`InboxTemplate`] (rather than making the
/// full page template itself the triage response) because a triage response
/// is not a page: it has no `<head>`, no quick-add form, nothing but the two
/// lists htmx is replacing.
#[derive(Template)]
#[template(path = "lists.html")]
pub(crate) struct ListsTemplate {
    pub(crate) captures: Vec<CaptureRow>,
    pub(crate) tasks: Vec<TaskRow>,
}

pub async fn show_inbox(State(pool): State<SqlitePool>) -> Result<Response, StatusCode> {
    let (captures, tasks) = build_lists(&pool, None).await.map_err(write_failed)?;
    Ok(render_template(
        StatusCode::OK,
        &InboxTemplate { captures, tasks },
    ))
}

/// Fetches the current inbox and task list, attaching `error` to whichever
/// capture's triage attempt just failed (if any). Shared by [`show_inbox`]
/// and the triage handler's page-originated response: "the page and
/// `POST /captures/{id}/triage` are one code path" extends to what gets
/// rendered afterward, not just to how the write itself happens.
pub(crate) async fn build_lists(
    pool: &SqlitePool,
    error: Option<(i64, String)>,
) -> Result<(Vec<CaptureRow>, Vec<TaskRow>), sqlx::Error> {
    let captures = store::capture::list_untriaged(pool)
        .await?
        .into_iter()
        .map(|capture| CaptureRow {
            id: capture.id,
            error: error
                .as_ref()
                .filter(|(id, _)| *id == capture.id)
                .map(|(_, message)| message.clone()),
            text: capture.raw_text,
        })
        .collect();
    let tasks = store::task::list_all(pool)
        .await?
        .into_iter()
        .map(|task| TaskRow {
            kind: task.kind,
            text: task.raw_text,
        })
        .collect();
    Ok((captures, tasks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_pool;
    use axum::body::to_bytes;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn get_inbox(pool: &SqlitePool) -> String {
        let app = crate::app::build_app(pool.clone());
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
