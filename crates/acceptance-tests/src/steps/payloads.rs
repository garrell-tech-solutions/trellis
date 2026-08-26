//! The canonical well-formed triage payload for each task kind, and the two
//! perturbations the validation features are built from.
//!
//! Every "…is rejected because field X is wrong" scenario needs a submission
//! that is valid in every respect *except* X. Written inline per scenario,
//! that makes the definition of a valid committed submission a literal
//! repeated across five step modules — so closing one more field's domain
//! means finding and editing every copy, and each scenario reads as a wall of
//! fields rather than as the one thing it varies. These builders give that
//! definition a single home, and let a step say what it changes.

use serde_json::{json, Value};

/// A deadline that parses (T-jiff-epoch-millis) and is inside every domain
/// the core checks.
pub const VALID_DEADLINE: &str = "2026-08-20T17:00:00Z";

/// A life area every fresh, migrated database seeds -- always available, so
/// a scenario that is not itself about life areas can name it without first
/// adding one.
pub const VALID_LIFE_AREA: &str = "Work";

/// A committed estimate inside the domain (#62: required, and must be
/// positive).
pub const VALID_ESTIMATED_MINUTES: i64 = 180;

pub fn pool() -> Value {
    json!({ "kind": "pool", "life_area": VALID_LIFE_AREA })
}

pub fn committed() -> Value {
    json!({
        "kind": "committed",
        "deadline": VALID_DEADLINE,
        "commitment": "at",
        "priority": "P1",
        "estimated_minutes": VALID_ESTIMATED_MINUTES,
        "life_area": VALID_LIFE_AREA,
    })
}

/// A quota name every canonical fixture uses (#138: triaging as quota is
/// what creates it).
pub const VALID_QUOTA_NAME: &str = "Piano";

/// A weekly hour target inside the domain -- a positive number of hours,
/// as free text (`scheduler_core::quota::QuotaDefinition::from_fields`
/// reads it the same way the retired quota-screen define form did).
pub const VALID_QUOTA_HOURS: &str = "4";

pub fn quota() -> Value {
    json!({
        "kind": "quota",
        "name": VALID_QUOTA_NAME,
        "hours": VALID_QUOTA_HOURS,
        "life_area": VALID_LIFE_AREA,
    })
}

fn object_mut(payload: &mut Value) -> &mut serde_json::Map<String, Value> {
    payload
        .as_object_mut()
        .expect("a triage payload is a JSON object")
}

/// The payload with `field` set to `value`.
pub fn with_field(mut payload: Value, field: &str, value: Value) -> Value {
    object_mut(&mut payload).insert(field.to_string(), value);
    payload
}

/// The payload with `field` absent entirely.
pub fn without_field(mut payload: Value, field: &str) -> Value {
    object_mut(&mut payload).remove(field);
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_canonical_payload_names_its_kind() {
        for (payload, kind) in [
            (pool(), "pool"),
            (committed(), "committed"),
            (quota(), "quota"),
        ] {
            assert_eq!(payload["kind"], json!(kind));
        }
    }

    #[test]
    fn with_field_replaces_a_present_field() {
        let payload = with_field(committed(), "priority", json!("P4"));
        assert_eq!(payload["priority"], json!("P4"));
    }

    #[test]
    fn with_field_adds_a_field_the_payload_did_not_have() {
        let payload = with_field(pool(), "deadline", json!(VALID_DEADLINE));
        assert_eq!(payload["deadline"], json!(VALID_DEADLINE));
    }

    #[test]
    fn without_field_removes_the_named_field_and_leaves_the_rest() {
        let payload = without_field(committed(), "deadline");
        assert_eq!(payload.get("deadline"), None);
        assert_eq!(payload["commitment"], json!("at"));
        assert_eq!(payload["priority"], json!("P1"));
    }

    #[test]
    fn without_field_is_a_no_op_for_a_field_that_was_never_there() {
        assert_eq!(without_field(pool(), "deadline"), pool());
    }
}
