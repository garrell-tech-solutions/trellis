//! Step handlers for `features/disclosures.feature`: a capture row offers
//! three kinds, and shows one set of fields at a time (#119).
//!
//! The Background, "a capture ... is waiting", "the inbox is viewed", and
//! the generic rejection steps ("the triage is rejected", "the rejection
//! names ...", "the task list is still empty") this feature also uses are
//! already matched generically by [`super::triage::dispatch`]. "The task
//! list shows ..." and "the inbox does not list ..." are
//! [`super::triage_from_page::dispatch`]'s. "The capture is triaged as a
//! committed task through the page with ... omitted" is also
//! [`super::triage_from_page::dispatch`]'s -- the real triage submission
//! (`POST /captures/{id}/triage`) never reads which kind's panel is open,
//! so that step works unchanged once a kind has been chosen. Nothing here
//! duplicates any of them.

use super::html;
use super::payloads;
use super::*;

static THEN_OFFERS_BUTTONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the row for "([^"]+)" offers the kind buttons "([^"]+)"$"#).unwrap()
});
static THEN_SHOWS_NO_KIND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the row for "([^"]+)" shows fields for no kind$"#).unwrap());
static WHEN_KIND_CHOSEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the kind "([^"]+)" is chosen for "([^"]+)"$"#).unwrap());
static THEN_SHOWS_FIELDS_FOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the row for "([^"]+)" shows fields for "([^"]+)"$"#).unwrap());
static THEN_SHOWS_NO_OTHER_KIND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the row for "([^"]+)" shows fields for no other kind$"#).unwrap()
});
static THEN_MARKS_CHOSEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the row for "([^"]+)" marks "([^"]+)" as the chosen kind$"#).unwrap()
});
static THEN_OFFERS_AS_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the row for "([^"]+)" offers "([^"]+)" as the quota name$"#).unwrap()
});
static THEN_NAME_CHANGEABLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the row for "([^"]+)" offers the quota name as something that can be changed$"#)
        .unwrap()
});
static THEN_ASKS_HOURS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the row for "([^"]+)" asks for hours a week$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = THEN_OFFERS_BUTTONS.captures(text) {
        return Some(dispatch_offers_buttons(world, example, &caps));
    }
    if let Some(caps) = THEN_SHOWS_NO_KIND.captures(text) {
        return Some(dispatch_shows_no_kind(world, example, &caps));
    }
    if let Some(caps) = WHEN_KIND_CHOSEN.captures(text) {
        return Some(dispatch_kind_chosen(world, example, &caps).await);
    }
    if let Some(caps) = THEN_SHOWS_FIELDS_FOR.captures(text) {
        return Some(dispatch_shows_fields_for(world, example, &caps));
    }
    if let Some(caps) = THEN_SHOWS_NO_OTHER_KIND.captures(text) {
        return Some(dispatch_shows_no_other_kind(world, example, &caps));
    }
    if let Some(caps) = THEN_MARKS_CHOSEN.captures(text) {
        return Some(dispatch_marks_chosen(world, example, &caps));
    }
    if let Some(caps) = THEN_OFFERS_AS_NAME.captures(text) {
        return Some(dispatch_offers_as_name(world, example, &caps));
    }
    if let Some(caps) = THEN_NAME_CHANGEABLE.captures(text) {
        return Some(dispatch_name_changeable(world, example, &caps));
    }
    if let Some(caps) = THEN_ASKS_HOURS.captures(text) {
        return Some(dispatch_asks_hours(world, example, &caps));
    }
    None
}

/// Resolves a captured value that may be a literal or an `Examples`
/// placeholder (`<chosen>`) written down verbatim in the step text. See
/// `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// The row for `raw_text`, scoped within the last response's captures
/// section -- whatever that response was, a full inbox view or a
/// `choose_kind` fragment swap, both carry the same `<ul id="captures">`
/// shape.
fn row_for<'a>(world: &'a World, raw_text: &str) -> Result<&'a str, String> {
    let body = super::html_body(world, "no HTML response recorded")?;
    let section = html::captures_section(body)?;
    html::row_containing(section, raw_text)
}

