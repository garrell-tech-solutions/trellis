//! Step handlers for `features/context_tags.feature` (#82).
//!
//! The Background ("the trellis server is running with an empty task
//! list") and "the inbox is viewed"/"the inbox lists ..."/"the inbox does
//! not contain an unescaped ..." steps this feature also uses are already
//! matched generically by [`super::triage::dispatch`] and
//! [`super::inbox_view::dispatch`], tried before this module.
//!
//! "The task list shows ... tagged ..." (scenario 07) is
//! [`super::life_areas::dispatch`]'s own step, generalized to a row-scoped
//! substring check that serves this feature's context tags and
//! `life_area_triage.feature`'s life areas identically -- see that
//! module's own doc comment on `then_task_list_shows_tagged`.

use super::inbox_view::{html_response, urlencode};
use super::life_areas::resolve;
use super::*;
use axum::body::Body;
use axum::http::Request;
use serde_json::json;

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

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = GIVEN_TAGGED_CAPTURE_WAITING.captures(text) {
        return Some(dispatch_tagged_capture_waiting(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_QUICK_ADD_TAGGED.captures(text) {
        return Some(dispatch_quick_add_tagged(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_QUICK_ADD_NO_TAG.captures(text) {
        return Some(when_quick_add_no_tag(world, &caps[1]).await);
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
        return Some(dispatch_inbox_shows_no_tag(world, example, &caps));
    }
    if let Some(caps) = THEN_SUGGESTIONS_EXACTLY.captures(text) {
        return Some(dispatch_suggestions_exactly(world, example, &caps));
    }
    None
}

async fn dispatch_tagged_capture_waiting(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    given_tagged_capture_waiting(world, &raw_text, &tag).await
}

async fn given_tagged_capture_waiting(
    world: &mut World,
    raw_text: &str,
    tag: &str,
) -> Result<(), String> {
    let pool = world.pool()?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, 'web', 0) RETURNING id",
    )
    .bind(raw_text)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("insert capture: {e}"))?;
    trellis_server::capture::store::set_context_tag(pool, id, tag)
        .await
        .map_err(|e| format!("set context tag: {e}"))?;
    world.last_capture_id = Some(id);
    Ok(())
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

async fn dispatch_quick_add_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    quick_add(world, &raw_text, Some(&tag)).await
}

async fn when_quick_add_no_tag(world: &mut World, raw_text: &str) -> Result<(), String> {
    quick_add(world, raw_text, None).await
}

async fn dispatch_triaged_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    super::triage::when_triaged(world, json!({ "kind": "pool", "context_tag": tag })).await
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no quick-add response recorded")
}

/// The captures list's own section when there is one, or the whole body
/// otherwise -- a quick-add's own response is the new row's markup alone
/// (`hx-swap="afterbegin"` targets `#captures` directly), without the
/// `<ul id="captures">` wrapper a full page or a triage/dismissal's
/// `#lists` swap carries (the same shape `life_areas::
/// then_quick_add_pool_preselects_none` already reads around).
fn captures_scope(body: &str) -> &str {
    html::captures_section(body).unwrap_or(body)
}

fn dispatch_inbox_lists_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    let section = captures_scope(html_body(world)?);
    let row = html::row_containing(section, &text)?;
    if row.contains(&tag) {
        Ok(())
    } else {
        Err(format!(
            "expected {tag:?} in the row for {text:?}, got:\n{row}"
        ))
    }
}

/// Whatever falls between `text` and the row's own `<form` is the tag (or
/// is not there at all) -- `capture_row.html` renders the text, then the
/// tag if one exists, then the triage forms.
fn text_after(row: &str, text: &str) -> Result<String, String> {
    let (_, after) = row
        .split_once(text)
        .ok_or_else(|| format!("expected {text:?} in row:\n{row}"))?;
    Ok(after.split("<form").next().unwrap_or("").trim().to_string())
}

fn dispatch_inbox_shows_no_tag(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let text = resolve(example, &caps[1])?;
    let section = captures_scope(html_body(world)?);
    let row = html::row_containing(section, &text)?;
    let trailing = text_after(row, &text)?;
    if trailing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "expected no context tag after {text:?}, got {trailing:?} in:\n{row}"
        ))
    }
}

