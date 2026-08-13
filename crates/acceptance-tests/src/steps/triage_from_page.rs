//! Step handlers for `features/triage_from_page.feature`: triage happens on
//! the page, through the same validation as the API (issue #33).
//!
//! Kept as its own module rather than added to `triage::dispatch`, which is
//! already over the project's complexity threshold (see the triage-validation
//! handoff brief's "known repo gotchas"). The Background, "a capture ... is
//! waiting" and the generic rejection steps ("the triage is rejected", "the
//! rejection names <field>", "the task list is still empty", "the capture is
//! still waiting") this feature also uses are already matched generically by
//! [`super::triage::dispatch`] and [`super::quota_triage_validation::dispatch`],
//! tried before this module — nothing here duplicates them.

use super::html;
use super::inbox_view::{html_response, urlencode};
use super::payloads;
use super::*;
use axum::body::Body;
use axum::http::Request;

static THEN_OFFERS_ALL_KINDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the inbox offers to triage "([^"]+)" as pool, committed and quota$"#).unwrap()
});
static WHEN_TRIAGED_AS_POOL_THROUGH_PAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the capture is triaged as a pool task through the page$").unwrap()
});
static THEN_PAGE_RESPONSE_NOT_REDIRECT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the page's triage response does not redirect the browser$").unwrap()
});
static THEN_INBOX_DOES_NOT_LIST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the inbox does not list "([^"]+)"$"#).unwrap());
static THEN_TASK_LIST_SHOWS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the task list shows "([^"]+)"$"#).unwrap());
static WHEN_TRIAGED_AS_COMMITTED_THROUGH_PAGE_OMITTING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the capture is triaged as a committed task through the page with "([^"]+)" omitted$"#,
    )
    .unwrap()
});
static WHEN_TRIAGED_AS_QUOTA_THROUGH_PAGE_OMITTING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the capture is triaged as a quota task through the page with "([^"]+)" omitted$"#,
    )
    .unwrap()
});
static THEN_OFFERS_DEADLINE_TYPE_CHOICES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the committed form offers exactly the deadline type choices "hard" and "soft"$"#)
        .unwrap()
});
static THEN_OFFERS_PRIORITY_CHOICES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the committed form offers exactly the priority choices "P1", "P2", "P3" and "P4"$"#,
    )
    .unwrap()
});
static THEN_TASK_LIST_NO_UNESCAPED_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the task list does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_TASK_LIST_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the task list contains the word "([^"]+)"$"#).unwrap());

