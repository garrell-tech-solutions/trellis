//! `GET /committed`.

use crate::committed::view::{self, CommittedRowView};
use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

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
    let rows = super::store::list_committed_tasks(&pool)
        .await
        .map_err(write_failed)?;
    let built = view::build(rows, clock.now_ms());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn get_committed(pool: &SqlitePool) -> (StatusCode, String) {
        let app = crate::platform::app::build_app(pool.clone(), Clock::pinned_at(1787562000000));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/committed")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
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
        let (capture_id, _) = crate::capture::create(&pool, "book the dentist", "web", None, 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            &pool,
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

        let (_, body) = get_committed(&pool).await;

        assert!(body.contains("book the dentist"), "got:\n{body}");
    }
}
