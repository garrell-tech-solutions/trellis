//! Step handlers for `features/quota_triage_validation.feature`: triaging a
//! capture as a quota requires a name and an hour target (#138), and a
//! name that collides with an existing quota is refused or warned the same
//! way the retired quota-screen define form's own guard did.
//!
//! Kept as its own module rather than added to `triage::dispatch`, which is
//! already over the project's complexity threshold. The shared
//! Background/rejection steps this feature also uses (empty task list,
//! capture waiting, the triage is rejected, the rejection names "<field>",
//! quota screen steps, and the fully-placeholder "triaged as a quota named
//! "<name>" with a target of "<hours>" hours a week" wording) are already
//! matched generically by [`super::triage::dispatch`] and
//! [`super::quota_screen::dispatch`], tried before this module — nothing
//! here duplicates them.

use super::payloads;
use super::triage::{then_rejection_reports_invalid, when_triaged};
use super::*;
use serde_json::{json, Value};

static WHEN_TRIAGED_MISSING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a quota with "<(\w+)>" <(\w+)>$"#).unwrap()
});
static WHEN_TRIAGED_NAMED_AGAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the capture is triaged as a quota named "([^"]+)" with a target of "([^"]+)" hours a week again$"#,
    )
    .unwrap()
});
static WHEN_TRIAGED_NAMED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the capture is triaged as a quota named "([^"]+)" with a target of "([^"]+)" hours a week$"#,
    )
    .unwrap()
});
static THEN_REPORTS_INVALID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the rejection reports "(\w+)" as invalid$"#).unwrap());
static THEN_WARNS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the triage warns "([^"]+)"$"#).unwrap());
static THEN_OFFERS_CREATE_CONTROL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the triage offers a create control named "([^"]+)"$"#).unwrap()
});

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = WHEN_TRIAGED_MISSING.captures(text) {
        return Some(dispatch_missing(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_NAMED_AGAIN.captures(text) {
        return Some(dispatch_named(world, example, &caps, true).await);
    }
    if let Some(caps) = WHEN_TRIAGED_NAMED.captures(text) {
        return Some(dispatch_named(world, example, &caps, false).await);
    }
    if let Some(caps) = THEN_REPORTS_INVALID.captures(text) {
        return Some(then_rejection_reports_invalid(world, &caps[1]));
    }
    if let Some(caps) = THEN_WARNS.captures(text) {
        return Some(then_warns(world, &caps[1]));
    }
    if let Some(caps) = THEN_OFFERS_CREATE_CONTROL.captures(text) {
        return Some(dispatch_offers_create_control(world, example, &caps));
    }
    None
}

/// See `quota_screen.rs`'s own copy of this function for the full
/// reasoning; duplicated rather than shared per this project's established
/// convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// `"<field>" <absence>`: two placeholders, the field that goes missing and
/// how -- `omitted` or `left empty`. Both cells are `<name>`-shaped
/// placeholders in the step text itself (this project's own convention),
/// resolved against the current example row rather than substituted ahead
/// of time.
async fn dispatch_missing(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let field = example_value(example, &caps[1])?;
    let absence = example_value(example, &caps[2])?;
    let payload = match absence {
        "omitted" => payloads::without_field(payloads::quota(), field),
        "left empty" => payloads::with_field(payloads::quota(), field, json!("")),
        other => return Err(format!("unrecognised absence {other:?}")),
    };
    when_triaged(world, payload).await
}

/// A name and hour target, either of which may be a literal
/// (`quota_triage_validation.feature`'s own scenarios mix a fixed `"Gym"`
/// or `"2"` with a varying placeholder) or a `<name>` placeholder resolved
/// against the example row. `confirmed_resubmission` carries the `again`
/// variant's own `confirmed` field, the "Create anyway" tap's resubmission.
async fn dispatch_named(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
    confirmed_resubmission: bool,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let hours = resolve(example, &caps[2])?;
    let mut payload = json!({ "kind": "quota", "name": name, "hours": hours });
    if confirmed_resubmission {
        payload["confirmed"] = json!(name);
    }
    when_triaged(world, payload).await
}

fn then_warns(world: &mut World, expected: &str) -> Result<(), String> {
    let body = world
        .last_response_body
        .as_ref()
        .ok_or_else(|| "no rejection body recorded".to_string())?;
    match body.get("message").and_then(Value::as_str) {
        Some(message) if message == expected => Ok(()),
        other => Err(format!(
            "expected the triage to warn {expected:?}, body reported {other:?}"
        )),
    }
}

fn dispatch_offers_create_control(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    then_offers_create_control(world, &expected)
}

fn then_offers_create_control(world: &mut World, expected: &str) -> Result<(), String> {
    let body = world
        .last_response_body
        .as_ref()
        .ok_or_else(|| "no rejection body recorded".to_string())?;
    match body.get("confirm_control").and_then(Value::as_str) {
        Some(control) if control == expected => Ok(()),
        other => Err(format!(
            "expected a create control named {expected:?}, body reported {other:?}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::super::triage::{given_capture_waiting, then_rejection_names};
    use super::*;

    #[tokio::test]
    async fn a_quota_triage_missing_a_required_name_is_rejected() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "go to the gym")
            .await
            .unwrap();
        let payload = payloads::without_field(payloads::quota(), "name");

        when_triaged(&mut world, payload).await.unwrap();

        then_rejection_names(&mut world, "name").unwrap();
    }

    #[tokio::test]
    async fn a_quota_triage_with_hours_left_empty_is_rejected_the_same_as_absent() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "go to the gym")
            .await
            .unwrap();
        let payload = payloads::with_field(payloads::quota(), "hours", json!(""));

        when_triaged(&mut world, payload).await.unwrap();

        then_rejection_names(&mut world, "hours").unwrap();
    }

    #[tokio::test]
    async fn a_quota_triage_with_unparseable_hours_is_rejected_and_reports_itself_as_invalid() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "go to the gym")
            .await
            .unwrap();
        let payload = payloads::with_field(payloads::quota(), "hours", json!("four"));

        when_triaged(&mut world, payload).await.unwrap();

        then_rejection_reports_invalid(&mut world, "hours").unwrap();
    }

    #[test]
    fn resolve_returns_a_literal_value_unchanged() {
        let example = BTreeMap::new();
        assert_eq!(resolve(&example, "Gym"), Ok("Gym".to_string()));
    }

    #[test]
    fn resolve_looks_up_a_placeholder_in_the_example_row() {
        let example = super::super::example(&[("hours", "2")]);
        assert_eq!(resolve(&example, "<hours>"), Ok("2".to_string()));
    }

    #[tokio::test]
    async fn then_warns_errors_when_the_message_does_not_match() {
        let mut world = World::new();
        world.last_response_body = Some(json!({ "message": "actual" }));
        assert!(then_warns(&mut world, "expected").is_err());
    }

    #[tokio::test]
    async fn then_offers_create_control_passes_when_the_label_matches() {
        let mut world = World::new();
        world.last_response_body = Some(json!({ "confirm_control": "Create anyway" }));
        assert_eq!(
            then_offers_create_control(&mut world, "Create anyway"),
            Ok(())
        );
    }
}
