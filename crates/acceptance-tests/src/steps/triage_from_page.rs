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
        return Some(dispatch_inbox_does_not_list(world, example, &caps));
    }
    if let Some(caps) = WHEN_TRIAGED_AS_COMMITTED_THROUGH_PAGE_OMITTING.captures(text) {
        return Some(dispatch_committed_omitting(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_AS_QUOTA_THROUGH_PAGE_OMITTING.captures(text) {
        return Some(dispatch_quota_omitting(world, example, &caps).await);
    }
    if THEN_OFFERS_COMMITMENT_CHOICES.is_match(text) {
        return Some(dispatch_offers_commitment_choices(world).await);
    }
    if THEN_OFFERS_PRIORITY_CHOICES.is_match(text) {
        return Some(dispatch_offers_priority_choices(world).await);
    }
    if THEN_POOL_NOT_BEHIND_CONTROL.is_match(text) {
        return Some(then_pool_not_behind_control(world));
    }
    if let Some(caps) = THEN_POOL_FEWER_INPUTS.captures(text) {
        return Some(dispatch_pool_fewer_inputs(world, example, &caps).await);
    }
    None
}

fn capture_id(world: &World) -> Result<i64, String> {
    world
        .last_capture_id
        .ok_or_else(|| "no capture set up for this scenario".to_string())
}

/// Which kind's fields panel a row shows is a display preference the
/// server remembers (#119, `inbox::set_shown_kind`), not something a
/// `<details>` toggles client-side -- so the two scenarios that inspect a
/// panel's own fields (committed's closed choices, and the pool-is-cheapest
/// comparison) must actually choose the kind first, the same POST a click on
/// its button sends, before there is a panel in the response to inspect.
pub(super) async fn choose_kind(world: &mut World, kind: &str) -> Result<(), String> {
    let capture_id = capture_id(world)?;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/captures/{capture_id}/kind"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!("kind={kind}")))
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
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

pub(super) fn quota_form_fields() -> [(&'static str, &'static str); 4] {
    [
        ("kind", "quota"),
        ("name", payloads::VALID_QUOTA_NAME),
        ("hours", payloads::VALID_QUOTA_HOURS),
        ("life_area", payloads::VALID_LIFE_AREA),
    ]
}

/// Resolves a captured value that may be a literal or an `Examples`
/// placeholder (`<missing_field>`) written down verbatim in the step text.
/// `triage_from_page.feature`'s own two omitting scenarios write the field
/// literally (no Examples table); `disclosures.feature`'s reuses the same
/// step wording with a genuine `<missing_field>` placeholder over an
/// Examples table (#119, `disclosures-submissions-unchanged-06`), so both
/// dispatch sites below must resolve rather than assume literal text. See
/// `pool_screen.rs`'s own copy of this function for the full reasoning.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

async fn dispatch_committed_omitting(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing_field = resolve(example, &caps[1])?;
    let fields: Vec<(&str, &str)> = committed_form_fields()
        .into_iter()
        .filter(|(name, _)| *name != missing_field)
        .collect();
    when_triaged_through_page(world, &fields).await
}

async fn dispatch_quota_omitting(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing_field = resolve(example, &caps[1])?;
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

fn dispatch_inbox_does_not_list(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    then_inbox_does_not_list(world, &raw_text)
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

/// Pool's own form (#119: three sibling `<form>`s in `.kinds`, not a
/// `<details>` among them -- `D-pool-is-default` keeps it a plain form
/// regardless of which other kind's panel is open).
fn pool_form_section(section: &str) -> Result<&str, String> {
    html::between(section, r#"<form class="kind-choice kind-wide""#, "</form>")
}

/// The one panel a row shows, whichever kind it is (#119): at most one of
/// `.fields-panel` exists in a row at a time, so this does not need to tell
/// committed's apart from quota's by name -- the caller is what ensures the
/// right one was chosen first (see [`choose_kind`]). Scoped to the next
/// `<form class="dismiss"` rather than counting nested `<div>`s: the panel's
/// own content nests a `<div class="actions">`, so a naive first-`</div>`
/// scope would truncate before the panel's own close, and `<form
/// class="dismiss"` is the row's own stable, always-present next sibling
/// regardless of which panel (or none) is open.
fn open_panel_section(section: &str) -> Result<&str, String> {
    html::between(
        section,
        r#"<div class="fields-panel">"#,
        r#"<form class="dismiss""#,
    )
}

pub(super) fn form_section<'a>(section: &'a str, kind: &str) -> Result<&'a str, String> {
    match kind {
        "pool" => pool_form_section(section),
        "committed" | "quota" => open_panel_section(section),
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

async fn dispatch_pool_fewer_inputs(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let other_kind = example_value(example, &caps[1])?.to_string();
    then_pool_fewer_inputs(world, &other_kind).await
}

fn fewer_inputs_result(
    pool_count: usize,
    other_count: usize,
    other_kind: &str,
) -> Result<(), String> {
    if pool_count < other_count {
        Ok(())
    } else {
        Err(format!(
            "expected pool ({pool_count} fields) to ask for fewer inputs than \
             {other_kind} ({other_count} fields)"
        ))
    }
}

/// Pool's own count comes from the row as already viewed -- its form is
/// present whether or not another kind's panel is open, so nothing needs to
/// change to read it. `other_kind`'s count needs its panel actually open
/// first ([`choose_kind`]), which re-renders `#lists` and is read from that
/// fresh response instead.
async fn then_pool_fewer_inputs(world: &mut World, other_kind: &str) -> Result<(), String> {
    let pool_count = field_count(form_section(captures_list_section(world)?, "pool")?);
    choose_kind(world, other_kind).await?;
    let other_count = field_count(form_section(captures_list_section(world)?, other_kind)?);
    fewer_inputs_result(pool_count, other_count, other_kind)
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

/// Committed's panel (#119) only exists in the response once committed has
/// been chosen for this row -- both `THEN_OFFERS_COMMITMENT_CHOICES` and
/// `THEN_OFFERS_PRIORITY_CHOICES` inspect that same panel, each choosing it
/// fresh rather than relying on the other having already done so, since
/// nothing in the Gherkin orders them relative to each other.
async fn dispatch_offers_commitment_choices(world: &mut World) -> Result<(), String> {
    choose_kind(world, "committed").await?;
    then_commitment_choices_offered_exactly(world, &["at", "by"])
}

async fn dispatch_offers_priority_choices(world: &mut World) -> Result<(), String> {
    choose_kind(world, "committed").await?;
    then_select_offers_exactly(world, "priority", &["P1", "P2", "P3", "P4"])
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
    async fn pool_triage_through_the_page_files_it_without_a_reload_and_the_row_stays() {
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

        let section = captures_list_section(&world).unwrap();
        assert!(
            section.contains("buy milk"),
            "expected the triaged row to stay in Recent, got:\n{section}"
        );
        assert!(
            section.contains("Pool \u{b7} no context"),
            "expected the row to read what it became, got:\n{section}"
        );
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

    /// Pool's own form (#119): the three sibling `<form>`s a row now offers,
    /// standing in for the real template's shape without depending on it.
    /// `.kind-choice.kind-wide` is the marker [`pool_form_section`] scopes
    /// by; the other two are plain `.kind-choice` buttons, no panel open.
    fn row_with_no_panel_open() -> String {
        concat!(
            r#"<li id="capture-row-1">buy milk"#,
            r#"<div class="kinds">"#,
            r#"<form class="kind-choice kind-wide"><input type="hidden" name="kind" value="pool"><select name="life_area"></select></form>"#,
            r#"<form class="kind-choice"><input type="hidden" name="kind" value="committed"><button>Committed</button></form>"#,
            r#"<form class="kind-choice"><input type="hidden" name="kind" value="quota"><button>Quota</button></form>"#,
            r#"</div>"#,
            r#"<form class="dismiss"><button type="submit">Dismiss</button></form>"#,
            r#"</li>"#,
        )
        .to_string()
    }

    /// The same row with committed's panel open (#119) -- what the response
    /// to [`choose_kind`]`(world, "committed")` looks like: its own nested
    /// at/by disclosures (#110: committed no longer offers its commitment
    /// choice through a `<select>`), wrapped in `.fields-panel`, followed by
    /// the row's own stable `<form class="dismiss"` sibling.
    fn row_with_committed_panel_open() -> String {
        concat!(
            r#"<li id="capture-row-1">buy milk"#,
            r#"<div class="kinds">"#,
            r#"<form class="kind-choice kind-wide"><input type="hidden" name="kind" value="pool"><select name="life_area"></select></form>"#,
            r#"<form class="kind-choice chosen"><input type="hidden" name="kind" value="committed"><button>Committed</button></form>"#,
            r#"<form class="kind-choice"><input type="hidden" name="kind" value="quota"><button>Quota</button></form>"#,
            r#"</div>"#,
            r#"<div class="fields-panel">"#,
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
            r#"</div>"#,
            r#"<form class="dismiss"><button type="submit">Dismiss</button></form>"#,
            r#"</li>"#,
        )
        .to_string()
    }

    /// The same row with quota's panel open instead (#119) -- only one panel
    /// is ever open at a time, so this and
    /// [`row_with_committed_panel_open`] are deliberately separate fixtures
    /// rather than one row combining both.
    fn row_with_quota_panel_open() -> String {
        concat!(
            r#"<li id="capture-row-1">buy milk"#,
            r#"<div class="kinds">"#,
            r#"<form class="kind-choice kind-wide"><input type="hidden" name="kind" value="pool"><select name="life_area"></select></form>"#,
            r#"<form class="kind-choice"><input type="hidden" name="kind" value="committed"><button>Committed</button></form>"#,
            r#"<form class="kind-choice chosen"><input type="hidden" name="kind" value="quota"><button>Quota</button></form>"#,
            r#"</div>"#,
            r#"<div class="fields-panel">"#,
            r#"<form><input type="hidden" name="kind" value="quota">"#,
            r#"<input type="text" name="name">"#,
            r#"<input type="number" name="hours"></form>"#,
            r#"</div>"#,
            r#"<form class="dismiss"><button type="submit">Dismiss</button></form>"#,
            r#"</li>"#,
        )
        .to_string()
    }

    #[test]
    fn form_section_scopes_pool_to_its_own_form_regardless_of_which_panel_is_open() {
        let row = row_with_committed_panel_open();
        let section = form_section(&row, "pool").unwrap();
        assert!(section.contains(r#"value="pool""#));
        assert!(!section.contains("At a time"));
    }

    #[test]
    fn form_section_scopes_committed_to_the_open_panel() {
        let row = row_with_committed_panel_open();
        let section = form_section(&row, "committed").unwrap();
        assert!(section.contains(r#"value="committed""#));
        assert!(!section.contains("Dismiss"));
    }

    #[test]
    fn form_section_scopes_committed_past_its_own_nested_at_and_by_disclosures() {
        let row = row_with_committed_panel_open();
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
        let mut world = world_with_row(&row_with_committed_panel_open());
        assert_eq!(
            then_commitment_choices_offered_exactly(&mut world, &["at", "by"]),
            Ok(())
        );
    }

    #[test]
    fn form_section_scopes_quota_to_the_open_panel() {
        let row = row_with_quota_panel_open();
        let section = form_section(&row, "quota").unwrap();
        assert!(section.contains(r#"value="quota""#));
        assert!(!section.contains("At a time"));
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
    fn then_pool_not_behind_control_passes_when_pool_precedes_any_panel() {
        let mut world = world_with_row(&row_with_no_panel_open());
        assert_eq!(then_pool_not_behind_control(&mut world), Ok(()));
    }

    #[test]
    fn then_pool_not_behind_control_errors_when_pool_is_missing() {
        let mut world =
            world_with_row(r#"<div class="fields-panel"></div><form class="dismiss"></form>"#);
        assert!(then_pool_not_behind_control(&mut world).is_err());
    }

    #[test]
    fn fewer_inputs_result_passes_when_pool_asks_for_fewer() {
        assert_eq!(fewer_inputs_result(2, 5, "committed"), Ok(()));
    }

    #[test]
    fn fewer_inputs_result_errors_when_pool_does_not_ask_for_fewer() {
        assert!(fewer_inputs_result(5, 1, "committed").is_err());
    }

    #[tokio::test]
    async fn then_pool_fewer_inputs_passes_against_committed_and_quota() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        html_response(&mut world, request).await.unwrap();

        then_pool_fewer_inputs(&mut world, "committed")
            .await
            .unwrap();
        then_pool_fewer_inputs(&mut world, "quota").await.unwrap();
    }
}
