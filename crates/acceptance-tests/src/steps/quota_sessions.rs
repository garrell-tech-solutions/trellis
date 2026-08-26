//! Step handlers for `features/quota_sessions.feature`: logging hours
//! against a quota (#93, the deferred half).
//!
//! "the owner's timezone is ..." and "the server believes it is ..." are
//! already matched generically by [`super::committed_date::dispatch`] and
//! [`super::triage::dispatch`]; "a quota named ... is defined" and "the
//! quota screen offers the quotas ..." by [`super::quota_screen::dispatch`]
//! — nothing here duplicates them.

use super::html;
use super::inbox_view::{html_response, urlencode};
use super::*;
use axum::body::Body;
use axum::http::Request;

/// `"Mon"` for `jiff::civil::Weekday::Monday`, and so on -- computed
/// independently of `scheduler_core::quota::Weekday`, in `jiff` terms
/// directly, the way `given_server_believes_it_is` already works with
/// clock instants. This harness verifies the product's own day labels; it
/// does not borrow the product's derivation to check the product's output:
/// `LABELS` is this file's own table, indexed by `jiff`'s own Monday-zero
/// ordinal, never `scheduler_core::quota`'s.
const LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn weekday_label(weekday: jiff::civil::Weekday) -> &'static str {
    LABELS[weekday.to_monday_zero_offset() as usize]
}

static WHEN_TAPPED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^"([^"]+)" is tapped on the quota "([^"]+)"$"#).unwrap());
static SESSION_LOGGED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^a session of "([^"]+)" minutes on "([^"]+)" is logged against "([^"]+)"$"#)
        .unwrap()
});
static THEN_OFFERS_DAYS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^logging a session against "([^"]+)" offers the days "([^"]+)"$"#).unwrap()
});
static THEN_WEEK_LISTS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^this week for "([^"]+)" lists "([^"]+)"$"#).unwrap());
static THEN_WEEK_SUMMARISES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^this week for "([^"]+)" summarises "([^"]+)"$"#).unwrap());
static THEN_WEEK_SAYS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^this week for "([^"]+)" says "([^"]+)"$"#).unwrap());
static WHEN_CORRECTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^that session is corrected to "([^"]+)" minutes on "([^"]+)"$"#).unwrap()
});
static WHEN_DELETED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the session on "([^"]+)" for "([^"]+)" is deleted$"#).unwrap());
static THEN_SESSION_REJECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the session is rejected$").unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = WHEN_TAPPED.captures(text) {
        return Some(dispatch_tapped(world, example, &caps).await);
    }
    if let Some(caps) = SESSION_LOGGED.captures(text) {
        return Some(dispatch_session_logged(world, example, &caps).await);
    }
    if let Some(caps) = THEN_OFFERS_DAYS.captures(text) {
        return Some(dispatch_offers_days(world, example, &caps));
    }
    if let Some(caps) = THEN_WEEK_LISTS.captures(text) {
        return Some(dispatch_week_lists(world, example, &caps));
    }
    if let Some(caps) = THEN_WEEK_SUMMARISES.captures(text) {
        return Some(dispatch_week_summarises(world, example, &caps));
    }
    if let Some(caps) = THEN_WEEK_SAYS.captures(text) {
        return Some(dispatch_week_says(world, example, &caps));
    }
    if let Some(caps) = WHEN_CORRECTED.captures(text) {
        return Some(dispatch_corrected(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_DELETED.captures(text) {
        return Some(dispatch_deleted(world, example, &caps).await);
    }
    if THEN_SESSION_REJECTED.is_match(text) {
        return Some(super::then_status_is(
            world,
            422,
            "no session response recorded",
        ));
    }
    None
}

/// See `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no quota response recorded")
}

/// `caps[1]` and `caps[2]` resolved together -- see `quota_screen.rs`'s own
/// copy for the full reasoning; duplicated rather than shared per this
/// project's established convention.
fn resolve_pair(
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(String, String), String> {
    Ok((resolve(example, &caps[1])?, resolve(example, &caps[2])?))
}

/// See `quota_screen.rs`'s own copy for the full reasoning; duplicated
/// rather than shared per this project's established convention.
fn expect_eq(actual: &str, expected: &str, mismatch: String) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(mismatch)
    }
}

/// The database id of the one quota named `name` -- every session route
/// this module drives needs it, and the scenario has just as certainly
/// created it (`a quota named ... with a target of ...`) before naming it
/// here.
async fn quota_id(world: &World, name: &str) -> Result<i64, String> {
    let pool = world.pool()?;
    sqlx::query_scalar("SELECT id FROM quotas WHERE name = ?")
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("find quota {name:?}: {e}"))
}

/// The owner's configured timezone (`T-timezone-is-a-setting`), read
/// straight from `settings` rather than through the product's own
/// `settings::current_timezone` -- this harness verifies the product's
/// output against an independent computation, not the product's own
/// derivation.
async fn owner_zone(world: &World) -> Result<jiff::tz::TimeZone, String> {
    let pool = world.pool()?;
    let zone_name: String = sqlx::query_scalar("SELECT timezone FROM settings WHERE id = 1")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("read the owner's timezone: {e}"))?;
    jiff::tz::TimeZone::get(&zone_name)
        .map_err(|_| format!("stored timezone {zone_name:?} does not resolve"))
}

