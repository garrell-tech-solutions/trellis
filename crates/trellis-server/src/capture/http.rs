//! `POST /captures`.
//!
//! One code path for both callers: the JSON API and the inbox's quick-add
//! box both end at this handler and the same [`store::insert`] call,
//! so a capture created either way is the same row (inbox-view brief:
//! "the box and `POST /captures` are one code path, not two"). Content type
//! is the only thing that differs — a plain form post (the quick-add box)
//! gets back the new row's markup to swap into the page; a JSON request (the
//! existing API) gets back exactly what it always has, unchanged.

use crate::capture::store;
use crate::inbox::view::CaptureRow;
use crate::life_areas::store as life_areas_store;
use crate::life_areas::view::LifeAreaOption;
use crate::platform::clock::Clock;
use crate::platform::request::content_type_is_json;
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::{FromRequest, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Form, Json};
use serde::Deserialize;
use sqlx::SqlitePool;

#[derive(Deserialize)]
pub struct CaptureRequest {
    pub raw_text: String,
    pub source: String,
}

/// A capture request extracted from either an `application/json` body (the
/// API) or a form-encoded one (the quick-add box's plain HTML `<form>`),
/// remembering which so the handler knows how to respond.
pub struct CaptureInput {
    pub payload: CaptureRequest,
    pub from_form: bool,
}

impl<S: Send + Sync> FromRequest<S> for CaptureInput {
    type Rejection = StatusCode;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let is_json = content_type_is_json(&req);
        if is_json {
            let Json(payload) = Json::<CaptureRequest>::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(CaptureInput {
                payload,
                from_form: false,
            })
        } else {
            let Form(payload) = Form::<CaptureRequest>::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(CaptureInput {
                payload,
                from_form: true,
            })
        }
    }
}

/// `life_areas` rides along for the same reason `inbox::lists::ListsTemplate`
/// carries it: this row's own triage forms offer the picker, and the picker
/// must reflect the current, active set on every render.
#[derive(Template)]
#[template(path = "capture_row.html")]
struct CaptureRowTemplate<'a> {
    capture: &'a CaptureRow,
    life_areas: Vec<LifeAreaOption>,
}

pub async fn create_capture(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    CaptureInput { payload, from_form }: CaptureInput,
) -> Result<Response, StatusCode> {
    let id = store::insert(&pool, &payload.raw_text, &payload.source, clock.now_ms())
        .await
        .map_err(write_failed)?;

    if from_form {
        let capture = CaptureRow {
            id,
            text: payload.raw_text,
            error: None,
        };
        let life_areas = life_areas_store::list_active(&pool)
            .await
            .map_err(write_failed)?
            .into_iter()
            .map(|row| LifeAreaOption {
                id: row.id,
                name: row.name,
            })
            .collect();
        Ok(render_template(
            StatusCode::CREATED,
            &CaptureRowTemplate {
                capture: &capture,
                life_areas,
            },
        ))
    } else {
        Ok(StatusCode::CREATED.into_response())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn post_capture(pool: &SqlitePool, content_type: &str, body: &str) -> Response {
        let app = crate::platform::app::build_app(pool.clone(), Clock::system());
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/captures")
                .header("content-type", content_type)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_json_submission_still_gets_back_a_bare_201_with_no_body() {
        let (_dir, pool) = test_pool().await;

        let response = post_capture(
            &pool,
            "application/json",
            r#"{"raw_text":"buy milk","source":"web"}"#,
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.is_empty(), "expected no body, got {body:?}");
    }

    #[tokio::test]
    async fn a_form_submission_is_not_a_redirect_and_its_body_includes_the_raw_text() {
        let (_dir, pool) = test_pool().await;

        let response = post_capture(
            &pool,
            "application/x-www-form-urlencoded",
            "raw_text=buy+milk&source=web",
        )
        .await;

        assert!(
            !response.status().is_redirection(),
            "quick-add must not redirect, got {}",
            response.status()
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body.contains("buy milk"),
            "expected the new capture's text in the response, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn a_form_submission_writes_the_same_row_a_json_submission_would() {
        let (_dir, pool) = test_pool().await;

        post_capture(
            &pool,
            "application/x-www-form-urlencoded",
            "raw_text=buy+milk&source=web",
        )
        .await;

        let row: (String, String) = sqlx::query_as("SELECT raw_text, source FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row, ("buy milk".to_string(), "web".to_string()));
    }
}
