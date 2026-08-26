//! Turning either transport into what the core decides on.
//!
//! `POST /captures/{id}/triage` accepts two shapes -- the JSON API's body and
//! the page's url-encoded form -- and the whole of the difference between
//! them ends here. Past this module a triage is a
//! [`scheduler_core::task::TriageFields`] plus the two facts neither the
//! transport nor the core adjudicates, and [`super::http`] cannot tell which
//! way it arrived except to choose a response shape.
//!
//! Separated from [`super::http`] because it is the only part of triage that
//! knows about `axum`'s extractors and `serde`: the decision, the writes and
//! the rejection contract are all reachable without it
//! (`T-module-boundary`).

use crate::platform::request::content_type_is_json;
use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::{Form, Json};
use scheduler_core::task::TriageFields;
use serde::Deserialize;
use serde_json::{json, Value};

fn string_field(payload: &Value, name: &str) -> Option<String> {
    payload
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Translates the JSON body into the core's triage input. This function is
/// the whole of the JSON transport's involvement: nothing it calls sees
/// `serde_json`.
fn triage_fields(payload: &Value) -> TriageFields {
    TriageFields {
        kind: string_field(payload, "kind"),
        deadline: string_field(payload, "deadline"),
        commitment: string_field(payload, "commitment"),
        priority: string_field(payload, "priority"),
        estimated_minutes: payload.get("estimated_minutes").and_then(Value::as_i64),
        quota_name: string_field(payload, "name"),
        quota_hours: string_field(payload, "hours"),
        // #110's date/time/zone trio is the page form's alternative to
        // `deadline`, never JSON's -- the API keeps sending one complete
        // instant, per `committed-date-json-still-takes-an-instant-05`.
        deadline_date: None,
        deadline_time: None,
        timezone: None,
    }
}

/// The page's triage controls submit as a plain HTML form: url-encoded,
/// every field optional, since which fields matter depends on `kind`. Mirrors
/// `scheduler_core::task::TriageFields` rather than being that type directly
/// — the core must not depend on `serde` (T-module-boundary).
///
/// `context_tag` rides along but is not part of `TriageFields`: it never
/// decides a kind or fails a submission (`context-tags-taggable-at-triage-08`
/// is a second chance to supply a fact the core does not adjudicate), so it
/// is extracted separately rather than smuggled into the type that exists to
/// answer "what kind of task is this".
#[derive(Deserialize, Default)]
pub struct TriageFormRequest {
    kind: Option<String>,
    deadline: Option<String>,
    /// `"2026-08-25"` -- the committed date-picker's value (#110). Absent
    /// for every other kind, and for a committed submission that still
    /// sends a complete `deadline` (an older client, or a test fixture).
    #[serde(default)]
    deadline_date: Option<String>,
    /// `"08:30"` -- the committed time-picker's value, present only for an
    /// *at*. A *by* never asks the page for one.
    #[serde(default)]
    deadline_time: Option<String>,
    commitment: Option<String>,
    priority: Option<String>,
    estimated_minutes: Option<i64>,
    /// A quota's name (#138) -- prefilled by the page with the capture's
    /// own words, but always required and editable.
    name: Option<String>,
    /// A quota's weekly hour target, as free text -- the canvas's own
    /// `step="0.5"` input.
    hours: Option<String>,
    #[serde(default)]
    context_tag: Option<String>,
    /// The candidate name a *similar*-name warning was already shown for,
    /// carried back by a "Create anyway" resubmission
    /// (`quota-triage-validation-similar-name-warns-04`) -- rides outside
    /// `TriageFields` for the same reason `context_tag` does: it decides no
    /// kind and fails no submission on its own, and the core has no
    /// database to check it against.
    #[serde(default)]
    confirmed: Option<String>,
}

impl From<&TriageFormRequest> for TriageFields {
    fn from(form: &TriageFormRequest) -> Self {
        TriageFields {
            kind: form.kind.clone(),
            deadline: form.deadline.clone(),
            deadline_date: form.deadline_date.clone(),
            deadline_time: form.deadline_time.clone(),
            // Set by `create_triage`, which has the database this
            // conversion does not (`T-core-owns-validation-order`: the
            // adapter keeps only what the core genuinely cannot have).
            timezone: None,
            commitment: form.commitment.clone(),
            priority: form.priority.clone(),
            estimated_minutes: form.estimated_minutes,
            quota_name: form.name.clone(),
            quota_hours: form.hours.clone(),
        }
    }
}

/// Either transport this endpoint accepts. JSON keeps the raw `Value`: a
/// wrong-typed `kind` (e.g. `7`) must still be echoed back by the rejection
/// (T-unknown-kind-rejected), which a strongly-typed `Option<String>` field
/// cannot represent — it would fail to deserialize at all. A form field has
/// no such case; every value a browser form submits is already a string.
pub enum TriageInput {
    Json(Value),
    Form(Box<TriageFormRequest>),
}

impl<S: Send + Sync> FromRequest<S> for TriageInput {
    type Rejection = StatusCode;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if content_type_is_json(&req) {
            let Json(payload) = Json::<Value>::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(Self::Json(payload))
        } else {
            let Form(form) = Form::<TriageFormRequest>::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(Self::Form(Box::new(form)))
        }
    }
}

