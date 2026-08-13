//! Composition root: wires the delivery layer's handlers onto routes and
//! hands them the pool they persist through.

use axum::routing::{get, post};
use axum::Router;
use sqlx::SqlitePool;

use crate::http::assets::htmx_js;
use crate::http::capture::create_capture;
use crate::http::inbox::show_inbox;
use crate::http::triage::create_triage;

pub fn build_app(pool: SqlitePool) -> Router {
    Router::new()
        .route("/", get(show_inbox))
        .route("/static/htmx.min.js", get(htmx_js))
        .route("/captures", post(create_capture))
        .route("/captures/{id}/triage", post(create_triage))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_pool;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use proptest::prelude::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn capture_request_persists_a_row_and_responds_within_50ms() {
        let (_dir, pool) = test_pool().await;
        let app = build_app(pool.clone());

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
                let app = build_app(pool.clone());

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
