//! Step handlers for `features/exceptions.feature`: a dated exception is
//! how the owner says a week is not normal (#61).
//!
//! The Background, "the server believes it is ...", "the owner's timezone
//! is ...", "the life area ... is saved with a guardrail band ...", "the
//! life area ... is saved as never scheduled", "the free time page is
//! viewed" and "the free time page reports ..." steps this feature also
//! uses are already matched generically by [`super::triage::dispatch`],
//! [`super::guardrails::dispatch`] and [`super::free_time::dispatch`],
//! tried before this module.

use super::html;
use super::inbox_view::{html_response, urlencode};
use super::life_areas::resolve;
use super::*;
use axum::body::Body;
use axum::http::Request;

static WHEN_MARKED_AWAY_LABELLED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the dates "([^"]+)" to "([^"]+)" are marked away for all life areas labelled "([^"]+)"$"#,
    )
    .unwrap()
});
static WHEN_MARKED_AWAY_SCOPED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the dates "([^"]+)" to "([^"]+)" are marked away for the life area "([^"]+)"$"#)
        .unwrap()
});
static WHEN_MARKED_AWAY_ALL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the dates "([^"]+)" to "([^"]+)" are marked away for all life areas$"#).unwrap()
});
static WHEN_EXCEPTION_REMOVED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the exception starting "([^"]+)" is removed$"#).unwrap());
static THEN_LIST_SHOWS_FOR_STARTING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the exceptions list shows "([^"]+)" for the exception starting "([^"]+)"$"#)
        .unwrap()
});
static THEN_LIST_EMPTY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the exceptions list is empty$").unwrap());
static THEN_EXCEPTION_REJECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the exception is rejected$").unwrap());
static THEN_REJECTION_SAYS_BACKWARDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the rejection says the last day precedes the first$").unwrap());
static THEN_LIST_NO_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the exceptions list does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_LIST_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the exceptions list contains the word "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = WHEN_MARKED_AWAY_LABELLED.captures(text) {
        return Some(dispatch_marked_away_labelled(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_MARKED_AWAY_SCOPED.captures(text) {
        return Some(dispatch_marked_away_scoped(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_MARKED_AWAY_ALL.captures(text) {
        return Some(dispatch_marked_away_all(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_EXCEPTION_REMOVED.captures(text) {
        return Some(dispatch_exception_removed(world, example, &caps).await);
    }
    if let Some(caps) = THEN_LIST_SHOWS_FOR_STARTING.captures(text) {
        return Some(dispatch_list_shows_for_starting(world, example, &caps));
    }
    if THEN_LIST_EMPTY.is_match(text) {
        return Some(then_list_empty(world));
    }
    if THEN_EXCEPTION_REJECTED.is_match(text) {
        return Some(super::then_status_is(
            world,
            422,
            "no exception response recorded",
        ));
    }
    if THEN_REJECTION_SAYS_BACKWARDS.is_match(text) {
        return Some(then_body_contains(world, "the last day precedes the first"));
    }
    if THEN_LIST_NO_SCRIPT.is_match(text) {
        return Some(then_list_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_LIST_CONTAINS_WORD.captures(text) {
        return Some(then_list_contains(world, &caps[1]));
    }
    None
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no page response recorded")
}

fn then_body_contains(world: &mut World, expected: &str) -> Result<(), String> {
    super::then_html_body_contains(world, expected, "no page response recorded")
}

fn exceptions_section(body: &str) -> Result<&str, String> {
    html::between(body, r#"<ul id="exceptions">"#, "</ul>")
}

fn then_list_empty(world: &mut World) -> Result<(), String> {
    let section = exceptions_section(html_body(world)?)?;
    if section.trim().is_empty() {
        Ok(())
    } else {
        Err(format!("expected no exceptions listed, got:\n{section}"))
    }
}

fn then_list_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let section = exceptions_section(html_body(world)?)?;
    if section.contains(forbidden) {
        Err(format!(
            "expected no {forbidden:?} in the exceptions list, got:\n{section}"
        ))
    } else {
        Ok(())
    }
}

fn then_list_contains(world: &mut World, expected: &str) -> Result<(), String> {
    let section = exceptions_section(html_body(world)?)?;
    if section.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected:?} in the exceptions list, got:\n{section}"
        ))
    }
}

/// The `<li id="exception-row-N">...` chunk naming `start_date` -- found by
/// its rendered start date rather than by position, the same way
/// `guardrails.rs::band_id_by_label` finds a band by its rendered label
/// rather than assuming which row is which.
fn exception_row<'a>(section: &'a str, start_date: &str) -> Result<&'a str, String> {
    section
        .split(r#"<li id="exception-row-"#)
        .skip(1)
        .find(|chunk| chunk.contains(start_date))
        .ok_or_else(|| format!("no exception starting {start_date:?} in:\n{section}"))
}

/// The id `POST /exceptions/{id}/remove` takes, read off the front of a
/// `exception_row` chunk (`"{id}">...`).
fn exception_id(row: &str) -> Result<i64, String> {
    let (id_str, _) = row
        .split_once('"')
        .ok_or_else(|| format!("malformed exception row: {row:?}"))?;
    id_str
        .parse()
        .map_err(|e| format!("bad exception id {id_str:?}: {e}"))
}

async fn post_exception(
    world: &mut World,
    from: &str,
    to: &str,
    life_area: Option<&str>,
    label: Option<&str>,
) -> Result<(), String> {
    let mut body = format!("start={}&end={}", urlencode(from), urlencode(to));
    if let Some(name) = life_area {
        body.push_str(&format!("&life_area={}", urlencode(name)));
    }
    if let Some(label) = label {
        body.push_str(&format!("&label={}", urlencode(label)));
    }
    let request = Request::builder()
        .method("POST")
        .uri("/exceptions")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

async fn dispatch_marked_away_all(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let from = resolve(example, &caps[1])?;
    let to = resolve(example, &caps[2])?;
    post_exception(world, &from, &to, None, None).await
}

async fn dispatch_marked_away_scoped(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let from = resolve(example, &caps[1])?;
    let to = resolve(example, &caps[2])?;
    let name = resolve(example, &caps[3])?;
    post_exception(world, &from, &to, Some(&name), None).await
}

async fn dispatch_marked_away_labelled(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let from = resolve(example, &caps[1])?;
    let to = resolve(example, &caps[2])?;
    let label = resolve(example, &caps[3])?;
    post_exception(world, &from, &to, None, Some(&label)).await
}

fn exception_row_for<'a>(world: &'a World, start_date: &str) -> Result<&'a str, String> {
    let section = exceptions_section(html_body(world)?)?;
    exception_row(section, start_date)
}

fn exception_id_by_start(world: &World, start_date: &str) -> Result<i64, String> {
    exception_id(exception_row_for(world, start_date)?)
}

async fn dispatch_exception_removed(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let start_date = resolve(example, &caps[1])?;
    let id = exception_id_by_start(world, &start_date)?;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/exceptions/{id}/remove"))
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

fn dispatch_list_shows_for_starting(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let start_date = resolve(example, &caps[2])?;
    let row = exception_row_for(world, &start_date)?;
    if row.contains(&expected) {
        Ok(())
    } else {
        Err(format!(
            "expected the exception starting {start_date:?} to show {expected:?}, got:\n{row}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The step text each dispatcher below is driven with, written once so
    /// a test says which step it is exercising and nothing else.
    const AWAY_ALL: &str =
        r#"the dates "2026-08-24" to "2026-08-28" are marked away for all life areas"#;
    const AWAY_SCOPED: &str =
        r#"the dates "2026-08-24" to "2026-08-28" are marked away for the life area "Work""#;
    const AWAY_LABELLED: &str = r#"the dates "2026-08-24" to "2026-08-28" are marked away for all life areas labelled "vacation""#;
    /// Deliberately backwards -- the last day precedes the first.
    const AWAY_BACKWARDS: &str =
        r#"the dates "2026-08-28" to "2026-08-24" are marked away for all life areas"#;

    /// A step's own captures. Every test here builds these the same way and
    /// none is testing that a step regex matches its own text, so the
    /// failure is one `expect` rather than a per-test `unwrap` chain.
    fn caps<'t>(regex: &Regex, text: &'t str) -> regex::Captures<'t> {
        regex
            .captures(text)
            .expect("a step's own text matches its own pattern")
    }

    #[test]
    fn exception_row_finds_the_chunk_naming_the_start_date() {
        let section = concat!(
            r#"<li id="exception-row-1">"#,
            r#"<span class="exception-scope">All life areas</span>"#,
            r#"<span class="exception-dates">2026-08-24 to 2026-08-28</span>"#,
            r#"</li>"#,
            r#"<li id="exception-row-2">"#,
            r#"<span class="exception-scope">Work</span>"#,
            r#"<span class="exception-dates">2026-09-01 to 2026-09-02</span>"#,
            r#"</li>"#,
        );
        let row = exception_row(section, "2026-09-01").unwrap();
        assert!(row.contains("Work"));
        assert!(!row.contains("All life areas"));
    }

    #[test]
    fn exception_row_errors_when_no_row_matches() {
        let section = r#"<li id="exception-row-1">2026-08-24 to 2026-08-28</li>"#;
        assert!(exception_row(section, "2026-09-01").is_err());
    }

    #[test]
    fn exception_id_reads_the_leading_id() {
        let row = r#"1">2026-08-24 to 2026-08-28</li>"#;
        assert_eq!(exception_id(row), Ok(1));
    }

    #[test]
    fn exception_id_errors_on_malformed_input() {
        assert!(exception_id("no quote here").is_err());
    }

    #[tokio::test]
    async fn marked_away_for_all_life_areas_creates_a_global_exception() {
        let mut world = migrated_world().await;

        dispatch_marked_away_all(
            &mut world,
            &example(&[]),
            &caps(&WHEN_MARKED_AWAY_ALL, AWAY_ALL),
        )
        .await
        .unwrap();

        then_list_contains(&mut world, "All life areas").unwrap();
    }

    #[tokio::test]
    async fn marked_away_for_a_named_life_area_scopes_the_exception() {
        let mut world = migrated_world().await;

        dispatch_marked_away_scoped(
            &mut world,
            &example(&[]),
            &caps(&WHEN_MARKED_AWAY_SCOPED, AWAY_SCOPED),
        )
        .await
        .unwrap();

        then_list_contains(&mut world, "Work").unwrap();
    }

    #[tokio::test]
    async fn marked_away_labelled_carries_the_label() {
        let mut world = migrated_world().await;

        dispatch_marked_away_labelled(
            &mut world,
            &example(&[]),
            &caps(&WHEN_MARKED_AWAY_LABELLED, AWAY_LABELLED),
        )
        .await
        .unwrap();

        then_list_contains(&mut world, "vacation").unwrap();
    }

    #[tokio::test]
    async fn removing_the_exception_starting_a_date_empties_the_list() {
        let mut world = migrated_world().await;
        dispatch_marked_away_all(
            &mut world,
            &example(&[]),
            &caps(&WHEN_MARKED_AWAY_ALL, AWAY_ALL),
        )
        .await
        .unwrap();

        dispatch_exception_removed(
            &mut world,
            &example(&[]),
            &WHEN_EXCEPTION_REMOVED
                .captures(r#"the exception starting "2026-08-24" is removed"#)
                .unwrap(),
        )
        .await
        .unwrap();

        then_list_empty(&mut world).unwrap();
    }

    #[tokio::test]
    async fn a_backwards_range_is_rejected_naming_the_reason() {
        let mut world = migrated_world().await;

        dispatch_marked_away_all(
            &mut world,
            &example(&[]),
            &caps(&WHEN_MARKED_AWAY_ALL, AWAY_BACKWARDS),
        )
        .await
        .unwrap();

        super::super::then_status_is(&mut world, 422, "no exception response recorded").unwrap();
        then_body_contains(&mut world, "the last day precedes the first").unwrap();
        then_list_empty(&mut world).unwrap();
    }
}
