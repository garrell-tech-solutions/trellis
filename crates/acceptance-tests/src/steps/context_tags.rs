//! Step handlers for `features/context_tags.feature`: a capture carries a
//! context tag, and the tags autocomplete on what came before (#82).
//!
//! The Background ("the trellis server is running with an empty task list"),
//! plain (untagged) capture and triage steps, and the read-only assertions
//! this feature shares with others ("the inbox lists ...", "the inbox does
//! not list ...", "the inbox does not contain an unescaped ...", "the inbox
//! contains the word ...") are already matched generically by
//! [`super::triage::dispatch`], [`super::inbox_view::dispatch`] and
//! [`super::triage_from_page::dispatch`], tried before this module.
//!
//! Several scenarios name a tag via a literal `Examples` placeholder
//! (`tagged "<tag>"`) and this Gherkin runner does not pre-substitute
//! those — a step's own regex sees the literal text `<tag>` and must
//! resolve it against the current example row itself, the pattern every
//! other step module already follows. [`resolve`] does that, and is
//! careful to require the *whole* captured value look like `<name>` before
//! treating it as a placeholder: `context-tags-escapes-hostile-text-09`'s
//! own tag, `<script>alert('boom')</script>`, both starts and ends with an
//! angle bracket without being one.

use super::html;
use super::inbox_view::{html_response, urlencode};
use super::payloads;
use super::triage::when_triaged;
use super::*;
use axum::body::Body;
use axum::http::Request;
use serde_json::Value;

