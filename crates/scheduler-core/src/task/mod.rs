//! The task model: what a task is, and what each kind of task requires.
//!
//! This is high-level policy and is deliberately free of HTTP, JSON, SQL and
//! async. A triage decision can be made — and tested — with nothing but this
//! module, which is the property T-core-no-tokio's "no tokio" rule exists to
//! protect.
//!
//! [`fields`] holds the closed-domain vocabulary (which field a rejection
//! names, and the fixed-set columns' parsing); this module composes that
//! vocabulary into the triage decision itself.

mod fields;

pub use fields::{Commitment, DeadlineType, Field, Priority};

use crate::quota::Field as QuotaField;
use crate::quota::{DefinitionRejection, QuotaDefinition, WeeklyTarget};

/// The stored discriminant for each kind. These three strings are the durable
/// contract shared by the `tasks.kind` column and every delivery mechanism.
pub const POOL: &str = "pool";
pub const COMMITTED: &str = "committed";
pub const QUOTA: &str = "quota";

/// The triage fields as they arrive from a delivery mechanism, before the core
/// has decided whether they describe a task at all.
///
/// This is the core's own input port. An adapter translates its transport
/// format — JSON today, a form post or a Telegram message later — into this
/// shape, so the dependency points inward and no adapter carries triage policy
/// of its own.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TriageFields {
    pub kind: Option<String>,
    pub deadline: Option<String>,
    /// A local civil date (`"2026-08-25"`), the page-form alternative to
    /// [`Self::deadline`]'s pre-resolved instant (#110). Ignored when
    /// `deadline` is present -- JSON keeps sending a complete instant and
    /// this trio is the page's only.
    pub deadline_date: Option<String>,
    /// A local civil time (`"08:30"`) paired with [`Self::deadline_date`]
    /// for an *at*. Absent for a *by*, which the page never asks a time
    /// for -- `commitment` decides which fields the page sent, never the
    /// other way around (`T-commitment-is-chosen-not-derived`).
    pub deadline_time: Option<String>,
    /// The zone [`Self::deadline_date`] is local to -- the one piece of
    /// this conversion an adapter must supply, because reading the
    /// owner's stored timezone needs a database the core does not have
    /// (`T-core-owns-validation-order`: the adapter keeps only what the
    /// core genuinely cannot). Ignored when `deadline` is present.
    pub timezone: Option<String>,
    pub commitment: Option<String>,
    pub priority: Option<String>,
    /// A quota's name (#138: triaging a capture as a quota is what creates
    /// it). Prefilled with the capture's own words by the page, but always
    /// required and editable -- renaming a quota is #148 and is not built,
    /// so whatever name arrives here is the name forever.
    pub quota_name: Option<String>,
    /// A quota's weekly hour target, as free text -- `"4"`, `"0.5"` -- the
    /// same shape the retired quota-screen define form read
    /// ([`crate::quota::QuotaDefinition::from_fields`] parses it, reused
    /// rather than restated here).
    pub quota_hours: Option<String>,
    /// Required for a committed task only (#62's capacity number cannot
    /// count what committed work asks for otherwise). Pool is never placed
    /// (`D-no-pool-on-calendar`) and a quota is bounded by its own weekly
    /// target, so neither needs this field.
    pub estimated_minutes: Option<i64>,
}

/// Why a set of triage fields does not describe a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriageRejection {
    /// A required field was absent, or submitted as an empty string
    /// (T-empty-equals-absent: the two report identically — a client rarely
    /// intends the distinction).
    MissingField(Field),
    /// A field was present but its value is outside the field's domain.
    InvalidField(Field),
    /// `kind` was absent, or named something that is not one of the three.
    ///
    /// Carries nothing. It once carried the submitted `kind`, but no adapter
    /// could use it: the delivery module already holds what arrived, in the
    /// transport's own types, and that is strictly the better copy — a JSON
    /// `{"kind": 7}` reaches the core as `None`, because reading it as a
    /// string is what turned it into one. Reporting what was submitted is
    /// the adapter's job precisely because the adapter is the only place it
    /// still exists.
    UnknownKind,
}

