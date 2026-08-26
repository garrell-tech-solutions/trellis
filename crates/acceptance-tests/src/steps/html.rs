//! Minimal HTML section scoping for acceptance assertions.
//!
//! Substring matching is the project's chosen tradeoff for asserting on
//! rendered pages (see the inbox-view handoff brief's open question on HTML
//! assertions: cheap and brittle beats a parsing dependency for M1). That
//! stopped being enough for a whole-body check the moment the page grew a
//! second list: `triage_from_page`'s task list renders its own `<li>`
//! elements, so "the inbox lists no captures" checked against the whole body
//! went from true to a false positive the instant one task existed. Scoping
//! a check to the text between a container's opening and closing tag is the
//! smallest fix that keeps substring matching honest.

/// The text strictly between the first occurrence of `start_tag` and the
/// next occurrence of `end_tag` after it.
pub fn between<'a>(body: &'a str, start_tag: &str, end_tag: &str) -> Result<&'a str, String> {
    let start = body
        .find(start_tag)
        .ok_or_else(|| format!("expected {start_tag:?} in the response, got:\n{body}"))?;
    let after_start = &body[start + start_tag.len()..];
    let end = after_start
        .find(end_tag)
        .ok_or_else(|| format!("expected a closing {end_tag:?} after {start_tag:?}"))?;
    Ok(&after_start[..end])
}

/// The inbox's own section of the page — everything `inbox_view.feature`'s
/// "the inbox ..." steps mean.
pub fn captures_section(body: &str) -> Result<&str, String> {
    between(body, r#"<ul id="captures">"#, "</ul>")
}

/// The task list's own section — everything `triage_from_page.feature`'s
/// "the task list ..." steps mean.
pub fn tasks_section(body: &str) -> Result<&str, String> {
    between(body, r#"<ul id="tasks">"#, "</ul>")
}

/// The shared header's own section — everything `pool_screen.feature`'s
/// "the tab bar ..." steps mean. Returned in #92 alongside the header
/// itself, which #88 had deleted along with the pages it linked between.
pub fn header_section(body: &str) -> Result<&str, String> {
    between(body, "<header>", "</header>")
}

/// The markup of the one row (an `<li` element, open or self-contained) in
/// `section` whose text contains `needle` — for asserting a second fact (a
/// tag, say) about specifically the row a first fact (raw capture text)
/// identifies, rather than about the section as a whole. Needed the moment
/// two rows can share the second fact
/// (`context-tags-case-is-one-tag-06`'s two rows share one tag): a
/// whole-section substring check cannot tell which row it came from.
/// Matches `<li` rather than the exact `<li>`, since a capture row carries
/// an `id` attribute (`<li id="capture-row-1">`) and a task row does not.
pub fn row_containing<'a>(section: &'a str, needle: &str) -> Result<&'a str, String> {
    let needle_at = section
        .find(needle)
        .ok_or_else(|| format!("expected {needle:?} in the section, got:\n{section}"))?;
    let start = section[..needle_at]
        .rfind("<li")
        .ok_or_else(|| format!("malformed row markup near {needle:?}"))?;
    let end = section[needle_at..]
        .find("</li>")
        .map(|offset| needle_at + offset + "</li>".len())
        .ok_or_else(|| format!("no closing </li> after {needle:?}"))?;
    Ok(&section[start..end])
}

/// The markup of the one trip panel labelled `tag` — split on the panel's
/// own opening tag rather than on `<div` generally, since a trip panel
/// nests several plain `<div>`s of its own. Shared by `pool_screen.rs` and
/// `trip_progress.rs`, unlike most of this project's step-parsing helpers:
/// it takes `body: &str` rather than `World`, so there is no per-module
/// state to duplicate around.
///
/// Matches on the `class` attribute's own prefix rather than the whole
/// opening tag, so an expanded trip (#120: `class="trip panel expanded"`)
/// still scopes correctly -- `T-qa-binds-tolerantly-to-markup`'s reasoning
/// applied to this harness's own step code, not only to QA's.
pub fn trip_section<'a>(body: &'a str, tag: &str) -> Result<&'a str, String> {
    let marker = r#"<div class="trip panel"#;
    let needle = format!(r#"<div class="trip-tag">{tag}</div>"#);
    let mut offset = 0;
    while let Some(rel_start) = body[offset..].find(marker) {
        let start = offset + rel_start;
        let after = start + marker.len();
        let end = body[after..]
            .find(marker)
            .map(|rel| after + rel)
            .unwrap_or(body.len());
        let candidate = &body[start..end];
        if candidate.contains(&needle) {
            return Ok(candidate);
        }
        offset = after;
    }
    Err(format!("no trip panel found for tag {tag:?} in:\n{body}"))
}

