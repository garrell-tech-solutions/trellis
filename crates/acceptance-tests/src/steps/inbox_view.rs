//! Step handlers for `features/inbox_view.feature`: the untriaged capture
//! queue rendered at `GET /` (D-visible-slices' first slice, issue #30).
//!
//! Kept as its own module rather than added to `triage::dispatch`, which is
//! already over the project's complexity threshold (see the triage-validation
//! handoff brief's "known repo gotchas", still true here). The Background and
//! "a capture ... is waiting"/"the capture is triaged as a pool task" steps
//! this feature also uses are already matched generically by
//! [`super::triage::dispatch`], tried before this module.
//!
//! "The inbox ..." assertions are scoped to `<ul id="captures">` via
//! [`super::html`] rather than checked against the whole page body — once
//! `triage_from_page` (#33) added a second list to the same page, a
//! whole-body check for "no captures" started passing against a page that
//! had zero captures and one task, which is not the same claim.

use super::html;
use super::*;
use axum::body::{to_bytes, Body};
use axum::http::Request;
use tower::ServiceExt;

static WHEN_INBOX_VIEWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the inbox is viewed$").unwrap());
static WHEN_QUICK_ADD_SUBMITS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the quick-add box submits a capture with raw text "([^"]+)"$"#).unwrap()
});
static THEN_NOT_REDIRECT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the quick-add submission does not redirect the browser$").unwrap()
});
static THEN_QUICK_ADD_RESPONSE_INCLUDES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the quick-add response includes "([^"]+)"$"#).unwrap());
static THEN_LISTS_BEFORE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the inbox lists "([^"]+)" before "([^"]+)"$"#).unwrap());
static THEN_LISTS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the inbox lists "([^"]+)"$"#).unwrap());
static THEN_LISTS_NO_CAPTURES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the inbox lists no captures$").unwrap());
static THEN_SHOWS_EMPTY_STATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the inbox shows an empty-state message$").unwrap());
static THEN_NO_UNESCAPED_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the inbox does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the inbox contains the word "([^"]+)"$"#).unwrap());

pub async fn dispatch(world: &mut World, text: &str) -> Option<Result<(), String>> {
    if WHEN_INBOX_VIEWED.is_match(text) {
        return Some(when_inbox_viewed(world).await);
    }
    if let Some(caps) = WHEN_QUICK_ADD_SUBMITS.captures(text) {
        return Some(when_quick_add_submits(world, &caps[1]).await);
    }
    if THEN_NOT_REDIRECT.is_match(text) {
        return Some(then_not_redirect(world));
    }
    if let Some(caps) = THEN_QUICK_ADD_RESPONSE_INCLUDES.captures(text) {
        return Some(then_html_body_contains(world, &caps[1]));
    }
    if let Some(caps) = THEN_LISTS_BEFORE.captures(text) {
        return Some(then_lists_before(world, &caps[1], &caps[2]));
    }
    if let Some(caps) = THEN_LISTS.captures(text) {
        return Some(then_captures_section_contains(world, &caps[1]));
    }
    if THEN_LISTS_NO_CAPTURES.is_match(text) {
        return Some(then_lists_no_captures(world));
    }
    if THEN_SHOWS_EMPTY_STATE.is_match(text) {
        return Some(then_html_body_contains(world, "Nothing to triage"));
    }
    if THEN_NO_UNESCAPED_SCRIPT.is_match(text) {
        return Some(then_captures_section_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_CONTAINS_WORD.captures(text) {
        return Some(then_captures_section_contains(world, &caps[1]));
    }
    None
}

async fn response_body_string(response: axum::response::Response) -> Result<String, String> {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| format!("read response body: {e}"))?;
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("response body not utf8: {e}"))
}

pub(super) async fn html_response(world: &mut World, request: Request<Body>) -> Result<(), String> {
    let pool = world.pool()?.clone();
    let app = trellis_server::platform::app::build_app(pool, world.clock());
    let response = app
        .oneshot(request)
        .await
        .map_err(|e| format!("send request: {e}"))?;
    world.last_status = Some(response.status().as_u16());
    world.last_html_body = Some(response_body_string(response).await?);
    Ok(())
}

async fn when_inbox_viewed(world: &mut World) -> Result<(), String> {
    let request = Request::builder()
        .uri("/")
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

async fn when_quick_add_submits(world: &mut World, raw_text: &str) -> Result<(), String> {
    let body = format!("raw_text={}&source=web", urlencode(raw_text));
    let request = Request::builder()
        .method("POST")
        .uri("/captures")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

/// Minimal `application/x-www-form-urlencoded` percent-encoding for the
/// handful of characters the acceptance suite actually submits — this step
/// handler is not a general-purpose form encoder.
pub(super) fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn then_not_redirect(world: &mut World) -> Result<(), String> {
    super::then_not_redirect(world, "no quick-add response recorded")
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no HTML response recorded")
}

fn then_html_body_contains(world: &mut World, expected: &str) -> Result<(), String> {
    super::then_html_body_contains(world, expected, "no HTML response recorded")
}

/// [`html::captures_section`], falling back to the whole body when there is
/// no `<ul id="captures">` wrapper to scope to. A quick-add response swaps
/// `#captures` from the *outside* (`hx-swap="afterbegin"` on the request),
/// so it is itself just the new row's own markup -- never wrapped in the
/// `<ul>` a full page carries. Falling back to the whole body is exactly
/// what a real `<ul id="captures">` element would have scoped to anyway,
/// one level up.
fn captures_section(world: &World) -> Result<&str, String> {
    let body = html_body(world)?;
    Ok(html::captures_section(body).unwrap_or(body))
}

fn then_captures_section_contains(world: &mut World, expected: &str) -> Result<(), String> {
    let section = captures_section(world)?;
    if section.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected:?} in the inbox, got:\n{section}"
        ))
    }
}