async fn dispatch_kind_chosen(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let kind = resolve(example, &caps[1])?;
    let raw_text = resolve(example, &caps[2])?;
    let capture_id = capture_id_by_text(world, &raw_text).await?;
    world.last_capture_id = Some(capture_id);

    match kind.to_lowercase().as_str() {
        "pool" => {
            super::triage_from_page::when_triaged_through_page(
                world,
                &[("kind", "pool"), ("life_area", payloads::VALID_LIFE_AREA)],
            )
            .await
        }
        "committed" | "quota" => {
            super::triage_from_page::choose_kind(world, &kind.to_lowercase()).await
        }
        other => Err(format!("unknown kind {other:?}")),
    }
}

/// The three kind labels in `expected_csv`, checked to appear in `row` in
/// that order -- `<button>Pool</button>`/`<button>Committed</button>`/
/// `<button>Quota</button>` are literal text, no markup to bind to beyond
/// the label itself (`T-qa-binds-tolerantly-to-markup`).
fn buttons_in_order(row: &str, expected_csv: &str) -> Result<(), String> {
    let expected: Vec<&str> = expected_csv.split(", ").collect();
    let mut positions = Vec::with_capacity(expected.len());
    for label in &expected {
        // Scoped to `>{label}</button>` rather than a bare substring search:
        // the kind buttons' own hidden `<input name="kind" value="pool">`
        // carries the same word as the button label, lowercased, so an
        // unscoped search matches the attribute instead of failing to find
        // a differently-cased label.
        let needle = format!(">{label}</button>");
        let pos = row
            .find(&needle)
            .ok_or_else(|| format!("expected the button {label:?} in the row, got:\n{row}"))?;
        positions.push(pos);
    }
    if positions.windows(2).all(|w| w[0] < w[1]) {
        Ok(())
    } else {
        Err(format!(
            "expected the kind buttons in the order {expected:?}, got:\n{row}"
        ))
    }
}

fn dispatch_offers_buttons(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let expected_csv = resolve(example, &caps[2])?;
    let row = row_for(world, &raw_text)?;
    buttons_in_order(row, &expected_csv)
}

const FIELDS_PANEL_OPEN: &str = r#"<div class="fields-panel">"#;

fn dispatch_shows_no_kind(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let row = row_for(world, &raw_text)?;
    if row.contains(FIELDS_PANEL_OPEN) {
        Err(format!(
            "expected no fields panel for {raw_text:?}, got:\n{row}"
        ))
    } else {
        Ok(())
    }
}

/// A marker that appears only inside `kind`'s own panel content -- distinct
/// enough from the other kind's that "shows X" and "shows no other kind"
/// can tell them apart without needing balanced-tag scoping into the panel.
fn kind_panel_marker(kind: &str) -> Result<&'static str, String> {
    match kind.to_lowercase().as_str() {
        "committed" => Ok("At a time"),
        "quota" => Ok("Hours a week"),
        other => Err(format!("kind {other:?} has no fields panel to show")),
    }
}

fn dispatch_shows_fields_for(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let kind = resolve(example, &caps[2])?;
    let row = row_for(world, &raw_text)?;
    let marker = kind_panel_marker(&kind)?;
    if row.contains(FIELDS_PANEL_OPEN) && row.contains(marker) {
        Ok(())
    } else {
        Err(format!(
            "expected {raw_text:?}'s row to show {kind}'s fields, got:\n{row}"
        ))
    }
}

fn dispatch_shows_no_other_kind(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let row = row_for(world, &raw_text)?;
    let committed_shown = row.contains(kind_panel_marker("committed")?);
    let quota_shown = row.contains(kind_panel_marker("quota")?);
    if committed_shown && quota_shown {
        Err(format!(
            "expected {raw_text:?}'s row to show only one kind's fields, got both, in:\n{row}"
        ))
    } else {
        Ok(())
    }
}

