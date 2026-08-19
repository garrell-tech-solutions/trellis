//! Step handlers for `features/capacity.feature`: what each life area
//! needs against what it has (#62).
//!
//! The Background, "the life area ... is saved with a guardrail band ...",
//! "the life area ... is saved as never scheduled", "the life area ... is
//! archived", "the server believes it is ..." and "the dates ... are
//! marked away for all life areas" steps this feature also uses are
//! already matched generically by [`super::triage::dispatch`],
//! [`super::guardrails::dispatch`], [`super::free_time::dispatch`],
//! [`super::life_areas::dispatch`] and [`super::exceptions::dispatch`],
//! tried before this module.

use super::html;
use super::inbox_view::html_response;
use super::life_areas::{life_area_id_by_name, resolve};
use super::payloads;
use super::*;
use axum::body::Body;
use axum::http::Request;
use serde_json::Value;

static GIVEN_COMMITTED_TASK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^a committed task in life area "([^"]+)" estimated at "([^"]+)" minutes is triaged$"#,
    )
    .unwrap()
});
static GIVEN_POOL_TASK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^a pool task in life area "([^"]+)" is triaged$"#).unwrap());
static GIVEN_QUOTA_TASK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^a quota task in life area "([^"]+)" targeting "([^"]+)" sessions of "([^"]+)" minutes per "([^"]+)" is triaged$"#,
    )
    .unwrap()
});
static WHEN_CAPACITY_VIEWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the capacity page is viewed$").unwrap());
static THEN_REPORTS_NEEDED_AVAILABLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^capacity reports "([^"]+)" hours needed and "([^"]+)" hours available for "([^"]+)"$"#,
    )
    .unwrap()
});
static THEN_REPORTS_PERCENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^capacity reports "([^"]+)" percent used for "([^"]+)"$"#).unwrap()
});
static THEN_DOES_NOT_WARN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^capacity does not warn for "([^"]+)"$"#).unwrap());
static THEN_WARNS_OVER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^capacity warns that "([^"]+)" is "([^"]+)" hours over$"#).unwrap()
});
static THEN_NEVER_SCHEDULED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^capacity reports "([^"]+)" as never scheduled$"#).unwrap());
static THEN_DOES_NOT_REPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^capacity does not report "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = GIVEN_COMMITTED_TASK.captures(text) {
        return Some(dispatch_committed_task(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_POOL_TASK.captures(text) {
        return Some(dispatch_pool_task(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_QUOTA_TASK.captures(text) {
        return Some(dispatch_quota_task(world, example, &caps).await);
    }
    if WHEN_CAPACITY_VIEWED.is_match(text) {
        return Some(when_capacity_viewed(world).await);
    }
    if let Some(caps) = THEN_REPORTS_NEEDED_AVAILABLE.captures(text) {
        return Some(dispatch_reports_needed_available(world, example, &caps).await);
    }
    if let Some(caps) = THEN_REPORTS_PERCENT.captures(text) {
        return Some(dispatch_reports_percent(world, example, &caps).await);
    }
    if let Some(caps) = THEN_DOES_NOT_WARN.captures(text) {
        return Some(dispatch_does_not_warn(world, example, &caps).await);
    }
    if let Some(caps) = THEN_WARNS_OVER.captures(text) {
        return Some(dispatch_warns_over(world, example, &caps).await);
    }
    if let Some(caps) = THEN_NEVER_SCHEDULED.captures(text) {
        return Some(dispatch_never_scheduled(world, example, &caps).await);
    }
    if let Some(caps) = THEN_DOES_NOT_REPORT.captures(text) {
        return Some(dispatch_does_not_report(world, example, &caps).await);
    }
    None
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no page response recorded")
}

async fn insert_capture(world: &World, raw_text: &str) -> Result<i64, String> {
    let pool = world.pool()?;
    sqlx::query_scalar(
        "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, 'web', 0) RETURNING id",
    )
    .bind(raw_text)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("insert capture: {e}"))
}

/// Triages a fresh, throwaway capture with `body`, erroring out if the
/// triage was refused -- these are setup steps, so a rejection here means
/// the fixture itself is wrong, not something under test.
async fn triage_fixture(world: &mut World, raw_text: &str, body: Value) -> Result<(), String> {
    let capture_id = insert_capture(world, raw_text).await?;
    let uri = format!("/captures/{capture_id}/triage");
    let response = super::app_client::post_json(world, &uri, &body).await?;
    if response.status >= 300 {
        return Err(format!(
            "capacity fixture triage was rejected: status {}, body {:?}",
            response.status, response.body
        ));
    }
    Ok(())
}

async fn dispatch_committed_task(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let minutes = resolved_i64(example, &caps[2], "estimated minutes")?;
    let body = payloads::with_field(
        payloads::with_field(payloads::committed(), "life_area", Value::from(name)),
        "estimated_minutes",
        Value::from(minutes),
    );
    triage_fixture(world, "capacity fixture: committed", body).await
}

async fn dispatch_pool_task(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let body = payloads::with_field(payloads::pool(), "life_area", Value::from(name));
    triage_fixture(world, "capacity fixture: pool", body).await
}

/// Resolves a step's `<name>`-or-literal capture and parses it as an
/// integer, naming `field` in the error so a bad fixture points back at its
/// source. Split out so [`dispatch_quota_task`]'s own body carries only the
/// four fields it assembles, not each one's own resolve-then-parse.
fn resolved_i64(example: &BTreeMap<String, String>, raw: &str, field: &str) -> Result<i64, String> {
    let value = resolve(example, raw)?;
    value
        .parse()
        .map_err(|e| format!("bad {field} {value:?}: {e}"))
}

async fn dispatch_quota_task(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let count = resolved_i64(example, &caps[2], "session count")?;
    let each = resolved_i64(example, &caps[3], "session minutes")?;
    let period = resolve(example, &caps[4])?;
    let body = payloads::quota();
    let body = payloads::with_field(body, "life_area", Value::from(name));
    let body = payloads::with_field(body, "target_count", Value::from(count));
    let body = payloads::with_field(body, "target_minutes_each", Value::from(each));
    let body = payloads::with_field(body, "period", Value::from(period));
    triage_fixture(world, "capacity fixture: quota", body).await
}

async fn when_capacity_viewed(world: &mut World) -> Result<(), String> {
    let request = Request::builder()
        .uri("/capacity")
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

fn capacity_row_section(body: &str, id: i64) -> Result<&str, String> {
    let start_tag = format!(r#"<div id="capacity-row-{id}">"#);
    html::between(body, &start_tag, "</div>")
}

async fn capacity_row<'a>(world: &'a World, name: &str) -> Result<&'a str, String> {
    let id = life_area_id_by_name(world, name).await?;
    let body = html_body(world)?;
    capacity_row_section(body, id)
}

async fn dispatch_reports_needed_available(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let needed = resolve(example, &caps[1])?;
    let available = resolve(example, &caps[2])?;
    let name = resolve(example, &caps[3])?;
    let section = capacity_row(world, &name).await?;
    // Anchored on the template's own preceding text (`"— {needed}h needed, {available}h
    // available"`) rather than a bare `"{needed}h needed"` needle: an unanchored needle
    // is a substring of any decimal ending in the same digit -- "4.4h needed" contains
    // "4h needed" -- so a wrong-by-a-fraction value would pass unnoticed.
    super::then_section_contains(section, &name, "report", &format!("— {needed}h needed"))?;
    super::then_section_contains(
        section,
        &name,
        "report",
        &format!("needed, {available}h available"),
    )
}

async fn dispatch_reports_percent(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let percent = resolve(example, &caps[1])?;
    let name = resolve(example, &caps[2])?;
    let section = capacity_row(world, &name).await?;
    let needle = format!("{percent}%");
    if section.contains(&needle) {
        Ok(())
    } else {
        Err(format!(
            "expected {name:?} to report {needle:?}, got:\n{section}"
        ))
    }
}

async fn dispatch_does_not_warn(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let section = capacity_row(world, &name).await?;
    if section.contains("over") {
        Err(format!(
            "expected {name:?} to carry no warning, got:\n{section}"
        ))
    } else {
        Ok(())
    }
}

async fn dispatch_warns_over(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let over = resolve(example, &caps[2])?;
    let section = capacity_row(world, &name).await?;
    let needle = format!("{over}h over");
    if section.contains(&needle) {
        Ok(())
    } else {
        Err(format!(
            "expected {name:?} to warn {needle:?}, got:\n{section}"
        ))
    }
}

async fn dispatch_never_scheduled(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let section = capacity_row(world, &name).await?;
    if section.contains("never scheduled") {
        Ok(())
    } else {
        Err(format!(
            "expected {name:?} to report never scheduled, got:\n{section}"
        ))
    }
}

async fn dispatch_does_not_report(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let id = life_area_id_by_name(world, &name).await?;
    let body = html_body(world)?;
    if capacity_row_section(body, id).is_ok() {
        Err(format!(
            "expected no capacity row for {name:?}, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_committed_task_reports_supply_against_demand() {
        let mut world = migrated_world().await;
        super::super::guardrails::dispatch(
            &mut world,
            r#"the life area "Fitness" is saved with a guardrail band on "Sat" from "09:00" to "11:00""#,
            &example(&[]),
        )
        .await
        .unwrap()
        .unwrap();

        dispatch_committed_task(
            &mut world,
            &example(&[]),
            &GIVEN_COMMITTED_TASK
                .captures(
                    r#"a committed task in life area "Fitness" estimated at "180" minutes is triaged"#,
                )
                .unwrap(),
        )
        .await
        .unwrap();

        when_capacity_viewed(&mut world).await.unwrap();

        dispatch_reports_needed_available(
            &mut world,
            &example(&[("needed", "3"), ("available", "4"), ("name", "Fitness")]),
            &THEN_REPORTS_NEEDED_AVAILABLE
                .captures(
                    r#"capacity reports "3" hours needed and "4" hours available for "Fitness""#,
                )
                .unwrap(),
        )
        .await
        .unwrap();

        dispatch_does_not_warn(
            &mut world,
            &example(&[("name", "Fitness")]),
            &THEN_DOES_NOT_WARN
                .captures(r#"capacity does not warn for "Fitness""#)
                .unwrap(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn a_never_scheduled_life_area_reports_never_scheduled() {
        let mut world = migrated_world().await;
        super::super::free_time::dispatch(
            &mut world,
            r#"the life area "Fitness" is saved as never scheduled"#,
            &example(&[]),
        )
        .await
        .unwrap()
        .unwrap();

        when_capacity_viewed(&mut world).await.unwrap();

        dispatch_never_scheduled(
            &mut world,
            &example(&[("name", "Fitness")]),
            &THEN_NEVER_SCHEDULED
                .captures(r#"capacity reports "Fitness" as never scheduled"#)
                .unwrap(),
        )
        .await
        .unwrap();
    }
}
