//! Step handlers for `features/unknown_kind_rejection.feature`: T16's
//! `422 {"unknown_kind": <submitted>}` behaviour, shipped since M1's triage
//! slice but never covered by Gherkin or a QA procedure until now.
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
use serde_json::json;

static WHEN_TRIAGED_AS_KIND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the capture is triaged as kind "<(\w+)>"$"#).unwrap());
static WHEN_TRIAGED_NO_KIND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the capture is triaged with no kind named$").unwrap());
static THEN_REPORTS_UNKNOWN_KIND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the rejection reports an unknown kind$").unwrap());

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
    if THEN_REPORTS_UNKNOWN_KIND.is_match(text) {
        return Some(then_rejection_reports_unknown_kind(world));
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

/// The rejection body always carries an `unknown_kind` key (whose value is
/// the submitted kind, or `null` when none was named) — checking it is
/// present is what the Gherkin step names; the more specific shape (that the
/// value echoes exactly what was submitted) is exercised at the unit level
/// in `http::triage`.
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
}
