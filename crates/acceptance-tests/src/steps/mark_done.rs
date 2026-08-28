//! Step handlers for `features/mark_done.feature`: a task you have done
//! leaves the screen it lives on (#97).
//!
//! The Background ("the trellis server is running with an empty task
//! list") and the setup/assertion steps this feature shares with the
//! screens it acts on ("a pool task ...", "a committed task ...", "the
//! pool/committed screen is viewed", "... reports ... beside its title",
//! "... does not mention ...", "... lists ...", "the pool screen offers no
//! reorder control") are already matched generically by
//! [`super::triage::dispatch`], [`super::pool_screen::dispatch`] and
//! [`super::committed_screen::dispatch`], tried before this module —
//! nothing here duplicates them.

use super::html;
use super::*;
use axum::body::Body;
use axum::http::Request;

static WHEN_MARKED_DONE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^"([^"]+)" is marked done$"#).unwrap());
static THEN_LISTED_AMONG_LOOSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the pool screen lists "([^"]+)" among the loose ends$"#).unwrap()
});
static THEN_NO_TRIPS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the pool screen offers no trips$").unwrap());
static THEN_NO_COMPLETED_LIST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the pool screen offers no list of completed work$").unwrap());
static THEN_RESPONSE_NO_UNESCAPED_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the response does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_RESPONSE_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the response contains the word "([^"]+)"$"#).unwrap());
static THEN_WAY_BACK_OFFERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the way back offers "([^"]*)"$"#).unwrap());
static WHEN_WAY_BACK_TAKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the way back to "([^"]+)" is taken$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    _example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = WHEN_MARKED_DONE.captures(text) {
        return Some(mark_done(world, &caps[1]).await);
    }
    if let Some(caps) = THEN_LISTED_AMONG_LOOSE.captures(text) {
        return Some(then_listed_among_loose(world, &caps[1]));
    }
    if THEN_NO_TRIPS.is_match(text) {
        return Some(then_no_trips(world));
    }
    if THEN_NO_COMPLETED_LIST.is_match(text) {
        return Some(then_body_excludes_any(
            world,
            &["Completed", "Done items", "completed-list"],
        ));
    }
    if THEN_RESPONSE_NO_UNESCAPED_SCRIPT.is_match(text) {
        return Some(then_response_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_RESPONSE_CONTAINS_WORD.captures(text) {
        return Some(then_response_contains(world, &caps[1]));
    }
    if let Some(caps) = THEN_WAY_BACK_OFFERS.captures(text) {
        return Some(then_way_back_offers(world, &caps[1]));
    }
    if let Some(caps) = WHEN_WAY_BACK_TAKEN.captures(text) {
        return Some(way_back_taken(world, &caps[1]).await);
    }
    None
}

/// A task is named by its text in this feature, not its id -- the id is a
/// database detail no scenario should have to know. Shared by [`mark_done`]
/// and [`way_back_taken`], which both need to resolve a name to the task
/// and the screen that owns it before routing.
async fn task_id_and_kind(world: &World, raw_text: &str) -> Result<(i64, String), String> {
    let pool = world.pool()?;
    sqlx::query_as(
        "SELECT tasks.id, tasks.kind FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE captures.raw_text = ?",
    )
    .bind(raw_text)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("find the task triaged from {raw_text:?}: {e}"))
}

/// Which screen owns the mark-done route for a task of this kind.
///
/// Extracted from [`mark_done`] rather than baselined: that function was
/// over `T-complexity-8`'s cap, and it is not a step dispatcher, so the
/// rule's own answer applies -- "a function over 8 is carrying logic that
/// is not the match; extract that". This mapping is the logic, it is the
/// part that grows when the fourth screen lands, and naming it is what
/// makes that growth one obvious line rather than a longer function.
fn done_route(kind: &str, task_id: i64) -> Result<String, String> {
    match kind {
        "pool" => Ok(format!("/pool/tasks/{task_id}/done")),
        "committed" => Ok(format!("/committed/tasks/{task_id}/done")),
        other => Err(format!("mark-done has no route for kind {other:?}")),
    }
}

