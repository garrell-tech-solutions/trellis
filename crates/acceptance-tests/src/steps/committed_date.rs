//! Step handlers for `features/committed_date.feature`: a committed date is
//! chosen, not typed, and lands on the day intended (#110).
//!
//! The Background ("the trellis server is running with an empty task list",
//! "the server believes it is ..."), "a capture with raw text ... is
//! waiting", "the committed screen is viewed", the generic rejection steps
//! ("the triage is rejected", "the rejection names ...", "the task list is
//! still empty") are already matched generically by [`super::triage::dispatch`]
//! and [`super::quota_triage_validation::dispatch`], tried before this
//! module — nothing here duplicates them.

use super::html;
use super::*;
use serde_json::json;

static GIVEN_TIMEZONE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the owner's timezone is "([^"]+)"$"#).unwrap());
static WHEN_TRIAGED_AT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the capture is triaged as a committed "(at|by)" on day "([^"]+)" at time "([^"]+)"$"#,
    )
    .unwrap()
});
static WHEN_TRIAGED_BY_NO_TIME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a committed "(at|by)" on day "([^"]+)" with no time$"#)
        .unwrap()
});
static WHEN_TRIAGED_OMITTING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a committed "(at|by)" with "([^"]+)" omitted$"#)
        .unwrap()
});
static WHEN_TRIAGED_OVER_API: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the capture is triaged over the API as a committed "(at|by)" with deadline "([^"]+)"$"#,
    )
    .unwrap()
});
static THEN_ACCEPTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the triage is accepted$").unwrap());
static THEN_SHOWS_WITH_CELL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the committed screen shows "([^"]+)" with the date cell "([^"]+)"$"#).unwrap()
});
static THEN_DUE_AT_END_OF_DAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^"([^"]+)" is due at the end of "([^"]+)" in the owner's timezone$"#).unwrap()
});
static THEN_NOT_SHOWN_AS_AT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the committed screen does not show "([^"]+)" as an at$"#).unwrap()
});

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = GIVEN_TIMEZONE.captures(text) {
        return Some(given_timezone(world, &caps[1]).await);
    }
    if let Some(caps) = WHEN_TRIAGED_AT.captures(text) {
        return Some(dispatch_triaged_at(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_BY_NO_TIME.captures(text) {
        return Some(dispatch_triaged_no_time(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_OMITTING.captures(text) {
        return Some(dispatch_triaged_omitting(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_OVER_API.captures(text) {
        return Some(dispatch_triaged_over_api(world, example, &caps).await);
    }
    if THEN_ACCEPTED.is_match(text) {
        return Some(then_triage_is_accepted(world));
    }
    if let Some(caps) = THEN_SHOWS_WITH_CELL.captures(text) {
        return Some(dispatch_shows_with_cell(world, example, &caps).await);
    }
    if let Some(caps) = THEN_DUE_AT_END_OF_DAY.captures(text) {
        return Some(dispatch_due_at_end_of_day(world, example, &caps).await);
    }
    if let Some(caps) = THEN_NOT_SHOWN_AS_AT.captures(text) {
        return Some(then_not_shown_as_at(world, &caps[1]).await);
    }
    None
}

/// Resolves a captured value that may be a literal or an `Examples`
/// placeholder (`<day>`) written down verbatim in the step text. See
/// `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// There is no way to set the timezone from the running app
/// (`qa/committed_date.md`: `T-timezone-is-a-setting` stored it, #88 took
/// the page it was edited on, #85 has not yet brought a replacement) --
/// writing the row directly is the documented way this feature sets up its
/// own Background.
async fn given_timezone(world: &mut World, zone: &str) -> Result<(), String> {
    let pool = world.pool()?.clone();
    sqlx::query("UPDATE settings SET timezone = ? WHERE id = 1")
        .bind(zone)
        .execute(&pool)
        .await
        .map_err(|e| format!("set timezone: {e}"))?;
    Ok(())
}

/// The committed form's own fields for `commitment`, always carrying
/// `deadline_date`; `time` is present only for an *at* -- a *by* never asks
/// the page for one (`committed-date-by-takes-a-day-02`).
fn committed_page_fields<'a>(
    commitment: &'a str,
    date: &'a str,
    time: Option<&'a str>,
) -> Vec<(&'a str, &'a str)> {
    let mut fields = vec![
        ("kind", "committed"),
        ("deadline_date", date),
        ("commitment", commitment),
        ("priority", "P1"),
        ("estimated_minutes", "30"),
    ];
    if let Some(time) = time {
        fields.push(("deadline_time", time));
    }
    fields
}

async fn triage_committed_via_page(
    world: &mut World,
    commitment: &str,
    date: &str,
    time: Option<&str>,
) -> Result<(), String> {
    let fields = committed_page_fields(commitment, date, time);
    super::triage_from_page::when_triaged_through_page(world, &fields).await
}

async fn dispatch_triaged_at(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let commitment = caps[1].to_string();
    let day = resolve(example, &caps[2])?;
    let time = resolve(example, &caps[3])?;
    triage_committed_via_page(world, &commitment, &day, Some(&time)).await
}

async fn dispatch_triaged_no_time(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let commitment = caps[1].to_string();
    let day = resolve(example, &caps[2])?;
    triage_committed_via_page(world, &commitment, &day, None).await
}

/// `missing_field` names a core `Field` ("deadline", "commitment"), not a
/// literal form field name -- "deadline" means the page's own
/// `deadline_date` (there is no bare `deadline` on this form at all), which
/// is exactly the point: whichever piece the page uses, its absence must
/// still be reported as `deadline`.
async fn dispatch_triaged_omitting(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let commitment = &caps[1];
    let missing_field = resolve(example, &caps[2])?;
    let omit_name = match missing_field.as_str() {
        "deadline" => "deadline_date",
        other => other,
    };
    let fields: Vec<(&str, &str)> = committed_page_fields(commitment, "2026-08-27", None)
        .into_iter()
        .filter(|(name, _)| *name != omit_name)
        .collect();
    super::triage_from_page::when_triaged_through_page(world, &fields).await
}

async fn dispatch_triaged_over_api(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let commitment = &caps[1];
    let deadline = resolve(example, &caps[2])?;
    let body = json!({
        "kind": "committed",
        "deadline": deadline,
        "commitment": commitment,
        "priority": "P1",
        "estimated_minutes": 30,
    });
    super::triage::when_triaged(world, body).await
}

fn then_triage_is_accepted(world: &mut World) -> Result<(), String> {
    match world.last_status {
        Some(201) => Ok(()),
        other => Err(format!(
            "expected the triage to be accepted (201), got {other:?}"
        )),
    }
}

async fn view_committed_screen(world: &mut World) -> Result<(), String> {
    let request = axum::http::Request::builder()
        .uri("/committed")
        .body(axum::body::Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    super::inbox_view::html_response(world, request).await
}

fn committed_rows_scope(body: &str) -> &str {
    html::between(body, r#"<ul class="committed-rows">"#, "</ul>").unwrap_or(body)
}

/// `the committed screen shows ...` does not depend on a prior `the
/// committed screen is viewed` step (some scenarios have one, some do not)
/// -- it re-fetches `/committed` itself so either shape of Gherkin works.
async fn dispatch_shows_with_cell(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let expected_cell = resolve(example, &caps[2])?;
    view_committed_screen(world).await?;
    let body = super::html_body(world, "no committed screen response recorded")?;
    let row = html::row_containing(committed_rows_scope(body), &raw_text)?;
    let cell = html::between(row, r#"<div class="committed-date">"#, "</div>")?;
    if cell == expected_cell {
        Ok(())
    } else {
        Err(format!(
            "expected the row for {raw_text:?} to show the date cell {expected_cell:?}, got {cell:?}"
        ))
    }
}

/// Independently computes the end of `day` in `zone` -- via `jiff` directly,
/// not `scheduler_core::task::fields::local_deadline_ms` (an assertion that
/// reuses the function under test proves nothing).
fn expected_end_of_day_ms(day: &str, zone: &str) -> Result<i64, String> {
    let date: jiff::civil::Date = day.parse().map_err(|e| format!("bad day {day:?}: {e}"))?;
    let next = date
        .tomorrow()
        .map_err(|e| format!("day {day} has no tomorrow: {e}"))?;
    let start_of_next = next
        .at(0, 0, 0, 0)
        .in_tz(zone)
        .map_err(|e| format!("bad zone {zone:?}: {e}"))?;
    Ok((start_of_next - std::time::Duration::from_millis(1))
        .timestamp()
        .as_millisecond())
}

/// The stored deadline, in milliseconds, for the task triaged from the
/// capture whose raw text was `raw_text`.
async fn stored_deadline_ms(pool: &sqlx::SqlitePool, raw_text: &str) -> Result<i64, String> {
    sqlx::query_scalar(
        "SELECT tasks.deadline FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE captures.raw_text = ?",
    )
    .bind(raw_text)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("read stored deadline for {raw_text:?}: {e}"))
}

async fn current_timezone(pool: &sqlx::SqlitePool) -> Result<String, String> {
    sqlx::query_scalar("SELECT timezone FROM settings WHERE id = 1")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("read timezone: {e}"))
}

fn due_at_end_of_day_result(
    raw_text: &str,
    day: &str,
    zone: &str,
    expected_ms: i64,
    actual_ms: i64,
) -> Result<(), String> {
    if actual_ms == expected_ms {
        Ok(())
    } else {
        Err(format!(
            "expected {raw_text:?} due at {expected_ms}ms (end of {day} in {zone}), got {actual_ms}ms"
        ))
    }
}

async fn dispatch_due_at_end_of_day(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let day = resolve(example, &caps[2])?;
    let pool = world.pool()?.clone();
    let zone = current_timezone(&pool).await?;

    let expected_ms = expected_end_of_day_ms(&day, &zone)?;
    let actual_ms = stored_deadline_ms(&pool, &raw_text).await?;

    due_at_end_of_day_result(&raw_text, &day, &zone, expected_ms, actual_ms)
}

async fn then_not_shown_as_at(world: &mut World, raw_text: &str) -> Result<(), String> {
    let pool = world.pool()?.clone();
    let commitment: String = sqlx::query_scalar(
        "SELECT tasks.commitment FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE captures.raw_text = ?",
    )
    .bind(raw_text)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("read stored commitment for {raw_text:?}: {e}"))?;

    if commitment == "at" {
        Err(format!(
            "expected {raw_text:?} not to be stored as an at, got commitment={commitment:?}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::triage::given_capture_waiting;
    use super::*;

    fn body_with_rows(rows: &str) -> String {
        format!(r#"<ul class="committed-rows">{rows}</ul>"#)
    }

    fn row(text: &str, date_cell: &str) -> String {
        format!(
            r#"<li class="committed-row"><div class="committed-date">{date_cell}</div><div class="committed-text">{text}</div></li>"#
        )
    }

    #[test]
    fn committed_page_fields_omits_time_for_a_by() {
        let fields = committed_page_fields("by", "2026-08-27", None);
        assert!(!fields.iter().any(|(name, _)| *name == "deadline_time"));
    }

    #[test]
    fn committed_page_fields_carries_time_for_an_at() {
        let fields = committed_page_fields("at", "2026-08-25", Some("08:30"));
        assert!(fields.contains(&("deadline_time", "08:30")));
    }

    #[tokio::test]
    async fn given_timezone_updates_the_settings_row() {
        let mut world = migrated_world().await;
        given_timezone(&mut world, "America/New_York")
            .await
            .unwrap();

        let pool = world.pool().unwrap().clone();
        let zone: String = sqlx::query_scalar("SELECT timezone FROM settings WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(zone, "America/New_York");
    }

    #[tokio::test]
    async fn triage_committed_via_page_creates_a_committed_task() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "book the dentist")
            .await
            .unwrap();

        triage_committed_via_page(&mut world, "at", "2026-08-25", Some("08:30"))
            .await
            .unwrap();

        assert_eq!(world.last_status, Some(201));
    }

    #[test]
    fn then_triage_is_accepted_passes_on_201() {
        let mut world = World::new();
        world.last_status = Some(201);
        assert_eq!(then_triage_is_accepted(&mut world), Ok(()));
    }

    #[test]
    fn then_triage_is_accepted_errors_on_422() {
        let mut world = World::new();
        world.last_status = Some(422);
        assert!(then_triage_is_accepted(&mut world).is_err());
    }

    #[test]
    fn dispatch_shows_with_cell_scoping_finds_the_named_rows_own_cell() {
        let body = body_with_rows(&format!(
            "{}{}",
            row("Book the dentist", "TUE 8:30"),
            row("File the tax return", "BY THU")
        ));
        let section = committed_rows_scope(&body);
        let row = html::row_containing(section, "File the tax return").unwrap();
        let cell = html::between(row, r#"<div class="committed-date">"#, "</div>").unwrap();
        assert_eq!(cell, "BY THU");
    }
}
