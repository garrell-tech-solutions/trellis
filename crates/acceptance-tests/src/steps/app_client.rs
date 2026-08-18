//! How a scenario talks to the application under test.
//!
//! Both the capture and the triage features act by POSTing JSON to the app
//! and then asserting over what came back, and each step module had grown its
//! own copy of "build the router from the world's pool, send the request,
//! record the outcome". Two copies of that meant no single place knew how a
//! scenario reaches the application — and they had already drifted: one timed
//! the round trip and dropped the body, the other kept the body and dropped
//! the timing, so a step could only assert whatever its own module happened
//! to have recorded.

use crate::world::World;
use axum::body::Body;
use axum::http::Request;
use serde_json::Value;
use std::time::Duration;
use tower::ServiceExt;

/// What the application sent back. Everything a `Then` step might want to
/// assert over, so no caller has to decide up front what to keep.
pub struct Response {
    pub status: u16,
    /// `None` when the body was not JSON — an empty body included.
    pub body: Option<Value>,
    pub elapsed: Duration,
}

/// POSTs `body` as JSON to `uri` against a router built on the scenario's
/// database pool.
pub async fn post_json(world: &World, uri: &str, body: &Value) -> Result<Response, String> {
    let app = trellis_server::platform::app::build_app(world.pool()?.clone(), world.clock());
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .map_err(|e| format!("build request: {e}"))?;

    let start = std::time::Instant::now();
    let response = app
        .oneshot(request)
        .await
        .map_err(|e| format!("send request: {e}"))?;
    let elapsed = start.elapsed();

    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| format!("read response body: {e}"))?;

    Ok(Response {
        status,
        body: serde_json::from_slice(&bytes).ok(),
        elapsed,
    })
}

#[cfg(test)]
mod tests {
    use super::super::migrated_world;
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn post_json_reports_the_status_and_body_the_app_returned() {
        let world = migrated_world().await;

        let response = post_json(&world, "/captures/1/triage", &json!({ "kind": "someday" }))
            .await
            .unwrap();

        assert_eq!(response.status, 422);
        assert_eq!(response.body, Some(json!({ "unknown_kind": "someday" })));
    }

    #[tokio::test]
    async fn post_json_reports_an_empty_body_as_no_json() {
        let world = migrated_world().await;

        let response = post_json(
            &world,
            "/captures",
            &json!({ "raw_text": "buy milk", "source": "web" }),
        )
        .await
        .unwrap();

        assert_eq!(response.status, 201);
        assert_eq!(response.body, None);
    }

    #[tokio::test]
    async fn post_json_errors_when_the_scenario_has_no_database() {
        let world = World::new();

        assert!(post_json(&world, "/captures", &json!({})).await.is_err());
    }
}
