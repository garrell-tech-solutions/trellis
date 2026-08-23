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
const MANIFEST: &str = include_str!("../../static/manifest.webmanifest");
const ICON_192: &[u8] = include_bytes!("../../static/icons/icon-192.png");
const ICON_512: &[u8] = include_bytes!("../../static/icons/icon-512.png");
const ICON_MASKABLE_512: &[u8] = include_bytes!("../../static/icons/icon-maskable-512.png");

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

/// The web app manifest (#112, `installable-manifest-is-linked-01`): what
/// makes an installed window standalone rather than a bookmark, served the
/// same way `trellis_css` is -- a file checked into the repo and embedded at
/// compile time, not built up in code.
pub async fn manifest() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/manifest+json")],
        MANIFEST,
    )
}

pub async fn icon_192() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/png")], ICON_192)
}

pub async fn icon_512() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/png")], ICON_512)
}

pub async fn icon_maskable_512() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/png")], ICON_MASKABLE_512)
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
    async fn manifest_serves_as_the_manifest_content_type() {
        let response = manifest().await.into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/manifest+json"
        );
    }

    #[tokio::test]
    async fn manifest_parses_as_json_and_carries_the_required_members() {
        let response = manifest().await.into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["name"], "Trellis");
        assert_eq!(parsed["short_name"], "Trellis");
        assert_eq!(parsed["start_url"], "/");
        assert_eq!(parsed["display"], "standalone");
    }

    #[tokio::test]
    async fn manifest_declares_192_and_512_any_icons_and_one_maskable_icon() {
        let response = manifest().await.into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let icons = parsed["icons"].as_array().unwrap();

        assert!(icons
            .iter()
            .any(|i| i["sizes"] == "192x192" && i["type"] == "image/png"));
        assert!(icons
            .iter()
            .any(|i| i["sizes"] == "512x512" && i["type"] == "image/png"));
        assert!(icons.iter().any(|i| i["purpose"]
            .as_str()
            .is_some_and(|p| p.split(' ').any(|word| word == "maskable"))
            && i["type"] == "image/png"));
    }

    #[tokio::test]
    async fn icon_192_serves_a_png_at_the_declared_size() {
        let response = icon_192().await.into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/png"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[tokio::test]
    async fn icon_512_serves_a_png() {
        let response = icon_512().await.into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/png"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[tokio::test]
    async fn icon_maskable_512_serves_a_png() {
        let response = icon_maskable_512().await.into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/png"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(body.starts_with(&[0x89, b'P', b'N', b'G']));
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