/// [`done_route`]'s inverse (#111): where undoing a completion of this kind
/// posts. Kept as its own small table rather than deriving `/undone` from
/// [`done_route`]'s own string -- the two routes happen to share a prefix
/// today, but that is incidental, not a rule this step should lean on.
fn undone_route(kind: &str, task_id: i64) -> Result<String, String> {
    match kind {
        "pool" => Ok(format!("/pool/tasks/{task_id}/undone")),
        "committed" => Ok(format!("/committed/tasks/{task_id}/undone")),
        other => Err(format!("mark-done has no undo route for kind {other:?}")),
    }
}

async fn mark_done(world: &mut World, raw_text: &str) -> Result<(), String> {
    let (task_id, kind) = task_id_and_kind(world, raw_text).await?;
    let path = done_route(&kind, task_id)?;
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    super::inbox_view::html_response(world, request).await
}

/// Posts to whichever screen's own undo route owns `raw_text`'s task
/// (#111). Resolved by a fresh database lookup rather than a link scraped
/// out of `world.last_html_body`: every scenario using this step takes the
/// way back straight off the completion's own response, which the step
/// immediately before this one already read, and re-parsing the same
/// fragment here would only duplicate that check rather than add one.
async fn way_back_taken(world: &mut World, raw_text: &str) -> Result<(), String> {
    let (task_id, kind) = task_id_and_kind(world, raw_text).await?;
    let path = undone_route(&kind, task_id)?;
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    super::inbox_view::html_response(world, request).await
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no mark-done response recorded")
}

/// The way back's own name, or `None` when no way-back line renders at all
/// -- `""` in Examples means exactly that absence (`THEN_WAY_BACK_OFFERS`'s
/// own `[^"]*`, not `+`).
fn way_back_name(body: &str) -> Option<&str> {
    html::between(body, r#"<span class="way-back-name">"#, "</span>").ok()
}

fn then_way_back_offers(world: &mut World, expected: &str) -> Result<(), String> {
    let body = html_body(world)?;
    let actual = way_back_name(body).unwrap_or("");
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the way back to offer {expected:?}, got {actual:?} in:\n{body}"
        ))
    }
}

fn loose_scope(body: &str) -> &str {
    html::between(body, r#"<ul class="loose">"#, "</ul>").unwrap_or(body)
}

fn then_listed_among_loose(world: &mut World, raw_text: &str) -> Result<(), String> {
    let body = html_body(world)?;
    match html::row_containing(loose_scope(body), raw_text) {
        Ok(_) => Ok(()),
        Err(_) => Err(format!(
            "expected {raw_text:?} among the loose ends, got:\n{body}"
        )),
    }
}

/// **Absent, not a zero-count section** -- a trip panel that still renders
/// with nothing left in it is the half-pass the spec warns about
/// (`T-trips-are-derived-not-ranked`: a done task must stop counting, not
/// just stop being listed).
fn then_no_trips(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(r#"class="trip panel""#) {
        Err(format!("expected no trip panel, got:\n{body}"))
    } else {
        Ok(())
    }
}

fn then_body_excludes_any(world: &mut World, forbidden: &[&str]) -> Result<(), String> {
    let body = html_body(world)?;
    for needle in forbidden {
        if body.contains(needle) {
            return Err(format!("expected no {needle:?}, found it in:\n{body}"));
        }
    }
    Ok(())
}

fn then_response_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(forbidden) {
        Err(format!("expected no {forbidden:?}, got:\n{body}"))
    } else {
        Ok(())
    }
}