pub async fn dispatch(world: &mut World, text: &str) -> Option<Result<(), String>> {
    if let Some(caps) = THEN_OFFERS_ALL_KINDS.captures(text) {
        return Some(then_offers_all_kinds(world, &caps[1]));
    }
    if WHEN_TRIAGED_AS_POOL_THROUGH_PAGE.is_match(text) {
        return Some(when_triaged_through_page(world, &[("kind", "pool")]).await);
    }
    if THEN_PAGE_RESPONSE_NOT_REDIRECT.is_match(text) {
        return Some(then_page_response_not_redirect(world));
    }
    if let Some(caps) = THEN_INBOX_DOES_NOT_LIST.captures(text) {
        return Some(then_inbox_does_not_list(world, &caps[1]));
    }
    if let Some(caps) = THEN_TASK_LIST_SHOWS.captures(text) {
        return Some(then_task_list_contains(world, &caps[1]));
    }
    if let Some(caps) = WHEN_TRIAGED_AS_COMMITTED_THROUGH_PAGE_OMITTING.captures(text) {
        return Some(dispatch_committed_omitting(world, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_AS_QUOTA_THROUGH_PAGE_OMITTING.captures(text) {
        return Some(dispatch_quota_omitting(world, &caps).await);
    }
    if THEN_OFFERS_DEADLINE_TYPE_CHOICES.is_match(text) {
        return Some(then_select_offers_exactly(
            world,
            "deadline_type",
            &["hard", "soft"],
        ));
    }
    if THEN_OFFERS_PRIORITY_CHOICES.is_match(text) {
        return Some(then_select_offers_exactly(
            world,
            "priority",
            &["P1", "P2", "P3", "P4"],
        ));
    }
    if THEN_TASK_LIST_NO_UNESCAPED_SCRIPT.is_match(text) {
        return Some(then_task_list_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_TASK_LIST_CONTAINS_WORD.captures(text) {
        return Some(then_task_list_contains(world, &caps[1]));
    }
    None
}

fn capture_id(world: &World) -> Result<i64, String> {
    world
        .last_capture_id
        .ok_or_else(|| "no capture set up for this scenario".to_string())
}

async fn when_triaged_through_page(
    world: &mut World,
    fields: &[(&str, &str)],
) -> Result<(), String> {
    let capture_id = capture_id(world)?;
    let body = fields
        .iter()
        .map(|(name, value)| format!("{name}={}", urlencode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let request = Request::builder()
        .method("POST")
        .uri(format!("/captures/{capture_id}/triage"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

/// A well-formed committed submission's fields as page form fields — the
/// same values `payloads::committed` uses for the JSON API, so the two
/// validation paths are exercised against the same canonical inputs.
fn committed_form_fields() -> [(&'static str, &'static str); 4] {
    [
        ("kind", "committed"),
        ("deadline", payloads::VALID_DEADLINE),
        ("deadline_type", "hard"),
        ("priority", "P1"),
    ]
}

fn quota_form_fields() -> [(&'static str, &'static str); 4] {
    [
        ("kind", "quota"),
        ("target_count", "3"),
        ("target_minutes_each", "45"),
        ("period", "week"),
    ]
}

/// This scenario has no Examples table — the omitted field's name is
/// written literally in the step text (`with "deadline" omitted`), not as a
/// `<placeholder>` — so `&caps[1]` is the field name itself, not something
/// to resolve against an example row.
async fn dispatch_committed_omitting(
    world: &mut World,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing_field = &caps[1];
    let fields: Vec<(&str, &str)> = committed_form_fields()
        .into_iter()
        .filter(|(name, _)| *name != missing_field)
        .collect();
    when_triaged_through_page(world, &fields).await
}

async fn dispatch_quota_omitting(
    world: &mut World,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing_field = &caps[1];
    let fields: Vec<(&str, &str)> = quota_form_fields()
        .into_iter()
        .filter(|(name, _)| *name != missing_field)
        .collect();
    when_triaged_through_page(world, &fields).await
}

fn then_offers_all_kinds(world: &mut World, raw_text: &str) -> Result<(), String> {
    let body = world
        .last_html_body
        .as_deref()
        .ok_or_else(|| "no HTML response recorded".to_string())?;
    let section = html::captures_section(body)?;
    for expected in [raw_text, "Pool", "Committed", "Quota"] {
        if !section.contains(expected) {
            return Err(format!(
                "expected the inbox to offer {expected:?} for {raw_text:?}, got:\n{section}"
            ));
        }
    }
    Ok(())
}

/// This is the page's own response to a page-originated triage — a `#lists`
/// fragment, not a page — so "not a redirect" is checked against
/// `world.last_status` exactly as `inbox_view`'s quick-add check is; the two
/// are not unified into one step only because the Gherkin gives them
/// different wording ("the page's triage response" vs. "the quick-add
/// submission").
fn then_page_response_not_redirect(world: &mut World) -> Result<(), String> {
    match world.last_status {
        Some(status) if !(300..400).contains(&status) => Ok(()),
        Some(status) => Err(format!("expected no redirect, got status {status}")),
        None => Err("no triage response recorded".to_string()),
    }
}

fn then_inbox_does_not_list(world: &mut World, raw_text: &str) -> Result<(), String> {
    let body = world
        .last_html_body
        .as_deref()
        .ok_or_else(|| "no HTML response recorded".to_string())?;
    let section = html::captures_section(body)?;
    if section.contains(raw_text) {
        Err(format!(
            "expected {raw_text:?} to be gone from the inbox, got:\n{section}"
        ))
    } else {
        Ok(())
    }
}

fn task_list_section(world: &World) -> Result<&str, String> {
    let body = world
        .last_html_body
        .as_deref()
        .ok_or_else(|| "no HTML response recorded".to_string())?;
    html::tasks_section(body)
}

fn then_task_list_contains(world: &mut World, expected: &str) -> Result<(), String> {
    let section = task_list_section(world)?;
    if section.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected:?} in the task list, got:\n{section}"
        ))
    }
}

fn then_task_list_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let section = task_list_section(world)?;
    if section.contains(forbidden) {
        Err(format!(
            "expected no {forbidden:?} in the task list, got:\n{section}"
        ))
    } else {
        Ok(())
    }
}

/// The values every `<option value="...">` in `<select name="field_name">`
/// carries, in document order.
fn select_option_values(section: &str, field_name: &str) -> Result<Vec<String>, String> {
    let start_tag = format!(r#"<select name="{field_name}">"#);
    let select = html::between(section, &start_tag, "</select>")?;
    Ok(select
        .split("<option value=\"")
        .skip(1)
        .filter_map(|chunk| chunk.split('"').next())
        .map(str::to_string)
        .collect())
}

fn then_select_offers_exactly(
    world: &mut World,
    field_name: &str,
    expected: &[&str],
) -> Result<(), String> {
    let body = world
        .last_html_body
        .as_deref()
        .ok_or_else(|| "no HTML response recorded".to_string())?;
    let section = html::captures_section(body)?;
    let actual = select_option_values(section, field_name)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the {field_name} choices {expected:?}, got {actual:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::triage::given_capture_waiting;
    use super::*;

    #[tokio::test]
    async fn pool_triage_through_the_page_moves_the_capture_into_the_task_list() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();

        when_triaged_through_page(&mut world, &[("kind", "pool")])
            .await
            .unwrap();
        then_page_response_not_redirect(&mut world).unwrap();

        world.last_html_body = None;
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        html_response(&mut world, request).await.unwrap();

        then_inbox_does_not_list(&mut world, "buy milk").unwrap();
        then_task_list_contains(&mut world, "buy milk").unwrap();
    }

    #[tokio::test]
    async fn committed_triage_missing_deadline_is_rejected_through_the_page() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "call the dentist")
            .await
            .unwrap();

        let fields: Vec<(&str, &str)> = committed_form_fields()
            .into_iter()
            .filter(|(name, _)| *name != "deadline")
            .collect();
        when_triaged_through_page(&mut world, &fields)
            .await
            .unwrap();

        assert_eq!(world.last_status, Some(422));
    }

    #[test]
    fn select_option_values_reads_the_options_in_order() {
        let section = r#"<select name="deadline_type"><option value="hard">hard</option><option value="soft">soft</option></select>"#;
        assert_eq!(
            select_option_values(section, "deadline_type").unwrap(),
            vec!["hard", "soft"]
        );
    }

    #[test]
    fn then_offers_all_kinds_errors_when_a_capture_is_missing_from_the_inbox() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<ul id="captures"></ul>"#.to_string());
        assert!(then_offers_all_kinds(&mut world, "buy milk").is_err());
    }

    fn world_with_select(field_name: &str, options: &[&str]) -> World {
        let opts: String = options
            .iter()
            .map(|v| format!(r#"<option value="{v}">{v}</option>"#))
            .collect();
        let mut world = World::new();
        world.last_html_body = Some(format!(
            r#"<ul id="captures"><select name="{field_name}">{opts}</select></ul>"#
        ));
        world
    }

    #[test]
    fn then_select_offers_exactly_passes_when_the_options_match_in_order() {
        let mut world = world_with_select("deadline_type", &["hard", "soft"]);
        assert_eq!(
            then_select_offers_exactly(&mut world, "deadline_type", &["hard", "soft"]),
            Ok(())
        );
    }

    #[test]
    fn then_select_offers_exactly_errors_when_an_option_is_missing() {
        let mut world = world_with_select("deadline_type", &["hard"]);
        assert!(
            then_select_offers_exactly(&mut world, "deadline_type", &["hard", "soft"]).is_err()
        );
    }

    #[test]
    fn then_select_offers_exactly_errors_when_an_extra_option_is_present() {
        let mut world = world_with_select("priority", &["P1", "P2", "P3", "P4", "P5"]);
        assert!(
            then_select_offers_exactly(&mut world, "priority", &["P1", "P2", "P3", "P4"]).is_err()
        );
    }
}
