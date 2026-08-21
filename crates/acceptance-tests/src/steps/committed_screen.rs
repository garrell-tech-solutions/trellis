//! Step handlers for `features/committed_screen.feature`: committed items
//! listed chronologically, the third Menu tab (#94).
//!
//! The Background ("the trellis server is running with an empty task list",
//! "the server believes it is ...") and the generic rejection steps ("the
//! triage is rejected", "the rejection names ...", "the task list is still
//! empty") this feature also uses are already matched generically by
//! [`super::triage::dispatch`] and [`super::quota_triage_validation::dispatch`],
//! tried before this module. The tab bar's own steps ("the tab bar offers
//! exactly ...", "the tab bar marks ... as the current tab") and "the
//! "<screen>" screen is viewed" are [`super::pool_screen`]'s, extended there
//! to route `"committed"` -- nothing here duplicates them.
//!
//! Several scenarios name a value via a literal `Examples` placeholder
//! (`"<order>"`, `"<meta>"`, ...) that this Gherkin runner does not
//! pre-substitute. [`resolve`] does that, the same helper `context_tags.rs`
//! and `pool_screen.rs` already established.

use super::html;
use super::payloads;
use super::triage::{given_capture_waiting, when_triaged};
use super::*;
use serde_json::{json, Value};

static GIVEN_COMMITTED_TAGGED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^a committed task "([^"]+)" tagged "([^"]+)" due "([^"]+)" as an? "(at|by)"$"#)
        .unwrap()
});
static GIVEN_COMMITTED_NO_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^a committed task "([^"]+)" with no context tag due "([^"]+)" as an? "(at|by)"$"#)
        .unwrap()
});
static WHEN_COMMITTED_SCREEN_VIEWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the committed screen is viewed$").unwrap());
static WHEN_TRIAGED_THROUGH_TRANSPORT_OMITTING_COMMITMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the capture is triaged as a committed task through the "([^"]+)" with "commitment" omitted$"#,
    )
    .unwrap()
});
static THEN_LISTS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the committed screen lists "([^"]+)"$"#).unwrap());
static THEN_ROW_SHOWS_CONTEXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the committed row "([^"]+)" shows the context "([^"]+)"$"#).unwrap()
});
static THEN_ROW_SHOWS_NO_CONTEXT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the committed row "([^"]+)" shows no context$"#).unwrap());
static THEN_ROW_SHOWS_DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the committed row "([^"]+)" shows the date "([^"]+)"$"#).unwrap()
});
static THEN_ROW_MARKED_PAST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the committed row "([^"]+)" is marked as past$"#).unwrap());
static THEN_ROW_NOT_MARKED_PAST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the committed row "([^"]+)" is not marked as past$"#).unwrap());
static THEN_META: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the committed screen reports "([^"]+)" beside its title$"#).unwrap()
});
static THEN_DOES_NOT_MENTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the committed screen does not mention "([^"]+)"$"#).unwrap());
static THEN_SHOWS_MESSAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the committed screen shows the message "([^"]+)"$"#).unwrap());
static THEN_WAY_BACK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the committed screen offers a way back to Capture$").unwrap());
static THEN_NO_UNESCAPED_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the committed screen does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the committed screen contains the word "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = GIVEN_COMMITTED_TAGGED.captures(text) {
        return Some(dispatch_given_tagged(world, &caps).await);
    }
    if let Some(caps) = GIVEN_COMMITTED_NO_TAG.captures(text) {
        return Some(dispatch_given_no_tag(world, &caps).await);
    }
    if WHEN_COMMITTED_SCREEN_VIEWED.is_match(text) {
        return Some(view_screen(world).await);
    }
    if let Some(caps) = WHEN_TRIAGED_THROUGH_TRANSPORT_OMITTING_COMMITMENT.captures(text) {
        return Some(dispatch_triaged_omitting_commitment(world, example, &caps).await);
    }
    if let Some(caps) = THEN_LISTS.captures(text) {
        return Some(dispatch_lists(world, example, &caps));
    }
    if let Some(caps) = THEN_ROW_SHOWS_CONTEXT.captures(text) {
        return Some(dispatch_row_shows_context(world, example, &caps));
    }
    if let Some(caps) = THEN_ROW_SHOWS_NO_CONTEXT.captures(text) {
        return Some(then_row_shows_no_context(world, &caps[1]));
    }
    if let Some(caps) = THEN_ROW_SHOWS_DATE.captures(text) {
        return Some(dispatch_row_shows_date(world, example, &caps));
    }
    if let Some(caps) = THEN_ROW_MARKED_PAST.captures(text) {
        return Some(then_row_marked_past(world, &caps[1]));
    }
    if let Some(caps) = THEN_ROW_NOT_MARKED_PAST.captures(text) {
        return Some(then_row_not_marked_past(world, &caps[1]));
    }
    if let Some(caps) = THEN_META.captures(text) {
        return Some(dispatch_meta(world, example, &caps));
    }
    if let Some(caps) = THEN_DOES_NOT_MENTION.captures(text) {
        return Some(then_does_not_mention(world, &caps[1]));
    }
    if let Some(caps) = THEN_SHOWS_MESSAGE.captures(text) {
        return Some(dispatch_shows_message(world, example, &caps));
    }
    if THEN_WAY_BACK.is_match(text) {
        return Some(then_way_back(world));
    }
    if THEN_NO_UNESCAPED_SCRIPT.is_match(text) {
        return Some(then_html_body_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_CONTAINS_WORD.captures(text) {
        return Some(then_html_body_contains(world, &caps[1]));
    }
    None
}

/// Resolves a captured value that may be a literal or an `Examples`
/// placeholder (`<order>`) written down verbatim in the step text. See
/// `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared because the two modules' captures come
/// from different `World` state and sharing would cost more than the four
/// lines it would save.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// A committed submission: text, an optional tag, a deadline and a
/// commitment (`at` or `by`).
async fn given_committed(
    world: &mut World,
    text: &str,
    tag: Option<&str>,
    deadline: &str,
    commitment: &str,
) -> Result<(), String> {
    given_capture_waiting(world, text).await?;
    let body = json!({
        "kind": "committed",
        "deadline": deadline,
        "commitment": commitment,
        "priority": "P1",
        "estimated_minutes": 30,
    });
    let body = match tag {
        Some(tag) => payloads::with_field(body, "context_tag", Value::from(tag)),
        None => body,
    };
    when_triaged(world, body).await
}

async fn dispatch_given_tagged(
    world: &mut World,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    given_committed(world, &caps[1], Some(&caps[2]), &caps[3], &caps[4]).await
}

async fn dispatch_given_no_tag(
    world: &mut World,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    given_committed(world, &caps[1], None, &caps[2], &caps[3]).await
}

async fn view_screen(world: &mut World) -> Result<(), String> {
    let request = axum::http::Request::builder()
        .uri("/committed")
        .body(axum::body::Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    super::inbox_view::html_response(world, request).await
}

/// `<transport>` names which of the two ways this endpoint accepts
/// submissions to use -- `"api"` posts the JSON `payloads::committed`
/// carries with `commitment` stripped, `"page"` does the same through the
/// page's own form fields (`triage_from_page::committed_form_fields`),
/// exercising the identical validation both callers are meant to share
/// (`T-required-fields-are-specified-per-transport`).
async fn dispatch_triaged_omitting_commitment(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    given_capture_waiting(world, "File the tax return").await?;
    let transport = resolve(example, &caps[1])?;
    match transport.as_str() {
        "api" => {
            let body = payloads::without_field(payloads::committed(), "commitment");
            when_triaged(world, body).await
        }
        "page" => {
            let fields: Vec<(&str, &str)> = super::triage_from_page::committed_form_fields()
                .into_iter()
                .filter(|(name, _)| *name != "commitment")
                .collect();
            super::triage_from_page::when_triaged_through_page(world, &fields).await
        }
        other => Err(format!("unknown transport {other:?}")),
    }
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no committed screen response recorded")
}

fn then_html_body_contains(world: &mut World, expected: &str) -> Result<(), String> {
    super::then_html_body_contains(world, expected, "no committed screen response recorded")
}

fn then_html_body_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    super::then_html_body_excludes(world, forbidden, "no committed screen response recorded")
}

/// The rows section, falling back to the whole body if the wrapper is
/// missing -- the empty state renders no `<ul class="committed-rows">` at
/// all.
fn committed_rows_scope(body: &str) -> &str {
    html::between(body, r#"<ul class="committed-rows">"#, "</ul>").unwrap_or(body)
}

/// Every `.committed-text` div's content, in document order -- what "the
/// committed screen lists ..." means.
fn committed_texts_in_order(body: &str) -> Vec<String> {
    committed_rows_scope(body)
        .split(r#"<div class="committed-text">"#)
        .skip(1)
        .filter_map(|chunk| chunk.split_once("</div>").map(|(text, _)| text.to_string()))
        .collect()
}

fn dispatch_lists(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected: Vec<String> = resolve(example, &caps[1])?
        .split(", ")
        .map(str::to_string)
        .collect();
    let body = html_body(world)?;
    let actual = committed_texts_in_order(body);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the committed screen to list {expected:?}, got {actual:?}"
        ))
    }
}

