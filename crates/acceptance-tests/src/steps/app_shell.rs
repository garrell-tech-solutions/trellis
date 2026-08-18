//! Step handlers for `features/app_shell.feature`: every page carries the
//! same navigation header, from one definition (#58).
//!
//! The Background, "the inbox/life areas/stats page is viewed", "a capture
//! ... is waiting", "a life area named ... was added" and "the capture is
//! triaged as a pool task through the page" steps this feature also uses are
//! already matched generically by [`super::triage::dispatch`],
//! [`super::inbox_view::dispatch`], [`super::life_areas::dispatch`],
//! [`super::stats_ratio::dispatch`] and [`super::triage_from_page::dispatch`],
//! tried before this module.

use super::html;
use super::inbox_view::html_response;
use super::*;
use axum::body::Body;
use axum::http::Request;

static THEN_LINKS_EXACTLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the header links are exactly "<(\w+)>"$"#).unwrap());
static THEN_MARKS_CURRENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the header marks "<(\w+)>" as the current page$"#).unwrap());
static THEN_EXACTLY_ONE_CURRENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the header marks exactly one link as the current page$").unwrap()
});
static THEN_DECLARES_422: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the page declares the 422 swap handling$").unwrap());
static THEN_LINK_POINTS_AT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the header link "<(\w+)>" points at "<(\w+)>"$"#).unwrap());
static WHEN_LINK_FOLLOWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the header link "<(\w+)>" is followed$"#).unwrap());
static THEN_TRIAGE_RESPONSE_NO_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the page's triage response carries no header$").unwrap());
static THEN_HEADER_APPEARS_ONCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the header appears exactly once$").unwrap());
static THEN_LINKS_ARE_PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^every header link is an ordinary link that loads a full page$").unwrap()
});
static THEN_HEADER_NO_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the header does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_HEADER_NO_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the header does not contain the word "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = THEN_LINKS_EXACTLY.captures(text) {
        return Some(dispatch_links_exactly(world, example, &caps));
    }
    if let Some(caps) = THEN_MARKS_CURRENT.captures(text) {
        return Some(dispatch_marks_current(world, example, &caps));
    }
    if THEN_EXACTLY_ONE_CURRENT.is_match(text) {
        return Some(then_exactly_one_current(world));
    }
    if THEN_DECLARES_422.is_match(text) {
        return Some(then_declares_422(world));
    }
    if let Some(caps) = THEN_LINK_POINTS_AT.captures(text) {
        return Some(dispatch_link_points_at(world, example, &caps));
    }
    if let Some(caps) = WHEN_LINK_FOLLOWED.captures(text) {
        return Some(dispatch_link_followed(world, example, &caps).await);
    }
    if THEN_TRIAGE_RESPONSE_NO_HEADER.is_match(text) {
        return Some(then_no_header(world));
    }
    if THEN_HEADER_APPEARS_ONCE.is_match(text) {
        return Some(then_header_appears_once(world));
    }
    if THEN_LINKS_ARE_PLAIN.is_match(text) {
        return Some(then_links_are_plain(world));
    }
    if THEN_HEADER_NO_SCRIPT.is_match(text) {
        return Some(then_header_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_HEADER_NO_WORD.captures(text) {
        return Some(then_header_excludes(world, &caps[1]));
    }
    None
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no page response recorded")
}

fn header_section(world: &World) -> Result<&str, String> {
    html::header_section(html_body(world)?)
}

/// The label text inside every `<a href="...">...</a>` in `section`, in
/// document order -- what "the header links are exactly ..." means.
fn header_link_labels(section: &str) -> Vec<String> {
    section
        .split("<a href=\"")
        .skip(1)
        .filter_map(|chunk| {
            let after_tag = chunk.split_once('>')?.1;
            Some(after_tag.split("</a>").next()?.trim().to_string())
        })
        .collect()
}

/// The whole `<a ...>label</a>` markup for the link labelled `label`, found
/// by its closing text rather than by position, so this does not care which
/// of the three links it is looking for.
fn header_link_html<'a>(section: &'a str, label: &str) -> Result<&'a str, String> {
    let closing = format!(">{label}</a>");
    let close_at = section
        .find(&closing)
        .ok_or_else(|| format!("no header link labelled {label:?} in:\n{section}"))?;
    let start = section[..close_at]
        .rfind("<a ")
        .ok_or_else(|| format!("malformed header markup near {label:?}"))?;
    Ok(&section[start..close_at + closing.len()])
}

