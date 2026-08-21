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
}
