//! `GET /stats`.
//!
//! Translation, and nothing else: read the clock, ask the core where the
//! window opens, ask the store what is in it, hand the counts back to the
//! core, render what it says. No arithmetic of its own — a handler that
//! subtracted the window length itself would be holding one end of a rule
//! whose other end lives in `scheduler_core::ratio`.

use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use crate::stats::store;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use scheduler_core::ratio::{window_start, CommittedShare};
use sqlx::SqlitePool;

#[derive(Template)]
#[template(path = "stats.html")]
struct StatsTemplate {
    share: CommittedShare,
    nav: Vec<NavLink>,
}

pub async fn show_stats(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
) -> Result<Response, StatusCode> {
    let now_ms = clock.now_ms();
    let counts = store::counts_in_window(&pool, window_start(now_ms), now_ms)
        .await
        .map_err(write_failed)?;
    let share = CommittedShare::from_counts(counts);
    Ok(render_template(
        StatusCode::OK,
        &StatsTemplate {
            share,
            nav: nav::links(Page::Stats),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use scheduler_core::ratio::WINDOW_MS;
    use scheduler_core::task::TaskKind;
    use tower::ServiceExt;

    async fn get_stats_at(pool: &SqlitePool, clock: Clock) -> String {
        let app = crate::platform::app::build_app(pool.clone(), clock);
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

    async fn get_stats(pool: &SqlitePool) -> String {
        get_stats_at(pool, Clock::system()).await
    }

    async fn given_a_task(pool: &SqlitePool, kind: &TaskKind, created_at_ms: i64) {
        let capture_id = crate::capture::store::insert(pool, "buy milk", "web", None, 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(pool, capture_id, kind, None, created_at_ms)
            .await
            .unwrap();
    }

    fn committed() -> TaskKind {
        TaskKind::Committed {
            deadline: 1787245200000,
            deadline_type: scheduler_core::task::DeadlineType::Hard,
            priority: scheduler_core::task::Priority::P1,
            estimated_minutes: 180,
        }
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
        let now = Clock::system().now_ms();
        for _ in 0..3 {
            given_a_task(&pool, &TaskKind::Pool, now).await;
        }
        for _ in 0..7 {
            given_a_task(&pool, &committed(), now).await;
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
        let now = Clock::system().now_ms();
        let outside_window = now - WINDOW_MS - 1;
        given_a_task(&pool, &TaskKind::Pool, outside_window).await;

        let body = get_stats(&pool).await;

        assert!(
            body.contains("0 pool"),
            "a task triaged outside the window must not be counted, got:\n{body}"
        );
    }

    /// The window is read through the server's own clock, so a server that
    /// believes it is later sees an older task fall out — the property
    /// `--now` exists to let the QA procedure observe, checked here without
    /// a process to restart or a global to put back.
    #[tokio::test]
    async fn the_window_moves_with_the_clock_the_server_was_built_with() {
        let (_dir, pool) = test_pool().await;
        let triaged_at = Clock::system().now_ms();
        given_a_task(&pool, &TaskKind::Pool, triaged_at).await;

        let while_fresh = get_stats(&pool).await;
        let much_later = get_stats_at(&pool, Clock::pinned_at(triaged_at + WINDOW_MS + 1)).await;

        assert!(
            while_fresh.contains("1 tasks in the window"),
            "the task should be in the window on a clock that agrees it is now, got:\n{while_fresh}"
        );
        assert!(
            much_later.contains("0 tasks in the window"),
            "the task should have fallen out of a window read a fortnight later, got:\n{much_later}"
        );
    }
}