/// T-three-task-kinds: task kind is a three-variant sum type. Each variant
/// carries exactly the fields that kind means, so "a pool task has no
/// deadline" and "a committed task has no quota target" are facts about the
/// type rather than assertions about one caller's payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {
    Pool,
    Committed {
        deadline: i64,
        commitment: Commitment,
        priority: Priority,
        /// Required at triage (#62): capacity cannot report what a
        /// committed task asks for if it carries no minutes. A stored task
        /// predating this column reads back as `None` on
        /// [`TaskAttributes`] -- this field itself is never optional,
        /// because it is only ever built from a fresh, validated
        /// submission.
        estimated_minutes: i64,
    },
    /// #138: triaging a capture as a quota is what *creates* the quota --
    /// the quota screen only displays what triage produced. `name` and
    /// `weekly_target` are required at triage (T-quota-targets-required's
    /// successor: a quota with no target can never be reckoned against),
    /// even though nothing about them is stored on the `tasks` row itself
    /// (`T-three-task-kinds`'s nullable columns are a storage fact, not a
    /// triage-time permission) -- they land in `quotas` instead, which is
    /// the delivery layer's concern, not this type's.
    Quota {
        name: String,
        weekly_target: WeeklyTarget,
    },
}

/// A task kind flattened into the nullable, row-shaped projection a
/// row-oriented consumer needs. At most one attribute group is ever
/// populated — that exclusivity is the invariant [`TaskKind`] exists to
/// guarantee, and expressing the projection here keeps it provable without
/// a database.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TaskAttributes {
    pub kind: &'static str,
    pub deadline: Option<i64>,
    pub commitment: Option<&'static str>,
    pub priority: Option<&'static str>,
    pub estimated_minutes: Option<i64>,
}

/// A committed deadline's required-ness is decided before its source is,
/// so this carries either shape [`TaskKind::require_deadline_input`] found
/// through to [`DeadlineInput::resolve_ms`] -- the one place that turns
/// either into an instant, and the one place [`TriageRejection::InvalidField`]
/// for a bad deadline can come from.
enum DeadlineInput {
    /// JSON's own shape: an already-complete instant.
    Instant(String),
    /// The page's shape: a local date, an optional local time (absent for
    /// a *by*), and the zone they are local to.
    LocalDate {
        date: String,
        time: Option<String>,
        zone: String,
    },
}

impl DeadlineInput {
    fn resolve_ms(self) -> Result<i64, TriageRejection> {
        match self {
            Self::Instant(text) => fields::parse_deadline_ms(&text)
                .ok_or(TriageRejection::InvalidField(Field::Deadline)),
            Self::LocalDate { date, time, zone } => {
                fields::local_deadline_ms(&date, time.as_deref(), &zone)
                    .map_err(|_| TriageRejection::InvalidField(Field::Deadline))
            }
        }
    }
}

/// Requires a string field to be both present and non-empty
/// (T-empty-equals-absent: absent and empty are the same submitter mistake,
/// so they report identically).
fn require(field: Field, value: &Option<String>) -> Result<String, TriageRejection> {
    match value {
        Some(v) if !v.is_empty() => Ok(v.clone()),
        _ => Err(TriageRejection::MissingField(field)),
    }
}

fn require_i64(field: Field, value: Option<i64>) -> Result<i64, TriageRejection> {
    value.ok_or(TriageRejection::MissingField(field))
}

/// An estimate of zero or fewer minutes defeats capacity math before a task
/// can ever be created.
fn require_positive(field: Field, value: i64) -> Result<i64, TriageRejection> {
    if value > 0 {
        Ok(value)
    } else {
        Err(TriageRejection::InvalidField(field))
    }
}
impl TaskKind {
    /// Decides which kind of task, if any, a set of triage fields describes.
    pub fn from_fields(fields: &TriageFields) -> Result<Self, TriageRejection> {
        match fields.kind.as_deref() {
            Some(POOL) => Ok(Self::Pool),
            Some(COMMITTED) => Self::committed_from(fields),
            Some(QUOTA) => Self::quota_from(fields),
            _ => Err(TriageRejection::UnknownKind),
        }
    }

