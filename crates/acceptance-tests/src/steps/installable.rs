//! Step handlers for `features/installable.feature`: Trellis installs to
//! the home screen (#112).
//!
//! The Background ("the trellis server is running with an empty task list")
//! and "the "<screen>" screen is viewed" are already matched generically by
//! [`super::triage::dispatch`] and [`super::pool_screen::dispatch`], tried
//! before this module -- nothing here duplicates them.

use super::*;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request};
use tower::ServiceExt;

static THEN_LINKS_MANIFEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the page links a web app manifest$").unwrap());
static THEN_MANIFEST_LINK_SERVES_JSON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^following the manifest link serves a document parsed as JSON$").unwrap()
});
static WHEN_MANIFEST_FETCHED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the manifest is fetched$").unwrap());
static THEN_MEMBER_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the manifest member "([^"]+)" is "([^"]+)"$"#).unwrap());
static THEN_DECLARES_ICON_OF_SIZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the manifest declares an icon of "([^"]+)"$"#).unwrap());
static THEN_ICON_SERVED_AS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^that icon is served as "([^"]+)"$"#).unwrap());
static THEN_DECLARES_ICON_WITH_PURPOSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the manifest declares an icon with purpose "([^"]+)"$"#).unwrap()
});
static THEN_DECLARES_THEME_COLOUR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the page declares a theme colour$").unwrap());
static THEN_THEME_COLOUR_MATCHES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the page's theme colour matches the manifest's$").unwrap());
static THEN_NO_SERVICE_WORKER_REGISTERED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the page registers no service worker$").unwrap());
static THEN_NO_SERVICE_WORKER_SERVED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^no service worker script is served$").unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if THEN_LINKS_MANIFEST.is_match(text) {
        return Some(then_links_manifest(world));
    }
    if THEN_MANIFEST_LINK_SERVES_JSON.is_match(text) {
        return Some(then_manifest_link_serves_json(world).await);
    }
    if WHEN_MANIFEST_FETCHED.is_match(text) {
        return Some(when_manifest_fetched(world).await);
    }
    if let Some(caps) = THEN_MEMBER_IS.captures(text) {
        return Some(dispatch_member_is(world, example, &caps));
    }
    if let Some(caps) = THEN_DECLARES_ICON_OF_SIZE.captures(text) {
        return Some(dispatch_declares_icon_of_size(world, example, &caps));
    }
    if let Some(caps) = THEN_ICON_SERVED_AS.captures(text) {
        return Some(dispatch_icon_served_as(world, example, &caps).await);
    }
    if let Some(caps) = THEN_DECLARES_ICON_WITH_PURPOSE.captures(text) {
        return Some(dispatch_declares_icon_with_purpose(world, example, &caps));
    }
    if THEN_DECLARES_THEME_COLOUR.is_match(text) {
        return Some(then_declares_theme_colour(world));
    }
    if THEN_THEME_COLOUR_MATCHES.is_match(text) {
        return Some(then_theme_colour_matches(world).await);
    }
    if THEN_NO_SERVICE_WORKER_REGISTERED.is_match(text) {
        return Some(then_no_service_worker_registered(world));
    }
    if THEN_NO_SERVICE_WORKER_SERVED.is_match(text) {
        return Some(then_no_service_worker_served(world).await);
    }
    None
}

/// Resolves a captured value that may be a literal or an `Examples`
/// placeholder (`<size>`) written down verbatim in the step text. See
/// `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

struct Fetched {
    status: u16,
    content_type: Option<String>,
    body: Vec<u8>,
}

/// Sends a raw GET and reads the response as bytes, never through the
/// lossy UTF-8 path `inbox_view::html_response` takes -- an icon is binary,
/// and a manifest fetch that ran it through `String::from_utf8` would
/// corrupt any icon reusing this helper by accident.
async fn send_get(world: &mut World, path: &str) -> Result<axum::response::Response, String> {
    let pool = world.pool()?.clone();
    let app = trellis_server::platform::app::build_app(pool, world.clock());
    let request = Request::builder()
        .uri(path)
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    app.oneshot(request)
        .await
        .map_err(|e| format!("send request: {e}"))
}

async fn read_fetched(response: axum::response::Response) -> Result<Fetched, String> {
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| format!("read body: {e}"))?
        .to_vec();
    Ok(Fetched {
        status,
        content_type,
        body,
    })
}

async fn fetch(world: &mut World, path: &str) -> Result<Fetched, String> {
    let response = send_get(world, path).await?;
    read_fetched(response).await
}

