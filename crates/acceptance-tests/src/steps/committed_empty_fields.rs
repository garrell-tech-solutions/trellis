//! Step handler for `features/committed_triage_validation.feature`'s
//! "left empty" scenario (T18: an empty required field is rejected the same
//! way as an absent one).
//!
//! Kept as its own module rather than added to `triage::dispatch`, which is
//! already over the project's complexity threshold (see the triage-validation
//! handoff brief's "known repo gotchas"). The shared Background/rejection
//! steps this scenario also uses (empty task list, capture waiting, the
//! triage is rejected, the rejection names "<field>", task list still empty,
//! capture still waiting) are already matched generically by
//! [`super::triage::dispatch`], tried before this module — nothing here
//! duplicates them.

use super::payloads;
use super::triage::when_triaged;
use super::*;
use serde_json::json;

static WHEN_TRIAGED_EMPTY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a committed task with "<(\w+)>" left empty$"#).unwrap()
});

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    let caps = WHEN_TRIAGED_EMPTY.captures(text)?;
    Some(dispatch_triaged_empty(world, example, &caps).await)
}

async fn dispatch_triaged_empty(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let empty_field = example_value(example, &caps[1])?;
    let payload = payloads::with_field(payloads::committed(), empty_field, json!(""));
    when_triaged(world, payload).await
}

#[cfg(test)]
mod tests {
    use super::super::triage::{given_capture_waiting, then_rejection_names};
    use super::*;

    #[tokio::test]
    async fn a_committed_triage_with_priority_left_empty_is_rejected_the_same_as_absent() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "call the dentist")
            .await
            .unwrap();
        let payload = payloads::with_field(payloads::committed(), "priority", json!(""));

        when_triaged(&mut world, payload).await.unwrap();

        then_rejection_names(&mut world, "priority").unwrap();
    }
}
