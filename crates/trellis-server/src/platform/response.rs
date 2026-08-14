//! The two things every capability's `http` module does with its answer:
//! render a template, or name a failed write.
//!
//! Neither decides anything about captures, triage or the inbox, which is
//! why they live in `platform` rather than in one of them — and why a
//! capability that needed a *third* shared helper would be a sign that the
//! capabilities are wrong, not that this module needs another function.
//! A handler's whole job is still translation: request body into a
//! `scheduler_core` input, core decision into a response, store failure into
//! a status code. Any rule that survives changing HTTP for something else
//! belongs in `scheduler_core`, not in a handler and not here.

use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// A failed write is the delivery layer's problem to name; the store reports
/// the database error and says nothing about HTTP.
pub(crate) fn write_failed(_error: sqlx::Error) -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}

/// Renders an Askama template to a response, or reports the one way that can
/// fail: the template itself. A render failure is a build-time bug (a
/// template referencing a field that does not exist) rather than anything a
/// request could trigger, so it maps to a 500 like any other adapter failure.
pub(crate) fn render_template(status: StatusCode, template: &impl Template) -> Response {
    match template.render() {
        Ok(body) => (status, Html(body)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_write_is_reported_as_an_internal_server_error() {
        assert_eq!(
            write_failed(sqlx::Error::RowNotFound),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[derive(askama::Template)]
    #[template(source = "hello {{ name }}", ext = "txt")]
    struct GreetingTemplate<'a> {
        name: &'a str,
    }

    #[tokio::test]
    async fn render_template_renders_the_template_body_at_the_given_status() {
        let response = render_template(StatusCode::CREATED, &GreetingTemplate { name: "world" });
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body, "hello world");
    }
}
