//! Step handlers for `features/trip_controls.feature`: the trip panel's own
//! controls (#120, #125).
//!
//! The Background, "N pool tasks tagged X", "the pool screen is viewed",
//! "X of them are marked done", "one struck item is unchecked", "the trip X
//! reads Y", "the trip X shows N struck/open items" and "the pool screen
//! reports X beside its title" are already matched generically by
//! [`super::pool_screen::dispatch`] and [`super::trip_progress::dispatch`],
//! tried before this module -- nothing here duplicates them. Expanding and
//! collapsing are not asserted here at all: see this feature's own header
//! comment and `qa/trip_controls.md` for why that is a QA concern, not an
//! acceptance one.

use super::html;
use super::inbox_view::{html_response, urlencode};
use super::*;
use axum::body::Body;
use axum::http::Request;

static THEN_SHOW_MORE_NAMED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the trip "([^"]+)" offers a show-more control named "([^"]+)"$"#).unwrap()
});
static THEN_ISSUES_NO_REQUEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^that control issues no request$").unwrap());
static THEN_COMPLETE_NAMED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the trip "([^"]+)" offers a complete-group control named "([^"]+)"$"#).unwrap()
});
static THEN_NO_COMPLETE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the trip "([^"]+)" offers no complete-group control$"#).unwrap()
});
static WHEN_GROUP_COMPLETED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the group "([^"]+)" is completed$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = THEN_SHOW_MORE_NAMED.captures(text) {
        return Some(dispatch_show_more_named(world, example, &caps));
    }
    if THEN_ISSUES_NO_REQUEST.is_match(text) {
        return Some(then_issues_no_request(world));
    }
    if let Some(caps) = THEN_COMPLETE_NAMED.captures(text) {
        return Some(dispatch_complete_named(world, example, &caps));
    }
    if let Some(caps) = THEN_NO_COMPLETE.captures(text) {
        return Some(dispatch_no_complete(world, example, &caps));
    }
    if let Some(caps) = WHEN_GROUP_COMPLETED.captures(text) {
        return Some(dispatch_group_completed(world, example, &caps).await);
    }
    None
}

/// Resolves a captured value that may be a literal or an `Examples`
/// placeholder (`<name>`) written down verbatim in the step text. See
/// `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// The `<button ...>` opening tag and text carrying `class_marker`, within
/// the trip panel labelled `tag` -- the "fetch the recorded page, scope to
/// the trip, find the button" chain every step in this module needs before
/// it can ask its own question of that button.
fn trip_button<'a>(
    world: &'a World,
    tag: &str,
    class_marker: &str,
) -> Result<(&'a str, &'a str), String> {
    let body = super::html_body(world, "no pool screen response recorded")?;
    let section = html::trip_section(body, tag)?;
    html::button(section, class_marker)
}

/// [`trip_button`]'s text alone, owned -- what every "offers a ... control
/// named" step actually compares.
fn button_label(world: &World, tag: &str, class_marker: &str) -> Result<String, String> {
    let (_, label) = trip_button(world, tag, class_marker)?;
    Ok(label.to_string())
}

/// The shared verdict behind "offers a show-more/complete-group control
/// named": same comparison, same sentence shape, differing only in which
/// control `what` names.
fn expect_button_label(label: &str, expected: &str, tag: &str, what: &str) -> Result<(), String> {
    if label == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the trip {tag:?} to offer a {what} named {expected:?}, got {label:?}"
        ))
    }
}

