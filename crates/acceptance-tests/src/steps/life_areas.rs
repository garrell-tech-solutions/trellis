//! Step handlers for `features/life_areas.feature` (managing life areas) and
//! `features/life_area_triage.feature` (tagging a task with one at triage).
//! One module for both -- they are the two feature files a single handoff
//! brief specified together (#47's "combined cut"), and most of what
//! `life_area_triage.feature` needs beyond "triage rejects/accepts" is the
//! same picker and tag vocabulary `life_areas.feature` already needs.
//!
//! Kept as its own module rather than folded into `triage::dispatch`, which
//! the complexity gate is already red on (handoff brief gotcha #2).
//!
//! Life-area-specific rejections reuse [`super::triage::then_rejection_names`]
//! directly: that helper already falls back from a JSON `missing_field` body
//! to an HTML `"{field} is required"` substring, which is exactly the two
//! shapes `life_area`'s own two "required" rejections take here (a triage
//! JSON rejection, and the management page's own HTML fragment).

use super::html;
use super::payloads;
use super::triage::{then_rejection_names, when_triaged};
use super::triage_from_page::{
    committed_form_fields, form_section, quota_form_fields, select_option_values,
    when_triaged_through_page,
};
use super::*;
use axum::body::Body;
use axum::http::Request;
use serde_json::{json, Value};

