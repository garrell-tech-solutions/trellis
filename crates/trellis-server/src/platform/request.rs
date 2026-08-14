//! The one thing every capability's `http` module does with an incoming
//! request before it decides anything: tell a JSON body from a form post.
//!
//! Symmetric to [`super::response`] on the way out. A handler still owns
//! everything about *which* transport it accepts and what it does with the
//! result — this just answers the one question both transports need
//! answered the same way.

use axum::extract::Request;
use axum::http::header;

pub(crate) fn content_type_is_json(req: &Request) -> bool {
    req.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    fn request_with_content_type(value: Option<&str>) -> Request {
        let mut builder = Request::builder();
        if let Some(value) = value {
            builder = builder.header(header::CONTENT_TYPE, value);
        }
        builder.body(Body::empty()).unwrap()
    }

    #[test]
    fn an_application_json_content_type_is_json() {
        assert!(content_type_is_json(&request_with_content_type(Some(
            "application/json"
        ))));
    }

    #[test]
    fn a_charset_suffix_on_application_json_is_still_json() {
        assert!(content_type_is_json(&request_with_content_type(Some(
            "application/json; charset=utf-8"
        ))));
    }

    #[test]
    fn a_form_content_type_is_not_json() {
        assert!(!content_type_is_json(&request_with_content_type(Some(
            "application/x-www-form-urlencoded"
        ))));
    }

    #[test]
    fn a_missing_content_type_is_not_json() {
        assert!(!content_type_is_json(&request_with_content_type(None)));
    }
}