/// The `href` a `<link rel="manifest" href="...">` in `body` names --
/// `T-qa-binds-tolerantly-to-markup`'s own reasoning applied here too: the
/// manifest's path is not the contract, the link is.
fn manifest_href(body: &str) -> Result<String, String> {
    let start_tag = r#"<link rel="manifest" href=""#;
    let start = body
        .find(start_tag)
        .ok_or_else(|| format!("expected a manifest <link> in the page, got:\n{body}"))?;
    let after = &body[start + start_tag.len()..];
    let end = after
        .find('"')
        .ok_or_else(|| "manifest <link> href has no closing quote".to_string())?;
    Ok(after[..end].to_string())
}

fn then_links_manifest(world: &mut World) -> Result<(), String> {
    let body = super::html_body(world, "no page response recorded")?;
    let href = manifest_href(body)?;
    world.last_manifest_href = Some(href);
    Ok(())
}

async fn then_manifest_link_serves_json(world: &mut World) -> Result<(), String> {
    let href = world
        .last_manifest_href
        .clone()
        .ok_or_else(|| "no manifest link recorded -- expected a prior link step".to_string())?;
    fetch_and_store_manifest(world, &href).await
}

async fn fetch_and_store_manifest(world: &mut World, href: &str) -> Result<(), String> {
    let fetched = fetch(world, href).await?;
    if fetched.status != 200 {
        return Err(format!(
            "expected the manifest at {href:?} to serve 200, got {}",
            fetched.status
        ));
    }
    let text = String::from_utf8(fetched.body)
        .map_err(|e| format!("manifest body at {href:?} was not UTF-8: {e}"))?;
    serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|e| format!("manifest at {href:?} did not parse as JSON: {e}\nbody:\n{text}"))?;
    world.last_manifest_body = Some(text);
    Ok(())
}

/// A standalone "the manifest is fetched" (scenarios with no preceding
/// "the page links a web app manifest" step): discovers the link the same
/// way, via Capture, rather than assuming the manifest's own path.
async fn when_manifest_fetched(world: &mut World) -> Result<(), String> {
    let fetched = fetch(world, "/").await?;
    let body = String::from_utf8(fetched.body)
        .map_err(|e| format!("Capture page body was not UTF-8: {e}"))?;
    let href = manifest_href(&body)?;
    world.last_manifest_href = Some(href.clone());
    fetch_and_store_manifest(world, &href).await
}

fn manifest_json(world: &World) -> Result<serde_json::Value, String> {
    let body = world
        .last_manifest_body
        .as_deref()
        .ok_or_else(|| "no manifest fetched yet".to_string())?;
    serde_json::from_str(body).map_err(|e| format!("stored manifest body was not JSON: {e}"))
}

fn dispatch_member_is(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let member = resolve(example, &caps[1])?;
    let expected = resolve(example, &caps[2])?;
    let manifest = manifest_json(world)?;
    let actual = manifest
        .get(&member)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("manifest has no string member {member:?}, got: {manifest}"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected manifest member {member:?} to be {expected:?}, got {actual:?}"
        ))
    }
}

fn icons(manifest: &serde_json::Value) -> Result<&Vec<serde_json::Value>, String> {
    manifest
        .get("icons")
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("manifest has no icons array, got: {manifest}"))
}

fn icon_has_size(icon: &serde_json::Value, size: &str) -> bool {
    icon.get("sizes").and_then(|v| v.as_str()) == Some(size)
}

fn icon_has_purpose(icon: &serde_json::Value, purpose: &str) -> bool {
    icon.get("purpose")
        .and_then(|v| v.as_str())
        .is_some_and(|value| value.split(' ').any(|word| word == purpose))
}

fn icon_src<'a>(icon: &'a serde_json::Value, describe: &str) -> Result<&'a str, String> {
    icon.get("src")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("icon {describe} has no src: {icon}"))
}

/// The one icon in `icons` `matches` picks out, or an error naming what was
/// asked for -- the shared shape behind both "declares an icon of size" and
/// "declares an icon with purpose", which differ only in `matches` and the
/// words their error names.
fn find_icon<'a>(
    icons: &'a [serde_json::Value],
    matches: impl Fn(&serde_json::Value) -> bool,
    describe: &str,
) -> Result<&'a serde_json::Value, String> {
    icons
        .iter()
        .find(|icon| matches(icon))
        .ok_or_else(|| format!("manifest declares no icon {describe}, got: {icons:?}"))
}

fn dispatch_declares_icon_of_size(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let size = resolve(example, &caps[1])?;
    let manifest = manifest_json(world)?;
    let icons = icons(&manifest)?;
    let icon = find_icon(
        icons,
        |icon| icon_has_size(icon, &size),
        &format!("of size {size:?}"),
    )?;
    world.last_icon_src = Some(icon_src(icon, &format!("of size {size:?}"))?.to_string());
    Ok(())
}

