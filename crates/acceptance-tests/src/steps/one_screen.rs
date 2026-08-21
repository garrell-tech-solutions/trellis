//! Step handlers for `features/one_screen.feature`: Trellis serves the
//! routes it has, and only those (`one-screen-routes-01`).
//!
//! The Background ("the trellis server is running with an empty task list")
//! is already matched generically by [`super::triage::dispatch`], tried
//! before this module.
//!
//! **The no-header scenario this module once served is gone, not moved
//! here again.** #92 restored the tab bar; its own
//! `pool-screen-tabs-08` now asserts what the header holds, in
//! `pool_screen.rs`.

use super::inbox_view::html_response;
use super::*;
use axum::body::Body;
use axum::http::Request;

static WHEN_PATH_REQUESTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the path "<(\w+)>" is requested$"#).unwrap());
static THEN_STATUS_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the response status is "<(\w+)>"$"#).unwrap());

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
}
