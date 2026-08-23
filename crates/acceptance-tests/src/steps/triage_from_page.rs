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
static THEN_OFFERS_COMMITMENT_CHOICES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the committed form offers exactly the commitment choices "at" and "by"$"#)
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
static THEN_POOL_NOT_BEHIND_CONTROL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the pool triage form is not behind a control that must be opened first$").unwrap()
});
static THEN_POOL_FEWER_INPUTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the pool triage form asks for fewer inputs than the <(\w+)> form$").unwrap()
});

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = THEN_OFFERS_ALL_KINDS.captures(text) {
        return Some(then_offers_all_kinds(world, &caps[1]));
    }
    if WHEN_TRIAGED_AS_POOL_THROUGH_PAGE.is_match(text) {
        return Some(
            when_triaged_through_page(
                world,
                &[("kind", "pool"), ("life_area", payloads::VALID_LIFE_AREA)],
            )
            .await,
        );
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
    if THEN_OFFERS_COMMITMENT_CHOICES.is_match(text) {
        return Some(then_commitment_choices_offered_exactly(
            world,
            &["at", "by"],
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
    if THEN_POOL_NOT_BEHIND_CONTROL.is_match(text) {
        return Some(then_pool_not_behind_control(world));
    }
    if let Some(caps) = THEN_POOL_FEWER_INPUTS.captures(text) {
        return Some(dispatch_pool_fewer_inputs(world, example, &caps));
    }
    None
}

fn capture_id(world: &World) -> Result<i64, String> {
    world
        .last_capture_id
        .ok_or_else(|| "no capture set up for this scenario".to_string())
}

pub(super) async fn when_triaged_through_page(
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
pub(super) fn committed_form_fields() -> [(&'static str, &'static str); 6] {
    [
        ("kind", "committed"),
        ("deadline", payloads::VALID_DEADLINE),
        ("commitment", "at"),
        ("priority", "P1"),
        ("estimated_minutes", "180"),
        ("life_area", payloads::VALID_LIFE_AREA),
    ]
}

pub(super) fn quota_form_fields() -> [(&'static str, &'static str); 5] {
    [
        ("kind", "quota"),
        ("target_count", "3"),
        ("target_minutes_each", "45"),
        ("period", "week"),
        ("life_area", payloads::VALID_LIFE_AREA),
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

/// The captures section of the last HTML response, mirroring
/// [`task_list_section`] for the row a page-originated triage acts on.
fn captures_list_section(world: &World) -> Result<&str, String> {
    let body = world
        .last_html_body
        .as_deref()
        .ok_or_else(|| "no HTML response recorded".to_string())?;
    html::captures_section(body)
}

fn then_offers_all_kinds(world: &mut World, raw_text: &str) -> Result<(), String> {
    let section = captures_list_section(world)?;
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
    let section = captures_list_section(world)?;
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
pub(super) fn select_option_values(section: &str, field_name: &str) -> Result<Vec<String>, String> {
    let start_tag = format!(r#"<select name="{field_name}">"#);
    let select = html::between(section, &start_tag, "</select>")?;
    Ok(select
        .split("<option value=\"")
        .skip(1)
        .filter_map(|chunk| chunk.split('"').next())
        .map(str::to_string)
        .collect())
}

/// One triage form's own markup within `section`: pool's is everything
/// before the row's first `<details>` (D-pool-is-default keeps it outside
/// any expanding control), committed's and quota's are each scoped to their
/// own `<details>` block by its `<summary>` text.
fn pool_form_section(section: &str) -> Result<&str, String> {
    let end = section
        .find("<details")
        .ok_or_else(|| format!("no <details> found in:\n{section}"))?;
    Ok(&section[..end])
}

/// Committed's own section spans two nested `<details>` disclosures --
/// "At a time" and "By a day" (#110, `committed-date-still-explicit-04`) --
/// so it needs [`html::details_section_after`]'s nesting-aware scoping
/// rather than [`html::between`]'s first-`</details>`-wins scan, which
/// would stop at the inner "At a time" block's own close.
fn committed_form_section(section: &str) -> Result<&str, String> {
    html::details_section_after(section, "<summary>Committed</summary>")
}

pub(super) fn form_section<'a>(section: &'a str, kind: &str) -> Result<&'a str, String> {
    match kind {
        "pool" => pool_form_section(section),
        "committed" => committed_form_section(section),
        "quota" => html::between(section, "<summary>Quota</summary>", "</details>"),
        other => Err(format!("unknown triage kind {other:?}")),
    }
}

/// The values every `<input type="hidden" name="{field_name}" value="...">`
/// carries, in document order -- [`select_option_values`]'s counterpart for
/// a choice made by which of several forms is submitted rather than by a
/// `<select>` (committed's at/by choice, #110:
/// `T-commitment-is-chosen-not-derived` -- the choice is which disclosure
/// the owner opens and fills in, not a value a script infers).
pub(super) fn hidden_input_values(section: &str, field_name: &str) -> Vec<String> {
    let start_tag = format!(r#"<input type="hidden" name="{field_name}" value=""#);
    section
        .split(&start_tag)
        .skip(1)
        .filter_map(|chunk| chunk.split('"').next())
        .map(str::to_string)
        .collect()
}

/// How many form fields (`<input>` and `<select>` elements) a form asks for
/// — AC-2's "fewer inputs" as something countable from the rendered markup.
pub(super) fn field_count(section: &str) -> usize {
    section.matches("<input").count() + section.matches("<select").count()
}

fn then_pool_not_behind_control(world: &mut World) -> Result<(), String> {
    let section = captures_list_section(world)?;
    let pool_section = form_section(section, "pool")?;
    if pool_section.contains(r#"name="kind" value="pool""#) {
        Ok(())
    } else {
        Err(format!(
            "expected the pool form outside any <details>, got:\n{section}"
        ))
    }
}

fn dispatch_pool_fewer_inputs(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let other_kind = example_value(example, &caps[1])?;
    then_pool_fewer_inputs(world, other_kind)
}

fn then_pool_fewer_inputs(world: &mut World, other_kind: &str) -> Result<(), String> {
    let section = captures_list_section(world)?;
    let pool_count = field_count(form_section(section, "pool")?);
    let other_count = field_count(form_section(section, other_kind)?);
    if pool_count < other_count {
        Ok(())
    } else {
        Err(format!(
            "expected pool ({pool_count} fields) to ask for fewer inputs than \
             {other_kind} ({other_count} fields)"
        ))
    }
}

/// [`then_select_offers_exactly`]'s counterpart for committed's at/by
/// choice, which is which of two nested `<details>` disclosures the owner
/// opens and submits (#110) rather than a `<select>`.
fn then_commitment_choices_offered_exactly(
    world: &mut World,
    expected: &[&str],
) -> Result<(), String> {
    let section = captures_list_section(world)?;
    let committed = form_section(section, "committed")?;
    let actual = hidden_input_values(committed, "commitment");
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the commitment choices {expected:?}, got {actual:?}"
        ))
    }
}

fn then_select_offers_exactly(
    world: &mut World,
    field_name: &str,
    expected: &[&str],
) -> Result<(), String> {
    let section = captures_list_section(world)?;
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

        when_triaged_through_page(
            &mut world,
            &[("kind", "pool"), ("life_area", payloads::VALID_LIFE_AREA)],
        )
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
        let section = r#"<select name="commitment"><option value="at">at</option><option value="by">by</option></select>"#;
        assert_eq!(
            select_option_values(section, "commitment").unwrap(),
            vec!["at", "by"]
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
        let mut world = world_with_select("commitment", &["at", "by"]);
        assert_eq!(
            then_select_offers_exactly(&mut world, "commitment", &["at", "by"]),
            Ok(())
        );
    }

    #[test]
    fn then_select_offers_exactly_errors_when_an_option_is_missing() {
        let mut world = world_with_select("commitment", &["at"]);
        assert!(then_select_offers_exactly(&mut world, "commitment", &["at", "by"]).is_err());
    }

    #[test]
    fn then_select_offers_exactly_errors_when_an_extra_option_is_present() {
        let mut world = world_with_select("priority", &["P1", "P2", "P3", "P4", "P5"]);
        assert!(
            then_select_offers_exactly(&mut world, "priority", &["P1", "P2", "P3", "P4"]).is_err()
        );
    }

    /// A capture row with two form fields on the pool form and committed's
    /// own nested at/by disclosures, standing in for the real template's
    /// shape without depending on it (#110: committed no longer offers its
    /// commitment choice through a `<select>`).
    fn row_with_forms() -> String {
        concat!(
            r#"<li id="capture-row-1">buy milk"#,
            r#"<form><input type="hidden" name="kind" value="pool"><select name="life_area"></select></form>"#,
            r#"<details><summary>Committed</summary>"#,
            r#"<details><summary>At a time</summary>"#,
            r#"<form><input type="hidden" name="kind" value="committed">"#,
            r#"<input type="hidden" name="commitment" value="at">"#,
            r#"<input type="date" name="deadline_date">"#,
            r#"<input type="time" name="deadline_time">"#,
            r#"<select name="priority"></select></form></details>"#,
            r#"<details><summary>By a day</summary>"#,
            r#"<form><input type="hidden" name="kind" value="committed">"#,
            r#"<input type="hidden" name="commitment" value="by">"#,
            r#"<input type="date" name="deadline_date">"#,
            r#"<select name="priority"></select></form></details>"#,
            r#"</details>"#,
            r#"<details><summary>Quota</summary>"#,
            r#"<form><input type="hidden" name="kind" value="quota">"#,
            r#"<input type="number" name="target_count">"#,
            r#"<input type="number" name="target_minutes_each"></form></details>"#,
            r#"</li>"#,
        )
        .to_string()
    }

    #[test]
    fn form_section_scopes_pool_to_everything_before_the_first_details() {
        let row = row_with_forms();
        let section = form_section(&row, "pool").unwrap();
        assert!(section.contains(r#"value="pool""#));
        assert!(!section.contains("Committed"));
    }

    #[test]
    fn form_section_scopes_committed_to_its_own_details_block() {
        let row = row_with_forms();
        let section = form_section(&row, "committed").unwrap();
        assert!(section.contains(r#"value="committed""#));
        assert!(!section.contains(r#"value="quota""#));
    }

    #[test]
    fn form_section_scopes_committed_past_its_own_nested_at_and_by_disclosures() {
        let row = row_with_forms();
        let section = form_section(&row, "committed").unwrap();
        assert!(section.contains("At a time"));
        assert!(section.contains("By a day"));
    }

    #[test]
    fn hidden_input_values_reads_every_matching_hidden_input_in_order() {
        let section = concat!(
            r#"<input type="hidden" name="commitment" value="at">"#,
            r#"<input type="hidden" name="commitment" value="by">"#,
        );
        assert_eq!(hidden_input_values(section, "commitment"), vec!["at", "by"]);
    }

    #[test]
    fn hidden_input_values_is_empty_when_the_field_is_absent() {
        assert_eq!(
            hidden_input_values(
                r#"<input type="hidden" name="kind" value="committed">"#,
                "commitment"
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn then_commitment_choices_offered_exactly_passes_for_the_templates_at_and_by_shape() {
        let mut world = world_with_row(&row_with_forms());
        assert_eq!(
            then_commitment_choices_offered_exactly(&mut world, &["at", "by"]),
            Ok(())
        );
    }

    #[test]
    fn form_section_scopes_quota_to_its_own_details_block() {
        let row = row_with_forms();
        let section = form_section(&row, "quota").unwrap();
        assert!(section.contains(r#"value="quota""#));
        assert!(!section.contains(r#"value="committed""#));
    }

    #[test]
    fn field_count_counts_inputs_and_selects() {
        assert_eq!(
            field_count(r#"<input type="hidden"><select></select><input type="text">"#),
            3
        );
    }

    #[test]
    fn field_count_is_zero_for_a_section_with_no_fields() {
        assert_eq!(field_count("<p>nothing here</p>"), 0);
    }

    fn world_with_row(row: &str) -> World {
        let mut world = World::new();
        world.last_html_body = Some(format!(r#"<ul id="captures">{row}</ul>"#));
        world
    }

    #[test]
    fn then_pool_not_behind_control_passes_when_pool_precedes_any_details() {
        let mut world = world_with_row(&row_with_forms());
        assert_eq!(then_pool_not_behind_control(&mut world), Ok(()));
    }

    #[test]
    fn then_pool_not_behind_control_errors_when_pool_is_missing() {
        let mut world = world_with_row("<details><summary>Committed</summary></details>");
        assert!(then_pool_not_behind_control(&mut world).is_err());
    }

    #[test]
    fn then_pool_fewer_inputs_passes_against_committed_and_quota() {
        let mut world = world_with_row(&row_with_forms());
        assert_eq!(then_pool_fewer_inputs(&mut world, "committed"), Ok(()));
        assert_eq!(then_pool_fewer_inputs(&mut world, "quota"), Ok(()));
    }

    #[test]
    fn then_pool_fewer_inputs_errors_when_pool_does_not_ask_for_fewer() {
        let row = concat!(
            r#"<li>"#,
            r#"<form><input type="hidden" name="kind" value="pool">"#,
            r#"<input type="text"><input type="text"><input type="text"></form>"#,
            r#"<details><summary>Committed</summary>"#,
            r#"<form><input type="hidden" name="kind" value="committed"></form></details>"#,
            r#"</li>"#,
        );
        let mut world = world_with_row(row);
        assert!(then_pool_fewer_inputs(&mut world, "committed").is_err());
    }
}
