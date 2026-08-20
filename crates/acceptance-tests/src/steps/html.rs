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

/// The shared header's own section — everything `app_shell.feature`'s "the
/// header ..." steps mean.
pub fn header_section(body: &str) -> Result<&str, String> {
    between(body, "<header>", "</header>")
}

/// The `<li>...</li>` markup for the row naming `needle`, found by content
/// rather than position. Shared by every step module that needs to check
/// what one specific row of a list says without the check bleeding into a
/// neighbouring row — `life_areas`' and `context_tags`' own "the task list
/// shows ... tagged ..." steps, and `schedule`'s placed/won't-fit rows.
///
/// Matches `<li` rather than the literal `<li>`: a capture row carries its
/// own id (`<li id="capture-row-{{ id }}">`, `capture_row.html`), while a
/// task row and `schedule`'s own rows do not -- both are still "the nearest
/// preceding row-opening tag", which `<li` alone names.
pub fn row_containing<'a>(section: &'a str, needle: &str) -> Result<&'a str, String> {
    let at = section
        .find(needle)
        .ok_or_else(|| format!("expected {needle:?} in:\n{section}"))?;
    let start = section[..at]
        .rfind("<li")
        .ok_or_else(|| format!("malformed row markup near {needle:?}"))?;
    let end = section[start..]
        .find("</li>")
        .ok_or_else(|| format!("unterminated row near {needle:?}"))?;
    Ok(&section[start..start + end + "</li>".len()])
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
    fn header_section_is_scoped_to_the_header_only() {
        let body = r#"<header><nav>Inbox</nav></header><ul id="captures"><li>buy milk</li></ul>"#;
        let section = header_section(body).unwrap();
        assert!(section.contains("Inbox"));
        assert!(!section.contains("buy milk"));
    }

    #[test]
    fn row_containing_finds_the_row_naming_the_needle() {
        let section = "<li>buy milk (Home)</li><li>call the dentist (Work)</li>";
        let row = row_containing(section, "buy milk").unwrap();
        assert!(row.starts_with("<li>buy milk"));
        assert!(row.ends_with("</li>"));
        assert!(!row.contains("call the dentist"));
    }

    #[test]
    fn row_containing_errors_when_the_needle_is_absent() {
        let section = "<li>something else</li>";
        assert!(row_containing(section, "missing").is_err());
    }

    #[test]
    fn row_containing_finds_a_row_that_carries_its_own_id() {
        let section =
            r#"<li id="capture-row-1">buy milk</li><li id="capture-row-2">call the dentist</li>"#;
        let row = row_containing(section, "buy milk").unwrap();
        assert!(row.starts_with(r#"<li id="capture-row-1">"#));
        assert!(!row.contains("call the dentist"));
    }
}
