//! Step handlers for `features/unknown_kind_rejection.feature`:
//! T-unknown-kind-rejected's `422 {"unknown_kind": <submitted>}` behaviour,
//! shipped since M1's triage slice but never covered by Gherkin or a QA
//! procedure until now.
//!
//! Kept as its own module rather than added to `triage::dispatch`, which is
//! already over the project's complexity threshold (see the triage-validation
//! handoff brief's "known repo gotchas"). The shared Background/rejection
//! steps this feature also uses (empty task list, capture waiting, the
//! triage is rejected, task list still empty, capture still waiting) are
//! already matched generically by [`super::triage::dispatch`], tried before
//! this module — nothing here duplicates them.

use super::triage::when_triaged;
use super::*;
use serde_json::{json, Value};

static WHEN_TRIAGED_AS_KIND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the capture is triaged as kind "<(\w+)>"$"#).unwrap());
static WHEN_TRIAGED_NO_KIND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the capture is triaged with no kind named$").unwrap());
static WHEN_TRIAGED_KIND_AS_NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the capture is triaged with kind submitted as the number (\d+)$").unwrap()
});
static THEN_REPORTS_UNKNOWN_KIND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the rejection reports an unknown kind$").unwrap());
static THEN_REPORTS_UNKNOWN_KIND_AS_NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the rejection reports the unknown kind as the number (\d+)$").unwrap()
});

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = WHEN_TRIAGED_AS_KIND.captures(text) {
        return Some(dispatch_triaged_as_kind(world, example, &caps).await);
    }
    if WHEN_TRIAGED_NO_KIND.is_match(text) {
        return Some(when_triaged(world, json!({})).await);
    }
    if let Some(caps) = WHEN_TRIAGED_KIND_AS_NUMBER.captures(text) {
        return Some(dispatch_triaged_kind_as_number(world, &caps).await);
    }
    if THEN_REPORTS_UNKNOWN_KIND.is_match(text) {
        return Some(then_rejection_reports_unknown_kind(world));
    }
    if let Some(caps) = THEN_REPORTS_UNKNOWN_KIND_AS_NUMBER.captures(text) {
        return Some(then_rejection_reports_unknown_kind_as_number(world, &caps));
    }
    None
}

async fn dispatch_triaged_as_kind(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let kind = example_value(example, &caps[1])?;
    when_triaged(world, json!({ "kind": kind })).await
}

async fn dispatch_triaged_kind_as_number(
    world: &mut World,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let number: i64 = caps[1]
        .parse()
        .map_err(|e| format!("bad kind number {:?}: {e}", &caps[1]))?;
    when_triaged(world, json!({ "kind": number })).await
}

/// The rejection body always carries an `unknown_kind` key (whose value is
/// the submitted kind, or `null` when none was named) — checking it is
/// present is what the Gherkin step names; the more specific shape (that the
/// value echoes exactly what was submitted) is exercised at the unit level
/// in `triage::http`.
fn then_rejection_reports_unknown_kind(world: &mut World) -> Result<(), String> {
    let body = world
        .last_response_body
        .as_ref()
        .ok_or_else(|| "no rejection body recorded".to_string())?;
    if body.get("unknown_kind").is_some() {
        Ok(())
    } else {
        Err(format!(
            "expected an unknown_kind key in the rejection body, got {body:?}"
        ))
    }
}

/// Folded in from the PR #31 review: a wrong-typed `kind` (e.g. the number
/// `7`) must be echoed back exactly, not dropped to `null`
/// (T-unknown-kind-rejected).
fn then_rejection_reports_unknown_kind_as_number(
    world: &mut World,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected: i64 = caps[1]
        .parse()
        .map_err(|e| format!("bad expected kind number {:?}: {e}", &caps[1]))?;
    then_unknown_kind_is(world, expected)
}

fn then_unknown_kind_is(world: &mut World, expected: i64) -> Result<(), String> {
    let body = world
        .last_response_body
        .as_ref()
        .ok_or_else(|| "no rejection body recorded".to_string())?;
    match body.get("unknown_kind").and_then(Value::as_i64) {
        Some(actual) if actual == expected => Ok(()),
        other => Err(format!(
            "expected the rejection to report unknown_kind {expected}, got {other:?}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::super::triage::given_capture_waiting;
    use super::*;

    #[tokio::test]
    async fn a_kind_outside_the_three_known_kinds_is_rejected_and_reports_it_was_unknown() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();

        when_triaged(&mut world, json!({ "kind": "someday" }))
            .await
            .unwrap();

        then_rejection_reports_unknown_kind(&mut world).unwrap();
    }

    #[tokio::test]
    async fn a_triage_with_no_kind_named_is_rejected_and_reports_it_was_unknown() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();

        when_triaged(&mut world, json!({})).await.unwrap();

        then_rejection_reports_unknown_kind(&mut world).unwrap();
    }

    #[tokio::test]
    async fn a_kind_submitted_as_a_number_is_rejected_and_echoed_back_exactly() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();

        when_triaged(&mut world, json!({ "kind": 7 }))
            .await
            .unwrap();

        then_unknown_kind_is(&mut world, 7).unwrap();
    }

    #[test]
    fn then_unknown_kind_is_errors_when_the_reported_number_differs() {
        let mut world = World::new();
        world.last_response_body = Some(json!({ "unknown_kind": 7 }));
        assert!(then_unknown_kind_is(&mut world, 8).is_err());
    }
}
