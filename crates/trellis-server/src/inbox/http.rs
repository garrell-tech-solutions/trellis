//! `GET /`: the quick-add box and `Recent` — what is waiting to be triaged,
//! and the last few that were (#30, #33, #140).
//!
//! `POST /captures/{id}/kind`: which kind's fields panel a still-untriaged
//! row is showing (#119) -- a display preference the inbox owns the same
//! way it owns membership, never read by triage validation.

use super::lists;
use super::lists::build_lists;
use super::shown_kind::ShownKind;
use super::CAPTURE_NOT_OPEN_MESSAGE;
use crate::inbox::view::CaptureRow;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

#[derive(Template)]
#[template(path = "inbox.html")]
struct InboxTemplate {
    captures: Vec<CaptureRow>,
    context_tag_suggestions: Vec<String>,
    nav: Vec<NavLink>,
}

/// The full page is the only thing that is not the `#lists` fragment, so it
/// is the only caller that takes `build_lists`' fields apart instead of
/// going through `lists::respond`. `inbox.html` `{% include %}`s
/// `lists.html`, and an Askama include renders in its parent's context, so
/// the page template has to carry the same fields by the same names.
///
/// The header returned in #92: `base.html` renders `nav` regardless of
/// which page extends it, so every page template carries it now.
pub async fn show_inbox(State(pool): State<SqlitePool>) -> Result<Response, StatusCode> {
    let lists = build_lists(&pool, None).await.map_err(write_failed)?;
    Ok(render_template(
        StatusCode::OK,
        &InboxTemplate {
            captures: lists.captures,
            context_tag_suggestions: lists.context_tag_suggestions,
            nav: nav::links(Page::Capture),
        },
    ))
}

#[derive(serde::Deserialize)]
pub struct SetShownKindRequest {
    kind: String,
}

pub async fn set_shown_kind(
    State(pool): State<SqlitePool>,
    Path(capture_id): Path<i64>,
    Form(payload): Form<SetShownKindRequest>,
) -> Result<Response, StatusCode> {
    // Outside the closed domain before anything else happens: `ShownKind`
    // owns which kinds have a panel at all, and a submission naming another
    // is refused here rather than reaching the column
    // (`D-pool-is-default` -- pool files on one tap and never opens one).
    let shown = ShownKind::parse(&payload.kind).ok_or(StatusCode::BAD_REQUEST)?;
    let open = super::capture_is_open(&pool, capture_id)
        .await
        .map_err(write_failed)?;
    let (status, error) = if open {
        super::store::set_shown_kind(&pool, capture_id, shown)
            .await
            .map_err(write_failed)?;
        (StatusCode::OK, None)
    } else {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some((capture_id, CAPTURE_NOT_OPEN_MESSAGE.to_string())),
        )
    };
    lists::respond(&pool, status, error).await
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
        let app =
            crate::platform::app::build_app(pool.clone(), crate::platform::clock::Clock::system());
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
        sqlx::query("UPDATE captures SET left_inbox_at = 1 WHERE id = ?")
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

    async fn set_kind_response(pool: &SqlitePool, capture_id: i64, kind: &str) -> Response {
        let app =
            crate::platform::app::build_app(pool.clone(), crate::platform::clock::Clock::system());
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/captures/{capture_id}/kind"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("kind={kind}")))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn body_string(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn shown_kind(pool: &SqlitePool, capture_id: i64) -> Option<String> {
        sqlx::query_scalar("SELECT shown_kind FROM captures WHERE id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn choosing_committed_records_it_and_responds_200() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = set_kind_response(&pool, capture_id, "committed").await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            shown_kind(&pool, capture_id).await.as_deref(),
            Some("committed")
        );
    }

    #[tokio::test]
    async fn choosing_a_second_kind_replaces_the_first() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;
        set_kind_response(&pool, capture_id, "committed").await;

        set_kind_response(&pool, capture_id, "quota").await;

        assert_eq!(
            shown_kind(&pool, capture_id).await.as_deref(),
            Some("quota")
        );
    }

    #[tokio::test]
    async fn choosing_on_one_capture_leaves_another_alone() {
        let (_dir, pool) = test_pool().await;
        let chosen = insert_untriaged_capture(&pool, "buy milk").await;
        let other = insert_untriaged_capture(&pool, "call the dentist").await;

        set_kind_response(&pool, chosen, "committed").await;

        assert_eq!(shown_kind(&pool, other).await, None);
    }

    #[tokio::test]
    async fn an_unrecognized_kind_is_rejected_as_a_bad_request() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = set_kind_response(&pool, capture_id, "pool").await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(shown_kind(&pool, capture_id).await, None);
    }

    #[tokio::test]
    async fn choosing_a_kind_for_a_capture_that_does_not_exist_is_rejected() {
        let (_dir, pool) = test_pool().await;

        let response = set_kind_response(&pool, 999, "committed").await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn choosing_a_kind_for_an_already_triaged_capture_is_rejected() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;
        crate::triage::store::insert_task(
            &pool,
            capture_id,
            &scheduler_core::task::TaskKind::Pool,
            0,
        )
        .await
        .unwrap();
        crate::inbox::close_capture(&pool, capture_id, 0)
            .await
            .unwrap();

        let response = set_kind_response(&pool, capture_id, "committed").await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(shown_kind(&pool, capture_id).await, None);
    }

    #[tokio::test]
    async fn choosing_committed_re_renders_the_lists_fragment_with_the_committed_panel_open() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = set_kind_response(&pool, capture_id, "committed").await;

        let body = body_string(response).await;
        assert!(
            body.contains("At a time") && body.contains("By a day"),
            "expected the committed panel's fields in the response, got:\n{body}"
        );
    }
}