/// Which weekday `instant_ms` falls on, in `zone` -- used both for "today"
/// (the quick-log buttons' own day) and for reading a stored session's
/// `day_ms` back as a label.
fn weekday_of(instant_ms: i64, zone: &jiff::tz::TimeZone) -> &'static str {
    let date = jiff::Timestamp::from_millisecond(instant_ms)
        .expect("instant_ms is a valid instant")
        .to_zoned(zone.clone())
        .date();
    weekday_label(date.weekday())
}

async fn post_path(world: &mut World, path: &str, fields: &[(&str, &str)]) -> Result<(), String> {
    let body = fields
        .iter()
        .map(|(name, value)| format!("{name}={}", urlencode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

/// `"+30m"` and `"+1h"` are the only two controls this feature names -- a
/// third value is a typo in the feature file, not a new control this step
/// needs to learn, so it fails loudly rather than guessing a number.
fn quick_log_minutes(control: &str) -> Result<&'static str, String> {
    match control {
        "+30m" => Ok("30"),
        "+1h" => Ok("60"),
        other => Err(format!("unknown quick-log control {other:?}")),
    }
}

async fn dispatch_tapped(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (control, name) = resolve_pair(example, caps)?;
    let minutes = quick_log_minutes(&control)?;
    let zone = owner_zone(world).await?;
    let today = weekday_of(world.clock().now_ms(), &zone);
    let id = quota_id(world, &name).await?;
    post_path(
        world,
        &format!("/quota/{id}/sessions"),
        &[("day", today), ("minutes", minutes)],
    )
    .await
}

async fn dispatch_session_logged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let minutes = resolve(example, &caps[1])?;
    let day = resolve(example, &caps[2])?;
    let name = resolve(example, &caps[3])?;
    let id = quota_id(world, &name).await?;
    post_path(
        world,
        &format!("/quota/{id}/sessions"),
        &[("day", &day), ("minutes", &minutes)],
    )
    .await
}

/// Every `<option value="...">`'s own value, in document order -- the day
/// picker's own offered days.
fn day_labels(select_section: &str) -> Vec<String> {
    select_section
        .split("<option value=\"")
        .skip(1)
        .filter_map(|chunk| chunk.split_once('"').map(|(day, _)| day.to_string()))
        .collect()
}

fn dispatch_offers_days(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (name, expected) = resolve_pair(example, caps)?;
    let body = html_body(world)?;
    let row = html::quota_row(body, &name)?;
    let select = html::between(row, r#"<select name="day">"#, "</select>")?;
    html::listed_in_order(
        &expected,
        day_labels(select),
        &format!("logging against {name:?} to offer"),
    )
}

/// Every `.quota-session-label` text within `row`, in document order --
/// what "this week for X lists ..." means.
fn session_labels(row: &str) -> Vec<String> {
    row.split(r#"<span class="quota-session-label">"#)
        .skip(1)
        .filter_map(|chunk| {
            chunk
                .split_once("</span>")
                .map(|(label, _)| label.to_string())
        })
        .collect()
}

fn dispatch_week_lists(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let expected = resolve(example, &caps[2])?;
    let body = html_body(world)?;
    let row = html::quota_row(body, &name)?;
    html::listed_in_order(
        &expected,
        session_labels(row),
        &format!("this week for {name:?} to list"),
    )
}

fn dispatch_week_summarises(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (name, expected) = resolve_pair(example, caps)?;
    let body = html_body(world)?;
    let row = html::quota_row(body, &name)?;
    let summary = html::between(row, r#"<div class="quota-sessions-summary">"#, "</div>")?;
    expect_eq(
        summary,
        &expected,
        format!("expected this week for {name:?} to summarise {expected:?}, got {summary:?}"),
    )
}

fn dispatch_week_says(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (name, expected) = resolve_pair(example, caps)?;
    let body = html_body(world)?;
    let row = html::quota_row(body, &name)?;
    let message = html::between(row, r#"<p class="quota-sessions-empty">"#, "</p>")?;
    expect_eq(
        message,
        &expected,
        format!("expected this week for {name:?} to say {expected:?}, got {message:?}"),
    )
}

/// The most recently logged session in the whole database -- unambiguous
/// because every scenario using "that session" has logged exactly one
/// before referring back to it.
async fn last_session_id(world: &World) -> Result<i64, String> {
    let pool = world.pool()?;
    sqlx::query_scalar("SELECT id FROM quota_sessions ORDER BY id DESC LIMIT 1")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("find the last logged session: {e}"))
}

async fn dispatch_corrected(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let minutes = resolve(example, &caps[1])?;
    let day = resolve(example, &caps[2])?;
    let session_id = last_session_id(world).await?;
    post_path(
        world,
        &format!("/quota/sessions/{session_id}"),
        &[("day", &day), ("minutes", &minutes)],
    )
    .await
}

/// The session logged for `day` against the quota named `name` -- found by
/// reading every one of that quota's sessions back and asking which
/// `day_ms` resolves to `day` in the owner's own zone
/// ([`weekday_of`], computed independently of the product's own).
/// Every session's own `(id, day_ms)` logged against the quota named
/// `name` -- [`session_id_on_day`]'s own fetch, split out so its search
/// stays a single, uncomplicated `find`.
async fn quota_session_days(world: &World, name: &str) -> Result<Vec<(i64, i64)>, String> {
    let id = quota_id(world, name).await?;
    let pool = world.pool()?;
    sqlx::query_as("SELECT id, day_ms FROM quota_sessions WHERE quota_id = ?")
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("list {name:?}'s sessions: {e}"))
}

async fn session_id_on_day(world: &World, name: &str, day: &str) -> Result<i64, String> {
    let zone = owner_zone(world).await?;
    let sessions = quota_session_days(world, name).await?;
    find_session_id(sessions, day, &zone)
        .ok_or_else(|| format!("no session logged on {day:?} for {name:?}"))
}

fn find_session_id(sessions: Vec<(i64, i64)>, day: &str, zone: &jiff::tz::TimeZone) -> Option<i64> {
    sessions
        .into_iter()
        .find(|(_, day_ms)| weekday_of(*day_ms, zone) == day)
        .map(|(id, _)| id)
}

async fn dispatch_deleted(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let day = resolve(example, &caps[1])?;
    let name = resolve(example, &caps[2])?;
    let session_id = session_id_on_day(world, &name, &day).await?;
    post_path(world, &format!("/quota/sessions/{session_id}/delete"), &[]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_a_literal_value_unchanged() {
        let example = BTreeMap::new();
        assert_eq!(resolve(&example, "Mon"), Ok("Mon".to_string()));
    }

    #[test]
    fn resolve_looks_up_a_placeholder_in_the_example_row() {
        let example = super::super::example(&[("day", "Tue")]);
        assert_eq!(resolve(&example, "<day>"), Ok("Tue".to_string()));
    }

    #[test]
    fn quick_log_minutes_recognizes_the_two_controls_this_feature_names() {
        assert_eq!(quick_log_minutes("+30m"), Ok("30"));
        assert_eq!(quick_log_minutes("+1h"), Ok("60"));
    }

    #[test]
    fn quick_log_minutes_rejects_anything_else() {
        assert!(quick_log_minutes("+2h").is_err());
    }

    #[test]
    fn session_labels_reads_every_label_in_document_order() {
        let row = concat!(
            r#"<li><span class="quota-session-label">Mon 25m</span></li>"#,
            r#"<li><span class="quota-session-label">Tue 35m</span></li>"#,
        );
        assert_eq!(
            session_labels(row),
            vec!["Mon 25m".to_string(), "Tue 35m".to_string()]
        );
    }

    #[test]
    fn dispatch_offers_days_reads_the_options_in_order() {
        let mut world = World::new();
        world.last_html_body = Some(
            concat!(
                r#"<div class="quota-row"><div class="quota-name">Piano</div>"#,
                r#"<select name="day"><option value="Mon">Mon</option>"#,
                r#"<option value="Tue">Tue</option></select></div>"#,
            )
            .to_string(),
        );
        let example = BTreeMap::new();
        let re = THEN_OFFERS_DAYS
            .captures(r#"logging a session against "Piano" offers the days "Mon, Tue""#)
            .unwrap();

        assert_eq!(dispatch_offers_days(&mut world, &example, &re), Ok(()));
    }

    #[test]
    fn dispatch_week_summarises_reads_the_summary_div() {
        let mut world = World::new();
        world.last_html_body = Some(
            concat!(
                r#"<div class="quota-row"><div class="quota-name">Piano</div>"#,
                r#"<div class="quota-sessions-summary">2 sessions · 1h</div></div>"#,
            )
            .to_string(),
        );
        let example = BTreeMap::new();
        let re = THEN_WEEK_SUMMARISES
            .captures(r#"this week for "Piano" summarises "2 sessions · 1h""#)
            .unwrap();

        assert_eq!(dispatch_week_summarises(&mut world, &example, &re), Ok(()));
    }

    #[test]
    fn dispatch_week_says_reads_the_empty_message() {
        let mut world = World::new();
        world.last_html_body = Some(concat!(
            r#"<div class="quota-row"><div class="quota-name">Piano</div>"#,
            r#"<p class="quota-sessions-empty">No sessions yet this week. Log one above when you have done it.</p></div>"#,
        ).to_string());
        let example = BTreeMap::new();
        let re = THEN_WEEK_SAYS
            .captures(r#"this week for "Piano" says "No sessions yet this week. Log one above when you have done it.""#)
            .unwrap();

        assert_eq!(dispatch_week_says(&mut world, &example, &re), Ok(()));
    }
}