fn committed_row<'a>(body: &'a str, raw_text: &str) -> Result<&'a str, String> {
    html::row_containing(committed_rows_scope(body), raw_text)
}

fn committed_row_tag(row: &str) -> Option<String> {
    html::between(row, r#"<div class="committed-tag">"#, "</div>")
        .ok()
        .map(|s| s.to_string())
}

fn dispatch_row_shows_context(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = &caps[1];
    let expected = resolve(example, &caps[2])?;
    let body = html_body(world)?;
    let row = committed_row(body, raw_text)?;
    match committed_row_tag(row) {
        Some(tag) if tag == expected => Ok(()),
        other => Err(format!(
            "expected the row for {raw_text:?} to show context {expected:?}, got {other:?}"
        )),
    }
}

fn then_row_shows_no_context(world: &mut World, raw_text: &str) -> Result<(), String> {
    let body = html_body(world)?;
    let row = committed_row(body, raw_text)?;
    match committed_row_tag(row) {
        None => Ok(()),
        Some(tag) => Err(format!(
            "expected the row for {raw_text:?} to show no context, got {tag:?}"
        )),
    }
}

fn dispatch_row_shows_date(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = &caps[1];
    let expected = resolve(example, &caps[2])?;
    let body = html_body(world)?;
    let row = committed_row(body, raw_text)?;
    let cell = html::between(row, r#"<div class="committed-date">"#, "</div>")?;
    if cell == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the row for {raw_text:?} to show the date {expected:?}, got {cell:?}"
        ))
    }
}

