//! Step handlers for `features/inbox_view.feature`: the untriaged capture
//! queue rendered at `GET /` (D14's first slice, issue #30).
//!
//! Kept as its own module rather than added to `triage::dispatch`, which is
//! already over the project's complexity threshold (see the triage-validation
//! handoff brief's "known repo gotchas", still true here). The Background and
//! "a capture ... is waiting"/"the capture is triaged as a pool task" steps
//! this feature also uses are already matched generically by
//! [`super::triage::dispatch`], tried before this module.

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
        return Some(then_html_body_contains(world, &caps[1]));
    }
    if THEN_LISTS_NO_CAPTURES.is_match(text) {
        return Some(then_lists_no_captures(world));
    }
    if THEN_SHOWS_EMPTY_STATE.is_match(text) {
        return Some(then_html_body_contains(world, "Nothing to triage"));
    }
    if THEN_NO_UNESCAPED_SCRIPT.is_match(text) {
        return Some(then_html_body_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_CONTAINS_WORD.captures(text) {
        return Some(then_html_body_contains(world, &caps[1]));
    }
    None
}

async fn html_response(world: &mut World, request: Request<Body>) -> Result<(), String> {
    let pool = world.pool()?.clone();
    let app = trellis_server::app::build_app(pool);
    let response = app
        .oneshot(request)
        .await
        .map_err(|e| format!("send request: {e}"))?;
    world.last_status = Some(response.status().as_u16());
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| format!("read response body: {e}"))?;
    world.last_html_body = Some(
        String::from_utf8(bytes.to_vec()).map_err(|e| format!("response body not utf8: {e}"))?,
    );
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
/// handful of characters the hostile-text scenario actually submits — this
/// step handler is not a general-purpose form encoder.
fn urlencode(value: &str) -> String {
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
    match world.last_status {
        Some(status) if !(300..400).contains(&status) => Ok(()),
        Some(status) => Err(format!("expected no redirect, got status {status}")),
        None => Err("no quick-add response recorded".to_string()),
    }
}

fn html_body(world: &World) -> Result<&str, String> {
    world
        .last_html_body
        .as_deref()
        .ok_or_else(|| "no HTML response recorded".to_string())
}

fn then_html_body_contains(world: &mut World, expected: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected:?} in the response, got:\n{body}"
        ))
    }
}

fn then_html_body_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(forbidden) {
        Err(format!(
            "expected no {forbidden:?} in the response, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

fn then_lists_before(world: &mut World, first: &str, second: &str) -> Result<(), String> {
    let body = html_body(world)?;
    let first_index = body
        .find(first)
        .ok_or_else(|| format!("expected {first:?} in the response, got:\n{body}"))?;
    let second_index = body
        .find(second)
        .ok_or_else(|| format!("expected {second:?} in the response, got:\n{body}"))?;
    if first_index < second_index {
        Ok(())
    } else {
        Err(format!(
            "expected {first:?} to list before {second:?}, got:\n{body}"
        ))
    }
}

fn then_lists_no_captures(world: &mut World) -> Result<(), String> {
    then_html_body_excludes(world, "<li>")
}

#[cfg(test)]
mod tests {
    use super::*;

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
