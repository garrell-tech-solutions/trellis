//! Step handlers for `features/trip_progress.feature`: a trip survives
//! being worked (#122).
//!
//! The Background, "N pool tasks tagged X", "the pool screen is viewed",
//! "the pool screen offers the trips X", "the trip X reads Y", "the pool
//! screen does not mention X", and "X is marked done" are already matched
//! generically by [`super::triage::dispatch`], [`super::pool_screen::dispatch`]
//! and [`super::mark_done::dispatch`], tried before this module — nothing
//! here duplicates them. `the trip "X" reads "Y"` in particular is why the
//! clear-done button lives outside `.trip-count` in the template: that step
//! reads `.trip-count` alone, and this module does not want a second copy
//! of it that would drift.

use super::html;
use super::inbox_view::{html_response, urlencode};
use super::*;
use axum::body::Body;
use axum::http::Request;

static WHEN_N_MARKED_DONE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^"([^"]+)" of them are marked done$"#).unwrap());
static WHEN_ONE_UNCHECKED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^one struck item is unchecked$").unwrap());
static WHEN_CLEARED_FROM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the done items are cleared from "([^"]+)"$"#).unwrap());
static THEN_SHOWS_STRUCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the trip "([^"]+)" shows "([^"]+)" struck items$"#).unwrap());
static THEN_SHOWS_OPEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the trip "([^"]+)" shows "([^"]+)" open items$"#).unwrap());
static THEN_NO_CLEAR_CONTROL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the trip "([^"]+)" offers no clear-done control$"#).unwrap());
static THEN_CLEAR_CONTROL_NAMED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the trip "([^"]+)" offers a clear-done control named "([^"]+)"$"#).unwrap()
});
static THEN_CONTROL_AFTER_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^that control sits after the trip's progress label$").unwrap());
static THEN_LOOSE_EMPTY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the loose ends list is empty$").unwrap());
static THEN_LOOSE_SHOWS_N: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the loose ends list shows "([^"]+)" items$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = WHEN_N_MARKED_DONE.captures(text) {
        return Some(dispatch_n_marked_done(world, example, &caps).await);
    }
    if WHEN_ONE_UNCHECKED.is_match(text) {
        return Some(uncheck_one_struck_item(world).await);
    }
    if let Some(caps) = WHEN_CLEARED_FROM.captures(text) {
        return Some(dispatch_cleared_from(world, example, &caps).await);
    }
    if let Some(caps) = THEN_SHOWS_STRUCK.captures(text) {
        return Some(dispatch_shows_struck(world, example, &caps));
    }
    if let Some(caps) = THEN_SHOWS_OPEN.captures(text) {
        return Some(dispatch_shows_open(world, example, &caps));
    }
    if let Some(caps) = THEN_NO_CLEAR_CONTROL.captures(text) {
        return Some(dispatch_no_clear_control(world, example, &caps));
    }
    if let Some(caps) = THEN_CLEAR_CONTROL_NAMED.captures(text) {
        return Some(dispatch_clear_control_named(world, example, &caps));
    }
    if THEN_CONTROL_AFTER_LABEL.is_match(text) {
        return Some(then_control_after_label(world));
    }
    if THEN_LOOSE_EMPTY.is_match(text) {
        return Some(then_loose_empty(world));
    }
    if let Some(caps) = THEN_LOOSE_SHOWS_N.captures(text) {
        return Some(dispatch_loose_shows_n(world, example, &caps));
    }
    None
}

/// Resolves a captured value that may be a literal or an `Examples`
/// placeholder (`<done>`) written down verbatim in the step text. See
/// `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

fn resolved_count(example: &BTreeMap<String, String>, raw: &str) -> Result<usize, String> {
    resolve(example, raw)?
        .parse()
        .map_err(|e| format!("bad count {raw:?}: {e}"))
}

async fn post_and_record(world: &mut World, path: String) -> Result<(), String> {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

/// Every scenario in this feature sets up exactly one tag's worth of pool
/// tasks before marking any done, so "N of them" needs no tag of its own to
/// disambiguate -- the N oldest still-open pool tasks in the whole database
/// are unambiguously "them". Marked through the real route
/// (`POST /pool/tasks/{id}/done`), never by writing `archived_at` directly:
/// this slice depends on exactly what that route leaves behind
/// (`qa/trip_progress.md`'s own warning).
async fn mark_n_pool_tasks_done(world: &mut World, count: usize) -> Result<(), String> {
    let pool = world.pool()?.clone();
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM tasks WHERE kind = 'pool' AND archived_at IS NULL \
         ORDER BY id ASC LIMIT ?",
    )
    .bind(count as i64)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("find open pool tasks: {e}"))?;
    if ids.len() < count {
        return Err(format!(
            "expected at least {count} open pool task(s), found {}",
            ids.len()
        ));
    }
    for id in ids {
        post_and_record(world, format!("/pool/tasks/{id}/done")).await?;
    }
    Ok(())
}