static WHEN_LIFE_AREAS_PAGE_VIEWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the life areas page is viewed$").unwrap());
static WHEN_LIFE_AREA_ADDED_FROM_PAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^a life area named "([^"]+)" is added from the life areas page$"#).unwrap()
});
static GIVEN_LIFE_AREA_WAS_ADDED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^a life area named "([^"]+)" was added$"#).unwrap());
static LIFE_AREA_IS_ARCHIVED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the life area "([^"]+)" is archived$"#).unwrap());
static THEN_LIFE_AREAS_LISTED_EXACTLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the life areas listed are exactly "([^"]+)"$"#).unwrap());
static THEN_ADD_NOT_REDIRECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the add response does not redirect the browser$").unwrap());
static THEN_ADD_REJECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the add is rejected$").unwrap());
static THEN_REJECTION_SAYS_ALREADY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the rejection says "([^"]+)" is already a life area$"#).unwrap()
});
static THEN_REJECTION_SAYS_NOT_A_LIFE_AREA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the rejection says "([^"]+)" is not a life area$"#).unwrap());
static THEN_REJECTION_NAMES_LITERAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the rejection names "([^"]+)"$"#).unwrap());
static THEN_ROW_REJECTION_NAMES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the rejection message on the capture's row names "([^"]+)"$"#).unwrap()
});
static THEN_LIFE_AREAS_LIST_NO_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the life areas list does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_LIFE_AREAS_LIST_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the life areas list contains the word "([^"]+)"$"#).unwrap());
static THEN_TRIAGE_CHOICES_EXACTLY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the triage life area choices are exactly "([^"]+)"$"#).unwrap()
});
static WHEN_NAMED_CAPTURE_TRIAGED_IN_LIFE_AREA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^"([^"]+)" is triaged as a pool task in life area "([^"]+)"$"#).unwrap()
});
static WHEN_CAPTURE_TRIAGED_IN_LIFE_AREA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a pool task in life area "([^"]+)"$"#).unwrap()
});
static WHEN_TRIAGED_OMITTING_LIFE_AREA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a (\S+) task with "life_area" omitted$"#).unwrap()
});
static THEN_KIND_FORM_PRESELECTS_NONE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the <(\w+)> triage form preselects no life area$").unwrap());
static THEN_QUICK_ADD_POOL_PRESELECTS_NONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the quick-add response's pool triage form preselects no life area$").unwrap()
});
static WHEN_TRIAGED_THROUGH_PAGE_NO_LIFE_AREA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^the capture is triaged as a <(\w+)> task through the page with no life area chosen$",
    )
    .unwrap()
});
static THEN_TASK_LIST_SHOWS_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the task list shows "([^"]+)" tagged "([^"]+)"$"#).unwrap());
static THEN_TRIAGE_SUCCEEDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the triage succeeds$").unwrap());
static THEN_TASK_LIST_SHOWS_NO_LIFE_AREA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the task list shows "([^"]+)" with no life area$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if WHEN_LIFE_AREAS_PAGE_VIEWED.is_match(text) {
        return Some(when_life_areas_page_viewed(world).await);
    }
    if let Some(caps) = WHEN_LIFE_AREA_ADDED_FROM_PAGE.captures(text) {
        return Some(dispatch_add_life_area(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_LIFE_AREA_WAS_ADDED.captures(text) {
        return Some(dispatch_add_life_area(world, example, &caps).await);
    }
    if let Some(caps) = LIFE_AREA_IS_ARCHIVED.captures(text) {
        return Some(dispatch_archive_life_area(world, example, &caps).await);
    }
    if let Some(caps) = THEN_LIFE_AREAS_LISTED_EXACTLY.captures(text) {
        return Some(dispatch_life_areas_listed_exactly(world, example, &caps));
    }
    if THEN_ADD_NOT_REDIRECT.is_match(text) {
        return Some(then_not_redirect(world));
    }
    if THEN_ADD_REJECTED.is_match(text) {
        return Some(then_status_is(world, 422));
    }
    if let Some(caps) = THEN_REJECTION_SAYS_ALREADY.captures(text) {
        return Some(dispatch_rejection_says_already(world, example, &caps));
    }
    if let Some(caps) = THEN_REJECTION_SAYS_NOT_A_LIFE_AREA.captures(text) {
        return Some(dispatch_rejection_says_not_a_life_area(
            world, example, &caps,
        ));
    }
    if let Some(caps) = THEN_REJECTION_NAMES_LITERAL.captures(text) {
        return Some(then_rejection_names(world, &caps[1]));
    }
    if let Some(caps) = THEN_ROW_REJECTION_NAMES.captures(text) {
        return Some(then_rejection_names(world, &caps[1]));
    }
    if THEN_LIFE_AREAS_LIST_NO_SCRIPT.is_match(text) {
        return Some(then_life_areas_list_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_LIFE_AREAS_LIST_CONTAINS_WORD.captures(text) {
        return Some(then_life_areas_list_contains(world, &caps[1]));
    }
    if let Some(caps) = THEN_TRIAGE_CHOICES_EXACTLY.captures(text) {
        return Some(dispatch_triage_choices_exactly(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_NAMED_CAPTURE_TRIAGED_IN_LIFE_AREA.captures(text) {
        return Some(dispatch_named_capture_triaged(world, &caps).await);
    }
    if let Some(caps) = WHEN_CAPTURE_TRIAGED_IN_LIFE_AREA.captures(text) {
        return Some(dispatch_capture_triaged_in_life_area(world, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_OMITTING_LIFE_AREA.captures(text) {
        return Some(dispatch_triaged_omitting_life_area(world, example, &caps).await);
    }
    if let Some(caps) = THEN_TASK_LIST_SHOWS_TAGGED.captures(text) {
        return Some(dispatch_task_list_shows_tagged(world, example, &caps));
    }
    if THEN_TRIAGE_SUCCEEDS.is_match(text) {
        return Some(then_status_is(world, 201));
    }
    if let Some(caps) = THEN_TASK_LIST_SHOWS_NO_LIFE_AREA.captures(text) {
        return Some(then_task_list_shows_no_life_area(world, &caps[1]));
    }
    if let Some(caps) = THEN_KIND_FORM_PRESELECTS_NONE.captures(text) {
        return Some(dispatch_kind_form_preselects_none(world, example, &caps));
    }
    if THEN_QUICK_ADD_POOL_PRESELECTS_NONE.is_match(text) {
        return Some(then_quick_add_pool_preselects_none(world));
    }
    if let Some(caps) = WHEN_TRIAGED_THROUGH_PAGE_NO_LIFE_AREA.captures(text) {
        return Some(dispatch_triaged_through_page_no_life_area(world, example, &caps).await);
    }
    None
}

static BRACKETED_PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());

/// A captured value that may be a literal, or a `<name>` placeholder to look
/// up in the current example row -- most steps here are shared by a plain
/// `Scenario` (a literal in the text) and a `Scenario Outline` (a
/// placeholder), and the parser leaves `<name>` unsubstituted either way.
///
/// The whole value must match `^<\w+>$`, not merely start with `<` and end
/// with `>` -- a hostile name like `<script>alert('boom')</script>` does
/// both and is not a placeholder at all.
pub(super) fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    match BRACKETED_PLACEHOLDER.captures(raw) {
        Some(caps) => example_value(example, &caps[1]).map(str::to_string),
        None => Ok(raw.to_string()),
    }
}

/// Delegates to [`super::inbox_view::html_response`], which already does
/// "send a request, record status and body" -- every request built in this
/// module only adds the method, route and form body.
async fn html_get(world: &mut World, uri: &str) -> Result<(), String> {
    let request = Request::builder()
        .uri(uri)
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    super::inbox_view::html_response(world, request).await
}

pub(super) async fn when_life_areas_page_viewed(world: &mut World) -> Result<(), String> {
    html_get(world, "/life-areas").await
}

async fn post_life_area_form(world: &mut World, name: &str) -> Result<(), String> {
    let body = format!("name={}", super::inbox_view::urlencode(name));
    let request = Request::builder()
        .method("POST")
        .uri("/life-areas")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .map_err(|e| format!("build request: {e}"))?;
    super::inbox_view::html_response(world, request).await
}

async fn dispatch_add_life_area(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    post_life_area_form(world, &name).await
}

/// Resolves a step's `<name>`-or-literal capture and looks up the life area
/// it names -- the two-step lookup most `dispatch_*` handlers across this
/// module and its siblings repeat by hand around a call to
/// [`life_area_id_by_name`].
pub(super) async fn resolved_life_area_id(
    world: &World,
    example: &BTreeMap<String, String>,
    raw: &str,
) -> Result<i64, String> {
    let name = resolve(example, raw)?;
    life_area_id_by_name(world, &name).await
}

pub(super) async fn life_area_id_by_name(world: &World, name: &str) -> Result<i64, String> {
    let pool = world.pool()?;
    let row = trellis_server::life_areas::store::find_by_name(pool, name)
        .await
        .map_err(|e| format!("find life area {name:?}: {e}"))?
        .ok_or_else(|| format!("no life area named {name:?}"))?;
    Ok(row.id)
}

async fn dispatch_archive_life_area(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let id = life_area_id_by_name(world, &name).await?;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/life-areas/{id}/archive"))
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    super::inbox_view::html_response(world, request).await
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no HTML response recorded")
}

fn life_areas_section(world: &World) -> Result<&str, String> {
    html::between(html_body(world)?, r#"<ul id="life-areas">"#, "</ul>")
}

/// The name inside each `<li id="life-area-row-N">...</li>`, in document
/// order -- read from its own `<span class="life-area-name">`, scoped
/// rather than "everything before the row's first `<form`" now that a
/// row's guardrail state (bands, or "no guardrail") renders between the
/// name and any control.
fn life_area_row_names(section: &str) -> Vec<String> {
    section
        .split(r#"<li id="life-area-row-"#)
        .skip(1)
        .filter_map(|chunk| {
            let name = html::between(chunk, r#"<span class="life-area-name">"#, "</span>").ok()?;
            Some(name.trim().to_string())
        })
        .collect()
}

fn dispatch_life_areas_listed_exactly(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let listed = resolve(example, &caps[1])?;
    let expected: Vec<String> = listed.split(", ").map(str::to_string).collect();
    let section = life_areas_section(world)?;
    let actual = life_area_row_names(section);
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected life areas {expected:?}, got {actual:?}"))
    }
}

fn then_not_redirect(world: &mut World) -> Result<(), String> {
    super::then_not_redirect(world, "no response recorded")
}

fn then_status_is(world: &mut World, expected: u16) -> Result<(), String> {
    super::then_status_is(world, expected, "no response recorded")
}

fn dispatch_rejection_says_already(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let needle = format!("{name} is already a life area");
    if body.contains(&needle) {
        Ok(())
    } else {
        Err(format!(
            "expected {needle:?} in the rejection, got:\n{body}"
        ))
    }
}

fn rejection_reports_unknown_life_area(body: &Value, name: &str) -> Result<(), String> {
    match body.get("unknown_life_area").and_then(Value::as_str) {
        Some(actual) if actual == name => Ok(()),
        other => Err(format!(
            "expected the rejection to report unknown_life_area {name:?}, body reported {other:?}"
        )),
    }
}

fn dispatch_rejection_says_not_a_life_area(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let body = world
        .last_response_body
        .as_ref()
        .ok_or_else(|| "no rejection body recorded".to_string())?;
    rejection_reports_unknown_life_area(body, &name)
}

fn then_life_areas_list_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let section = life_areas_section(world)?;
    if section.contains(forbidden) {
        Err(format!(
            "expected no {forbidden:?} in the life areas list, got:\n{section}"
        ))
    } else {
        Ok(())
    }
}

fn then_life_areas_list_contains(world: &mut World, expected: &str) -> Result<(), String> {
    let section = life_areas_section(world)?;
    if section.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected:?} in the life areas list, got:\n{section}"
        ))
    }
}

/// Reads the current triage picker's own rendered `<select>`, the same
/// element a real triage form offers -- checking the underlying query
/// directly would miss a bug where the data is right and the template is
/// not. Some scenarios reach this step with no untriaged capture left (the
/// only one in scope has already been triaged), so there may be no picker on
/// the page to read; a throwaway capture guarantees one without disturbing
/// anything the scenario itself is asserting on (the task list and the
/// life-areas list are both unaffected by one more untriaged row).
async fn current_triage_life_area_choices(world: &mut World) -> Result<Vec<String>, String> {
    insert_probe_capture(world).await?;
    html_get(world, "/").await?;
    let body = html_body(world)?;
    let section = html::captures_section(body)?;
    let options = select_option_values(section, "life_area")?;
    Ok(exclude_blank_placeholder(options))
}

/// The throwaway capture [`current_triage_life_area_choices`] needs to
/// guarantee a picker is on the page; not meaningful on its own.
async fn insert_probe_capture(world: &World) -> Result<(), String> {
    let pool = world.pool()?.clone();
    sqlx::query(
        "INSERT INTO captures (raw_text, source, created_at_ms) \
         VALUES ('__life_area_picker_probe__', 'test', 0)",
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("insert probe capture: {e}"))?;
    Ok(())
}

/// The picker's unselected placeholder (`<option value="">`,
/// `D-manual-triage-until-llm`) is not a life area choice -- dropped so "the
/// triage life area choices are exactly ..." keeps naming only real ones.
fn exclude_blank_placeholder(options: Vec<String>) -> Vec<String> {
    options
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect()
}

async fn dispatch_triage_choices_exactly(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let listed = resolve(example, &caps[1])?;
    let expected: Vec<String> = listed.split(", ").map(str::to_string).collect();
    let actual = current_triage_life_area_choices(world).await?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected triage life area choices {expected:?}, got {actual:?}"
        ))
    }
}

pub(super) async fn capture_id_by_text(world: &World, raw_text: &str) -> Result<i64, String> {
    let pool = world.pool()?;
    sqlx::query_scalar("SELECT id FROM captures WHERE raw_text = ? ORDER BY id DESC LIMIT 1")
        .bind(raw_text)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("query capture by text: {e}"))?
        .ok_or_else(|| format!("no capture found with raw text {raw_text:?}"))
}

async fn triage_pool_in_life_area(
    world: &mut World,
    capture_id: i64,
    life_area: &str,
) -> Result<(), String> {
    let uri = format!("/captures/{capture_id}/triage");
    let response = super::app_client::post_json(
        world,
        &uri,
        &json!({ "kind": "pool", "life_area": life_area }),
    )
    .await?;
    world.last_status = Some(response.status);
    world.last_response_body = response.body;
    Ok(())
}

async fn dispatch_named_capture_triaged(
    world: &mut World,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = &caps[1];
    let life_area = &caps[2];
    let capture_id = capture_id_by_text(world, raw_text).await?;
    triage_pool_in_life_area(world, capture_id, life_area).await
}

async fn dispatch_capture_triaged_in_life_area(
    world: &mut World,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let life_area = &caps[1];
    let capture_id = world
        .last_capture_id
        .ok_or_else(|| "no capture set up for this scenario".to_string())?;
    triage_pool_in_life_area(world, capture_id, life_area).await
}

fn payload_for_kind(kind: &str) -> Result<Value, String> {
    match kind {
        "pool" => Ok(payloads::pool()),
        "committed" => Ok(payloads::committed()),
        "quota" => Ok(payloads::quota()),
        other => Err(format!("unknown kind {other:?}")),
    }
}

async fn dispatch_triaged_omitting_life_area(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let kind = resolve(example, &caps[1])?;
    let payload = payloads::without_field(payload_for_kind(&kind)?, "life_area");
    when_triaged(world, payload).await
}

/// Whether `form`'s (pool/committed/quota) rendered `life_area` picker
/// preselects nothing -- the first `<option>` is the blank placeholder
/// (`D-manual-triage-until-llm`), which is what "preselects nothing" means
/// for a `<select>` with no `selected` attribute anywhere in it.
fn then_form_preselects_no_life_area(section: &str, kind: &str) -> Result<(), String> {
    let form = form_section(section, kind)?;
    let options = select_option_values(form, "life_area")?;
    if options.first().map(String::as_str) == Some("") {
        Ok(())
    } else {
        Err(format!(
            "expected the {kind} form's life_area picker to preselect nothing, got {options:?}"
        ))
    }
}

fn dispatch_kind_form_preselects_none(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let kind = example_value(example, &caps[1])?;
    let body = html_body(world)?;
    let section = html::captures_section(body)?;
    then_form_preselects_no_life_area(section, kind)
}

/// The quick-add response is the capture row's own markup (`capture_row.html`
/// rendered directly, without a `<ul id="captures">` wrapper around it), so
/// this reads the body itself rather than scoping through
/// `html::captures_section` the way an inbox-page response would.
fn then_quick_add_pool_preselects_none(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    then_form_preselects_no_life_area(body, "pool")
}

fn without_life_area(
    fields: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> Vec<(&'static str, &'static str)> {
    fields
        .into_iter()
        .filter(|(name, _)| *name != "life_area")
        .collect()
}

fn form_fields_without_life_area(kind: &str) -> Result<Vec<(&'static str, &'static str)>, String> {
    let fields = match kind {
        "pool" => vec![("kind", "pool")],
        "committed" => without_life_area(committed_form_fields()),
        "quota" => without_life_area(quota_form_fields()),
        other => return Err(format!("unknown kind {other:?}")),
    };
    Ok(fields)
}

async fn dispatch_triaged_through_page_no_life_area(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let kind = example_value(example, &caps[1])?;
    let fields = form_fields_without_life_area(kind)?;
    when_triaged_through_page(world, &fields).await
}

/// "The task list shows TEXT tagged VALUE" is one step text shared by two
/// features that mean two different things by it -- `life_area_triage.
/// feature`'s life area (rendered `" (Name)"`) and `context_tags.feature`'s
/// context tag (rendered bare, `"@tag"`). Both dispatch here, since
/// `life_areas::dispatch` is tried first in the chain; a value-anywhere-in-
/// the-row check is correct for either rendering without needing to know
/// which one it is, and row-scoping (`html::row_containing`) is what keeps
/// it from matching a neighbouring row's own value instead.
fn dispatch_task_list_shows_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let text = resolve(example, &caps[1])?;
    let value = resolve(example, &caps[2])?;
    then_task_list_shows_tagged(world, &text, &value)
}

fn then_task_list_shows_tagged(world: &mut World, text: &str, value: &str) -> Result<(), String> {
    let body = html_body(world)?;
    let section = html::tasks_section(body)?;
    let row = html::row_containing(section, text)?;
    if row.contains(value) {
        Ok(())
    } else {
        Err(format!(
            "expected {value:?} in the row for {text:?}, got:\n{row}"
        ))
    }
}

/// The task list shows `text`'s own row with no life area rendered at all --
/// `life-area-triage-optional-03`/`-page-optional-09`'s own assertion, now
/// that a life area is not required: a task triaged with none must show
/// none, not a silent default (`D-manual-triage-until-llm`).
fn then_task_list_shows_no_life_area(world: &mut World, text: &str) -> Result<(), String> {
    let body = html_body(world)?;
    let section = html::tasks_section(body)?;
    let row = html::row_containing(section, text)?;
    if row.contains(" (") {
        Err(format!(
            "expected no life area in the row for {text:?}, got:\n{row}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_a_literal_as_is() {
        assert_eq!(resolve(&BTreeMap::new(), "Work"), Ok("Work".to_string()));
    }

    #[test]
    fn resolve_looks_up_a_bracketed_placeholder() {
        let ex = example(&[("name", "Side project")]);
        assert_eq!(resolve(&ex, "<name>"), Ok("Side project".to_string()));
    }

    #[test]
    fn resolve_errors_when_the_placeholder_is_missing_from_the_example() {
        assert!(resolve(&BTreeMap::new(), "<name>").is_err());
    }

    #[test]
    fn life_area_row_names_reads_names_in_document_order() {
        let section = concat!(
            r#"<li id="life-area-row-1"><span class="life-area-name">Work</span> <form></form></li>"#,
            r#"<li id="life-area-row-2"><span class="life-area-name">Fitness</span> <form></form></li>"#,
        );
        assert_eq!(
            life_area_row_names(section),
            vec!["Work".to_string(), "Fitness".to_string()]
        );
    }

    #[test]
    fn dispatch_life_areas_listed_exactly_passes_when_the_names_match_in_order() {
        let mut world = World::new();
        world.last_html_body = Some(format!(
            r#"<ul id="life-areas">{}</ul>"#,
            concat!(
                r#"<li id="life-area-row-1"><span class="life-area-name">Work</span></li>"#,
                r#"<li id="life-area-row-2"><span class="life-area-name">Fitness</span></li>"#,
            )
        ));
        let ex = example(&[("names", "Work, Fitness")]);
        let caps = THEN_LIFE_AREAS_LISTED_EXACTLY
            .captures(r#"the life areas listed are exactly "<names>""#)
            .unwrap();

        assert_eq!(
            dispatch_life_areas_listed_exactly(&mut world, &ex, &caps),
            Ok(())
        );
    }

    #[test]
    fn dispatch_life_areas_listed_exactly_errors_when_the_names_differ() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="life-areas"><li id="life-area-row-1"><span class="life-area-name">Work</span></li></ul>"#
                .to_string(),
        );
        let ex = example(&[("names", "Work, Fitness")]);
        let caps = THEN_LIFE_AREAS_LISTED_EXACTLY
            .captures(r#"the life areas listed are exactly "<names>""#)
            .unwrap();

        assert!(dispatch_life_areas_listed_exactly(&mut world, &ex, &caps).is_err());
    }

    #[test]
    fn then_life_areas_list_excludes_passes_when_the_forbidden_text_is_absent() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<ul id="life-areas"><li>Work</li></ul>"#.to_string());

        assert_eq!(
            then_life_areas_list_excludes(&mut world, "<script>"),
            Ok(())
        );
    }

    #[test]
    fn then_life_areas_list_excludes_errors_when_the_forbidden_text_is_present() {
        let mut world = World::new();
        world.last_html_body =
            Some(r#"<ul id="life-areas"><li><script>boom</script></li></ul>"#.to_string());

        assert!(then_life_areas_list_excludes(&mut world, "<script>").is_err());
    }

    #[test]
    fn then_life_areas_list_contains_passes_when_the_expected_text_is_present() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<ul id="life-areas"><li>Work</li></ul>"#.to_string());

        assert_eq!(then_life_areas_list_contains(&mut world, "Work"), Ok(()));
    }

    #[test]
    fn then_life_areas_list_contains_errors_when_the_expected_text_is_absent() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<ul id="life-areas"><li>Fitness</li></ul>"#.to_string());

        assert!(then_life_areas_list_contains(&mut world, "Work").is_err());
    }

    #[test]
    fn rejection_reports_unknown_life_area_passes_when_the_name_matches() {
        let body = json!({ "unknown_life_area": "Gardening" });
        assert_eq!(
            rejection_reports_unknown_life_area(&body, "Gardening"),
            Ok(())
        );
    }

    #[test]
    fn rejection_reports_unknown_life_area_errors_when_the_name_differs() {
        let body = json!({ "unknown_life_area": "Gardening" });
        assert!(rejection_reports_unknown_life_area(&body, "Fitness").is_err());
    }

    #[test]
    fn dispatch_rejection_says_already_errors_when_the_message_is_absent() {
        let mut world = World::new();
        world.last_html_body = Some("<p>something else entirely</p>".to_string());
        let ex = example(&[]);
        let caps = THEN_REJECTION_SAYS_ALREADY
            .captures(r#"the rejection says "Work" is already a life area"#)
            .unwrap();

        assert!(dispatch_rejection_says_already(&mut world, &ex, &caps).is_err());
    }

    #[test]
    fn dispatch_rejection_says_not_a_life_area_errors_when_no_body_was_recorded() {
        let mut world = World::new();
        let ex = example(&[]);
        let caps = THEN_REJECTION_SAYS_NOT_A_LIFE_AREA
            .captures(r#"the rejection says "Gardening" is not a life area"#)
            .unwrap();

        assert!(dispatch_rejection_says_not_a_life_area(&mut world, &ex, &caps).is_err());
    }

    #[tokio::test]
    async fn dispatch_triage_choices_exactly_errors_when_the_choices_differ() {
        let mut world = migrated_world().await;
        let ex = example(&[("choices", "Work, Fitness")]);
        let caps = THEN_TRIAGE_CHOICES_EXACTLY
            .captures(r#"the triage life area choices are exactly "<choices>""#)
            .unwrap();

        assert!(dispatch_triage_choices_exactly(&mut world, &ex, &caps)
            .await
            .is_err());
    }

    fn row_with_life_area_preselected(value: &str) -> String {
        format!(
            r#"<li><form><select name="life_area"><option value="{value}">{value}</option></select></form><details></details></li>"#
        )
    }

    #[test]
    fn dispatch_kind_form_preselects_none_errors_when_a_real_life_area_is_preselected() {
        let mut world = World::new();
        world.last_html_body = Some(format!(
            r#"<ul id="captures">{}</ul>"#,
            row_with_life_area_preselected("Work")
        ));
        let ex = example(&[("kind", "pool")]);
        let re = Regex::new(r"^the <(\w+)> triage form preselects no life area$").unwrap();
        let caps = re
            .captures("the <kind> triage form preselects no life area")
            .unwrap();

        assert!(dispatch_kind_form_preselects_none(&mut world, &ex, &caps).is_err());
    }

    #[test]
    fn then_quick_add_pool_preselects_none_errors_when_a_real_life_area_is_preselected() {
        let mut world = World::new();
        world.last_html_body = Some(row_with_life_area_preselected("Work"));

        assert!(then_quick_add_pool_preselects_none(&mut world).is_err());
    }

    #[test]
    fn then_not_redirect_passes_for_a_non_redirect_status() {
        let mut world = World::new();
        world.last_status = Some(201);
        assert_eq!(then_not_redirect(&mut world), Ok(()));
    }

    #[test]
    fn then_not_redirect_errors_for_a_redirect_status() {
        let mut world = World::new();
        world.last_status = Some(302);
        assert!(then_not_redirect(&mut world).is_err());
    }

    #[test]
    fn then_status_is_passes_when_it_matches() {
        let mut world = World::new();
        world.last_status = Some(422);
        assert_eq!(then_status_is(&mut world, 422), Ok(()));
    }

    #[test]
    fn then_status_is_errors_when_it_does_not_match() {
        let mut world = World::new();
        world.last_status = Some(200);
        assert!(then_status_is(&mut world, 422).is_err());
    }

    #[test]
    fn dispatch_rejection_says_already_finds_the_message() {
        let mut world = World::new();
        world.last_html_body = Some("<p>Work is already a life area</p>".to_string());
        let ex = example(&[]);
        let caps = THEN_REJECTION_SAYS_ALREADY
            .captures(r#"the rejection says "Work" is already a life area"#)
            .unwrap();
        assert_eq!(
            dispatch_rejection_says_already(&mut world, &ex, &caps),
            Ok(())
        );
    }

    #[test]
    fn dispatch_rejection_says_not_a_life_area_matches_the_json_body() {
        let mut world = World::new();
        world.last_response_body = Some(json!({ "unknown_life_area": "Gardening" }));
        let ex = example(&[]);
        let caps = THEN_REJECTION_SAYS_NOT_A_LIFE_AREA
            .captures(r#"the rejection says "Gardening" is not a life area"#)
            .unwrap();
        assert_eq!(
            dispatch_rejection_says_not_a_life_area(&mut world, &ex, &caps),
            Ok(())
        );
    }

    #[test]
    fn then_task_list_shows_tagged_finds_the_text_and_life_area_together() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="tasks"><li>[pool] sketch the landing page (Learning)</li></ul>"#.to_string(),
        );
        assert_eq!(
            then_task_list_shows_tagged(&mut world, "sketch the landing page", "Learning"),
            Ok(())
        );
    }

    #[test]
    fn then_task_list_shows_tagged_errors_when_the_pair_does_not_match() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="tasks"><li>[pool] sketch the landing page (Home)</li></ul>"#.to_string(),
        );
        assert!(
            then_task_list_shows_tagged(&mut world, "sketch the landing page", "Learning").is_err()
        );
    }

    #[test]
    fn then_task_list_shows_no_life_area_passes_when_the_row_carries_none() {
        let mut world = World::new();
        world.last_html_body =
            Some(r#"<ul id="tasks"><li>[pool] sketch the landing page</li></ul>"#.to_string());
        assert_eq!(
            then_task_list_shows_no_life_area(&mut world, "sketch the landing page"),
            Ok(())
        );
    }

    #[test]
    fn then_task_list_shows_no_life_area_errors_when_the_row_carries_one() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="tasks"><li>[pool] sketch the landing page (Learning)</li></ul>"#.to_string(),
        );
        assert!(then_task_list_shows_no_life_area(&mut world, "sketch the landing page").is_err());
    }

    #[test]
    fn payload_for_kind_names_the_kind_requested() {
        assert_eq!(payload_for_kind("pool").unwrap()["kind"], json!("pool"));
        assert_eq!(
            payload_for_kind("committed").unwrap()["kind"],
            json!("committed")
        );
        assert_eq!(payload_for_kind("quota").unwrap()["kind"], json!("quota"));
    }

    #[test]
    fn payload_for_kind_errors_on_an_unknown_kind() {
        assert!(payload_for_kind("someday").is_err());
    }

    #[tokio::test]
    async fn when_life_areas_page_viewed_records_the_seeded_list() {
        let mut world = migrated_world().await;

        when_life_areas_page_viewed(&mut world).await.unwrap();

        assert_eq!(world.last_status, Some(200));
        assert!(html_body(&world).unwrap().contains("Work"));
    }

    #[tokio::test]
    async fn post_life_area_form_adds_a_new_active_life_area() {
        let mut world = migrated_world().await;

        post_life_area_form(&mut world, "Side project")
            .await
            .unwrap();

        assert_eq!(world.last_status, Some(201));
        let pool = world.pool().unwrap();
        assert!(
            trellis_server::life_areas::store::find_by_name(pool, "Side project")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn capture_id_by_text_finds_the_matching_capture() {
        let mut world = migrated_world().await;
        super::super::triage::given_capture_waiting(&mut world, "buy milk")
            .await
            .unwrap();

        let id = capture_id_by_text(&world, "buy milk").await.unwrap();

        assert_eq!(Some(id), world.last_capture_id);
    }

    #[tokio::test]
    async fn capture_id_by_text_errors_when_no_capture_matches() {
        let world = migrated_world().await;

        assert!(capture_id_by_text(&world, "nothing here").await.is_err());
    }

    fn row_with_pool_life_area_options(options: &[&str]) -> String {
        let opts: String = std::iter::once(r#"<option value="">— choose —</option>"#.to_string())
            .chain(
                options
                    .iter()
                    .map(|v| format!(r#"<option value="{v}">{v}</option>"#)),
            )
            .collect();
        format!(
            r#"<li><form><select name="life_area">{opts}</select></form><details></details></li>"#
        )
    }

    #[test]
    fn then_form_preselects_no_life_area_passes_when_the_first_option_is_blank() {
        let row = row_with_pool_life_area_options(&["Work", "Home"]);
        assert_eq!(then_form_preselects_no_life_area(&row, "pool"), Ok(()));
    }

    #[test]
    fn then_form_preselects_no_life_area_errors_when_the_first_option_is_a_real_choice() {
        let row = format!(
            r#"<li><form><select name="life_area">{}</select></form><details></details></li>"#,
            r#"<option value="Work">Work</option>"#
        );
        assert!(then_form_preselects_no_life_area(&row, "pool").is_err());
    }

    #[tokio::test]
    async fn dispatch_kind_form_preselects_none_reads_the_kind_from_the_example() {
        let mut world = World::new();
        world.last_html_body = Some(format!(
            r#"<ul id="captures">{}</ul>"#,
            row_with_pool_life_area_options(&["Work"])
        ));
        let ex = example(&[("kind", "pool")]);
        let re = Regex::new(r"^the <(\w+)> triage form preselects no life area$").unwrap();
        let caps = re
            .captures("the <kind> triage form preselects no life area")
            .unwrap();

        assert_eq!(
            dispatch_kind_form_preselects_none(&mut world, &ex, &caps),
            Ok(())
        );
    }

    #[test]
    fn then_quick_add_pool_preselects_none_reads_the_body_directly_with_no_ul_wrapper() {
        let mut world = World::new();
        world.last_html_body = Some(row_with_pool_life_area_options(&["Work"]));

        assert_eq!(then_quick_add_pool_preselects_none(&mut world), Ok(()));
    }

    #[test]
    fn form_fields_without_life_area_drops_life_area_from_committed() {
        let fields = form_fields_without_life_area("committed").unwrap();
        assert!(!fields.iter().any(|(name, _)| *name == "life_area"));
        assert!(fields.iter().any(|(name, _)| *name == "deadline"));
    }

    #[test]
    fn form_fields_without_life_area_drops_life_area_from_quota() {
        let fields = form_fields_without_life_area("quota").unwrap();
        assert!(!fields.iter().any(|(name, _)| *name == "life_area"));
        assert!(fields.iter().any(|(name, _)| *name == "target_count"));
    }

    #[test]
    fn form_fields_without_life_area_for_pool_is_just_the_kind() {
        assert_eq!(
            form_fields_without_life_area("pool").unwrap(),
            vec![("kind", "pool")]
        );
    }

    #[test]
    fn form_fields_without_life_area_errors_for_an_unknown_kind() {
        assert!(form_fields_without_life_area("banana").is_err());
    }

    #[tokio::test]
    async fn triaging_through_the_page_with_no_life_area_chosen_succeeds() {
        let mut world = migrated_world().await;
        super::super::triage::given_capture_waiting(&mut world, "buy milk")
            .await
            .unwrap();

        dispatch_triaged_through_page_no_life_area(
            &mut world,
            &example(&[("kind", "pool")]),
            &WHEN_TRIAGED_THROUGH_PAGE_NO_LIFE_AREA
                .captures("the capture is triaged as a <kind> task through the page with no life area chosen")
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(world.last_status, Some(201));
    }
}
