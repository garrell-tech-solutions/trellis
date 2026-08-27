//! Step handlers for `features/inbox_view.feature`: the capture page is a
//! box and what is recent (#140, D-visible-slices' first slice, issue #30).
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
//! had zero captures and one task, which is not the same claim. #140 deletes
//! that second list, but the scoping stays: "the row for ..." and "the inbox
//! ..." steps are about `Recent`, and "the capture page shows ... once" is
//! the one check that deliberately looks at the whole page instead
//! (`inbox-view-triaged-row-stays-04`'s own guard against a second list
//! coming back).

use super::html;
use super::payloads;
use super::*;
use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
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
static WHEN_TRIAGED_AS_KIND_TAGGED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a "([^"]+)" task tagged "([^"]*)"$"#).unwrap()
});
static THEN_ROW_READS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the row for "([^"]+)" reads "([^"]+)"$"#).unwrap());
static THEN_OFFERS_NO_KIND_BUTTONS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the row for "([^"]+)" offers no kind buttons$"#).unwrap());
static THEN_SHOWN_ONCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the capture page shows "([^"]+)" once$"#).unwrap());
static GIVEN_TRIAGED_IN_ORDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the captures "([^"]+)" have each been triaged as pool tasks in that order$"#)
        .unwrap()
});
static GIVEN_ROW_FOR_IT_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the row for it is "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
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
        return Some(dispatch_lists(world, example, &caps));
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
    if let Some(caps) = WHEN_TRIAGED_AS_KIND_TAGGED.captures(text) {
        return Some(dispatch_triaged_as_kind_tagged(world, example, &caps).await);
    }
    if let Some(caps) = THEN_ROW_READS.captures(text) {
        return Some(dispatch_row_reads(world, example, &caps));
    }
    if let Some(caps) = THEN_OFFERS_NO_KIND_BUTTONS.captures(text) {
        return Some(dispatch_offers_no_kind_buttons(world, example, &caps));
    }
    if let Some(caps) = THEN_SHOWN_ONCE.captures(text) {
        return Some(dispatch_shown_once(world, example, &caps));
    }
    if let Some(caps) = GIVEN_TRIAGED_IN_ORDER.captures(text) {
        return Some(dispatch_triaged_in_order(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_ROW_FOR_IT_IS.captures(text) {
        return Some(dispatch_row_for_it_is(world, example, &caps).await);
    }
    None
}

/// Resolves a captured value that may be a literal or an `Examples`
/// placeholder (`<listed>`) written down verbatim in the step text. See
/// `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// [`resolve`]'s counterpart for a captured value that carries a placeholder
/// alongside literal text (`"Pool · <tag>"`) rather than being one on its
/// own. Every `<name>` substring is swapped for its example value when
/// `name` is an actual placeholder in this row; a `<name>` whose `name` is
/// not one is left untouched, so hostile text that merely looks like a tag
/// (`<script>`, with no such example column) still renders back literally.
fn resolve_embedded(example: &BTreeMap<String, String>, raw: &str) -> String {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<(\w+)>").unwrap());
    PLACEHOLDER
        .replace_all(raw, |caps: &regex::Captures| {
            example
                .get(&caps[1])
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
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

/// "The inbox lists ..." names either one item (plain containment, the
/// long-standing meaning) or several, comma-separated
/// (`inbox-view-three-most-recent-05`'s own `"<listed>"` -- several rows in
/// one strict-recency order). A single name is exactly the one-item case of
/// the same check, so this replaces the old plain-containment handler rather
/// than sitting beside it.
fn dispatch_lists(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let section = captures_section(world)?;
    let mut last_index: Option<usize> = None;
    for item in expected.split(", ") {
        let index = index_of_or_error(section, item)?;
        if let Some(last) = last_index {
            if index <= last {
                return Err(format!(
                    "expected {expected:?} in that order, got:\n{section}"
                ));
            }
        }
        last_index = Some(index);
    }
    Ok(())
}

/// A committed triage needs more than a kind and a tag to be well-formed;
/// this fills the rest with the same valid defaults `payloads::committed`
/// always has, since `inbox-view-triaged-row-stays-04`'s own Examples table
/// varies only kind, tag and the meta it expects back -- the deadline,
/// commitment, priority and estimate are not what that scenario is testing.
fn triage_payload(kind: &str, tag: &str) -> Result<Value, String> {
    let mut body = match kind {
        "pool" => payloads::pool(),
        "committed" => payloads::committed(),
        "quota" => payloads::quota(),
        other => return Err(format!("unknown kind {other:?}")),
    };
    if !tag.is_empty() {
        body = payloads::with_field(body, "context_tag", json!(tag));
    }
    Ok(body)
}

async fn dispatch_triaged_as_kind_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let kind = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    let body = triage_payload(&kind, &tag)?;
    super::triage::when_triaged(world, body).await
}

fn dispatch_row_reads(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let expected = resolve_embedded(example, &caps[2]);
    let section = captures_section(world)?;
    let row = html::row_containing(section, &raw_text)?;
    let meta = html::between(row, r#"<div class="row-meta">"#, "</div>")?;
    if meta == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the row for {raw_text:?} to read {expected:?}, got {meta:?}"
        ))
    }
}

fn dispatch_offers_no_kind_buttons(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let section = captures_section(world)?;
    let row = html::row_containing(section, &raw_text)?;
    if row.contains(r#"class="kinds""#) {
        Err(format!(
            "expected no kind buttons for {raw_text:?}, got:\n{row}"
        ))
    } else {
        Ok(())
    }
}

/// Scoped to the *whole* response body, not [`captures_section`] --
/// `inbox-view-triaged-row-stays-04`'s own guard against the deleted `Tasks`
/// list coming back and duplicating a triaged row somewhere else on the
/// page. A check confined to `#captures` could not see a second occurrence
/// living outside it.
fn dispatch_shown_once(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let count = body.matches(raw_text.as_str()).count();
    if count == 1 {
        Ok(())
    } else {
        Err(format!(
            "expected {raw_text:?} to appear exactly once, found {count}, in:\n{body}"
        ))
    }
}

/// Captures and triages each name in `captured`, in order -- real writes
/// through the real endpoints, timestamped by the world's own clock, so
/// "in that order" is the actual recency `list_recent`'s `ORDER BY` reads
/// rather than an order this fixture asserts by construction.
async fn dispatch_triaged_in_order(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let captured = resolve(example, &caps[1])?;
    for raw_text in captured.split(", ") {
        super::triage::given_capture_waiting(world, raw_text).await?;
        super::triage::when_triaged(world, payloads::pool()).await?;
    }
    Ok(())
}

/// `"triaged"` files the most recently captured row (pool, the cheapest
/// kind) as `inbox-view-escapes-hostile-text-07`'s own second row state;
/// `"untriaged"` leaves it exactly as the preceding "is waiting" step left
/// it.
async fn dispatch_row_for_it_is(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let state = resolve(example, &caps[1])?;
    match state.as_str() {
        "triaged" => super::triage::when_triaged(world, payloads::pool()).await,
        "untriaged" => Ok(()),
        other => Err(format!("unknown row state {other:?}")),
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
    fn resolve_embedded_substitutes_a_placeholder_alongside_literal_text() {
        let example = super::super::example(&[("tag", "@homedepot")]);
        assert_eq!(
            resolve_embedded(&example, "Pool · <tag>"),
            "Pool · @homedepot"
        );
    }

    #[test]
    fn resolve_embedded_leaves_a_whole_value_placeholder_working_too() {
        let example = super::super::example(&[("meta", "Pool · @homedepot")]);
        assert_eq!(resolve_embedded(&example, "<meta>"), "Pool · @homedepot");
    }

    #[test]
    fn resolve_embedded_leaves_hostile_text_with_no_matching_placeholder_untouched() {
        let example = BTreeMap::new();
        let hostile = "<script>alert('boom')</script>";
        assert_eq!(resolve_embedded(&example, hostile), hostile);
    }

    #[test]
    fn dispatch_row_reads_resolves_an_embedded_placeholder_in_the_expected_text() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="captures"><li id="capture-row-1"><div class="row-text">buy screws</div><div class="row-meta">Pool · @homedepot</div></li></ul>"#
                .to_string(),
        );
        let example = super::super::example(&[("tag", "@homedepot")]);
        let caps = THEN_ROW_READS
            .captures(r#"the row for "buy screws" reads "Pool · <tag>""#)
            .unwrap();
        assert_eq!(dispatch_row_reads(&mut world, &example, &caps), Ok(()));
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