    /// Required fields are checked before values, and in a fixed order, so a
    /// submission with several problems names the same one every time.
    fn committed_from(fields: &TriageFields) -> Result<Self, TriageRejection> {
        let (deadline, commitment, priority, estimated_minutes) =
            Self::require_committed_fields(fields)?;
        Self::parse_committed_fields(deadline, commitment, priority, estimated_minutes)
    }

    fn require_committed_fields(
        fields: &TriageFields,
    ) -> Result<(DeadlineInput, String, String, i64), TriageRejection> {
        let deadline = Self::require_deadline_input(fields)?;
        let commitment = require(Field::Commitment, &fields.commitment)?;
        let priority = require(Field::Priority, &fields.priority)?;
        let estimated_minutes = require_i64(Field::EstimatedMinutes, fields.estimated_minutes)?;
        Ok((deadline, commitment, priority, estimated_minutes))
    }

    /// `deadline` (JSON's pre-resolved instant) wins when present, exactly
    /// as before #110; a page submission carries `deadline_date` instead,
    /// paired with the `timezone` an adapter fetched because the core
    /// cannot (`T-core-owns-validation-order`). Required-ness is checked
    /// here, over whichever source is in play, so a submission missing
    /// both still reports one `MissingField(Deadline)` -- never two, and
    /// always in the same position among the four required fields.
    fn require_deadline_input(fields: &TriageFields) -> Result<DeadlineInput, TriageRejection> {
        match Self::instant_deadline_input(fields) {
            Some(input) => Ok(input),
            None => Self::local_date_deadline_input(fields),
        }
    }

    /// JSON's pre-resolved instant, when present and non-empty.
    fn instant_deadline_input(fields: &TriageFields) -> Option<DeadlineInput> {
        let deadline = fields.deadline.as_ref()?;
        (!deadline.is_empty()).then(|| DeadlineInput::Instant(deadline.clone()))
    }

    /// The page transport's alternative: `deadline_date` paired with the
    /// `timezone` an adapter fetched, since the core cannot
    /// (`T-core-owns-validation-order`).
    fn local_date_deadline_input(fields: &TriageFields) -> Result<DeadlineInput, TriageRejection> {
        match &fields.deadline_date {
            Some(date) if !date.is_empty() => {
                let zone = require(Field::Deadline, &fields.timezone)?;
                let time = fields.deadline_time.clone().filter(|t| !t.is_empty());
                Ok(DeadlineInput::LocalDate {
                    date: date.clone(),
                    time,
                    zone,
                })
            }
            _ => Err(TriageRejection::MissingField(Field::Deadline)),
        }
    }

    fn parse_committed_fields(
        deadline: DeadlineInput,
        commitment: String,
        priority: String,
        estimated_minutes: i64,
    ) -> Result<Self, TriageRejection> {
        let deadline = deadline.resolve_ms()?;
        let commitment = Commitment::parse(&commitment)
            .ok_or(TriageRejection::InvalidField(Field::Commitment))?;
        let priority =
            Priority::parse(&priority).ok_or(TriageRejection::InvalidField(Field::Priority))?;
        let estimated_minutes = require_positive(Field::EstimatedMinutes, estimated_minutes)?;

        Ok(Self::Committed {
            deadline,
            commitment,
            priority,
            estimated_minutes,
        })
    }

    /// Reuses [`QuotaDefinition::from_fields`] rather than restating name
    /// and hours validation here: it is the same rule the retired
    /// quota-screen define form checked, now reached from triage instead
    /// (`quota_triage_validation.feature`'s own reasoning -- "the rules
    /// survived, the surface moved").
    fn quota_from(fields: &TriageFields) -> Result<Self, TriageRejection> {
        let definition = QuotaDefinition::from_fields(
            fields.quota_name.as_deref(),
            fields.quota_hours.as_deref(),
        )
        .map_err(quota_rejection_to_triage_rejection)?;
        Ok(Self::Quota {
            name: definition.name,
            weekly_target: definition.weekly_target,
        })
    }