fn then_row_marked_past(world: &mut World, raw_text: &str) -> Result<(), String> {
    let body = html_body(world)?;
    let row = committed_row(body, raw_text)?;
    if row.contains("committed-past-badge") {
        Ok(())
    } else {
        Err(format!(
            "expected the row for {raw_text:?} to be marked past, got:\n{row}"
        ))
    }
}

fn then_row_not_marked_past(world: &mut World, raw_text: &str) -> Result<(), String> {
    let body = html_body(world)?;
    let row = committed_row(body, raw_text)?;
    if row.contains("committed-past-badge") {
        Err(format!(
            "expected the row for {raw_text:?} not to be marked past, got:\n{row}"
        ))
    } else {
        Ok(())
    }
}

fn dispatch_meta(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let meta = html::between(body, r#"<div class="committed-meta">"#, "</div>")?;
    if meta.trim() == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the committed screen to report {expected:?}, got {:?}",
            meta.trim()
        ))
    }
}

fn then_does_not_mention(world: &mut World, text: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(text) {
        Err(format!("expected no mention of {text:?}, got:\n{body}"))
    } else {
        Ok(())
    }
}

fn dispatch_shows_message(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    then_html_body_contains(world, &expected)
}

/// **Absent, not disabled and not hidden** -- the whole page, not a scoped
/// section, since a way-back link could in principle appear anywhere.
fn then_way_back(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(r#"href="/""#) && body.contains("Go to Capture") {
        Ok(())
    } else {
        Err(format!("expected a link back to Capture, got:\n{body}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_a_literal_value_unchanged() {
        let example = BTreeMap::new();
        assert_eq!(resolve(&example, "3 dated"), Ok("3 dated".to_string()));
    }

    #[test]
    fn resolve_looks_up_a_placeholder_in_the_example_row() {
        let example = super::super::example(&[("meta", "3 dated")]);
        assert_eq!(resolve(&example, "<meta>"), Ok("3 dated".to_string()));
    }

    #[test]
    fn resolve_does_not_mistake_hostile_text_for_a_placeholder() {
        let example = BTreeMap::new();
        let hostile = "<script>alert('boom')</script>";
        assert_eq!(resolve(&example, hostile), Ok(hostile.to_string()));
    }

    #[tokio::test]
    async fn given_committed_creates_a_triaged_committed_task_with_a_tag() {
        let mut world = migrated_world().await;

        given_committed(
            &mut world,
            "book the dentist",
            Some("@phone"),
            "2026-08-25T08:30:00Z",
            "at",
        )
        .await
        .unwrap();

        let pool = world.pool().unwrap().clone();
        let tag: Option<String> = sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tag.as_deref(), Some("@phone"));
        let (kind, commitment): (String, Option<String>) =
            sqlx::query_as("SELECT kind, commitment FROM tasks")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kind, "committed");
        assert_eq!(commitment.as_deref(), Some("at"));
    }

    #[tokio::test]
    async fn given_committed_with_no_tag_stores_none() {
        let mut world = migrated_world().await;

        given_committed(
            &mut world,
            "file the tax return",
            None,
            "2026-08-27T17:00:00Z",
            "by",
        )
        .await
        .unwrap();

        let pool = world.pool().unwrap().clone();
        let tag: Option<String> = sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tag, None);
    }

    fn body_with_rows(rows: &str) -> String {
        format!(r#"<ul class="committed-rows">{rows}</ul>"#)
    }

    fn row(text: &str, date: &str, tag: Option<&str>, past: bool) -> String {
        let class = if past {
            "committed-row past"
        } else {
            "committed-row"
        };
        let badge = if past {
            r#"<span class="committed-past-badge">Past</span>"#
        } else {
            ""
        };
        let tag_div = tag
            .map(|t| format!(r#"<div class="committed-tag">{t}</div>"#))
            .unwrap_or_default();
        format!(
            r#"<li class="{class}"><div class="committed-date">{date}</div><div class="committed-text">{text}</div>{badge}{tag_div}</li>"#
        )
    }

    #[test]
    fn committed_texts_in_order_reads_every_row_in_document_order() {
        let body = body_with_rows(&format!(
            "{}{}",
            row("Book the dentist", "TUE 8:30", None, false),
            row("Q3 planning doc", "BY THU", None, false)
        ));
        assert_eq!(
            committed_texts_in_order(&body),
            vec![
                "Book the dentist".to_string(),
                "Q3 planning doc".to_string()
            ]
        );
    }

    #[test]
    fn dispatch_lists_matches_the_resolved_order() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_rows(&row(
            "Book the dentist",
            "TUE 8:30",
            None,
            false,
        )));
        let example = super::super::example(&[("order", "Book the dentist")]);
        let re = THEN_LISTS
            .captures(r#"the committed screen lists "<order>""#)
            .unwrap();
        assert_eq!(dispatch_lists(&mut world, &example, &re), Ok(()));
    }

    #[test]
    fn dispatch_row_shows_context_passes_when_the_row_carries_the_tag() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_rows(&row(
            "book the dentist",
            "TUE 8:30",
            Some("@phone"),
            false,
        )));
        let re = THEN_ROW_SHOWS_CONTEXT
            .captures(r#"the committed row "book the dentist" shows the context "<ctx>""#)
            .unwrap();
        let example = super::super::example(&[("ctx", "@phone")]);
        assert_eq!(
            dispatch_row_shows_context(&mut world, &example, &re),
            Ok(())
        );
    }

    #[test]
    fn then_row_shows_no_context_passes_for_an_untagged_row() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_rows(&row(
            "furnace service window",
            "FRI 13:00",
            None,
            false,
        )));
        assert_eq!(
            then_row_shows_no_context(&mut world, "furnace service window"),
            Ok(())
        );
    }

    #[test]
    fn then_row_shows_no_context_errors_when_a_tag_is_present() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_rows(&row(
            "book the dentist",
            "TUE 8:30",
            Some("@phone"),
            false,
        )));
        assert!(then_row_shows_no_context(&mut world, "book the dentist").is_err());
    }

    #[test]
    fn dispatch_row_shows_date_reads_the_named_rows_own_cell() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_rows(&format!(
            "{}{}",
            row("Book the dentist", "TUE 8:30", None, false),
            row("File the tax return", "BY THU", None, false)
        )));
        let example = super::super::example(&[("by_cell", "BY THU")]);
        let re = THEN_ROW_SHOWS_DATE
            .captures(r#"the committed row "File the tax return" shows the date "<by_cell>""#)
            .unwrap();
        assert_eq!(dispatch_row_shows_date(&mut world, &example, &re), Ok(()));
    }

    #[test]
    fn then_row_marked_past_passes_for_a_past_row() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_rows(&row(
            "renew the passport",
            "MON 9:00",
            None,
            true,
        )));
        assert_eq!(
            then_row_marked_past(&mut world, "renew the passport"),
            Ok(())
        );
    }

    #[test]
    fn then_row_not_marked_past_errors_for_a_past_row() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_rows(&row(
            "renew the passport",
            "MON 9:00",
            None,
            true,
        )));
        assert!(then_row_not_marked_past(&mut world, "renew the passport").is_err());
    }

    #[test]
    fn then_row_not_marked_past_passes_for_a_future_row() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_rows(&row(
            "book the dentist",
            "TUE 8:30",
            None,
            false,
        )));
        assert_eq!(
            then_row_not_marked_past(&mut world, "book the dentist"),
            Ok(())
        );
    }

    #[test]
    fn dispatch_meta_reads_the_meta_div() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<div class="committed-header"><div class="committed-meta">3 dated</div></div>"#
                .to_string(),
        );
        let example = super::super::example(&[("meta", "3 dated")]);
        let re = THEN_META
            .captures(r#"the committed screen reports "<meta>" beside its title"#)
            .unwrap();
        assert_eq!(dispatch_meta(&mut world, &example, &re), Ok(()));
    }

    #[test]
    fn then_way_back_passes_when_the_capture_link_and_its_words_are_present() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<a href="/" class="committed-go-capture">Go to Capture &rarr;</a>"#.to_string(),
        );
        assert_eq!(then_way_back(&mut world), Ok(()));
    }

    #[test]
    fn then_way_back_errors_when_absent() {
        let mut world = World::new();
        world.last_html_body = Some("<main></main>".to_string());
        assert!(then_way_back(&mut world).is_err());
    }
}