async fn dispatch_n_marked_done(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let count = resolved_count(example, &caps[1])?;
    mark_n_pool_tasks_done(world, count).await
}

/// The one struck-and-not-cleared pool task, for the one scenario that
/// unchecks a single item without caring which -- unchecking is symmetric
/// across whichever of the marked-done items it targets.
async fn uncheck_one_struck_item(world: &mut World) -> Result<(), String> {
    let pool = world.pool()?.clone();
    let id: i64 = sqlx::query_scalar(
        "SELECT id FROM tasks WHERE kind = 'pool' AND archived_at IS NOT NULL \
         AND cleared_at IS NULL ORDER BY id ASC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("find a struck pool task: {e}"))?;
    post_and_record(world, format!("/pool/tasks/{id}/undone")).await
}

async fn dispatch_cleared_from(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    post_and_record(world, format!("/pool/trips/{}/clear", urlencode(&tag))).await
}

fn count_matching_items(section: &str, done: bool) -> usize {
    let needle = if done {
        r#"<li class="done">"#
    } else {
        r#"<li class="">"#
    };
    section.matches(needle).count()
}

/// `html::trip_section`'s count when the tag names no panel at all, for the one
/// scenario that clears a group below threshold and then still asks "how
/// many struck items does the trip show" -- a below-threshold group renders
/// no panel (`T-trips-are-derived-not-ranked`: it is recomputed, not kept
/// around half-empty), so the honest answer is zero, not an error
/// (`trip-progress-clear-done-04`).
fn count_in_trip_or_zero(body: &str, tag: &str, done: bool) -> usize {
    html::trip_section(body, tag)
        .map(|section| count_matching_items(section, done))
        .unwrap_or(0)
}

fn dispatch_shows_struck(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let expected = resolved_count(example, &caps[2])?;
    let body = super::html_body(world, "no pool screen response recorded")?;
    let actual = count_in_trip_or_zero(body, &tag, true);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected {expected} struck item(s) in the trip {tag:?}, got {actual}, in:\n{body}"
        ))
    }
}

fn dispatch_shows_open(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let expected = resolved_count(example, &caps[2])?;
    let body = super::html_body(world, "no pool screen response recorded")?;
    let actual = count_in_trip_or_zero(body, &tag, false);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected {expected} open item(s) in the trip {tag:?}, got {actual}, in:\n{body}"
        ))
    }
}

fn dispatch_no_clear_control(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let body = super::html_body(world, "no pool screen response recorded")?;
    let section = html::trip_section(body, &tag)?;
    if section.contains("clear-done") {
        Err(format!(
            "expected no clear-done control for the trip {tag:?}, got:\n{section}"
        ))
    } else {
        Ok(())
    }
}