    pub fn attributes(&self) -> TaskAttributes {
        match self {
            Self::Pool => TaskAttributes {
                kind: POOL,
                ..TaskAttributes::default()
            },
            Self::Committed {
                deadline,
                commitment,
                priority,
                estimated_minutes,
            } => TaskAttributes {
                kind: COMMITTED,
                deadline: Some(*deadline),
                commitment: Some(commitment.as_str()),
                priority: Some(priority.as_str()),
                estimated_minutes: Some(*estimated_minutes),
            },
            // A quota's name and target are not a `tasks` column at all
            // (they land in `quotas` instead, the delivery layer's write to
            // make): the row this triage produces carries no more than a
            // pool task's does.
            Self::Quota { .. } => TaskAttributes {
                kind: QUOTA,
                ..TaskAttributes::default()
            },
        }
    }
}

/// [`QuotaDefinition::from_fields`]'s rejection, restated in triage's own
/// vocabulary -- the same two field names, the same two shapes, because a
/// quota triage's name and hours are exactly the fields the retired
/// define form checked.
fn quota_rejection_to_triage_rejection(rejection: DefinitionRejection) -> TriageRejection {
    match rejection {
        DefinitionRejection::MissingField(QuotaField::Name) => {
            TriageRejection::MissingField(Field::Name)
        }
        DefinitionRejection::MissingField(QuotaField::Hours) => {
            TriageRejection::MissingField(Field::Hours)
        }
        DefinitionRejection::InvalidField(QuotaField::Name) => {
            TriageRejection::InvalidField(Field::Name)
        }
        DefinitionRejection::InvalidField(QuotaField::Hours) => {
            TriageRejection::InvalidField(Field::Hours)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- TaskKind::from_fields — pool --------------------------------------

    fn kind_named(name: &str) -> TriageFields {
        TriageFields {
            kind: Some(name.to_string()),
            ..TriageFields::default()
        }
    }

    #[test]
    fn pool_fields_describe_a_pool_task() {
        assert_eq!(TaskKind::from_fields(&kind_named(POOL)), Ok(TaskKind::Pool));
    }

    #[test]
    fn a_pool_task_has_no_deadline_and_no_quota_target() {
        assert_eq!(
            TaskKind::Pool.attributes(),
            TaskAttributes {
                kind: POOL,
                ..TaskAttributes::default()
            }
        );
    }

    // --- TaskKind::from_fields — committed ----------------------------------

    fn committed_fields() -> TriageFields {
        TriageFields {
            kind: Some(COMMITTED.to_string()),
            deadline: Some("2026-08-20T17:00:00Z".to_string()),
            commitment: Some("at".to_string()),
            priority: Some("P1".to_string()),
            estimated_minutes: Some(180),
            ..TriageFields::default()
        }
    }

    #[test]
    fn committed_fields_describe_a_committed_task_carrying_its_parsed_metadata() {
        assert_eq!(
            TaskKind::from_fields(&committed_fields()),
            Ok(TaskKind::Committed {
                deadline: 1787245200000,
                commitment: Commitment::At,
                priority: Priority::P1,
                estimated_minutes: 180,
            })
        );
    }

    #[test]
    fn a_committed_task_reports_its_metadata_and_no_quota_target() {
        let kind = TaskKind::from_fields(&committed_fields()).unwrap();
        assert_eq!(
            kind.attributes(),
            TaskAttributes {
                kind: COMMITTED,
                deadline: Some(1787245200000),
                commitment: Some("at"),
                priority: Some("P1"),
                estimated_minutes: Some(180),
            }
        );
    }

    #[test]
    fn committed_fields_without_an_estimate_are_rejected_as_missing() {
        let mut fields = committed_fields();
        fields.estimated_minutes = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::EstimatedMinutes))
        );
    }

    #[test]
    fn committed_fields_with_a_zero_estimate_are_rejected_as_invalid() {
        let mut fields = committed_fields();
        fields.estimated_minutes = Some(0);
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::EstimatedMinutes))
        );
    }

    #[test]
    fn committed_fields_with_a_negative_estimate_are_rejected_as_invalid() {
        let mut fields = committed_fields();
        fields.estimated_minutes = Some(-5);
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::EstimatedMinutes))
        );
    }

    #[test]
    fn committed_fields_without_a_deadline_are_rejected_as_missing() {
        let mut fields = committed_fields();
        fields.deadline = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Deadline))
        );
    }

    #[test]
    fn committed_fields_with_an_empty_deadline_are_rejected_the_same_as_absent() {
        let mut fields = committed_fields();
        fields.deadline = Some(String::new());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Deadline))
        );
    }

    #[test]
    fn committed_fields_without_a_commitment_are_rejected_as_missing() {
        let mut fields = committed_fields();
        fields.commitment = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Commitment))
        );
    }

    #[test]
    fn committed_fields_without_a_priority_are_rejected_as_missing() {
        let mut fields = committed_fields();
        fields.priority = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Priority))
        );
    }

    #[test]
    fn a_submission_missing_several_required_fields_names_the_first_of_them() {
        let mut fields = committed_fields();
        fields.deadline = None;
        fields.priority = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Deadline))
        );
    }

    #[test]
    fn committed_fields_with_an_unparseable_deadline_are_rejected_naming_it_invalid() {
        let mut fields = committed_fields();
        fields.deadline = Some("banana".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::Deadline))
        );
    }

    // --- committed via deadline_date/deadline_time/timezone (#110) --------

    fn committed_fields_via_local_date() -> TriageFields {
        TriageFields {
            kind: Some(COMMITTED.to_string()),
            deadline_date: Some("2026-08-25".to_string()),
            deadline_time: Some("08:30".to_string()),
            timezone: Some("America/New_York".to_string()),
            commitment: Some("at".to_string()),
            priority: Some("P1".to_string()),
            estimated_minutes: Some(180),
            ..TriageFields::default()
        }
    }

    #[test]
    fn a_local_date_and_time_resolve_to_the_instant_they_name_in_the_zone() {
        assert_eq!(
            TaskKind::from_fields(&committed_fields_via_local_date()),
            Ok(TaskKind::Committed {
                deadline: 1787661000000,
                commitment: Commitment::At,
                priority: Priority::P1,
                estimated_minutes: 180,
            })
        );
    }

    #[test]
    fn a_local_date_with_no_time_resolves_to_the_end_of_that_day() {
        let mut fields = committed_fields_via_local_date();
        fields.deadline_date = Some("2026-08-27".to_string());
        fields.deadline_time = None;
        fields.commitment = Some("by".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Ok(TaskKind::Committed {
                deadline: 1787889599999,
                commitment: Commitment::By,
                priority: Priority::P1,
                estimated_minutes: 180,
            })
        );
    }

    #[test]
    fn deadline_wins_over_deadline_date_when_both_are_present() {
        let mut fields = committed_fields_via_local_date();
        fields.deadline = Some("2026-08-20T17:00:00Z".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Ok(TaskKind::Committed {
                deadline: 1787245200000,
                commitment: Commitment::At,
                priority: Priority::P1,
                estimated_minutes: 180,
            })
        );
    }

    #[test]
    fn deadline_date_without_a_timezone_is_rejected_as_missing_deadline() {
        let mut fields = committed_fields_via_local_date();
        fields.timezone = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Deadline))
        );
    }

    #[test]
    fn neither_deadline_nor_deadline_date_is_rejected_as_missing_deadline() {
        let mut fields = committed_fields_via_local_date();
        fields.deadline_date = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Deadline))
        );
    }

    #[test]
    fn an_empty_deadline_date_is_rejected_the_same_as_absent() {
        let mut fields = committed_fields_via_local_date();
        fields.deadline_date = Some(String::new());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Deadline))
        );
    }

    #[test]
    fn an_unparseable_local_date_is_rejected_as_invalid_deadline() {
        let mut fields = committed_fields_via_local_date();
        fields.deadline_date = Some("banana".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::Deadline))
        );
    }

    #[test]
    fn an_unknown_zone_is_rejected_as_invalid_deadline() {
        let mut fields = committed_fields_via_local_date();
        fields.timezone = Some("Nowhere/Imaginary".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::Deadline))
        );
    }

    #[test]
    fn committed_fields_with_an_invalid_commitment_are_rejected_naming_it_invalid() {
        let mut fields = committed_fields();
        fields.commitment = Some("hard".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::Commitment))
        );
    }

    #[test]
    fn committed_fields_with_an_invalid_priority_are_rejected_naming_it_invalid() {
        let mut fields = committed_fields();
        fields.priority = Some("P9".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::Priority))
        );
    }

    // --- TaskKind::from_fields — quota (#138) -------------------------------

    fn quota_fields() -> TriageFields {
        TriageFields {
            kind: Some(QUOTA.to_string()),
            quota_name: Some("Piano".to_string()),
            quota_hours: Some("4".to_string()),
            ..TriageFields::default()
        }
    }

    #[test]
    fn quota_fields_describe_a_quota_task_carrying_its_name_and_target() {
        assert_eq!(
            TaskKind::from_fields(&quota_fields()),
            Ok(TaskKind::Quota {
                name: "Piano".to_string(),
                weekly_target: WeeklyTarget::from_minutes(240).unwrap(),
            })
        );
    }

    #[test]
    fn a_quota_task_reports_no_deadline_and_no_metadata_of_its_own() {
        let kind = TaskKind::from_fields(&quota_fields()).unwrap();
        assert_eq!(
            kind.attributes(),
            TaskAttributes {
                kind: QUOTA,
                ..TaskAttributes::default()
            }
        );
    }

    #[test]
    fn quota_fields_without_a_name_are_rejected_as_missing() {
        let mut fields = quota_fields();
        fields.quota_name = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Name))
        );
    }

    #[test]
    fn quota_fields_with_a_blank_name_are_rejected_the_same_as_absent() {
        let mut fields = quota_fields();
        fields.quota_name = Some("   ".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Name))
        );
    }

    #[test]
    fn quota_fields_without_hours_are_rejected_as_missing() {
        let mut fields = quota_fields();
        fields.quota_hours = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Hours))
        );
    }

    #[test]
    fn quota_fields_with_zero_hours_are_rejected_as_invalid() {
        let mut fields = quota_fields();
        fields.quota_hours = Some("0".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::Hours))
        );
    }

    #[test]
    fn quota_fields_with_negative_hours_are_rejected_as_invalid() {
        let mut fields = quota_fields();
        fields.quota_hours = Some("-2".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::Hours))
        );
    }

    #[test]
    fn quota_fields_with_unparseable_hours_are_rejected_as_invalid() {
        let mut fields = quota_fields();
        fields.quota_hours = Some("four".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::Hours))
        );
    }

    #[test]
    fn quota_fields_accept_a_half_hour_target() {
        let mut fields = quota_fields();
        fields.quota_hours = Some("0.5".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Ok(TaskKind::Quota {
                name: "Piano".to_string(),
                weekly_target: WeeklyTarget::from_minutes(30).unwrap(),
            })
        );
    }

    // --- unknown kind --------------------------------------------------------

    #[test]
    fn an_unrecognised_kind_is_rejected_and_reports_what_was_submitted() {
        assert_eq!(
            TaskKind::from_fields(&kind_named("someday")),
            Err(TriageRejection::UnknownKind)
        );
    }

    #[test]
    fn an_absent_kind_is_rejected_with_nothing_to_report() {
        assert_eq!(
            TaskKind::from_fields(&TriageFields::default()),
            Err(TriageRejection::UnknownKind)
        );
    }
}
