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
static THEN_NO_UNDO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the pool screen offers no way to un-do a completed task$").unwrap()
});
static THEN_NO_COMPLETED_LIST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the pool screen offers no list of completed work$").unwrap());
static THEN_RESPONSE_NO_UNESCAPED_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the response does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_RESPONSE_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the response contains the word "([^"]+)"$"#).unwrap());

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
    if THEN_NO_UNDO.is_match(text) {
        return Some(then_body_excludes_any(
            world,
            &["Undo", "Un-do", "Restore", "Un-archive", "Uncomplete"],
        ));
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
    None
}

/// A task is named by its text in this feature, not its id -- the id is a
/// database detail no scenario should have to know. Looks the task up by
/// the capture it came from and routes to whichever screen's own front
/// door owns it (`T-one-front-door-per-capability`: `pool` and `committed`
/// each own their own mark-done route).
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

async fn mark_done(world: &mut World, raw_text: &str) -> Result<(), String> {
    let pool = world.pool()?.clone();
    let (task_id, kind): (i64, String) = sqlx::query_as(
        "SELECT tasks.id, tasks.kind FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE captures.raw_text = ?",
    )
    .bind(raw_text)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("find the task triaged from {raw_text:?}: {e}"))?;

    let path = done_route(&kind, task_id)?;
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
}