static GIVEN_TAGGED_CAPTURE_WAITING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^a capture with raw text "([^"]+)" tagged "([^"]+)" is waiting in the untriaged queue$"#,
    )
    .unwrap()
});
static WHEN_QUICK_ADD_TAGGED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the quick-add box submits a capture with raw text "([^"]+)" tagged "([^"]+)"$"#)
        .unwrap()
});
static WHEN_QUICK_ADD_NO_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the quick-add box submits a capture with raw text "([^"]+)" with no context tag$"#,
    )
    .unwrap()
});
static WHEN_TRIAGED_TAGGED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a pool task tagged "([^"]+)"$"#).unwrap()
});
static THEN_QUICK_ADD_ACCEPTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the quick-add submission is accepted$").unwrap());
static THEN_INBOX_LISTS_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the inbox lists "([^"]+)" tagged "([^"]+)"$"#).unwrap());
static THEN_INBOX_SHOWS_NO_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the inbox shows no context tag for "([^"]+)"$"#).unwrap());
static THEN_SUGGESTIONS_EXACTLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the context tag suggestions are exactly "([^"]+)"$"#).unwrap());
static THEN_TASK_LIST_SHOWS_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the task list shows "([^"]+)" tagged "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = GIVEN_TAGGED_CAPTURE_WAITING.captures(text) {
        return Some(dispatch_given_tagged_capture_waiting(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_QUICK_ADD_TAGGED.captures(text) {
        return Some(dispatch_quick_add_tagged(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_QUICK_ADD_NO_TAG.captures(text) {
        return Some(quick_add(world, &caps[1], None).await);
    }
    if let Some(caps) = WHEN_TRIAGED_TAGGED.captures(text) {
        return Some(dispatch_triaged_tagged(world, example, &caps).await);
    }
    if THEN_QUICK_ADD_ACCEPTED.is_match(text) {
        return Some(then_status_is(world, 201, "no quick-add response recorded"));
    }
    if let Some(caps) = THEN_INBOX_LISTS_TAGGED.captures(text) {
        return Some(dispatch_inbox_lists_tagged(world, example, &caps));
    }
    if let Some(caps) = THEN_INBOX_SHOWS_NO_TAG.captures(text) {
        return Some(then_inbox_shows_no_tag(world, &caps[1]));
    }
    if let Some(caps) = THEN_SUGGESTIONS_EXACTLY.captures(text) {
        return Some(dispatch_suggestions_exactly(world, example, &caps));
    }
    if let Some(caps) = THEN_TASK_LIST_SHOWS_TAGGED.captures(text) {
        return Some(dispatch_task_list_shows_tagged(world, example, &caps));
    }
    None
}

/// Resolves a captured value that may be a literal (`@homedepot`, `   `,
/// even `<script>alert('boom')</script>`) or an `Examples` placeholder
/// (`<tag>`) written down verbatim in the step text. Only a value that is
/// *entirely* `<` + word characters + `>` counts as a placeholder — a
/// hostile string that merely starts and ends with an angle bracket must
/// not be mistaken for one.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

async fn dispatch_given_tagged_capture_waiting(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    let pool = world.pool()?.clone();
    let id = trellis_server::capture::store::insert(&pool, &raw_text, "web", 0)
        .await
        .map_err(|e| format!("insert capture: {e}"))?;
    trellis_server::capture::store::set_context_tag(&pool, id, &tag)
        .await
        .map_err(|e| format!("set context tag: {e}"))?;
    world.last_capture_id = Some(id);
    Ok(())
}

async fn dispatch_quick_add_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    quick_add(world, &raw_text, Some(&tag)).await
}

async fn dispatch_triaged_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let body = payloads::with_field(payloads::pool(), "context_tag", Value::from(tag));
    when_triaged(world, body).await
}

/// Scoped to `row-text`, not the whole `<li>` — the row's own triage forms
/// carry `placeholder="@homedepot"` on their `context_tag` inputs
/// regardless of this capture's actual tag (see [`then_inbox_shows_no_tag`]),
/// so a whole-row check for `@homedepot` specifically would pass whether or
/// not the tag actually rendered.
/// [`html::captures_section`], falling back to the whole body when there is
/// no `<ul id="captures">` wrapper to scope to. Several scenarios assert
/// straight off a quick-add response, which swaps `#captures` from the
/// *outside* (`hx-swap="afterbegin"` on the request) and so is itself just
/// the new row's own markup plus the OOB datalist -- never wrapped in the
/// `<ul>` a full page or a `#lists` re-render carries. Falling back to the
/// whole body is exactly what a real `<ul id="captures">` element would have
/// scoped to anyway, one level up.
fn captures_scope(body: &str) -> &str {
    html::captures_section(body).unwrap_or(body)
}

fn dispatch_inbox_lists_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    let section = captures_scope(html_body(world)?);
    let row = html::row_containing(section, &raw_text)?;
    let row_text = html::between(row, r#"<div class="row-text">"#, "</div>")?;
    then_row_contains(row_text, &raw_text, &tag)
}

fn dispatch_task_list_shows_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    let section = html::tasks_section(html_body(world)?)?;
    let row = html::row_containing(section, &raw_text)?;
    then_row_contains(row, &raw_text, &tag)
}

fn then_row_contains(row: &str, raw_text: &str, tag: &str) -> Result<(), String> {
    if row.contains(tag) {
        Ok(())
    } else {
        Err(format!(
            "expected the row for {raw_text:?} to carry the tag {tag:?}, got:\n{row}"
        ))
    }
}

fn dispatch_suggestions_exactly(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected: Vec<String> = resolve(example, &caps[1])?
        .split(", ")
        .map(str::to_string)
        .collect();
    let actual = suggested_tags(html_body(world)?)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected context tag suggestions {expected:?}, got {actual:?}"
        ))
    }
}