fn dispatch_show_more_named(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let expected = resolve(example, &caps[2])?;
    let label = button_label(world, &tag, r#"class="trip-more-toggle""#)?;
    expect_button_label(&label, &expected, &tag, "show-more control")?;
    world.last_show_more_control_tag = Some(tag);
    Ok(())
}

/// A control's opening tag carries no request-triggering attribute -- the
/// same three htmx uses everywhere else in this app (`hx-get`, `hx-post`,
/// `hx-trigger`) plus `href`, which would make a plain `<button>` navigate.
fn issues_no_request(opening_tag: &str) -> bool {
    !opening_tag.contains("hx-get")
        && !opening_tag.contains("hx-post")
        && !opening_tag.contains("hx-trigger")
        && !opening_tag.contains("href")
}

fn then_issues_no_request(world: &mut World) -> Result<(), String> {
    let tag = world
        .last_show_more_control_tag
        .clone()
        .ok_or_else(|| "no show-more control recorded -- expected a prior \"offers a show-more control named\" step".to_string())?;
    let (opening_tag, _) = trip_button(world, &tag, r#"class="trip-more-toggle""#)?;
    if issues_no_request(opening_tag) {
        Ok(())
    } else {
        Err(format!(
            "expected the show-more control to issue no request, got:\n{opening_tag}"
        ))
    }
}

fn dispatch_complete_named(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let expected = resolve(example, &caps[2])?;
    let label = button_label(world, &tag, r#"class="trip-complete""#)?;
    expect_button_label(&label, &expected, &tag, "complete-group control")
}

fn dispatch_no_complete(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let body = super::html_body(world, "no pool screen response recorded")?;
    let section = html::trip_section(body, &tag)?;
    match html::button(section, r#"class="trip-complete""#) {
        Err(_) => Ok(()),
        Ok(_) => Err(format!(
            "expected the trip {tag:?} to offer no complete-group control, got:\n{section}"
        )),
    }
}

async fn dispatch_group_completed(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/pool/trips/{}/complete", urlencode(&tag)))
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trip_panel(tag: &str, more_button: &str, complete_button: &str) -> String {
        format!(
            r#"<div class="trip panel"><div class="trip-header"><div class="trip-tag">{tag}</div><div class="trip-count">8 things</div></div>{complete_button}<ul class="trip-items"></ul>{more_button}</div>"#
        )
    }

    #[test]
    fn dispatch_show_more_named_passes_and_records_the_tag() {
        let mut world = World::new();
        world.last_html_body = Some(trip_panel(
            "@homedepot",
            r#"<button type="button" class="trip-more-toggle">Show 5 more</button>"#,
            "",
        ));
        let re = Regex::new(r#"^the trip "([^"]+)" offers a show-more control named "([^"]+)"$"#)
            .unwrap();
        let caps = re
            .captures(r#"the trip "@homedepot" offers a show-more control named "Show 5 more""#)
            .unwrap();

        dispatch_show_more_named(&mut world, &BTreeMap::new(), &caps).unwrap();

        assert_eq!(
            world.last_show_more_control_tag.as_deref(),
            Some("@homedepot")
        );
    }

    #[test]
    fn issues_no_request_is_true_for_a_bare_button() {
        assert!(issues_no_request(
            r#"<button type="button" class="trip-more-toggle">"#
        ));
    }

    #[test]
    fn issues_no_request_is_false_for_an_hx_get_button() {
        assert!(!issues_no_request(
            r#"<button type="button" class="trip-more-toggle" hx-get="/pool/trips/x/expand">"#
        ));
    }

    #[test]
    fn then_issues_no_request_passes_after_the_show_more_step_recorded_a_tag() {
        let mut world = World::new();
        world.last_html_body = Some(trip_panel(
            "@homedepot",
            r#"<button type="button" class="trip-more-toggle">Show 5 more</button>"#,
            "",
        ));
        world.last_show_more_control_tag = Some("@homedepot".to_string());

        assert_eq!(then_issues_no_request(&mut world), Ok(()));
    }

    #[test]
    fn then_issues_no_request_errors_without_a_prior_show_more_step() {
        let mut world = World::new();
        assert!(then_issues_no_request(&mut world).is_err());
    }

    /// A `World` whose recorded pool screen holds a trip carrying a
    /// complete-group control -- the fixture both `dispatch_complete_named`
    /// tests below share.
    fn world_with_complete_control() -> World {
        let mut world = World::new();
        world.last_html_body = Some(trip_panel(
            "@homedepot",
            "",
            r#"<div class="trip-actions"><button type="button" class="trip-complete">Complete all 8</button></div>"#,
        ));
        world
    }

    #[test]
    fn dispatch_complete_named_passes_on_a_match() {
        let mut world = world_with_complete_control();
        let re =
            Regex::new(r#"^the trip "([^"]+)" offers a complete-group control named "([^"]+)"$"#)
                .unwrap();
        let caps = re
            .captures(
                r#"the trip "@homedepot" offers a complete-group control named "Complete all 8""#,
            )
            .unwrap();

        assert_eq!(
            dispatch_complete_named(&mut world, &BTreeMap::new(), &caps),
            Ok(())
        );
    }

    #[test]
    fn dispatch_no_complete_passes_when_absent() {
        let mut world = World::new();
        world.last_html_body = Some(trip_panel("@homedepot", "", ""));
        let re = Regex::new(r#"^the trip "([^"]+)" offers no complete-group control$"#).unwrap();
        let caps = re
            .captures(r#"the trip "@homedepot" offers no complete-group control"#)
            .unwrap();

        assert_eq!(
            dispatch_no_complete(&mut world, &BTreeMap::new(), &caps),
            Ok(())
        );
    }

    #[test]
    fn dispatch_no_complete_errors_when_present() {
        let mut world = world_with_complete_control();
        let re = Regex::new(r#"^the trip "([^"]+)" offers no complete-group control$"#).unwrap();
        let caps = re
            .captures(r#"the trip "@homedepot" offers no complete-group control"#)
            .unwrap();

        assert!(dispatch_no_complete(&mut world, &BTreeMap::new(), &caps).is_err());
    }
}