fn extract_href(link: &str) -> Result<String, String> {
    let after = link
        .strip_prefix("<a href=\"")
        .ok_or_else(|| format!("malformed link markup: {link:?}"))?;
    let (href, _) = after
        .split_once('"')
        .ok_or_else(|| format!("malformed link markup: {link:?}"))?;
    Ok(href.to_string())
}

fn dispatch_links_exactly(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected: Vec<String> = example_value(example, &caps[1])?
        .split(", ")
        .map(str::to_string)
        .collect();
    let section = header_section(world)?;
    let actual = header_link_labels(section);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected header links {expected:?}, got {actual:?}"
        ))
    }
}

fn then_marks_current(section: &str, label: &str) -> Result<(), String> {
    let link = header_link_html(section, label)?;
    if link.contains(r#"aria-current="page""#) {
        Ok(())
    } else {
        Err(format!(
            "expected {label:?} to be marked the current page, got:\n{link}"
        ))
    }
}

fn dispatch_marks_current(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let label = example_value(example, &caps[1])?;
    then_marks_current(header_section(world)?, label)
}

fn then_exactly_one_current(world: &mut World) -> Result<(), String> {
    let section = header_section(world)?;
    let count = section.matches(r#"aria-current="page""#).count();
    if count == 1 {
        Ok(())
    } else {
        Err(format!(
            "expected exactly one current link, found {count} in:\n{section}"
        ))
    }
}

fn then_declares_422(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    let needle = r#"htmx.config.responseHandling.unshift({code: "422", swap: true});"#;
    if body.contains(needle) {
        Ok(())
    } else {
        Err(format!(
            "expected the 422 swap config in the page, got:\n{body}"
        ))
    }
}

fn dispatch_link_points_at(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let label = example_value(example, &caps[1])?;
    let path = example_value(example, &caps[2])?;
    let section = header_section(world)?;
    let link = header_link_html(section, label)?;
    let needle = format!(r#"href="{path}""#);
    if link.contains(&needle) {
        Ok(())
    } else {
        Err(format!(
            "expected {label:?} to link to {path:?}, got:\n{link}"
        ))
    }
}

fn href_for_label(world: &mut World, label: &str) -> Result<String, String> {
    let section = header_section(world)?;
    let link = header_link_html(section, label)?;
    extract_href(link)
}

async fn dispatch_link_followed(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let label = example_value(example, &caps[1])?.to_string();
    let path = href_for_label(world, &label)?;
    let request = Request::builder()
        .uri(path)
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

fn then_no_header(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains("<header") {
        Err(format!(
            "expected no header in the triage response, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

fn then_header_appears_once(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    let count = body.matches("<header").count();
    if count == 1 {
        Ok(())
    } else {
        Err(format!(
            "expected exactly one header, found {count} in:\n{body}"
        ))
    }
}

fn then_links_are_plain(world: &mut World) -> Result<(), String> {
    let section = header_section(world)?;
    if section.contains("hx-") {
        Err(format!(
            "expected ordinary links with no hx- attributes, got:\n{section}"
        ))
    } else {
        Ok(())
    }
}

fn then_header_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let section = header_section(world)?;
    if section.contains(forbidden) {
        Err(format!(
            "expected no {forbidden:?} in the header, got:\n{section}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_body(links: &[(&str, &str, bool)]) -> String {
        let items: String = links
            .iter()
            .map(|(label, path, current)| {
                let attr = if *current {
                    " aria-current=\"page\""
                } else {
                    ""
                };
                format!(r#"<li><a href="{path}"{attr}>{label}</a></li>"#)
            })
            .collect();
        format!(r#"<header><nav><ul>{items}</ul></nav></header><h1>Trellis</h1>"#)
    }

    fn world_with_header(links: &[(&str, &str, bool)]) -> World {
        let mut world = World::new();
        world.last_html_body = Some(header_body(links));
        world
    }

    #[test]
    fn header_link_labels_reads_labels_in_document_order() {
        let section = r#"<a href="/">Inbox</a><a href="/life-areas">Life areas</a><a href="/stats">Stats</a>"#;
        assert_eq!(
            header_link_labels(section),
            vec!["Inbox", "Life areas", "Stats"]
        );
    }

    #[test]
    fn header_link_html_finds_the_named_links_own_markup() {
        let section = r#"<a href="/">Inbox</a><a href="/stats" aria-current="page">Stats</a>"#;
        let link = header_link_html(section, "Stats").unwrap();
        assert!(link.contains("aria-current"));
        assert!(!link.contains("Inbox"));
    }

    #[test]
    fn header_link_html_errors_when_the_label_is_not_present() {
        let section = r#"<a href="/">Inbox</a>"#;
        assert!(header_link_html(section, "Nowhere").is_err());
    }

    #[test]
    fn extract_href_reads_the_links_target() {
        assert_eq!(
            extract_href(r#"<a href="/life-areas">Life areas</a>"#),
            Ok("/life-areas".to_string())
        );
    }

    #[test]
    fn dispatch_links_exactly_reads_the_labels_from_the_example() {
        let mut world = world_with_header(&[
            ("Inbox", "/", true),
            ("Life areas", "/life-areas", false),
            ("Stats", "/stats", false),
        ]);
        let ex = example(&[("links", "Inbox, Life areas, Stats")]);
        let re = Regex::new(r#""<(\w+)>""#).unwrap();
        let caps = re.captures(r#""<links>""#).unwrap();

        assert_eq!(dispatch_links_exactly(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn then_marks_current_passes_for_the_marked_link() {
        let section = r#"<a href="/">Inbox</a><a href="/stats" aria-current="page">Stats</a>"#;
        assert_eq!(then_marks_current(section, "Stats"), Ok(()));
    }

    #[test]
    fn then_marks_current_errors_for_an_unmarked_link() {
        let section = r#"<a href="/">Inbox</a><a href="/stats" aria-current="page">Stats</a>"#;
        assert!(then_marks_current(section, "Inbox").is_err());
    }

    #[test]
    fn then_exactly_one_current_passes_when_exactly_one_link_is_marked() {
        let mut world = world_with_header(&[
            ("Inbox", "/", true),
            ("Life areas", "/life-areas", false),
            ("Stats", "/stats", false),
        ]);
        assert_eq!(then_exactly_one_current(&mut world), Ok(()));
    }

    #[test]
    fn then_exactly_one_current_errors_when_two_links_are_marked() {
        let mut world = world_with_header(&[
            ("Inbox", "/", true),
            ("Life areas", "/life-areas", true),
            ("Stats", "/stats", false),
        ]);
        assert!(then_exactly_one_current(&mut world).is_err());
    }

    #[test]
    fn then_exactly_one_current_errors_when_no_link_is_marked() {
        let mut world = world_with_header(&[("Inbox", "/", false)]);
        assert!(then_exactly_one_current(&mut world).is_err());
    }

    #[test]
    fn then_declares_422_passes_when_the_config_line_is_present() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<script>htmx.config.responseHandling.unshift({code: "422", swap: true});</script>"#
                .to_string(),
        );
        assert_eq!(then_declares_422(&mut world), Ok(()));
    }

    #[test]
    fn then_declares_422_errors_when_the_config_line_is_absent() {
        let mut world = World::new();
        world.last_html_body = Some("<p>nothing here</p>".to_string());
        assert!(then_declares_422(&mut world).is_err());
    }

    #[test]
    fn then_no_header_passes_for_a_fragment_response() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<div id="lists"></div>"#.to_string());
        assert_eq!(then_no_header(&mut world), Ok(()));
    }

    #[test]
    fn then_no_header_errors_when_a_header_is_present() {
        let mut world = World::new();
        world.last_html_body = Some("<header></header>".to_string());
        assert!(then_no_header(&mut world).is_err());
    }

    #[test]
    fn then_header_appears_once_passes_for_a_single_header() {
        let mut world = world_with_header(&[("Inbox", "/", true)]);
        assert_eq!(then_header_appears_once(&mut world), Ok(()));
    }

    #[test]
    fn then_header_appears_once_errors_for_a_duplicated_header() {
        let mut world = World::new();
        world.last_html_body = Some("<header></header><header></header>".to_string());
        assert!(then_header_appears_once(&mut world).is_err());
    }

    #[test]
    fn then_links_are_plain_passes_when_no_link_carries_an_hx_attribute() {
        let mut world = world_with_header(&[("Inbox", "/", true)]);
        assert_eq!(then_links_are_plain(&mut world), Ok(()));
    }

    #[test]
    fn then_links_are_plain_errors_when_a_link_carries_hx_boost() {
        let mut world = World::new();
        world.last_html_body =
            Some(r#"<header><a href="/" hx-boost="true">Inbox</a></header>"#.to_string());
        assert!(then_links_are_plain(&mut world).is_err());
    }

    #[test]
    fn then_header_excludes_passes_when_the_forbidden_text_is_absent() {
        let mut world = world_with_header(&[("Inbox", "/", true)]);
        assert_eq!(then_header_excludes(&mut world, "<script>"), Ok(()));
    }

    #[test]
    fn then_header_excludes_errors_when_the_forbidden_text_is_present() {
        let mut world = World::new();
        world.last_html_body = Some("<header><script>alert('boom')</script></header>".to_string());
        assert!(then_header_excludes(&mut world, "<script>").is_err());
    }
}