/// **Cannot be a bare substring check.** Every capture row's triage forms
/// carry `placeholder="@homedepot"` on their own `context_tag` input
/// regardless of whether this capture has one, so any tag-shaped string is
/// already present in an untagged row's markup. What "no tag" means is
/// scoped to `row-text` specifically, which renders nothing but
/// `capture.text` when `capture.context_tag` is `None`
/// (`{% if let Some(tag) = capture.context_tag %} {{ tag }}{% endif %}`) —
/// so the row's text is exactly `raw_text`, nothing appended.
fn then_inbox_shows_no_tag(world: &mut World, raw_text: &str) -> Result<(), String> {
    let section = captures_scope(html_body(world)?);
    let row = html::row_containing(section, raw_text)?;
    let row_text = html::between(row, r#"<div class="row-text">"#, "</div>")?;
    if row_text.trim() == raw_text {
        Ok(())
    } else {
        Err(format!(
            "expected {raw_text:?} to carry no context tag, got row text {row_text:?}"
        ))
    }
}

/// Every `<option value="...">` inside the suggestions `<datalist>`, in
/// document order — what "the context tag suggestions are exactly ..."
/// means. Scoped to the datalist rather than the whole body: the datalist's
/// own `<option>` markup is otherwise indistinguishable from a `<select>`
/// picker's, if this product ever grows one again.
///
/// Found by `id="..."` rather than a fixed opening tag: the OOB fragment's
/// datalist carries an extra `hx-swap-oob="true"` attribute the page
/// render's own copy does not, and both are "the suggestions datalist" as
/// far as this helper is concerned.
/// The datalist's own markup, from its opening tag's `id="..."` attribute
/// (wherever it falls among the tag's other attributes) to its closing tag —
/// the section [`suggested_tags`] reads `<option>`s out of.
fn suggestions_datalist_section(body: &str) -> Result<&str, String> {
    let marker = body
        .find(r#"id="context-tag-suggestions""#)
        .ok_or_else(|| format!("expected a context-tag-suggestions datalist in:\n{body}"))?;
    html::between(&body[marker..], ">", "</datalist>")
}

fn suggested_tags(body: &str) -> Result<Vec<String>, String> {
    let section = suggestions_datalist_section(body)?;
    Ok(section
        .split("<option value=\"")
        .skip(1)
        .filter_map(|chunk| chunk.split_once('"').map(|(value, _)| value.to_string()))
        .collect())
}

async fn quick_add(world: &mut World, raw_text: &str, tag: Option<&str>) -> Result<(), String> {
    let mut body = format!("raw_text={}&source=web", urlencode(raw_text));
    if let Some(tag) = tag {
        body.push_str(&format!("&context_tag={}", urlencode(tag)));
    }
    let request = Request::builder()
        .method("POST")
        .uri("/captures")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no page response recorded")
}

fn then_status_is(
    world: &mut World,
    expected: u16,
    no_response_message: &str,
) -> Result<(), String> {
    super::then_status_is(world, expected, no_response_message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_a_literal_value_unchanged() {
        let example = BTreeMap::new();
        assert_eq!(
            resolve(&example, "@homedepot"),
            Ok("@homedepot".to_string())
        );
    }

    #[test]
    fn resolve_looks_up_a_placeholder_in_the_example_row() {
        let example = super::super::example(&[("tag", "@homedepot")]);
        assert_eq!(resolve(&example, "<tag>"), Ok("@homedepot".to_string()));
    }

    #[test]
    fn resolve_errors_when_a_placeholder_names_no_column() {
        let example = BTreeMap::new();
        assert!(resolve(&example, "<tag>").is_err());
    }

    #[test]
    fn resolve_does_not_mistake_hostile_text_for_a_placeholder() {
        let example = BTreeMap::new();
        let hostile = "<script>alert('boom')</script>";
        assert_eq!(resolve(&example, hostile), Ok(hostile.to_string()));
    }

    #[test]
    fn resolve_passes_through_whitespace_only_text() {
        let example = BTreeMap::new();
        assert_eq!(resolve(&example, "   "), Ok("   ".to_string()));
    }

    #[tokio::test]
    async fn given_tagged_capture_waiting_stores_the_tag() {
        let mut world = migrated_world().await;
        let example = BTreeMap::new();
        let re = GIVEN_TAGGED_CAPTURE_WAITING.captures(
            r#"a capture with raw text "buy screws" tagged "@homedepot" is waiting in the untriaged queue"#,
        ).unwrap();

        dispatch_given_tagged_capture_waiting(&mut world, &example, &re)
            .await
            .unwrap();

        let pool = world.pool().unwrap().clone();
        let tag: Option<String> = sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tag.as_deref(), Some("@homedepot"));
        assert!(world.last_capture_id.is_some());
    }

    #[tokio::test]
    async fn quick_add_with_a_tag_saves_it() {
        let mut world = migrated_world().await;

        quick_add(&mut world, "buy screws", Some("@homedepot"))
            .await
            .unwrap();

        assert_eq!(world.last_status, Some(201));
        let pool = world.pool().unwrap().clone();
        let tag: Option<String> = sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tag.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn quick_add_with_no_tag_is_accepted_and_stores_none() {
        let mut world = migrated_world().await;

        quick_add(&mut world, "buy screws", None).await.unwrap();

        assert_eq!(world.last_status, Some(201));
        let pool = world.pool().unwrap().clone();
        let tag: Option<String> = sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tag, None);
    }

    #[tokio::test]
    async fn quick_add_response_includes_an_out_of_band_suggestions_datalist() {
        let mut world = migrated_world().await;

        quick_add(&mut world, "buy screws", Some("@homedepot"))
            .await
            .unwrap();

        let body = html_body(&world).unwrap();
        assert!(body.contains("hx-swap-oob=\"true\""));
        assert_eq!(
            suggested_tags(body).unwrap(),
            vec!["@homedepot".to_string()]
        );
    }

    #[tokio::test]
    async fn dispatch_triaged_tagged_writes_the_tag_onto_the_capture() {
        let mut world = migrated_world().await;
        super::super::triage::given_capture_waiting(&mut world, "buy screws")
            .await
            .unwrap();
        let example = BTreeMap::new();
        let re = WHEN_TRIAGED_TAGGED
            .captures(r#"the capture is triaged as a pool task tagged "@homedepot""#)
            .unwrap();

        dispatch_triaged_tagged(&mut world, &example, &re)
            .await
            .unwrap();

        assert_eq!(world.last_status, Some(201));
        let pool = world.pool().unwrap().clone();
        let tag: Option<String> = sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tag.as_deref(), Some("@homedepot"));
    }

    fn world_with_captures_section(html: &str) -> World {
        let mut world = World::new();
        world.last_html_body = Some(format!(r#"<ul id="captures">{html}</ul>"#));
        world
    }

    #[test]
    fn captures_scope_uses_the_wrapped_section_when_one_is_present() {
        let body = r#"<ul id="captures"><li>buy milk</li></ul><ul id="tasks"><li>[pool] call the dentist</li></ul>"#;
        let scope = captures_scope(body);
        assert!(scope.contains("buy milk"));
        assert!(!scope.contains("call the dentist"));
    }

    #[test]
    fn captures_scope_falls_back_to_the_whole_body_for_a_bare_row_response() {
        let body = r#"<li id="capture-row-1">buy screws @homedepot</li><datalist id="context-tag-suggestions" hx-swap-oob="true"></datalist>"#;
        assert_eq!(captures_scope(body), body);
    }

    #[test]
    fn then_inbox_shows_no_tag_passes_for_an_untagged_row() {
        let mut world = world_with_captures_section(
            r#"<li id="capture-row-1"><div class="row-text">buy screws</div><input placeholder="@homedepot"></li>"#,
        );
        assert_eq!(then_inbox_shows_no_tag(&mut world, "buy screws"), Ok(()));
    }

    #[test]
    fn then_inbox_shows_no_tag_errors_when_a_tag_is_present() {
        let mut world = world_with_captures_section(
            r#"<li id="capture-row-1"><div class="row-text">buy screws @homedepot</div></li>"#,
        );
        assert!(then_inbox_shows_no_tag(&mut world, "buy screws").is_err());
    }

    #[test]
    fn dispatch_inbox_lists_tagged_does_not_false_positive_on_the_forms_own_placeholder() {
        let mut world = world_with_captures_section(
            r#"<li id="capture-row-1"><div class="row-text">buy screws</div><input placeholder="@homedepot"></li>"#,
        );
        let example = BTreeMap::new();
        let re = THEN_INBOX_LISTS_TAGGED
            .captures(r#"the inbox lists "buy screws" tagged "@homedepot""#)
            .unwrap();

        assert!(dispatch_inbox_lists_tagged(&mut world, &example, &re).is_err());
    }

    #[test]
    fn dispatch_inbox_lists_tagged_passes_when_the_row_text_carries_the_tag() {
        let mut world = world_with_captures_section(
            r#"<li id="capture-row-1"><div class="row-text">buy screws @homedepot</div></li>"#,
        );
        let example = BTreeMap::new();
        let re = THEN_INBOX_LISTS_TAGGED
            .captures(r#"the inbox lists "buy screws" tagged "@homedepot""#)
            .unwrap();

        assert_eq!(
            dispatch_inbox_lists_tagged(&mut world, &example, &re),
            Ok(())
        );
    }

    #[test]
    fn dispatch_inbox_lists_tagged_scopes_to_the_named_row_when_two_rows_share_a_tag() {
        let mut world = world_with_captures_section(
            r#"<li id="capture-row-1"><div class="row-text">return the drill @HomeDepot</div></li><li id="capture-row-2"><div class="row-text">buy screws @HomeDepot</div></li>"#,
        );
        let example = BTreeMap::new();
        let re = THEN_INBOX_LISTS_TAGGED
            .captures(r#"the inbox lists "buy screws" tagged "@HomeDepot""#)
            .unwrap();

        assert_eq!(
            dispatch_inbox_lists_tagged(&mut world, &example, &re),
            Ok(())
        );
    }

    fn world_with_tasks_section(html: &str) -> World {
        let mut world = World::new();
        world.last_html_body = Some(format!(r#"<ul id="tasks">{html}</ul>"#));
        world
    }

    #[test]
    fn dispatch_task_list_shows_tagged_passes_when_the_row_carries_the_tag() {
        let mut world = world_with_tasks_section(r#"<li>[pool] buy screws @homedepot</li>"#);
        let example = BTreeMap::new();
        let re = THEN_TASK_LIST_SHOWS_TAGGED
            .captures(r#"the task list shows "buy screws" tagged "@homedepot""#)
            .unwrap();

        assert_eq!(
            dispatch_task_list_shows_tagged(&mut world, &example, &re),
            Ok(())
        );
    }

    #[test]
    fn dispatch_task_list_shows_tagged_errors_when_the_tag_is_absent() {
        let mut world = world_with_tasks_section(r#"<li>[pool] buy screws</li>"#);
        let example = BTreeMap::new();
        let re = THEN_TASK_LIST_SHOWS_TAGGED
            .captures(r#"the task list shows "buy screws" tagged "@homedepot""#)
            .unwrap();

        assert!(dispatch_task_list_shows_tagged(&mut world, &example, &re).is_err());
    }

    #[test]
    fn suggested_tags_reads_the_plain_page_render_datalist() {
        let body = r#"<datalist id="context-tag-suggestions">
<option value="@homedepot">
<option value="@supermarket">
</datalist>"#;
        assert_eq!(
            suggested_tags(body).unwrap(),
            vec!["@homedepot".to_string(), "@supermarket".to_string()]
        );
    }

    #[test]
    fn suggested_tags_reads_the_out_of_band_fragments_datalist() {
        let body = r#"<datalist id="context-tag-suggestions" hx-swap-oob="true">
<option value="@homedepot">
</datalist>"#;
        assert_eq!(
            suggested_tags(body).unwrap(),
            vec!["@homedepot".to_string()]
        );
    }

    #[test]
    fn suggested_tags_is_empty_when_no_option_is_present() {
        let body = r#"<datalist id="context-tag-suggestions">
</datalist>"#;
        assert_eq!(suggested_tags(body).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn dispatch_suggestions_exactly_splits_the_expected_list_on_comma_space() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<datalist id="context-tag-suggestions">
<option value="@homedepot">
<option value="@supermarket">
</datalist>"#
                .to_string(),
        );
        let example = BTreeMap::new();
        let re = THEN_SUGGESTIONS_EXACTLY
            .captures(r#"the context tag suggestions are exactly "@homedepot, @supermarket""#)
            .unwrap();

        assert_eq!(
            dispatch_suggestions_exactly(&mut world, &example, &re),
            Ok(())
        );
    }
}
