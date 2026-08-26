//! Why a triage was refused, in the two vocabularies a refusal has to speak.
//!
//! [`rejected`] is the API's contract -- a `422` whose body names what was
//! wrong (`T-422-is-product-wide`). [`rejection_message`] is the same refusal
//! as one line of prose for the page's per-row error slot. They are kept side
//! by side because they must never disagree about *what* was refused, only
//! about how to say it.
//!
//! Nothing here touches a database, an extractor or a template, which is why
//! it is not in [`super::http`]: the rejection contract is a product-wide
//! promise, and burying it among `axum` handlers made it look like a detail
//! of one route.

use crate::inbox::CAPTURE_NOT_OPEN_MESSAGE;
use axum::http::StatusCode;
use axum::Json;
use scheduler_core::task::TriageRejection;
use serde_json::{json, Value};

/// Every way triage can be refused. `Core` wraps `scheduler_core`'s own
/// rejection — well-formedness, decidable without a database.
pub(super) enum Rejection {
    Core(TriageRejection),
    /// The capture named by the URL has already left the inbox -- by triage
    /// or by dismissal, either of which needs the database to know
    /// (`dismiss-capture-no-triage-after-dismissal-05`). Which of the two it
    /// was is not reported: the rejection is the same either way, "not open
    /// for business", and nothing downstream needs to tell them apart.
    CaptureNotOpen,
    /// A well-formed quota name collides with an existing quota once case,
    /// spaces and punctuation are folded away
    /// (`D-quotas-are-selected-not-typed`'s guard, moved from the retired
    /// define form to triage's own front door, #138). Never bypassable --
    /// filing into an existing quota is deferred, so the only way out is a
    /// different name.
    QuotaNameExists {
        existing_name: String,
        existing_minutes: i64,
    },
    /// A well-formed quota name merely resembles an existing one -- warned
    /// once, and created on confirmation
    /// (`quota-triage-validation-similar-name-warns-04`). The candidate
    /// name itself already decided `super::http`'s own quota-name check, so
    /// nothing further downstream needs to carry it.
    QuotaNameSimilar {
        existing_name: String,
        existing_minutes: i64,
    },
}

/// The JSON body for a `Rejection::Core` -- the well-formedness half, shared
/// between [`rejected`] and [`rejection_message`], split out so neither has
/// to match both the outer `Rejection` and the inner `TriageRejection` at
/// once.
fn core_rejection_body(core: &TriageRejection, kind_submitted: &Value) -> Value {
    match core {
        TriageRejection::MissingField(field) => json!({ "missing_field": field.name() }),
        TriageRejection::InvalidField(field) => json!({ "invalid_field": field.name() }),
        TriageRejection::UnknownKind => json!({ "unknown_kind": kind_submitted }),
    }
}

/// The prose half of a `Rejection::Core`, for the same reason as
/// [`core_rejection_body`].
fn core_rejection_message(core: &TriageRejection, kind_submitted: &Value) -> String {
    match core {
        TriageRejection::MissingField(field) => format!("{} is required", field.name()),
        TriageRejection::InvalidField(field) => format!("{} is invalid", field.name()),
        TriageRejection::UnknownKind => format!("unrecognised kind: {kind_submitted}"),
    }
}

/// The rejection contract: a client error whose body names what was wrong.
/// A rejection that does not say what was wrong is a failure even with the
/// right status code.
///
/// `kind_submitted` comes from the request rather than from the rejection:
/// the core sees `kind` only after `string_field` has turned a wrong-typed
/// value into `None`, so this module holds the only surviving copy of what
/// actually arrived. That is what lets `{"kind": 7}` echo back `7` instead
/// of `null` (T-unknown-kind-rejected: report what was submitted).
pub(super) fn rejected(rejection: &Rejection, kind_submitted: &Value) -> (StatusCode, Json<Value>) {
    let body = match rejection {
        Rejection::Core(core) => core_rejection_body(core, kind_submitted),
        Rejection::CaptureNotOpen => {
            json!({ "capture_not_open": CAPTURE_NOT_OPEN_MESSAGE })
        }
        Rejection::QuotaNameExists { .. } => {
            json!({ "quota_conflict": "exact", "message": rejection_message(rejection, kind_submitted) })
        }
        Rejection::QuotaNameSimilar { .. } => json!({
            "quota_conflict": "similar",
            "message": rejection_message(rejection, kind_submitted),
            "confirm_control": QUOTA_CONFIRM_CONTROL,
        }),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body))
}

/// A one-line summary of a rejection for the page's per-row error slot. Not
/// the API's rejection contract (that stays `rejected`'s job) — this is
/// prose for a human reading the form they just submitted.
pub(super) fn rejection_message(rejection: &Rejection, kind_submitted: &Value) -> String {
    match rejection {
        Rejection::Core(core) => core_rejection_message(core, kind_submitted),
        Rejection::CaptureNotOpen => CAPTURE_NOT_OPEN_MESSAGE.to_string(),
        Rejection::QuotaNameExists {
            existing_name,
            existing_minutes,
        } => format!(
            "\u{201c}{existing_name}\u{201d} already exists at {}. Log your time against that \
             one, or give this a different name.",
            crate::quota::hours_a_week(*existing_minutes)
        ),
        Rejection::QuotaNameSimilar {
            existing_name,
            existing_minutes,
            ..
        } => format!(
            "That reads a lot like \u{201c}{existing_name}\u{201d} ({}). Same thing?",
            crate::quota::hours_a_week(*existing_minutes)
        ),
    }
}

/// The "Create anyway" control a similar-name warning offers, on both
/// transports -- the JSON contract's own `confirm_control` and the page's
/// button label alike (`quota-triage-validation-similar-name-warns-04`).
const QUOTA_CONFIRM_CONTROL: &str = "Create anyway";

#[cfg(test)]
mod tests {
    use super::*;
    use scheduler_core::task::Field;

    #[test]
    fn a_missing_field_rejection_names_the_field() {
        let (status, Json(body)) = rejected(
            &Rejection::Core(TriageRejection::MissingField(Field::Priority)),
            &Value::Null,
        );
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "missing_field": "priority" }));
    }

    #[test]
    fn an_invalid_field_rejection_names_the_field() {
        let (status, Json(body)) = rejected(
            &Rejection::Core(TriageRejection::InvalidField(Field::Deadline)),
            &Value::Null,
        );
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "invalid_field": "deadline" }));
    }

    #[test]
    fn an_unknown_kind_rejection_echoes_the_value_it_is_given() {
        let (status, Json(body)) = rejected(
            &Rejection::Core(TriageRejection::UnknownKind),
            &json!({ "not": "a string" }),
        );
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "unknown_kind": { "not": "a string" } }));
    }

    #[test]
    fn an_invalid_field_rejection_message_names_the_field() {
        let message = rejection_message(
            &Rejection::Core(TriageRejection::InvalidField(Field::Deadline)),
            &Value::Null,
        );
        assert_eq!(message, "deadline is invalid");
    }

    #[test]
    fn an_unknown_kind_rejection_message_echoes_the_value_it_is_given() {
        let message = rejection_message(
            &Rejection::Core(TriageRejection::UnknownKind),
            &json!({ "not": "a string" }),
        );
        assert_eq!(message, r#"unrecognised kind: {"not":"a string"}"#);
    }
}