/// The one kind-choice `<form>` naming `kind`, found by its own hidden
/// `kind` input and scoped back to its opening `<form class="kind-choice`
/// and forward to its own `</form>` -- the same rfind/find-boundary shape
/// [`html::row_containing`] uses for a capture row.
fn kind_button_section<'a>(row: &'a str, kind: &str) -> Result<&'a str, String> {
    let needle = format!(r#"value="{kind}""#);
    let needle_at = row
        .find(&needle)
        .ok_or_else(|| format!("expected a {kind} kind button in the row, got:\n{row}"))?;
    let start = row[..needle_at]
        .rfind(r#"<form class="kind-choice"#)
        .ok_or_else(|| format!("malformed kind button markup near {needle:?}"))?;
    let end = row[needle_at..]
        .find("</form>")
        .map(|offset| needle_at + offset + "</form>".len())
        .ok_or_else(|| format!("no closing </form> after {needle:?}"))?;
    Ok(&row[start..end])
}

fn dispatch_marks_chosen(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let kind = resolve(example, &caps[2])?;
    let row = row_for(world, &raw_text)?;
    let button = kind_button_section(row, &kind.to_lowercase())?;
    if button.contains("chosen") {
        Ok(())
    } else {
        Err(format!(
            "expected the {kind} button marked chosen, got:\n{button}"
        ))
    }
}

/// The quota panel's own `<input type="text" name="name" ...>` — scoped to
/// its own tag rather than searched for as a bare substring, so a later
/// step can inspect its `value` and its other attributes independently
/// (`disclosures-quota-offers-the-captures-words-07`).
fn quota_name_input<'a>(row: &'a str, raw_text: &str) -> Result<&'a str, String> {
    let needle = r#"<input type="text" name="name""#;
    row.find(needle)
        .and_then(|start| {
            row[start..]
                .find('>')
                .map(|end| &row[start..start + end + 1])
        })
        .ok_or_else(|| {
            format!("expected {raw_text:?}'s row to offer a quota name field, got:\n{row}")
        })
}