/// The markup of the one quota row named `name` -- [`trip_section`]'s same
/// shape, for the same reason: `quota_body.html`'s row nests its own
/// `<ul class="quota-sessions"><li class="quota-session">...</li></ul>`, so
/// scanning for a matching `</div>` or `</li>` would stop at the first
/// session's own closing tag rather than the row's. Scanning to the *next*
/// `<div class="quota-row"` instead sidesteps nesting depth entirely,
/// whatever is inside one row.
pub fn quota_row<'a>(body: &'a str, name: &str) -> Result<&'a str, String> {
    let marker = r#"<div class="quota-row""#;
    let needle = format!(r#"<div class="quota-name">{name}</div>"#);
    let mut offset = 0;
    while let Some(rel_start) = body[offset..].find(marker) {
        let start = offset + rel_start;
        let after = start + marker.len();
        let end = body[after..]
            .find(marker)
            .map(|rel| after + rel)
            .unwrap_or(body.len());
        let candidate = &body[start..end];
        if candidate.contains(&needle) {
            return Ok(candidate);
        }
        offset = after;
    }
    Err(format!("no quota row found for {name:?} in:\n{body}"))
}

/// The `<button ...>...</button>` whose opening tag contains `class_marker`
/// (e.g. `r#"class="trip-more-toggle""#`) -- its full opening tag (so a
/// caller can check for an `hx-get`/`hx-post`/`hx-trigger` attribute, #120's
/// own "issues no request") and its trimmed text content. Shared by
/// `pool_screen.rs` and `trip_controls.rs`, the same "second caller earns a
/// shared home" reasoning [`trip_section`] and [`listed_in_order`] already
/// follow.
/// The byte offset where the `<button ...>` opening tag carrying
/// `class_marker` starts -- found by locating the class marker and walking
/// back to its enclosing `<button`.
fn button_tag_start(section: &str, class_marker: &str) -> Result<usize, String> {
    let class_at = section
        .find(class_marker)
        .ok_or_else(|| format!("expected a button with {class_marker}, got:\n{section}"))?;
    section[..class_at]
        .rfind("<button")
        .ok_or_else(|| format!("malformed button markup near {class_marker}"))
}

/// The `(start, end)` byte offsets of the `<button ...>` opening tag whose
/// attributes contain `class_marker` -- split out of [`button`] itself so
/// that function's own three sequential lookups (the tag, then its own
/// text) do not all live behind one cyclomatic count.
fn button_opening_tag_span(section: &str, class_marker: &str) -> Result<(usize, usize), String> {
    let tag_start = button_tag_start(section, class_marker)?;
    let rel_tag_close = section[tag_start..]
        .find('>')
        .ok_or_else(|| format!("no closing '>' on the button carrying {class_marker}"))?;
    Ok((tag_start, tag_start + rel_tag_close + 1))
}

pub fn button<'a>(section: &'a str, class_marker: &str) -> Result<(&'a str, &'a str), String> {
    let (tag_start, tag_end) = button_opening_tag_span(section, class_marker)?;
    let opening_tag = &section[tag_start..tag_end];
    let after_open = &section[tag_end..];
    let text_end = after_open
        .find("</button>")
        .ok_or_else(|| format!("no closing </button> after {class_marker}"))?;
    Ok((opening_tag, after_open[..text_end].trim()))
}

