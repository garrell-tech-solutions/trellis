//! Step handlers for `features/committed_field_domains.feature`: committed
//! triage validates the *values* of deadline, deadline type and priority,
//! not just their presence (`committed_triage_validation` covers presence).
//!
//! Kept as its own module rather than added to `triage::dispatch`, which is
//! already over the project's complexity threshold (see the triage-validation
//! handoff brief's "known repo gotchas"). The shared Background/rejection
//! steps this feature also uses (empty task list, capture waiting, the
//! triage is rejected, task list still empty, capture still waiting) are
//! already matched generically by [`super::triage::dispatch`], tried before
//! this module — nothing here duplicates them.

use super::triage::{task_row, then_rejection_reports_invalid, when_triaged};
use super::*;
use serde_json::json;

static WHEN_TRIAGED_WITH_DEADLINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a committed task with a deadline of "<(\w+)>"$"#)
        .unwrap()
});
static WHEN_TRIAGED_WITH_DEADLINE_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a committed task with a deadline type of "<(\w+)>"$"#)
        .unwrap()
});
static WHEN_TRIAGED_WITH_PRIORITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a committed task with a priority of "<(\w+)>"$"#)
        .unwrap()
});
static THEN_DEADLINE_IS_INSTANT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the resulting task's deadline is the instant "<(\w+)>" milliseconds since the epoch$"#,
    )
    .unwrap()
});
static THEN_REPORTS_INVALID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the rejection reports "(\w+)" as invalid$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = WHEN_TRIAGED_WITH_DEADLINE.captures(text) {
        return Some(dispatch_with_deadline(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_WITH_DEADLINE_TYPE.captures(text) {
        return Some(dispatch_with_deadline_type(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_WITH_PRIORITY.captures(text) {
        return Some(dispatch_with_priority(world, example, &caps).await);
    }
    if let Some(caps) = THEN_DEADLINE_IS_INSTANT.captures(text) {
        return Some(dispatch_deadline_is_instant(world, example, &caps).await);
    }
    if let Some(caps) = THEN_REPORTS_INVALID.captures(text) {
        return Some(then_rejection_reports_invalid(world, &caps[1]));
    }
    None
}

async fn dispatch_deadline_is_instant(
    world: &World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected_epoch_ms = example_value(example, &caps[1])?;
    then_deadline_is_instant(world, expected_epoch_ms).await
}

async fn then_deadline_is_instant(world: &World, expected_epoch_ms: &str) -> Result<(), String> {
    let expected: i64 = expected_epoch_ms
        .parse()
        .map_err(|e| format!("bad expected_epoch_ms {expected_epoch_ms:?}: {e}"))?;
    let row = task_row(world).await?;
    if row.deadline == Some(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected deadline {expected}ms since the epoch, got {:?}",
            row.deadline
        ))
    }
}

async fn dispatch_with_deadline(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let deadline = example_value(example, &caps[1])?;
    when_triaged(
        world,
        json!({
            "kind": "committed",
            "deadline": deadline,
            "deadline_type": "hard",
            "priority": "P1",
        }),
    )
    .await
}

async fn dispatch_with_deadline_type(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let deadline_type = example_value(example, &caps[1])?;
    when_triaged(
        world,
        json!({
            "kind": "committed",
            "deadline": "2026-08-20T17:00:00Z",
            "deadline_type": deadline_type,
            "priority": "P1",
        }),
    )
    .await
}

async fn dispatch_with_priority(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let priority = example_value(example, &caps[1])?;
    when_triaged(
        world,
        json!({
            "kind": "committed",
            "deadline": "2026-08-20T17:00:00Z",
            "deadline_type": "hard",
            "priority": priority,
        }),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::super::triage::given_capture_waiting;
    use super::*;

    #[tokio::test]
    async fn a_valid_deadline_is_stored_as_the_instant_it_names() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();
        when_triaged(
            &mut world,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P1",
            }),
        )
        .await
        .unwrap();

        then_deadline_is_instant(&world, "1787245200000")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_field_outside_its_domain_is_rejected_and_reports_itself_as_invalid() {
        for (field, value) in [("deadline_type", "squishy"), ("priority", "P9")] {
            let mut world = migrated_world().await;
            given_capture_waiting(&mut world, "buy milk").await.unwrap();
            let mut payload = json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P1",
            });
            payload[field] = json!(value);

            when_triaged(&mut world, payload).await.unwrap();

            then_rejection_reports_invalid(&mut world, field).unwrap();
        }
    }
}
