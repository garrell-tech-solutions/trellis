//! Step handlers for `features/free_time.feature`: the free time page
//! reports when each life area is actually free (#60).
//!
//! The Background, "the life area ... is saved with a guardrail band ...",
//! and "a life area named ... was added" steps this feature also uses are
//! already matched generically by [`super::triage::dispatch`],
//! [`super::guardrails::dispatch`] and [`super::life_areas::dispatch`],
//! tried before this module.

use super::html;
use super::inbox_view::html_response;
use super::life_areas::{life_area_id_by_name, resolve, resolved_life_area_id};
use super::*;
use axum::body::Body;
use axum::http::Request;

static GIVEN_SERVER_BELIEVES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the server believes it is "([^"]+)"$"#).unwrap());
static GIVEN_OWNER_TIMEZONE_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the owner's timezone is "([^"]+)"$"#).unwrap());
static GIVEN_SAVED_NEVER_SCHEDULED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the life area "([^"]+)" is saved as never scheduled$"#).unwrap()
});
static WHEN_FREE_TIME_VIEWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the free time page is viewed$").unwrap());
static THEN_REPORTS_HOURS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the free time page reports "([^"]+)" hours free for "([^"]+)"$"#).unwrap()
});
static THEN_LISTS_INTERVALS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the free time page lists "([^"]+)" free intervals for "([^"]+)"$"#).unwrap()
});
static THEN_PAGE_NO_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the free time page does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_PAGE_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the free time page contains the word "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = GIVEN_SERVER_BELIEVES.captures(text) {
        return Some(dispatch_server_believes(world, example, &caps));
    }
    if let Some(caps) = GIVEN_OWNER_TIMEZONE_IS.captures(text) {
        return Some(dispatch_owner_timezone_is(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_SAVED_NEVER_SCHEDULED.captures(text) {
        return Some(dispatch_saved_never_scheduled(world, example, &caps).await);
    }
    if WHEN_FREE_TIME_VIEWED.is_match(text) {
        return Some(when_free_time_viewed(world).await);
    }
    if let Some(caps) = THEN_REPORTS_HOURS.captures(text) {
        return Some(dispatch_reports_hours(world, example, &caps).await);
    }
    if let Some(caps) = THEN_LISTS_INTERVALS.captures(text) {
        return Some(dispatch_lists_intervals(world, example, &caps).await);
    }
    if THEN_PAGE_NO_SCRIPT.is_match(text) {
        return Some(super::then_html_body_excludes(
            world,
            "<script>",
            "no page response recorded",
        ));
    }
    if let Some(caps) = THEN_PAGE_CONTAINS_WORD.captures(text) {
        return Some(super::then_html_body_contains(
            world,
            &caps[1],
            "no page response recorded",
        ));
    }
    None
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no page response recorded")
}

/// Parses an RFC 3339 instant into epoch milliseconds -- the same shape
/// `trellis serve --now` accepts, since this step is that flag's
/// acceptance-side counterpart.
fn parse_instant_ms(text: &str) -> Result<i64, String> {
    text.parse::<jiff::Timestamp>()
        .map(|ts| ts.as_millisecond())
        .map_err(|e| format!("bad instant {text:?}: {e}"))
}

fn dispatch_server_believes(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let instant = resolve(example, &caps[1])?;
    world.pinned_now_ms = Some(parse_instant_ms(&instant)?);
    Ok(())
}

/// Sets the owner's timezone directly, ignoring the response -- this is
/// background state for a scenario about free time, not a test of
/// `POST /timezone` itself (`timezone_setting.feature` already owns that).
/// Delegates to [`super::guardrails::post_timezone`], the same POST that
/// step's own "is set to" wording drives.
async fn dispatch_owner_timezone_is(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let zone = resolve(example, &caps[1])?;
    super::guardrails::post_timezone(world, &zone).await
}

/// "Never scheduled" is this feature's own wording for the same submission
/// `guardrails.feature`'s "is saved as pool-only" step already drives --
/// delegates to [`super::guardrails::post_guardrail`] rather than reissuing
/// the same POST.
async fn dispatch_saved_never_scheduled(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let id = resolved_life_area_id(world, example, &caps[1]).await?;
    super::guardrails::post_guardrail(world, id, &[("pool_only", "on")]).await
}

async fn when_free_time_viewed(world: &mut World) -> Result<(), String> {
    let request = Request::builder()
        .uri("/free-time")
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

fn free_time_row_section(body: &str, id: i64) -> Result<&str, String> {
    let start_tag = format!(r#"<div id="free-time-row-{id}">"#);
    html::between(body, &start_tag, "</div>")
}

async fn free_time_row<'a>(world: &'a World, name: &str) -> Result<&'a str, String> {
    let id = life_area_id_by_name(world, name).await?;
    let body = html_body(world)?;
    free_time_row_section(body, id)
}

async fn dispatch_reports_hours(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let hours = resolve(example, &caps[1])?;
    let name = resolve(example, &caps[2])?;
    let section = free_time_row(world, &name).await?;
    super::then_section_contains(section, &name, "report", &format!("{hours}h"))
}

async fn dispatch_lists_intervals(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected: usize = resolve(example, &caps[1])?
        .parse()
        .map_err(|e| format!("bad interval count: {e}"))?;
    let name = resolve(example, &caps[2])?;
    let section = free_time_row(world, &name).await?;
    let interval_list = html::between(section, r#"<ul class="free-time-intervals">"#, "</ul>")?;
    let actual = interval_list.matches("<li>").count();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected {name:?} to list {expected} free intervals, got {actual} in:\n{interval_list}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_instant_ms_reads_an_rfc3339_instant() {
        let ms = parse_instant_ms("2027-03-13T12:00:00-05:00").unwrap();
        assert_eq!(
            ms,
            jiff::Timestamp::from_millisecond(ms)
                .unwrap()
                .as_millisecond()
        );
        assert!(parse_instant_ms("not an instant").is_err());
    }

    #[tokio::test]
    async fn server_believes_pins_the_worlds_clock() {
        let mut world = migrated_world().await;
        let caps = GIVEN_SERVER_BELIEVES
            .captures(r#"the server believes it is "2027-03-13T12:00:00-05:00""#)
            .unwrap();

        dispatch_server_believes(&mut world, &BTreeMap::new(), &caps).unwrap();

        assert_eq!(
            world.pinned_now_ms,
            Some(parse_instant_ms("2027-03-13T12:00:00-05:00").unwrap())
        );
    }

    #[tokio::test]
    async fn owner_timezone_is_changes_the_stored_zone() {
        let mut world = migrated_world().await;

        let caps = GIVEN_OWNER_TIMEZONE_IS
            .captures(r#"the owner's timezone is "Europe/London""#)
            .unwrap();
        dispatch_owner_timezone_is(&mut world, &BTreeMap::new(), &caps)
            .await
            .unwrap();

        let pool = world.pool().unwrap();
        assert_eq!(
            trellis_server::settings::store::get_timezone(pool)
                .await
                .unwrap(),
            "Europe/London"
        );
    }

    #[tokio::test]
    async fn saved_never_scheduled_marks_the_life_area_pool_only() {
        let mut world = migrated_world().await;

        let caps = GIVEN_SAVED_NEVER_SCHEDULED
            .captures(r#"the life area "Work" is saved as never scheduled"#)
            .unwrap();
        dispatch_saved_never_scheduled(&mut world, &example(&[]), &caps)
            .await
            .unwrap();

        when_free_time_viewed(&mut world).await.unwrap();
        let section = free_time_row(&world, "Work").await.unwrap();
        assert!(section.contains("0h"), "got:\n{section}");
    }

    #[tokio::test]
    async fn reports_hours_and_lists_intervals_round_trip_a_saved_band() {
        let mut world = migrated_world().await;
        let ex = example(&[("days", "Mon"), ("start", "09:00"), ("end", "17:00")]);
        super::super::guardrails::dispatch(
            &mut world,
            r#"the life area "Work" is saved with a guardrail band on "<days>" from "<start>" to "<end>""#,
            &ex,
        )
        .await
        .unwrap()
        .unwrap();

        when_free_time_viewed(&mut world).await.unwrap();

        let hours_caps = THEN_REPORTS_HOURS
            .captures(r#"the free time page reports "16" hours free for "Work""#)
            .unwrap();
        dispatch_reports_hours(
            &mut world,
            &example(&[("hours", "16"), ("name", "Work")]),
            &hours_caps,
        )
        .await
        .unwrap();

        let intervals_caps = THEN_LISTS_INTERVALS
            .captures(r#"the free time page lists "2" free intervals for "Work""#)
            .unwrap();
        dispatch_lists_intervals(
            &mut world,
            &example(&[("intervals", "2"), ("name", "Work")]),
            &intervals_caps,
        )
        .await
        .unwrap();
    }

    #[test]
    fn then_page_no_script_regex_matches_the_exact_step_text() {
        assert!(THEN_PAGE_NO_SCRIPT
            .is_match(r#"the free time page does not contain an unescaped "<script>" tag"#));
    }
}
