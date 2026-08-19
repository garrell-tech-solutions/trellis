//! `POST /schedule/generate`, `GET /schedule`.
//!
//! Translation only: run a fresh forward pass or read the stored plan back,
//! render what the store says. No arithmetic of its own -- ordering, the
//! four-reason report and every invariant live in `scheduler_core::
//! schedule`.

use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

#[derive(Template)]
#[template(path = "schedule.html")]
struct ScheduleTemplate {
    placed: Vec<super::view::PlacedRow>,
    unplaceable: Vec<super::view::UnplaceableRow>,
    placed_count: usize,
    nav: Vec<NavLink>,
}

async fn render_schedule(pool: &SqlitePool) -> Result<Response, StatusCode> {
    let placed = super::placed_rows(pool).await.map_err(write_failed)?;
    let unplaceable = super::unplaceable_rows(pool).await.map_err(write_failed)?;
    let placed_count = placed.len();
    Ok(render_template(
        StatusCode::OK,
        &ScheduleTemplate {
            placed,
            unplaceable,
            placed_count,
            nav: nav::links(Page::Schedule),
        },
    ))
}

/// Reads the plan as it was last written -- `R-incremental-patching`: never
/// recomputed on load.
pub async fn show_schedule(State(pool): State<SqlitePool>) -> Result<Response, StatusCode> {
    render_schedule(&pool).await
}

/// Runs a fresh forward pass over every active committed task and replaces
/// the stored plan wholesale, then renders it -- the page's own "Generate"
/// control.
pub async fn generate_schedule(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
) -> Result<Response, StatusCode> {
    super::generate(&pool, &clock).await.map_err(write_failed)?;
    render_schedule(&pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{get_ok, seeded_life_area_id, test_pool};
    use axum::body::Body;
    use axum::http::Request;
    use scheduler_core::task::{DeadlineType, Priority, TaskKind};
    use tower::ServiceExt;

    fn pinned_monday_nine() -> Clock {
        Clock::pinned_at(1_786_957_200_000) // 2026-08-17T09:00:00Z
    }

    async fn post_generate(pool: &SqlitePool, clock: Clock) -> (StatusCode, String) {
        let app = crate::platform::app::build_app(pool.clone(), clock);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/schedule/generate")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn a_fresh_database_shows_the_empty_state_message() {
        let (_dir, pool) = test_pool().await;

        let body = get_ok(&pool, Clock::system(), "/schedule").await;

        assert!(body.contains("Nothing to schedule"), "got:\n{body}");
    }

    #[tokio::test]
    async fn generating_then_viewing_shows_the_placed_task() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        crate::life_areas::store::insert_guardrail_band(&pool, work, "Mon", 540, 1020)
            .await
            .unwrap();
        let capture_id = crate::capture::store::insert(&pool, "write the Q3 deck", "web", 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            &pool,
            capture_id,
            &TaskKind::Committed {
                deadline: 1_786_957_200_000 + 4 * 24 * 3_600_000,
                deadline_type: DeadlineType::Hard,
                priority: Priority::P2,
                estimated_minutes: 120,
            },
            Some(work),
            0,
        )
        .await
        .unwrap();

        let (status, body) = post_generate(&pool, pinned_monday_nine()).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("write the Q3 deck"), "got:\n{body}");

        let body = get_ok(&pool, pinned_monday_nine(), "/schedule").await;
        assert!(body.contains("write the Q3 deck"), "got:\n{body}");
        assert!(body.contains("2026-08-17T09:00:00Z"), "got:\n{body}");
        assert!(body.contains("2026-08-17T11:00:00Z"), "got:\n{body}");
    }

    #[tokio::test]
    async fn an_unplaceable_task_names_its_reason() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        crate::life_areas::store::insert_guardrail_band(&pool, work, "Mon", 540, 660)
            .await
            .unwrap();
        let capture_id = crate::capture::store::insert(&pool, "rebuild the deck", "web", 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            &pool,
            capture_id,
            &TaskKind::Committed {
                deadline: 1_786_957_200_000 + 10 * 24 * 3_600_000,
                deadline_type: DeadlineType::Soft,
                priority: Priority::P2,
                estimated_minutes: 180,
            },
            Some(work),
            0,
        )
        .await
        .unwrap();

        post_generate(&pool, pinned_monday_nine()).await;
        let body = get_ok(&pool, pinned_monday_nine(), "/schedule").await;

        assert!(body.contains("rebuild the deck"), "got:\n{body}");
        assert!(body.contains("chunk_policy_unsatisfiable"), "got:\n{body}");
    }

    #[tokio::test]
    async fn hostile_task_text_stays_escaped() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        crate::life_areas::store::insert_guardrail_band(&pool, work, "Mon", 540, 1020)
            .await
            .unwrap();
        let capture_id =
            crate::capture::store::insert(&pool, "<script>alert('boom')</script>", "web", 0)
                .await
                .unwrap();
        crate::triage::store::insert_task(
            &pool,
            capture_id,
            &TaskKind::Committed {
                deadline: 1_786_957_200_000 + 4 * 24 * 3_600_000,
                deadline_type: DeadlineType::Hard,
                priority: Priority::P2,
                estimated_minutes: 120,
            },
            Some(work),
            0,
        )
        .await
        .unwrap();

        post_generate(&pool, pinned_monday_nine()).await;
        let body = get_ok(&pool, pinned_monday_nine(), "/schedule").await;

        assert!(!body.contains("<script>"), "got:\n{body}");
        assert!(body.contains("boom"), "got:\n{body}");
    }
}
