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

pub use fields::{Commitment, DeadlineType, Field, Period, Priority};

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
    pub target_count: Option<i64>,
    pub target_minutes_each: Option<i64>,
    pub period: Option<String>,
    /// Required for a committed task only (#62's capacity number cannot
    /// count what committed work asks for otherwise). Pool is never placed
    /// (`D-no-pool-on-calendar`) and quota already carries
    /// `target_minutes_each`, so neither needs this field.
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
    /// T-quota-targets-required: a quota task cannot be scheduled at M8 or
    /// reported on at the reckoning without a target, so all three fields are
    /// required at triage even though the columns stay nullable
    /// (T-three-task-kinds) for schema reasons — one `tasks` table shared by
    /// three kinds.
    Quota {
        target_count: i64,
        target_minutes_each: i64,
        period: Period,
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
    pub target_count: Option<i64>,
    pub target_minutes_each: Option<i64>,
    pub period: Option<&'static str>,
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

/// A quota target of zero or fewer sessions/minutes defeats the reckoning's
/// own math (`count(done)/target_count`) before a task can ever be created.
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
        if let Some(deadline) = &fields.deadline {
            if !deadline.is_empty() {
                return Ok(DeadlineInput::Instant(deadline.clone()));
            }
        }
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

    fn quota_from(fields: &TriageFields) -> Result<Self, TriageRejection> {
        let (target_count, target_minutes_each, period) = Self::require_quota_fields(fields)?;
        Self::parse_quota_fields(target_count, target_minutes_each, period)
    }

    fn require_quota_fields(fields: &TriageFields) -> Result<(i64, i64, String), TriageRejection> {
        let target_count = require_i64(Field::TargetCount, fields.target_count)?;
        let target_minutes_each =
            require_i64(Field::TargetMinutesEach, fields.target_minutes_each)?;
        let period = require(Field::Period, &fields.period)?;
        Ok((target_count, target_minutes_each, period))
    }

    fn parse_quota_fields(
        target_count: i64,
        target_minutes_each: i64,
        period: String,
    ) -> Result<Self, TriageRejection> {
        let target_count = require_positive(Field::TargetCount, target_count)?;
        let target_minutes_each = require_positive(Field::TargetMinutesEach, target_minutes_each)?;
        let period = Period::parse(&period).ok_or(TriageRejection::InvalidField(Field::Period))?;

        Ok(Self::Quota {
            target_count,
            target_minutes_each,
            period,
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
                ..TaskAttributes::default()
            },
            Self::Quota {
                target_count,
                target_minutes_each,
                period,
            } => TaskAttributes {
                kind: QUOTA,
                target_count: Some(*target_count),
                target_minutes_each: Some(*target_minutes_each),
                period: Some(period.as_str()),
                ..TaskAttributes::default()
            },
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
                ..TaskAttributes::default()
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

    // --- TaskKind::from_fields — quota --------------------------------------

    fn quota_fields() -> TriageFields {
        TriageFields {
            kind: Some(QUOTA.to_string()),
            target_count: Some(3),
            target_minutes_each: Some(45),
            period: Some("week".to_string()),
            ..TriageFields::default()
        }
    }

    #[test]
    fn quota_fields_describe_a_quota_task_carrying_its_target() {
        assert_eq!(
            TaskKind::from_fields(&quota_fields()),
            Ok(TaskKind::Quota {
                target_count: 3,
                target_minutes_each: 45,
                period: Period::Week,
            })
        );
    }

    #[test]
    fn a_quota_task_reports_its_target_and_no_deadline() {
        let kind = TaskKind::from_fields(&quota_fields()).unwrap();
        assert_eq!(
            kind.attributes(),
            TaskAttributes {
                kind: QUOTA,
                target_count: Some(3),
                target_minutes_each: Some(45),
                period: Some("week"),
                ..TaskAttributes::default()
            }
        );
    }

    #[test]
    fn quota_fields_without_a_target_count_are_rejected_as_missing() {
        let mut fields = quota_fields();
        fields.target_count = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::TargetCount))
        );
    }

    #[test]
    fn quota_fields_without_a_target_minutes_each_are_rejected_as_missing() {
        let mut fields = quota_fields();
        fields.target_minutes_each = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::TargetMinutesEach))
        );
    }

    #[test]
    fn quota_fields_without_a_period_are_rejected_as_missing() {
        let mut fields = quota_fields();
        fields.period = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Period))
        );
    }

    #[test]
    fn quota_fields_with_an_empty_period_are_rejected_the_same_as_absent() {
        let mut fields = quota_fields();
        fields.period = Some(String::new());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Period))
        );
    }

    #[test]
    fn quota_fields_with_an_invalid_period_are_rejected_naming_it_invalid() {
        let mut fields = quota_fields();
        fields.period = Some("fortnight".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::Period))
        );
    }

    #[test]
    fn quota_fields_with_a_zero_target_count_are_rejected_naming_it_invalid() {
        let mut fields = quota_fields();
        fields.target_count = Some(0);
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::TargetCount))
        );
    }

    #[test]
    fn quota_fields_with_a_negative_target_count_are_rejected_naming_it_invalid() {
        let mut fields = quota_fields();
        fields.target_count = Some(-1);
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::TargetCount))
        );
    }

    #[test]
    fn quota_fields_with_a_zero_target_minutes_each_are_rejected_naming_it_invalid() {
        let mut fields = quota_fields();
        fields.target_minutes_each = Some(0);
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::TargetMinutesEach))
        );
    }

    #[test]
    fn quota_fields_with_a_negative_target_minutes_each_are_rejected_naming_it_invalid() {
        let mut fields = quota_fields();
        fields.target_minutes_each = Some(-5);
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::TargetMinutesEach))
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
