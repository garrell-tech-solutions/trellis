//! Static assets — machinery, not a capability, which is why they are here
//! and not under the page that loads them.
//!
//! HTMX is vendored (`stack.prompt`: no Node build step, no
//! CDN dependency for the page to work) and embedded into the binary at
//! compile time, so the release build stays the single static executable
//! `release_binary.feature` asserts.

use axum::http::header;
use axum::response::IntoResponse;

const HTMX_JS: &str = include_str!("../../static/htmx.min.js");

pub async fn htmx_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        HTMX_JS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn htmx_js_serves_the_vendored_script_as_javascript() {
        let response = htmx_js().await.into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/javascript; charset=utf-8"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(
            body.starts_with(b"var htmx"),
            "expected the vendored htmx source, got {} bytes starting {:?}",
            body.len(),
            &body[..body.len().min(20)]
        );
    }
}