fn dispatch_declares_icon_with_purpose(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let purpose = resolve(example, &caps[1])?;
    let manifest = manifest_json(world)?;
    let icons = icons(&manifest)?;
    let icon = find_icon(
        icons,
        |icon| icon_has_purpose(icon, &purpose),
        &format!("with purpose {purpose:?}"),
    )?;
    world.last_icon_src = Some(icon_src(icon, &format!("with purpose {purpose:?}"))?.to_string());
    Ok(())
}

fn check_status_200(src: &str, fetched: &Fetched) -> Result<(), String> {
    if fetched.status == 200 {
        Ok(())
    } else {
        Err(format!(
            "expected the icon at {src:?} to serve 200, got {}",
            fetched.status
        ))
    }
}

fn check_content_type(src: &str, expected_type: &str, fetched: &Fetched) -> Result<(), String> {
    if fetched.content_type.as_deref() == Some(expected_type) {
        Ok(())
    } else {
        Err(format!(
            "expected the icon at {src:?} to serve as {expected_type:?}, got {:?}",
            fetched.content_type
        ))
    }
}

/// A manifest naming an icon that 404s falls back silently to a screenshot
/// in a real browser, so the status and content-type alone would not catch
/// it -- `qa/installable.md`'s own "check the bytes, not just the status".
fn check_png_signature(src: &str, fetched: &Fetched) -> Result<(), String> {
    if fetched.body.starts_with(&[0x89, b'P', b'N', b'G']) {
        Ok(())
    } else {
        Err(format!(
            "the icon at {src:?} claims image/png but its bytes are not a PNG signature \
             (got {} bytes starting {:?})",
            fetched.body.len(),
            &fetched.body[..fetched.body.len().min(8)]
        ))
    }
}

fn recorded_icon_src(world: &World) -> Result<String, String> {
    world
        .last_icon_src
        .clone()
        .ok_or_else(|| "no icon recorded -- expected a prior \"declares an icon\" step".to_string())
}

fn check_served_icon(src: &str, expected_type: &str, fetched: &Fetched) -> Result<(), String> {
    check_status_200(src, fetched)?;
    check_content_type(src, expected_type, fetched)?;
    if expected_type == "image/png" {
        check_png_signature(src, fetched)?;
    }
    Ok(())
}

async fn dispatch_icon_served_as(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected_type = resolve(example, &caps[1])?;
    let src = recorded_icon_src(world)?;
    let fetched = fetch(world, &src).await?;
    check_served_icon(&src, &expected_type, &fetched)
}

/// The `<meta name="theme-color" content="...">` a page names.
fn theme_colour(body: &str) -> Result<String, String> {
    let start_tag = r#"<meta name="theme-color" content=""#;
    let start = body
        .find(start_tag)
        .ok_or_else(|| format!("expected a theme-color <meta> in the page, got:\n{body}"))?;
    let after = &body[start + start_tag.len()..];
    let end = after
        .find('"')
        .ok_or_else(|| "theme-color <meta> content has no closing quote".to_string())?;
    Ok(after[..end].to_string())
}

fn then_declares_theme_colour(world: &mut World) -> Result<(), String> {
    let body = super::html_body(world, "no page response recorded")?;
    let colour = theme_colour(body)?;
    world.last_theme_colour = Some(colour);
    Ok(())
}

fn manifest_theme_colour(manifest: &serde_json::Value) -> Result<&str, String> {
    manifest
        .get("theme_color")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("manifest has no theme_color member, got: {manifest}"))
}

fn colours_match(page_colour: &str, manifest_colour: &str) -> Result<(), String> {
    if page_colour == manifest_colour {
        Ok(())
    } else {
        Err(format!(
            "expected the page's theme colour {page_colour:?} to match the manifest's {manifest_colour:?}"
        ))
    }
}

async fn then_theme_colour_matches(world: &mut World) -> Result<(), String> {
    let page_colour = world.last_theme_colour.clone().ok_or_else(|| {
        "no page theme colour recorded -- expected a prior declares-a-theme-colour step".to_string()
    })?;
    when_manifest_fetched(world).await?;
    let manifest = manifest_json(world)?;
    colours_match(&page_colour, manifest_theme_colour(&manifest)?)
}

