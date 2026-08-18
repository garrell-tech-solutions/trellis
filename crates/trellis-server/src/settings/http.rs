//! `POST /timezone`.
//!
//! Follows `T-forms-swap-one-fragment`: the response is the `#timezone`
//! fragment, `422` on rejection, carrying why. A rejection leaves the
//! stored zone untouched and reports it back rather than echoing the
//! failed submission as if it had taken effect.

use crate::platform::response::{render_template, write_failed};
use crate::settings::store;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Form;
use scheduler_core::timezone::validate;
use serde::Deserialize;
use sqlx::SqlitePool;

/// The `#timezone` fragment on its own -- what both an accepted change and
/// a rejected one swap in.
#[derive(Template)]
#[template(path = "timezone.html")]
struct TimezoneTemplate {
    timezone: String,
    timezone_error: Option<String>,
}

#[derive(Deserialize)]
pub struct SetTimezoneRequest {
    zone: String,
}

pub async fn set_timezone(
    State(pool): State<SqlitePool>,
    Form(payload): Form<SetTimezoneRequest>,
) -> Result<Response, StatusCode> {
    let (status, timezone, timezone_error) = match validate(&payload.zone) {
        Ok(()) => {
            store::set_timezone(&pool, &payload.zone)
                .await
                .map_err(write_failed)?;
            (StatusCode::OK, payload.zone.clone(), None)
        }
        Err(_) => {
            let current = store::get_timezone(&pool).await.map_err(write_failed)?;
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                current,
                Some(format!("{} is not a timezone", payload.zone)),
            )
        }
    };
    Ok(render_template(
        status,
        &TimezoneTemplate {
            timezone,
            timezone_error,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::clock::Clock;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn body_string(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn post_timezone(pool: &SqlitePool, zone: &str) -> Response {
        let app = crate::platform::app::build_app(pool.clone(), Clock::system());
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/timezone")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("zone={zone}")))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn setting_a_real_zone_is_accepted_and_stored() {
        let (_dir, pool) = test_pool().await;

        let response = post_timezone(&pool, "Europe%2FLondon").await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(store::get_timezone(&pool).await.unwrap(), "Europe/London");
        let body = body_string(response).await;
        assert!(body.contains("Europe/London"), "got:\n{body}");
    }

    #[tokio::test]
    async fn setting_an_unknown_zone_is_rejected_and_leaves_the_setting_unchanged() {
        let (_dir, pool) = test_pool().await;

        let response = post_timezone(&pool, "Mars%2FOlympus").await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(store::get_timezone(&pool).await.unwrap(), "UTC");
        let body = body_string(response).await;
        assert!(body.contains("Mars/Olympus"), "got:\n{body}");
        assert!(body.contains("not a timezone"), "got:\n{body}");
    }

    #[tokio::test]
    async fn hostile_text_submitted_as_a_zone_is_escaped_in_the_rejection() {
        let (_dir, pool) = test_pool().await;

        let response = post_timezone(&pool, "%3Cscript%3Ealert('boom')%3C%2Fscript%3E").await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_string(response).await;
        assert!(!body.contains("<script>"), "got:\n{body}");
        assert!(body.contains("boom"), "got:\n{body}");
    }
}