fn then_captures_section_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let section = captures_section(world)?;
    if section.contains(forbidden) {
        Err(format!(
            "expected no {forbidden:?} in the inbox, got:\n{section}"
        ))
    } else {
        Ok(())
    }
}

fn index_of_or_error(section: &str, needle: &str) -> Result<usize, String> {
    section
        .find(needle)
        .ok_or_else(|| format!("expected {needle:?} in the inbox, got:\n{section}"))
}

fn then_lists_before(world: &mut World, first: &str, second: &str) -> Result<(), String> {
    let section = captures_section(world)?;
    let first_index = index_of_or_error(section, first)?;
    let second_index = index_of_or_error(section, second)?;
    if first_index < second_index {
        Ok(())
    } else {
        Err(format!(
            "expected {first:?} to list before {second:?}, got:\n{section}"
        ))
    }
}

/// Emptiness is checked by content, not by the absence of an `<li` tag — the
/// inbox's own `<li>` grew an `id` attribute once triage-from-page needed one
/// to aim its controls at, so `<li>` alone no longer appears even when a
/// capture is present.
fn then_lists_no_captures(world: &mut World) -> Result<(), String> {
    let section = captures_section(world)?;
    if section.trim().is_empty() {
        Ok(())
    } else {
        Err(format!("expected no captures listed, got:\n{section}"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::triage::given_capture_waiting;
    use super::*;

    #[tokio::test]
    async fn the_inbox_shows_the_empty_state_message_when_there_is_nothing_to_triage() {
        let mut world = migrated_world().await;

        when_inbox_viewed(&mut world).await.unwrap();

        then_html_body_contains(&mut world, "Nothing to triage").unwrap();
    }

    #[tokio::test]
    async fn the_inbox_lists_a_waiting_capture() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();

        when_inbox_viewed(&mut world).await.unwrap();

        then_captures_section_contains(&mut world, "buy milk").unwrap();
    }

    #[test]
    fn then_captures_section_contains_falls_back_to_the_whole_body_for_a_bare_row_response() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<li id="capture-row-1">buy screws @homedepot</li><datalist id="context-tag-suggestions" hx-swap-oob="true"></datalist>"#
                .to_string(),
        );

        then_captures_section_contains(&mut world, "buy screws").unwrap();
    }

    #[tokio::test]
    async fn the_inbox_lists_no_captures_ignores_the_task_lists_own_li_elements() {
        let mut world = migrated_world().await;

        when_inbox_viewed(&mut world).await.unwrap();
        world.last_html_body = Some(
            r#"<ul id="captures"></ul><ul id="tasks"><li>[pool] call the dentist</li></ul>"#
                .to_string(),
        );

        then_lists_no_captures(&mut world).unwrap();
    }

    #[test]
    fn then_not_redirect_passes_for_a_non_redirect_status() {
        let mut world = World::new();
        world.last_status = Some(200);
        assert_eq!(then_not_redirect(&mut world), Ok(()));
    }

    #[test]
    fn then_not_redirect_errors_for_a_redirect_status() {
        let mut world = World::new();
        world.last_status = Some(302);
        assert!(then_not_redirect(&mut world).is_err());
    }

    #[test]
    fn then_not_redirect_errors_when_no_response_was_recorded() {
        let mut world = World::new();
        assert!(then_not_redirect(&mut world).is_err());
    }

    fn world_with_captures_section(html: &str) -> World {
        let mut world = World::new();
        world.last_html_body = Some(format!(r#"<ul id="captures">{html}</ul>"#));
        world
    }

    #[test]
    fn then_lists_before_passes_when_the_first_name_appears_first() {
        let mut world = world_with_captures_section("<li>buy milk</li><li>call the dentist</li>");
        assert_eq!(
            then_lists_before(&mut world, "buy milk", "call the dentist"),
            Ok(())
        );
    }

    #[test]
    fn then_lists_before_errors_when_the_order_is_reversed() {
        let mut world = world_with_captures_section("<li>buy milk</li><li>call the dentist</li>");
        assert!(then_lists_before(&mut world, "call the dentist", "buy milk").is_err());
    }

    #[test]
    fn then_lists_before_errors_when_a_name_is_missing() {
        let mut world = world_with_captures_section("<li>buy milk</li>");
        assert!(then_lists_before(&mut world, "buy milk", "call the dentist").is_err());
    }

    #[test]
    fn urlencode_leaves_alphanumerics_untouched() {
        assert_eq!(urlencode("buymilk123"), "buymilk123");
    }

    #[test]
    fn urlencode_encodes_a_space_as_a_plus() {
        assert_eq!(urlencode("buy milk"), "buy+milk");
    }

    #[test]
    fn urlencode_percent_encodes_hostile_markup() {
        assert_eq!(
            urlencode("<script>alert('boom')</script>"),
            "%3Cscript%3Ealert%28%27boom%27%29%3C%2Fscript%3E"
        );
    }
}