fn then_no_service_worker_registered(world: &mut World) -> Result<(), String> {
    let body = super::html_body(world, "no page response recorded")?;
    if body.contains("serviceWorker") {
        Err(format!(
            "expected no service worker registration, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

/// Conventional service-worker paths -- there is no route table to consult
/// (that is the point: nothing should register one, so nothing should serve
/// one either), so this tries the paths a service worker would
/// conventionally live at and requires each to be absent.
const CONVENTIONAL_SERVICE_WORKER_PATHS: [&str; 2] = ["/service-worker.js", "/sw.js"];

async fn then_no_service_worker_served(world: &mut World) -> Result<(), String> {
    for path in CONVENTIONAL_SERVICE_WORKER_PATHS {
        let fetched = fetch(world, path).await?;
        if fetched.status == 200 {
            return Err(format!(
                "expected no service worker script at {path}, but it served 200"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_href_reads_the_link_out_of_the_page() {
        let body = r#"<link rel="manifest" href="/manifest.webmanifest">"#;
        assert_eq!(manifest_href(body), Ok("/manifest.webmanifest".to_string()));
    }

    #[test]
    fn manifest_href_errors_when_absent() {
        assert!(manifest_href("<html></html>").is_err());
    }

    #[test]
    fn theme_colour_reads_the_meta_tag() {
        let body = r##"<meta name="theme-color" content="#0f4e0f">"##;
        assert_eq!(theme_colour(body), Ok("#0f4e0f".to_string()));
    }

    #[test]
    fn theme_colour_errors_when_absent() {
        assert!(theme_colour("<html></html>").is_err());
    }

    #[tokio::test]
    async fn then_links_manifest_records_the_href() {
        let mut world = migrated_world().await;
        world.last_html_body =
            Some(r#"<link rel="manifest" href="/manifest.webmanifest">"#.to_string());
        then_links_manifest(&mut world).unwrap();
        assert_eq!(
            world.last_manifest_href.as_deref(),
            Some("/manifest.webmanifest")
        );
    }

    #[tokio::test]
    async fn then_manifest_link_serves_json_fetches_and_parses_it() {
        let mut world = migrated_world().await;
        world.last_manifest_href = Some("/manifest.webmanifest".to_string());
        then_manifest_link_serves_json(&mut world).await.unwrap();
        assert!(world.last_manifest_body.is_some());
        let manifest = manifest_json(&world).unwrap();
        assert_eq!(manifest["name"], "Trellis");
    }

    #[tokio::test]
    async fn when_manifest_fetched_discovers_the_link_via_capture() {
        let mut world = migrated_world().await;
        when_manifest_fetched(&mut world).await.unwrap();
        let manifest = manifest_json(&world).unwrap();
        assert_eq!(manifest["display"], "standalone");
    }

    #[tokio::test]
    async fn dispatch_declares_icon_of_size_finds_the_192_icon() {
        let mut world = migrated_world().await;
        when_manifest_fetched(&mut world).await.unwrap();
        let re = Regex::new(r#"^the manifest declares an icon of "([^"]+)"$"#).unwrap();
        let caps = re
            .captures(r#"the manifest declares an icon of "192x192""#)
            .unwrap();
        dispatch_declares_icon_of_size(&mut world, &BTreeMap::new(), &caps).unwrap();
        assert_eq!(
            world.last_icon_src.as_deref(),
            Some("/static/icons/icon-192.png")
        );
    }

    #[tokio::test]
    async fn icon_served_as_checks_content_type_and_png_bytes() {
        let mut world = migrated_world().await;
        world.last_icon_src = Some("/static/icons/icon-192.png".to_string());
        let re = Regex::new(r#"^that icon is served as "([^"]+)"$"#).unwrap();
        let caps = re
            .captures(r#"that icon is served as "image/png""#)
            .unwrap();
        dispatch_icon_served_as(&mut world, &BTreeMap::new(), &caps)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn icon_served_as_errors_when_the_icon_404s() {
        let mut world = migrated_world().await;
        world.last_icon_src = Some("/static/icons/does-not-exist.png".to_string());
        let re = Regex::new(r#"^that icon is served as "([^"]+)"$"#).unwrap();
        let caps = re
            .captures(r#"that icon is served as "image/png""#)
            .unwrap();
        assert!(dispatch_icon_served_as(&mut world, &BTreeMap::new(), &caps)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn then_no_service_worker_registered_passes_when_absent() {
        let mut world = migrated_world().await;
        world.last_html_body = Some("<html><body>hi</body></html>".to_string());
        then_no_service_worker_registered(&mut world).unwrap();
    }

    #[tokio::test]
    async fn then_no_service_worker_registered_fails_when_present() {
        let mut world = migrated_world().await;
        world.last_html_body =
            Some("<script>navigator.serviceWorker.register('/sw.js')</script>".to_string());
        assert!(then_no_service_worker_registered(&mut world).is_err());
    }

    #[tokio::test]
    async fn then_no_service_worker_served_passes_when_nothing_is_served() {
        let mut world = migrated_world().await;
        then_no_service_worker_served(&mut world).await.unwrap();
    }
}