fn dispatch_clear_control_named(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let expected_name = resolve(example, &caps[2])?;
    let body = super::html_body(world, "no pool screen response recorded")?;
    let section = html::trip_section(body, &tag)?;
    let needle = format!(r#"aria-label="{expected_name}""#);
    if section.contains("clear-done") && section.contains(&needle) {
        Ok(())
    } else {
        Err(format!(
            "expected a clear-done control named {expected_name:?} for the trip {tag:?}, got:\n{section}"
        ))
    }
}

/// The last "shows a clear-done control" or "offers no clear-done control"
/// check's own trip tag, so the very next step ("that control sits after
/// ...") does not have to repeat it -- the Gherkin itself never names the
/// tag twice in a row (`trip-progress-clear-control-appears-with-work-07`).
fn then_control_after_label(world: &mut World) -> Result<(), String> {
    let body = super::html_body(world, "no pool screen response recorded")?;
    let count_at = body
        .find(r#"<div class="trip-count">"#)
        .ok_or_else(|| format!("expected a trip-count label, got:\n{body}"))?;
    let clear_at = body
        .find("clear-done")
        .ok_or_else(|| format!("expected a clear-done control, got:\n{body}"))?;
    if count_at < clear_at {
        Ok(())
    } else {
        Err(format!(
            "expected the clear-done control after the progress label, got:\n{body}"
        ))
    }
}

/// The loose-ends `<ul>`'s own content, or `None` when the whole list is
/// missing -- `pool_body.html` wraps the entire `<ul class="loose">` in
/// `{% if !loose.is_empty() %}`, so an empty pool renders no such element
/// at all rather than an empty one. [`html::between`]'s own fallback to
/// the whole body would make [`then_loose_empty`] check the entire page
/// instead of "is there anything here", which is a false positive waiting
/// to happen the day the page grows more loose-looking markup elsewhere.
fn loose_section(body: &str) -> Option<&str> {
    html::between(body, r#"<ul class="loose">"#, "</ul>").ok()
}

fn then_loose_empty(world: &mut World) -> Result<(), String> {
    let body = super::html_body(world, "no pool screen response recorded")?;
    match loose_section(body) {
        None => Ok(()),
        Some(section) if section.trim().is_empty() => Ok(()),
        Some(section) => Err(format!("expected no loose ends, got:\n{section}")),
    }
}

fn dispatch_loose_shows_n(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolved_count(example, &caps[1])?;
    let body = super::html_body(world, "no pool screen response recorded")?;
    let actual = loose_section(body)
        .map(|section| section.matches(r#"class="loose-item""#).count())
        .unwrap_or(0);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected {expected} loose end(s), got {actual}, in:\n{body}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trip_panel(tag: &str, count_label: &str, items: &str, offers_clear: bool) -> String {
        let clear = if offers_clear {
            r#"<button type="button" class="clear-done" aria-label="Clear done">&times;</button>"#
        } else {
            ""
        };
        format!(
            r#"<div class="trip panel"><div class="trip-header"><div class="trip-tag">{tag}</div><div class="trip-progress"><div class="trip-count">{count_label}</div>{clear}</div></div><ul class="trip-items">{items}</ul></div>"#
        )
    }

    fn item(text: &str, done: bool) -> String {
        let class = if done { "done" } else { "" };
        format!(r#"<li class="{class}"><span class="trip-item-text">{text}</span></li>"#)
    }

    #[test]
    fn trip_section_finds_the_named_panel() {
        let body = format!(
            "{}{}",
            trip_panel("@homedepot", "3 things", "", false),
            trip_panel("@supermarket", "3 things", "", false)
        );
        let section = html::trip_section(&body, "@homedepot").unwrap();
        assert!(section.contains("@homedepot"));
        assert!(!section.contains("@supermarket"));
    }

    #[test]
    fn count_matching_items_counts_struck_and_open_separately() {
        let items = format!("{}{}{}", item("a", true), item("b", false), item("c", true));
        let section = trip_panel("@homedepot", "2 of 3 done", &items, true);
        assert_eq!(count_matching_items(&section, true), 2);
        assert_eq!(count_matching_items(&section, false), 1);
    }

    #[test]
    fn dispatch_shows_struck_passes_with_zero_when_the_trip_has_fallen_to_loose_ends() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<div class="trips"></div>"#.to_string());
        let re = Regex::new(r#"^the trip "([^"]+)" shows "([^"]+)" struck items$"#).unwrap();
        let caps = re
            .captures(r#"the trip "@homedepot" shows "0" struck items"#)
            .unwrap();
        dispatch_shows_struck(&mut world, &BTreeMap::new(), &caps).unwrap();
    }

    #[test]
    fn dispatch_no_clear_control_passes_when_absent() {
        let mut world = World::new();
        world.last_html_body = Some(trip_panel("@homedepot", "3 things", "", false));
        let re = Regex::new(r#"^the trip "([^"]+)" offers no clear-done control$"#).unwrap();
        let caps = re
            .captures(r#"the trip "@homedepot" offers no clear-done control"#)
            .unwrap();
        dispatch_no_clear_control(&mut world, &BTreeMap::new(), &caps).unwrap();
    }

    #[test]
    fn dispatch_clear_control_named_passes_when_present_and_named() {
        let mut world = World::new();
        world.last_html_body = Some(trip_panel("@homedepot", "1 of 3 done", "", true));
        let re = Regex::new(r#"^the trip "([^"]+)" offers a clear-done control named "([^"]+)"$"#)
            .unwrap();
        let caps = re
            .captures(r#"the trip "@homedepot" offers a clear-done control named "Clear done""#)
            .unwrap();
        dispatch_clear_control_named(&mut world, &BTreeMap::new(), &caps).unwrap();
    }

    #[test]
    fn then_control_after_label_passes_when_the_button_follows_the_count() {
        let mut world = World::new();
        world.last_html_body = Some(trip_panel("@homedepot", "1 of 3 done", "", true));
        then_control_after_label(&mut world).unwrap();
    }

    #[test]
    fn then_loose_empty_passes_on_an_empty_section() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<ul class="loose"></ul>"#.to_string());
        then_loose_empty(&mut world).unwrap();
    }

    #[test]
    fn then_loose_empty_errors_when_something_is_listed() {
        let mut world = World::new();
        world.last_html_body =
            Some(r#"<ul class="loose"><li class="loose-item"></li></ul>"#.to_string());
        assert!(then_loose_empty(&mut world).is_err());
    }

    #[test]
    fn then_loose_empty_passes_when_the_list_is_absent_entirely() {
        let mut world = World::new();
        world.last_html_body = Some(trip_panel("@homedepot", "3 of 5 done", "", true));
        then_loose_empty(&mut world).unwrap();
    }

    #[test]
    fn dispatch_loose_shows_n_reports_zero_when_the_list_is_absent_entirely() {
        let mut world = World::new();
        world.last_html_body = Some(trip_panel("@homedepot", "3 of 5 done", "", true));
        let re = Regex::new(r#"^the loose ends list shows "([^"]+)" items$"#).unwrap();
        let caps = re
            .captures(r#"the loose ends list shows "0" items"#)
            .unwrap();
        dispatch_loose_shows_n(&mut world, &BTreeMap::new(), &caps).unwrap();
    }
}
