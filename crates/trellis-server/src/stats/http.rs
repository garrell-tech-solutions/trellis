//! `GET /stats`.

use crate::platform::clock::now_ms;
use crate::platform::response::{render_template, write_failed};
use crate::stats::store;
use crate::stats::view::StatsView;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

#[derive(Template)]
#[template(path = "stats.html")]
struct StatsTemplate {
    view: StatsView,
}

pub async fn show_stats(State(pool): State<SqlitePool>) -> Result<Response, StatusCode> {
    let now = now_ms();
    let window_start = now - store::WINDOW_MS;
    let counts = store::counts_in_window(&pool, window_start, now)
        .await
        .map_err(write_failed)?;
    let view = StatsView::from_counts(counts.committed, counts.pool, counts.quota);
    Ok(render_template(StatusCode::OK, &StatsTemplate { view }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use scheduler_core::task::TaskKind;
    use tower::ServiceExt;

    async fn get_stats(pool: &SqlitePool) -> String {
        let app = crate::platform::app::build_app(pool.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    async fn given_a_task(pool: &SqlitePool, kind: &TaskKind, created_at_ms: i64) {
        let capture_id = crate::capture::store::insert(pool, "buy milk", "web", 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(pool, capture_id, kind, created_at_ms)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn an_empty_database_reports_no_share_and_does_not_error() {
        let (_dir, pool) = test_pool().await;

        let body = get_stats(&pool).await;

        assert!(
            !body.contains("0%"),
            "an empty window must not report 0%, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn counts_and_share_reflect_tasks_triaged_inside_the_window() {
        let (_dir, pool) = test_pool().await;
        let now = now_ms();
        for _ in 0..3 {
            given_a_task(&pool, &TaskKind::Pool, now).await;
        }
        for _ in 0..7 {
            given_a_task(
                &pool,
                &TaskKind::Committed {
                    deadline: 1787245200000,
                    deadline_type: scheduler_core::task::DeadlineType::Hard,
                    priority: scheduler_core::task::Priority::P1,
                },
                now,
            )
            .await;
        }

        let body = get_stats(&pool).await;

        assert!(body.contains("70%"), "expected the share, got:\n{body}");
        assert!(
            body.contains("over the line"),
            "70% should be reported over the line, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn a_task_triaged_outside_the_window_is_excluded() {
        let (_dir, pool) = test_pool().await;
        let now = now_ms();
        let outside_window = now - store::WINDOW_MS - 1;
        given_a_task(&pool, &TaskKind::Pool, outside_window).await;

        let body = get_stats(&pool).await;

        assert!(
            body.contains("0 pool"),
            "a task triaged outside the window must not be counted, got:\n{body}"
        );
    }
}