fn dispatch_offers_as_name(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let expected = resolve(example, &caps[2])?;
    let row = row_for(world, &raw_text)?;
    let input = quota_name_input(row, &raw_text)?;
    let needle = format!(r#"value="{expected}""#);
    if input.contains(&needle) {
        Ok(())
    } else {
        Err(format!(
            "expected the quota name field prefilled with {expected:?}, got:\n{input}"
        ))
    }
}

fn dispatch_name_changeable(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let row = row_for(world, &raw_text)?;
    let input = quota_name_input(row, &raw_text)?;
    if input.contains("readonly") || input.contains("disabled") {
        Err(format!(
            "expected the quota name field to be editable, got:\n{input}"
        ))
    } else {
        Ok(())
    }
}

fn dispatch_asks_hours(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let row = row_for(world, &raw_text)?;
    if row.contains(kind_panel_marker("quota")?) {
        Ok(())
    } else {
        Err(format!(
            "expected {raw_text:?}'s row to ask for hours a week, got:\n{row}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(committed_chosen: bool, quota_chosen: bool, panel: &str) -> String {
        format!(
            concat!(
                r#"<li id="capture-row-1">buy screws"#,
                r#"<div class="kinds">"#,
                r#"<form class="kind-choice kind-wide"><input type="hidden" name="kind" value="pool"><button>Pool</button></form>"#,
                r#"<form class="kind-choice{}"><input type="hidden" name="kind" value="committed"><button>Committed</button></form>"#,
                r#"<form class="kind-choice{}"><input type="hidden" name="kind" value="quota"><button>Quota</button></form>"#,
                r#"</div>"#,
                r#"{}"#,
                r#"<form class="dismiss"><button type="submit">Dismiss</button></form>"#,
                r#"</li>"#,
            ),
            if committed_chosen { " chosen" } else { "" },
            if quota_chosen { " chosen" } else { "" },
            panel,
        )
    }

    fn captures_ul(row: &str) -> String {
        format!(r#"<ul id="captures">{row}</ul>"#)
    }

    #[test]
    fn buttons_in_order_passes_for_the_templates_own_order() {
        let markup = row(false, false, "");
        assert_eq!(buttons_in_order(&markup, "Pool, Committed, Quota"), Ok(()));
    }

    #[test]
    fn buttons_in_order_errors_when_reversed() {
        let markup = row(false, false, "");
        assert!(buttons_in_order(&markup, "Quota, Committed, Pool").is_err());
    }

    #[test]
    fn buttons_in_order_does_not_match_a_hidden_inputs_value_attribute() {
        // The pool button's own hidden input carries value="pool"
        // (lowercase); the visible button text is "Pool". A lowercase
        // needle must fail to find the differently-cased label rather than
        // matching the attribute instead.
        let markup = row(false, false, "");
        assert!(buttons_in_order(&markup, "pool, Committed, Quota").is_err());
    }

    #[test]
    fn kind_button_section_finds_the_named_kinds_own_form() {
        let markup = row(true, false, "");
        let section = kind_button_section(&markup, "committed").unwrap();
        assert!(section.contains("chosen"));
        assert!(!section.contains(r#"value="quota""#));
    }

    #[tokio::test]
    async fn dispatch_shows_no_kind_passes_when_no_panel_is_open() {
        let mut world = World::new();
        world.last_html_body = Some(captures_ul(&row(false, false, "")));
        let re = Regex::new(r#"^the row for "([^"]+)" shows fields for no kind$"#).unwrap();
        let caps = re
            .captures(r#"the row for "buy screws" shows fields for no kind"#)
            .unwrap();
        dispatch_shows_no_kind(&mut world, &BTreeMap::new(), &caps).unwrap();
    }

    #[tokio::test]
    async fn dispatch_shows_fields_for_passes_when_committeds_panel_is_open() {
        let panel = concat!(
            r#"<div class="fields-panel">"#,
            r#"<details><summary>At a time</summary></details>"#,
            r#"<details><summary>By a day</summary></details>"#,
            r#"</div>"#,
        );
        let mut world = World::new();
        world.last_html_body = Some(captures_ul(&row(true, false, panel)));
        let re = Regex::new(r#"^the row for "([^"]+)" shows fields for "([^"]+)"$"#).unwrap();
        let caps = re
            .captures(r#"the row for "buy screws" shows fields for "Committed""#)
            .unwrap();
        dispatch_shows_fields_for(&mut world, &BTreeMap::new(), &caps).unwrap();
    }

    #[tokio::test]
    async fn dispatch_shows_fields_for_errors_when_the_named_kinds_panel_is_not_open() {
        let mut world = World::new();
        world.last_html_body = Some(captures_ul(&row(false, false, "")));
        let re = Regex::new(r#"^the row for "([^"]+)" shows fields for "([^"]+)"$"#).unwrap();
        let caps = re
            .captures(r#"the row for "buy screws" shows fields for "Committed""#)
            .unwrap();
        assert!(dispatch_shows_fields_for(&mut world, &BTreeMap::new(), &caps).is_err());
    }

    #[tokio::test]
    async fn dispatch_shows_no_other_kind_errors_when_both_markers_are_present() {
        let panel = concat!(
            r#"<div class="fields-panel">"#,
            r#"At a time Hours a week"#,
            r#"</div>"#,
        );
        let mut world = World::new();
        world.last_html_body = Some(captures_ul(&row(true, true, panel)));
        let re = Regex::new(r#"^the row for "([^"]+)" shows fields for no other kind$"#).unwrap();
        let caps = re
            .captures(r#"the row for "buy screws" shows fields for no other kind"#)
            .unwrap();
        assert!(dispatch_shows_no_other_kind(&mut world, &BTreeMap::new(), &caps).is_err());
    }
}
