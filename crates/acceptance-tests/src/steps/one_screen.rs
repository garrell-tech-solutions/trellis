//! Step handlers for `features/one_screen.feature`: with #88's demolition
//! done, `/` is the only route, and there is nothing left to navigate
//! between (`one-screen-removed-routes-404-01`, `one-screen-no-header-02`).
//!
//! The Background ("the trellis server is running with an empty task list")
//! and "the inbox is viewed" steps this feature also uses are already
//! matched generically by [`super::triage::dispatch`] and
//! [`super::inbox_view::dispatch`], tried before this module.

use super::inbox_view::html_response;
use super::*;
use axum::body::Body;
use axum::http::Request;

static WHEN_PATH_REQUESTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the path "<(\w+)>" is requested$"#).unwrap());
static THEN_STATUS_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the response status is "<(\w+)>"$"#).unwrap());
static THEN_NO_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the page renders no navigation header$").unwrap());
static THEN_NO_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the page contains no link to another page$").unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = WHEN_PATH_REQUESTED.captures(text) {
        let path = match example_value(example, &caps[1]) {
            Ok(path) => path,
            Err(e) => return Some(Err(e)),
        };
        return Some(when_path_requested(world, path).await);
    }
    if let Some(caps) = THEN_STATUS_IS.captures(text) {
        let raw = match example_value(example, &caps[1]) {
            Ok(raw) => raw,
            Err(e) => return Some(Err(e)),
        };
        let expected: u16 = match raw.parse() {
            Ok(v) => v,
            Err(e) => return Some(Err(format!("bad status code: {e}"))),
        };
        return Some(then_status_is(world, expected));
    }
    if THEN_NO_HEADER.is_match(text) {
        return Some(then_no_header(world));
    }
    if THEN_NO_LINK.is_match(text) {
        return Some(then_no_link(world));
    }
    None
}

async fn when_path_requested(world: &mut World, path: &str) -> Result<(), String> {
    let request = Request::builder()
        .uri(path)
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

fn then_status_is(world: &mut World, expected: u16) -> Result<(), String> {
    super::then_status_is(world, expected, "no response recorded")
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no page response recorded")
}

/// **No `<header>` and no `<nav>`** (qa/one_screen.md's own wording): the
/// two elements `base.html` used to always render and now never does.
fn then_no_header(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains("<header") || body.contains("<nav") {
        Err(format!(
            "expected no header or nav in the page, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

/// Every page in this product used to reach another one only through the
/// header's own links, which are gone; nothing left renders an `<a>` at
/// all, so this checks the whole body rather than a scoped section.
fn then_no_link(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains("<a ") {
        Err(format!("expected no link in the page, got:\n{body}"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn when_path_requested_records_the_status_a_real_route_returns() {
        let mut world = migrated_world().await;

        when_path_requested(&mut world, "/").await.unwrap();

        assert_eq!(world.last_status, Some(200));
    }

    #[tokio::test]
    async fn when_path_requested_records_404_for_a_removed_route() {
        let mut world = migrated_world().await;

        when_path_requested(&mut world, "/stats").await.unwrap();

        assert_eq!(world.last_status, Some(404));
    }

    #[test]
    fn then_status_is_passes_for_a_matching_status() {
        let mut world = World::new();
        world.last_status = Some(404);
        assert_eq!(then_status_is(&mut world, 404), Ok(()));
    }

    #[test]
    fn then_status_is_errors_for_a_mismatched_status() {
        let mut world = World::new();
        world.last_status = Some(200);
        assert!(then_status_is(&mut world, 404).is_err());
    }

    #[test]
    fn then_no_header_passes_when_neither_element_is_present() {
        let mut world = World::new();
        world.last_html_body = Some("<main><h1>Trellis</h1></main>".to_string());
        assert_eq!(then_no_header(&mut world), Ok(()));
    }

    #[test]
    fn then_no_header_errors_when_a_header_is_present() {
        let mut world = World::new();
        world.last_html_body = Some("<header></header><main></main>".to_string());
        assert!(then_no_header(&mut world).is_err());
    }

    #[test]
    fn then_no_header_errors_when_a_nav_is_present_without_a_header() {
        let mut world = World::new();
        world.last_html_body = Some("<nav></nav><main></main>".to_string());
        assert!(then_no_header(&mut world).is_err());
    }

    #[test]
    fn then_no_link_passes_when_the_body_carries_no_anchor() {
        let mut world = World::new();
        world.last_html_body = Some("<main><h1>Trellis</h1></main>".to_string());
        assert_eq!(then_no_link(&mut world), Ok(()));
    }

    #[test]
    fn then_no_link_errors_when_an_anchor_is_present() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<a href="/stats">Stats</a>"#.to_string());
        assert!(then_no_link(&mut world).is_err());
    }
}