fn then_response_contains(world: &mut World, expected: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected:?} in the response, got:\n{body}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_with_loose(rows: &str) -> String {
        format!(r#"<ul class="loose">{rows}</ul>"#)
    }

    fn loose_row(text: &str) -> String {
        format!(r#"<li class="loose-item"><div class="loose-text">{text}</div></li>"#)
    }

    #[test]
    fn then_listed_among_loose_passes_when_present() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_loose(&loose_row("fix the door latch")));
        assert_eq!(
            then_listed_among_loose(&mut world, "fix the door latch"),
            Ok(())
        );
    }

    #[test]
    fn then_listed_among_loose_errors_when_absent() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_loose(&loose_row("buy milk")));
        assert!(then_listed_among_loose(&mut world, "fix the door latch").is_err());
    }

    #[test]
    fn then_no_trips_passes_when_none_render() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<div class="trips"></div>"#.to_string());
        assert_eq!(then_no_trips(&mut world), Ok(()));
    }

    #[test]
    fn then_no_trips_errors_when_a_trip_panel_renders() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<div class="trip panel"></div>"#.to_string());
        assert!(then_no_trips(&mut world).is_err());
    }

    #[test]
    fn then_body_excludes_any_passes_when_none_of_the_forbidden_words_appear() {
        let mut world = World::new();
        world.last_html_body = Some("<main></main>".to_string());
        assert_eq!(
            then_body_excludes_any(&mut world, &["Undo", "Restore"]),
            Ok(())
        );
    }

    #[test]
    fn then_body_excludes_any_errors_when_a_forbidden_word_appears() {
        let mut world = World::new();
        world.last_html_body = Some("<button>Undo</button>".to_string());
        assert!(then_body_excludes_any(&mut world, &["Undo", "Restore"]).is_err());
    }

    #[test]
    fn then_response_excludes_errors_when_the_forbidden_text_is_present() {
        let mut world = World::new();
        world.last_html_body = Some("<script>alert('boom')</script>".to_string());
        assert!(then_response_excludes(&mut world, "<script>").is_err());
    }

    #[test]
    fn then_response_contains_passes_when_the_word_is_present() {
        let mut world = World::new();
        world.last_html_body = Some("&lt;script&gt;alert('boom')&lt;/script&gt;".to_string());
        assert_eq!(then_response_contains(&mut world, "boom"), Ok(()));
    }

    fn body_with_way_back(name: &str) -> String {
        format!(
            r#"<div class="way-back"><span class="way-back-text"><span class="way-back-name">{name}</span> done.</span></div>"#
        )
    }

    #[test]
    fn way_back_name_reads_the_named_task() {
        assert_eq!(
            way_back_name(&body_with_way_back("buy screws")),
            Some("buy screws")
        );
    }

    #[test]
    fn way_back_name_is_none_when_no_way_back_renders() {
        assert_eq!(way_back_name("<div class=\"pool-header\"></div>"), None);
    }

    #[test]
    fn then_way_back_offers_passes_when_the_name_matches() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_way_back("buy screws"));
        assert_eq!(then_way_back_offers(&mut world, "buy screws"), Ok(()));
    }

    #[test]
    fn then_way_back_offers_empty_string_passes_when_nothing_renders() {
        let mut world = World::new();
        world.last_html_body = Some("<div class=\"pool-header\"></div>".to_string());
        assert_eq!(then_way_back_offers(&mut world, ""), Ok(()));
    }

    #[test]
    fn then_way_back_offers_errors_on_a_mismatched_name() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_way_back("buy screws"));
        assert!(then_way_back_offers(&mut world, "fix the door latch").is_err());
    }

    #[test]
    fn done_route_and_undone_route_agree_on_which_screens_they_know() {
        assert_eq!(done_route("pool", 7), Ok("/pool/tasks/7/done".to_string()));
        assert_eq!(
            undone_route("pool", 7),
            Ok("/pool/tasks/7/undone".to_string())
        );
        assert_eq!(
            done_route("committed", 7),
            Ok("/committed/tasks/7/done".to_string())
        );
        assert_eq!(
            undone_route("committed", 7),
            Ok("/committed/tasks/7/undone".to_string())
        );
        assert!(undone_route("quota", 7).is_err());
    }
}
