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

/// The text strictly between `start_tag` and the `</details>` that balances
/// the `<details>` element already open when `start_tag` appears --
/// depth starts at 1 for that already-open element, so a nested
/// `<details>...</details>` block after `start_tag` (the committed
/// sub-form's own at/by disclosures, #110) is skipped rather than mistaken
/// for the section's own close. [`between`] can't do this: it stops at the
/// first `</details>` regardless of nesting.
/// Which side of a `<details>`/`</details>` pair comes next in a scan, and
/// where -- [`details_section_after`]'s own nesting decision, named so the
/// scan loop reads as a match, not a four-way tuple guard.
enum DetailsBoundary {
    Open(usize),
    Close(usize),
}

/// Whichever of `<details` or `</details>` appears first in `remaining`.
fn next_details_boundary(remaining: &str) -> Option<DetailsBoundary> {
    let next_open = remaining.find("<details");
    let next_close = remaining.find("</details>");
    match (next_open, next_close) {
        (Some(open), Some(close)) if open < close => Some(DetailsBoundary::Open(open)),
        (_, Some(close)) => Some(DetailsBoundary::Close(close)),
        (Some(open), None) => Some(DetailsBoundary::Open(open)),
        (None, None) => None,
    }
}

pub fn details_section_after<'a>(body: &'a str, start_tag: &str) -> Result<&'a str, String> {
    let start = body
        .find(start_tag)
        .ok_or_else(|| format!("expected {start_tag:?} in the response, got:\n{body}"))?;
    let content = &body[start + start_tag.len()..];
    let mut depth: i32 = 1;
    let mut pos = 0usize;
    loop {
        match next_details_boundary(&content[pos..]) {
            Some(DetailsBoundary::Open(offset)) => {
                depth += 1;
                pos += offset + "<details".len();
            }
            Some(DetailsBoundary::Close(offset)) => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&content[..pos + offset]);
                }
                pos += offset + "</details>".len();
            }
            None => return Err(format!("no closing </details> balancing {start_tag:?}")),
        }
    }
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
    fn details_section_after_skips_a_nested_details_block_to_find_the_true_close() {
        let body = concat!(
            r#"<details class="kind"><summary>Committed</summary>"#,
            r#"<details class="commitment-choice"><summary>At a time</summary>inner-at</details>"#,
            r#"<details class="commitment-choice"><summary>By a day</summary>inner-by</details>"#,
            r#"</details><details><summary>Quota</summary>outer-quota</details>"#,
        );
        let section = details_section_after(body, "<summary>Committed</summary>").unwrap();
        assert!(section.contains("inner-at"));
        assert!(section.contains("inner-by"));
        assert!(!section.contains("outer-quota"));
    }

    #[test]
    fn details_section_after_works_with_no_nesting_at_all() {
        let body = r#"<details><summary>Quota</summary>plain</details><p>after</p>"#;
        let section = details_section_after(body, "<summary>Quota</summary>").unwrap();
        assert_eq!(section, "plain");
    }

    #[test]
    fn details_section_after_errors_when_the_start_tag_is_missing() {
        assert!(details_section_after("nothing here", "<summary>Committed</summary>").is_err());
    }

    #[test]
    fn details_section_after_errors_when_no_closing_tag_balances_it() {
        let body = r#"<summary>Committed</summary><details>unbalanced"#;
        assert!(details_section_after(body, "<summary>Committed</summary>").is_err());
    }
}
