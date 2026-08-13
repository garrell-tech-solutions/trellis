//! Step handlers for `features/quota_triage_validation.feature`: quota
//! triage requires `target_count`, `target_minutes_each` and `period`
//! (T-quota-targets-required), and `period` is closed to `week | month`
//! (T-period-closed-set).
//!
//! Kept as its own module rather than added to `triage::dispatch`, which is
//! already over the project's complexity threshold (see the triage-validation
//! handoff brief's "known repo gotchas"). The shared Background/rejection
//! steps this feature also uses (empty task list, capture waiting, the
//! triage is rejected, the rejection names "<field>", task list still empty,
//! capture still waiting) are already matched generically by
//! [`super::triage::dispatch`], tried before this module — nothing here
//! duplicates them.

use super::payloads;
use super::triage::{then_rejection_reports_invalid, when_triaged};
use super::*;
use serde_json::json;

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
        return Some(when_triaged_with_period(world, json!("")).await);
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

async fn when_triaged_with_period(
    world: &mut World,
    period: serde_json::Value,
) -> Result<(), String> {
    when_triaged(
        world,
        payloads::with_field(payloads::quota(), "period", period),
    )
    .await
}

async fn dispatch_missing(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing_field = example_value(example, &caps[1])?;
    when_triaged(
        world,
        payloads::without_field(payloads::quota(), missing_field),
    )
    .await
}

async fn dispatch_with_period(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let period = example_value(example, &caps[1])?;
    when_triaged_with_period(world, json!(period)).await
}

#[cfg(test)]
mod tests {
    use super::super::triage::{given_capture_waiting, then_rejection_names};
    use super::*;

    #[tokio::test]
    async fn a_quota_triage_missing_a_required_field_is_rejected() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "go to the gym")
            .await
            .unwrap();
        let payload = payloads::without_field(payloads::quota(), "target_count");

        when_triaged(&mut world, payload).await.unwrap();

        then_rejection_names(&mut world, "target_count").unwrap();
    }

    #[tokio::test]
    async fn a_quota_triage_with_period_left_empty_is_rejected_the_same_as_absent() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "go to the gym")
            .await
            .unwrap();

        when_triaged_with_period(&mut world, json!(""))
            .await
            .unwrap();

        then_rejection_names(&mut world, "period").unwrap();
    }

    #[tokio::test]
    async fn a_quota_triage_with_an_invalid_period_is_rejected_and_reports_itself_as_invalid() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "go to the gym")
            .await
            .unwrap();

        when_triaged_with_period(&mut world, json!("fortnight"))
            .await
            .unwrap();

        then_rejection_reports_invalid(&mut world, "period").unwrap();
    }
}