/// The tag control's `<datalist>` own suggestions, in document order --
/// `<option value="...">` values, the same parsing shape `triage_from_page::
/// select_option_values` uses for a `<select>`'s own options.
fn suggested_tags(body: &str) -> Result<Vec<String>, String> {
    let section = html::between(
        body,
        r#"<datalist id="context-tag-suggestions">"#,
        "</datalist>",
    )?;
    Ok(section
        .split("<option value=\"")
        .skip(1)
        .filter_map(|chunk| chunk.split('"').next())
        .map(str::to_string)
        .collect())
}

fn dispatch_suggestions_exactly(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let listed = resolve(example, &caps[1])?;
    let expected: Vec<String> = listed.split(", ").map(str::to_string).collect();
    let actual = suggested_tags(html_body(world)?)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected suggestions {expected:?}, got {actual:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_tags_reads_option_values_in_document_order() {
        let body = r#"<datalist id="context-tag-suggestions"><option value="@homedepot"><option value="@supermarket"></datalist>"#;
        assert_eq!(
            suggested_tags(body).unwrap(),
            vec!["@homedepot".to_string(), "@supermarket".to_string()]
        );
    }

    #[test]
    fn suggested_tags_is_empty_for_an_empty_datalist() {
        let body = r#"<datalist id="context-tag-suggestions"></datalist>"#;
        assert_eq!(suggested_tags(body).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn text_after_finds_the_tag_between_the_text_and_the_form() {
        let row = r#"<li id="capture-row-1">buy screws @homedepot<form></form></li>"#;
        assert_eq!(text_after(row, "buy screws").unwrap(), "@homedepot");
    }

    #[test]
    fn text_after_is_empty_when_no_tag_is_present() {
        let row = r#"<li id="capture-row-1">buy milk<form></form></li>"#;
        assert_eq!(text_after(row, "buy milk").unwrap(), "");
    }

    #[test]
    fn dispatch_inbox_lists_tagged_finds_the_tag_in_the_captures_row() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="captures"><li id="capture-row-1">buy screws @homedepot<form></form></li></ul>"#
                .to_string(),
        );
        let ex = example(&[]);
        let caps = THEN_INBOX_LISTS_TAGGED
            .captures(r#"the inbox lists "buy screws" tagged "@homedepot""#)
            .unwrap();

        assert_eq!(dispatch_inbox_lists_tagged(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn dispatch_inbox_shows_no_tag_passes_when_the_row_carries_none() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="captures"><li id="capture-row-1">buy milk<form></form></li></ul>"#
                .to_string(),
        );
        let ex = example(&[]);
        let caps = THEN_INBOX_SHOWS_NO_TAG
            .captures(r#"the inbox shows no context tag for "buy milk""#)
            .unwrap();

        assert_eq!(dispatch_inbox_shows_no_tag(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn dispatch_inbox_shows_no_tag_errors_when_a_tag_is_present() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="captures"><li id="capture-row-1">buy screws @homedepot<form></form></li></ul>"#
                .to_string(),
        );
        let ex = example(&[]);
        let caps = THEN_INBOX_SHOWS_NO_TAG
            .captures(r#"the inbox shows no context tag for "buy screws""#)
            .unwrap();

        assert!(dispatch_inbox_shows_no_tag(&mut world, &ex, &caps).is_err());
    }

    #[tokio::test]
    async fn given_tagged_capture_waiting_stores_the_tag_and_records_the_capture_id() {
        let mut world = migrated_world().await;

        given_tagged_capture_waiting(&mut world, "buy screws", "@homedepot")
            .await
            .unwrap();

        let id = world.last_capture_id.unwrap();
        let tag: Option<String> =
            sqlx::query_scalar("SELECT context_tag FROM captures WHERE id = ?")
                .bind(id)
                .fetch_one(world.pool().unwrap())
                .await
                .unwrap();
        assert_eq!(tag.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn quick_add_with_a_tag_stores_it() {
        let mut world = migrated_world().await;

        quick_add(&mut world, "buy screws", Some("@homedepot"))
            .await
            .unwrap();

        let tag: Option<String> = sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(world.pool().unwrap())
            .await
            .unwrap();
        assert_eq!(tag.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn quick_add_with_no_tag_stores_none() {
        let mut world = migrated_world().await;

        quick_add(&mut world, "buy milk", None).await.unwrap();

        let tag: Option<String> = sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(world.pool().unwrap())
            .await
            .unwrap();
        assert_eq!(tag, None);
    }
}