/// Fields both transports carry alongside [`TriageFields`] without the core
/// deciding either: `context_tag` decides no kind and fails no submission
/// (`context-tags-taggable-at-triage-08`), and `confirmed` needs a database
/// the core does not have (#138). Bundled rather than a longer tuple, since
/// [`fields_from_input`] now has three things to return besides the fields
/// themselves.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SideFields {
    pub(super) context_tag: Option<String>,
    pub(super) confirmed: Option<String>,
}

/// The three things every branch of [`TriageInput`] must produce: the fields
/// the core decides on, the raw `kind` a rejection echoes back
/// (T-unknown-kind-rejected), and the side fields decided by neither
/// transport nor core.
pub(super) fn fields_from_input(input: TriageInput) -> (TriageFields, Value, SideFields) {
    match input {
        TriageInput::Json(payload) => {
            let kind_submitted = payload.get("kind").cloned().unwrap_or(Value::Null);
            let side = SideFields {
                context_tag: string_field(&payload, "context_tag"),
                confirmed: string_field(&payload, "confirmed"),
            };
            (triage_fields(&payload), kind_submitted, side)
        }
        TriageInput::Form(form) => {
            let kind_submitted = json!(form.kind);
            let side = SideFields {
                context_tag: form.context_tag.clone(),
                confirmed: form.confirmed.clone(),
            };
            (TriageFields::from(form.as_ref()), kind_submitted, side)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::{prop_assert_eq, proptest, test_runner::Config as ProptestConfig};

    #[test]
    fn triage_fields_reads_every_field_the_core_asks_for() {
        let payload = json!({
            "kind": "quota",
            "deadline": "2026-08-20T17:00:00Z",
            "commitment": "at",
            "priority": "P1",
            "estimated_minutes": 180,
            "name": "Piano",
            "hours": "4"
        });

        assert_eq!(
            triage_fields(&payload),
            TriageFields {
                kind: Some("quota".to_string()),
                deadline: Some("2026-08-20T17:00:00Z".to_string()),
                commitment: Some("at".to_string()),
                priority: Some("P1".to_string()),
                estimated_minutes: Some(180),
                quota_name: Some("Piano".to_string()),
                quota_hours: Some("4".to_string()),
                ..TriageFields::default()
            }
        );
    }

    #[test]
    fn triage_fields_treats_an_empty_body_as_nothing_submitted() {
        assert_eq!(triage_fields(&json!({})), TriageFields::default());
    }

    #[test]
    fn triage_fields_ignores_a_field_submitted_with_the_wrong_json_type() {
        let payload = json!({ "kind": 7, "name": 3 });
        assert_eq!(triage_fields(&payload), TriageFields::default());
    }
    /// A submission expressed as JSON and as a form, field for field.
    fn json_body(fields: &TriageFields) -> Value {
        let mut body = serde_json::Map::new();
        let mut put = |name: &str, value: Option<Value>| {
            if let Some(value) = value {
                body.insert(name.to_string(), value);
            }
        };
        put("kind", fields.kind.clone().map(Value::from));
        put("deadline", fields.deadline.clone().map(Value::from));
        put("commitment", fields.commitment.clone().map(Value::from));
        put("priority", fields.priority.clone().map(Value::from));
        put(
            "estimated_minutes",
            fields.estimated_minutes.map(Value::from),
        );
        put("name", fields.quota_name.clone().map(Value::from));
        put("hours", fields.quota_hours.clone().map(Value::from));
        Value::Object(body)
    }

    fn form_request(fields: &TriageFields) -> TriageFormRequest {
        TriageFormRequest {
            kind: fields.kind.clone(),
            deadline: fields.deadline.clone(),
            deadline_date: fields.deadline_date.clone(),
            deadline_time: fields.deadline_time.clone(),
            commitment: fields.commitment.clone(),
            priority: fields.priority.clone(),
            estimated_minutes: fields.estimated_minutes,
            name: fields.quota_name.clone(),
            hours: fields.quota_hours.clone(),
            context_tag: None,
            confirmed: None,
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

        /// "The page and `POST /captures/{id}/triage` are one code path, not
        /// two" is this module's stated design, and this is the seam where it
        /// could quietly stop being true: two hand-written, field-by-field
        /// translations into the same core input. Adding a field to
        /// `TriageFields` and wiring it into only one of them still compiles,
        /// still passes every example test that does not happen to use it,
        /// and silently makes the page and the API disagree. It fails here.
        #[test]
        #[ignore]
        fn both_transports_translate_one_submission_into_the_same_core_input(
            kind in proptest::option::of(".{0,12}"),
            deadline in proptest::option::of(".{0,30}"),
            commitment in proptest::option::of(".{0,10}"),
            priority in proptest::option::of(".{0,6}"),
            estimated_minutes in proptest::option::of(any::<i64>()),
            quota_name in proptest::option::of(".{0,20}"),
            quota_hours in proptest::option::of(".{0,10}"),
        ) {
            let submission = TriageFields {
                kind,
                deadline,
                commitment,
                priority,
                estimated_minutes,
                quota_name,
                quota_hours,
                // #110's deadline_date/deadline_time/timezone are the page
                // form's own fields, with no JSON equivalent to round-trip
                // against -- left at their default `None` here and covered
                // by their own example tests instead.
                ..TriageFields::default()
            };

            let from_json = triage_fields(&json_body(&submission));
            let from_form: TriageFields = TriageFields::from(&form_request(&submission));

            prop_assert_eq!(&from_json, &submission);
            prop_assert_eq!(from_json, from_form);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

        /// The context-tag counterpart of the property above: not part of
        /// `TriageFields` (it decides no kind and fails no submission), but
        /// still a field both transports must agree on
        /// (`context-tags-taggable-at-triage-08`).
        #[test]
        #[ignore]
        fn both_transports_extract_the_same_context_tag(
            context_tag in proptest::option::of(".{0,20}"),
        ) {
            let mut json_payload = json_body(&TriageFields::default());
            if let Some(tag) = &context_tag {
                json_payload["context_tag"] = Value::from(tag.clone());
            }
            let (_, _, from_json) = fields_from_input(TriageInput::Json(json_payload));

            let mut form = form_request(&TriageFields::default());
            form.context_tag = context_tag.clone();
            let (_, _, from_form) = fields_from_input(TriageInput::Form(Box::new(form)));

            prop_assert_eq!(from_json.context_tag, context_tag.clone());
            prop_assert_eq!(from_form.context_tag, context_tag);
        }
    }
}
