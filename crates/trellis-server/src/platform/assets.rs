//! Static assets — machinery, not a capability, which is why they are here
//! and not under the page that loads them.
//!
//! HTMX is vendored (`stack.prompt`: no Node build step, no
//! CDN dependency for the page to work) and embedded into the binary at
//! compile time, so the release build stays the single static executable
//! `release_binary.feature` asserts.
//!
//! The stylesheet and the typeface it names arrive the same way and for the
//! same reason. A design system delivered as a CDN link is a design system
//! that stops existing on a phone with no signal, and capture is the one
//! thing this product has to survive that.

use axum::http::header;
use axum::response::IntoResponse;

const HTMX_JS: &str = include_str!("../../static/htmx.min.js");
const TRELLIS_CSS: &str = include_str!("../../static/trellis.css");
const SPACE_GROTESK_WOFF2: &[u8] =
    include_bytes!("../../static/fonts/space-grotesk-variable.woff2");

/// A year, which is what the fingerprint-free URLs here can honestly claim:
/// the font is a versioned file that never changes, and the stylesheet ships
/// inside the binary, so a new stylesheet means a new binary and a restart.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

pub async fn htmx_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        HTMX_JS,
    )
}

pub async fn trellis_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        TRELLIS_CSS,
    )
}

pub async fn space_grotesk_woff2() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, IMMUTABLE),
        ],
        SPACE_GROTESK_WOFF2,
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

    #[tokio::test]
    async fn trellis_css_serves_the_stylesheet_as_css() {
        let response = trellis_css().await.into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/css; charset=utf-8"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body.contains("--color-primary-800"),
            "expected the design system's tokens in the stylesheet"
        );
    }

    /// The stylesheet names the font at exactly the path the router serves
    /// it from. Nothing in the type system connects a `url()` in a text file
    /// to a route, and the failure is silent: the page renders in a fallback
    /// face and looks merely a bit wrong.
    #[tokio::test]
    async fn the_stylesheet_asks_for_the_font_at_the_path_the_router_serves() {
        let response = trellis_css().await.into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body.contains("url('/static/fonts/space-grotesk-variable.woff2')"),
            "expected the stylesheet to name the font's own route"
        );
    }

    #[tokio::test]
    async fn space_grotesk_serves_the_embedded_font_as_woff2() {
        let response = space_grotesk_woff2().await.into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "font/woff2"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(
            body.starts_with(b"wOF2"),
            "expected a woff2 signature, got {:?}",
            &body[..body.len().min(4)]
        );
    }
}
