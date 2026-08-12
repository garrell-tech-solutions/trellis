//! Step handlers for `features/quota_triage_validation.feature`: quota
//! triage requires `target_count`, `target_minutes_each` and `period` (T17),
//! and `period` is closed to `week | month` (T19).
//!
//! Kept as its own module rather than added to `triage::dispatch`, which is
//! already over the project's complexity threshold (see the triage-validation
//! handoff brief's "known repo gotchas"). The shared Background/rejection
//! steps this feature also uses (empty task list, capture waiting, the
//! triage is rejected, the rejection names "<field>", task list still empty,
//! capture still waiting) are already matched generically by
//! [`super::triage::dispatch`], tried before this module — nothing here
//! duplicates them.

use super::triage::when_triaged;
use super::*;
use serde_json::{json, Value};

static WHEN_TRIAGED_MISSING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a quota task with "<(\w+)>" omitted$"#).unwrap()
});
static WHEN_TRIAGED_PERIOD_EMPTY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the capture is triaged as a quota task with period left empty$").unwrap()
});
static WHEN_TRIAGED_WITH_PERIOD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a quota task with a period of "<(\w+)>"$"#).unwrap()
});
/// The "period left empty" scenario has no Examples table, so the field name
/// is written directly in the step text rather than as a `<placeholder>`.
static THEN_REJECTION_NAMES_LITERAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the rejection names "(\w+)"$"#).unwrap());
static THEN_REPORTS_INVALID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the rejection reports "(\w+)" as invalid$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = WHEN_TRIAGED_MISSING.captures(text) {
        return Some(dispatch_missing(world, example, &caps).await);
    }
    if WHEN_TRIAGED_PERIOD_EMPTY.is_match(text) {
        return Some(when_triaged_period_empty(world).await);
    }
    if let Some(caps) = WHEN_TRIAGED_WITH_PERIOD.captures(text) {
        return Some(dispatch_with_period(world, example, &caps).await);
    }
    if let Some(caps) = THEN_REJECTION_NAMES_LITERAL.captures(text) {
        return Some(super::triage::then_rejection_names(world, &caps[1]));
    }
    if let Some(caps) = THEN_REPORTS_INVALID.captures(text) {
        return Some(then_rejection_reports_invalid(world, &caps[1]));
    }
    None
}

fn complete_quota_payload() -> serde_json::Value {
    json!({
        "kind": "quota",
        "target_count": 3,
        "target_minutes_each": 45,
        "period": "week",
    })
}

async fn dispatch_missing(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing_field = example_value(example, &caps[1])?;
    let mut payload = complete_quota_payload();
    payload
        .as_object_mut()
        .expect("quota payload is an object")
        .remove(missing_field);
    when_triaged(world, payload).await
}

async fn when_triaged_period_empty(world: &mut World) -> Result<(), String> {
    let mut payload = complete_quota_payload();
    payload["period"] = json!("");
    when_triaged(world, payload).await
}

async fn dispatch_with_period(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let period = example_value(example, &caps[1])?;
    let mut payload = complete_quota_payload();
    payload["period"] = json!(period);
    when_triaged(world, payload).await
}

fn then_rejection_reports_invalid(world: &mut World, expected_field: &str) -> Result<(), String> {
    let body = world
        .last_response_body
        .as_ref()
        .ok_or_else(|| "no rejection body recorded".to_string())?;
    match body.get("invalid_field").and_then(Value::as_str) {
        Some(field) if field == expected_field => Ok(()),
        other => Err(format!(
            "expected rejection to report {expected_field} as invalid, body reported {other:?}"
        )),
    }
}
