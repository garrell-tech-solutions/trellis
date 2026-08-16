//! Composition root: wires each capability's handlers onto routes and hands
//! them the pool they persist through and the clock they stamp rows with.
//!
//! The one place in the crate that names every business domain at once —
//! which is what a route table is, and why it sits in `platform` rather than
//! inside any capability. Read top to bottom it is also the shortest
//! statement of what this server does.
//!
//! It is also the one place that decides what a handler may reach for. Both
//! of [`AppState`]'s fields are things the outside world supplies — a
//! database and a clock — so composing them here is what keeps a handler from
//! going and finding either for itself.

use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;
use sqlx::SqlitePool;

use crate::capture::http::create_capture;
use crate::inbox::http::show_inbox;
use crate::life_areas::http::{archive_life_area, create_life_area, show_life_areas};
use crate::platform::assets::htmx_js;
use crate::platform::clock::Clock;
use crate::stats::http::show_stats;
use crate::triage::http::create_triage;

/// What a handler is given: somewhere to persist, and an answer to "what time
/// is it". The [`FromRef`] impls below let each handler extract only the half
/// it uses, so `show_inbox` still names nothing but a pool.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub clock: Clock,
}

impl FromRef<AppState> for SqlitePool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Clock {
    fn from_ref(state: &AppState) -> Self {
        state.clock
    }
}

pub fn build_app(pool: SqlitePool, clock: Clock) -> Router {
    Router::new()
        .route("/", get(show_inbox))
        .route("/static/htmx.min.js", get(htmx_js))
        .route("/captures", post(create_capture))
        .route("/captures/{id}/triage", post(create_triage))
        .route("/stats", get(show_stats))
        .route("/life-areas", get(show_life_areas).post(create_life_area))
        .route("/life-areas/{id}/archive", post(archive_life_area))
        .with_state(AppState { pool, clock })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use proptest::prelude::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn capture_request_persists_a_row_and_responds_within_50ms() {
        let (_dir, pool) = test_pool().await;
        let app = build_app(pool.clone(), Clock::system());

        let start = std::time::Instant::now();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/captures")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"raw_text":"buy milk","source":"web"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert_eq!(response.status(), StatusCode::CREATED);
        assert!(elapsed.as_millis() < 50, "took {elapsed:?}");

        let row: (String, String) = sqlx::query_as("SELECT raw_text, source FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row, ("buy milk".to_string(), "web".to_string()));
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]
        #[test]
        #[ignore]
        fn capture_round_trips_arbitrary_text_and_source(raw_text in ".{0,200}", source in ".{0,50}") {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (status, stored) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;
                let app = build_app(pool.clone(), Clock::system());

                let body = serde_json::json!({"raw_text": raw_text, "source": source}).to_string();
                let response = app
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri("/captures")
                            .header("content-type", "application/json")
                            .body(Body::from(body))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let status = response.status();

                let row: (String, String) = sqlx::query_as("SELECT raw_text, source FROM captures")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                (status, row)
            });

            prop_assert_eq!(status, StatusCode::CREATED);
            prop_assert_eq!(stored, (raw_text, source));
        }
    }
}