/// A comma-separated example value, compared against what a screen actually
/// listed, in order.
///
/// Six step handlers across four screens had written this out: split the
/// example on `", "`, collect, compare to a `Vec<String>` the caller
/// extracted from the body, and report both sides on a mismatch. It is the
/// same move `html`, `payloads` and `app_client` already are -- when the
/// second caller appears the shared thing gets its own home -- applied to
/// the one assertion every "lists exactly" step makes.
///
/// `what` names the thing being listed, so the failure still reads as that
/// screen's own sentence rather than a generic one.
pub(super) fn listed_in_order(
    expected_csv: &str,
    actual: Vec<String>,
    what: &str,
) -> Result<(), String> {
    let expected: Vec<String> = expected_csv.split(", ").map(str::to_string).collect();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {what} {expected:?}, got {actual:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn between_returns_the_text_strictly_inside_the_tags() {
        assert_eq!(between("<a>hello</a>", "<a>", "</a>"), Ok("hello"));
    }

    #[test]
    fn between_errors_when_the_start_tag_is_missing() {
        assert!(between("nothing here", "<a>", "</a>").is_err());
    }

    #[test]
    fn between_errors_when_the_end_tag_is_missing() {
        assert!(between("<a>hello", "<a>", "</a>").is_err());
    }

    #[test]
    fn captures_section_is_scoped_to_the_captures_list_only() {
        let body = r#"<ul id="captures"><li>buy milk</li></ul><ul id="tasks"><li>[pool] call the dentist</li></ul>"#;
        let section = captures_section(body).unwrap();
        assert!(section.contains("buy milk"));
        assert!(!section.contains("call the dentist"));
    }

    #[test]
    fn tasks_section_is_scoped_to_the_task_list_only() {
        let body = r#"<ul id="captures"><li>buy milk</li></ul><ul id="tasks"><li>[pool] call the dentist</li></ul>"#;
        let section = tasks_section(body).unwrap();
        assert!(section.contains("call the dentist"));
        assert!(!section.contains("buy milk"));
    }

    #[test]
    fn row_containing_finds_the_row_with_an_id_attribute() {
        let section = r#"<li id="capture-row-1">buy milk @homedepot</li><li id="capture-row-2">call the dentist</li>"#;
        let row = row_containing(section, "buy milk").unwrap();
        assert!(row.contains("@homedepot"));
        assert!(!row.contains("call the dentist"));
    }

    #[test]
    fn row_containing_finds_the_row_with_no_id_attribute() {
        let section = "<li>[pool] buy milk @homedepot</li><li>[pool] call the dentist</li>";
        let row = row_containing(section, "buy milk").unwrap();
        assert!(row.contains("@homedepot"));
        assert!(!row.contains("call the dentist"));
    }

    #[test]
    fn row_containing_does_not_bleed_into_the_next_row_sharing_the_same_tag() {
        let section = r#"<li id="capture-row-1">return the drill @HomeDepot</li><li id="capture-row-2">buy screws @HomeDepot</li>"#;
        let row = row_containing(section, "buy screws").unwrap();
        assert!(!row.contains("return the drill"));
    }

    #[test]
    fn row_containing_errors_when_the_needle_is_absent() {
        let section = "<li>buy milk</li>";
        assert!(row_containing(section, "call the dentist").is_err());
    }

    #[test]
    fn header_section_is_scoped_to_the_header_only() {
        let body = r#"<header><nav>Pool</nav></header><ul id="captures"><li>buy milk</li></ul>"#;
        let section = header_section(body).unwrap();
        assert!(section.contains("Pool"));
        assert!(!section.contains("buy milk"));
    }

    #[test]
    fn button_finds_the_text_and_opening_tag_by_its_class() {
        let section = r#"<div><button type="button" class="trip-more-toggle" data-collapsed-label="Show 2 more">Show fewer</button></div>"#;
        let (opening_tag, text) = button(section, r#"class="trip-more-toggle""#).unwrap();
        assert_eq!(text, "Show fewer");
        assert!(opening_tag.contains("data-collapsed-label=\"Show 2 more\""));
        assert!(!opening_tag.contains("hx-post"));
    }

    #[test]
    fn button_reports_an_hx_post_attribute_when_present() {
        let section = r#"<button type="button" class="trip-complete" hx-post="/pool/trips/%40homedepot/complete">Complete all 8</button>"#;
        let (opening_tag, text) = button(section, r#"class="trip-complete""#).unwrap();
        assert_eq!(text, "Complete all 8");
        assert!(opening_tag.contains("hx-post"));
    }

    #[test]
    fn button_errors_when_no_button_carries_the_class() {
        let section = r#"<button type="button" class="clear-done">&times;</button>"#;
        assert!(button(section, r#"class="trip-more-toggle""#).is_err());
    }

    #[test]
    fn trip_section_finds_a_panel_carrying_extra_classes() {
        let body = concat!(
            r#"<div class="trip panel expanded" data-tag="@homedepot">"#,
            r#"<div class="trip-tag">@homedepot</div>3 things</div>"#,
            r#"<div class="trip panel" data-tag="@supermarket">"#,
            r#"<div class="trip-tag">@supermarket</div>3 things</div>"#,
        );
        let section = trip_section(body, "@homedepot").unwrap();
        assert!(section.contains("expanded"));
        assert!(!section.contains("@supermarket"));
    }

    /// The failure mode `quota_row` exists to avoid: a naive scan for the
    /// row's own closing tag would stop at the first nested session's
    /// `</li>` instead, silently dropping the second session and the
    /// summary that follows it.
    #[test]
    fn quota_row_survives_a_nested_session_list() {
        let body = concat!(
            r#"<div class="quota-row"><div class="quota-name">Piano</div>"#,
            r#"<ul class="quota-sessions">"#,
            r#"<li class="quota-session">Mon 25m</li>"#,
            r#"<li class="quota-session">Tue 35m</li>"#,
            r#"</ul>"#,
            r#"<div class="quota-sessions-summary">2 sessions</div></div>"#,
            r#"<div class="quota-row"><div class="quota-name">Running</div></div>"#,
        );
        let row = quota_row(body, "Piano").unwrap();
        assert!(row.contains("Mon 25m"), "got:\n{row}");
        assert!(row.contains("Tue 35m"), "got:\n{row}");
        assert!(row.contains("2 sessions"), "got:\n{row}");
        assert!(!row.contains("Running"), "got:\n{row}");
    }

    #[test]
    fn quota_row_errors_when_no_row_matches() {
        let body = r#"<div class="quota-row"><div class="quota-name">Piano</div></div>"#;
        assert!(quota_row(body, "Running").is_err());
    }
}
