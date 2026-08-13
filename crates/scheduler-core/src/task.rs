//! The task model: what a task is, and what each kind of task requires.
//!
//! This is high-level policy and is deliberately free of HTTP, JSON, SQL and
//! async. A triage decision can be made — and tested — with nothing but this
//! module, which is the property T4's "no tokio" rule exists to protect.

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
    pub deadline_type: Option<String>,
    pub priority: Option<String>,
    pub target_count: Option<i64>,
    pub target_minutes_each: Option<i64>,
    pub period: Option<String>,
}

/// A field a triage submission must supply, or supply a valid value for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Deadline,
    DeadlineType,
    Priority,
    TargetCount,
    TargetMinutesEach,
    Period,
}

impl Field {
    /// The name reported back to whoever submitted the triage.
    pub fn name(self) -> &'static str {
        match self {
            Self::Deadline => "deadline",
            Self::DeadlineType => "deadline_type",
            Self::Priority => "priority",
            Self::TargetCount => "target_count",
            Self::TargetMinutesEach => "target_minutes_each",
            Self::Period => "period",
        }
    }
}

/// `deadline_type`'s closed domain (D3: the M3 scheduler branches on this
/// field, so it cannot carry an undefined value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineType {
    Hard,
    Soft,
}

impl DeadlineType {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "hard" => Some(Self::Hard),
            "soft" => Some(Self::Soft),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hard => "hard",
            Self::Soft => "soft",
        }
    }
}

/// `priority`'s closed domain, for the same reason as [`DeadlineType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    P1,
    P2,
    P3,
    P4,
}

impl Priority {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "P1" => Some(Self::P1),
            "P2" => Some(Self::P2),
            "P3" => Some(Self::P3),
            "P4" => Some(Self::P4),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
            Self::P4 => "P4",
        }
    }
}

/// `period`'s closed domain (T20): the same M8 cadence-math reason as
/// [`DeadlineType`] and [`Priority`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Week,
    Month,
}

impl Period {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "week" => Some(Self::Week),
            "month" => Some(Self::Month),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Week => "week",
            Self::Month => "month",
        }
    }
}

/// Parses a deadline to UTC epoch milliseconds (T3). Rejects anything that
/// does not name a real instant — a syntactically plausible but invalid
/// timestamp (`2026-13-45T99:99:99Z`) fails the same as free text.
fn parse_deadline_ms(value: &str) -> Option<i64> {
    value
        .parse::<jiff::Timestamp>()
        .ok()
        .map(|ts| ts.as_millisecond())
}

/// Why a set of triage fields does not describe a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriageRejection {
    /// A required field was absent, or submitted as an empty string (T19:
    /// the two report identically — a client rarely intends the distinction).
    MissingField(Field),
    /// A field was present but its value is outside the field's domain.
    InvalidField(Field),
    /// `kind` was absent, or named something that is not one of the three.
    /// Carries what was submitted, so the rejection can say so.
    UnknownKind(Option<String>),
}

/// T11: task kind is a three-variant sum type. Each variant carries exactly
/// the fields that kind means, so "a pool task has no deadline" and "a
/// committed task has no quota target" are facts about the type rather than
/// assertions about one caller's payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {
    Pool,
    Committed {
        deadline: i64,
        deadline_type: DeadlineType,
        priority: Priority,
    },
    /// T18: a quota task cannot be scheduled at M8 or reported on at the
    /// reckoning without a target, so all three fields are required at
    /// triage even though the columns stay nullable (T11) for schema
    /// reasons — one `tasks` table shared by three kinds.
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
    pub deadline_type: Option<&'static str>,
    pub priority: Option<&'static str>,
    pub target_count: Option<i64>,
    pub target_minutes_each: Option<i64>,
    pub period: Option<&'static str>,
}

/// Requires a string field to be both present and non-empty (T19: absent and
/// empty are the same submitter mistake, so they report identically).
fn require(field: Field, value: &Option<String>) -> Result<String, TriageRejection> {
    match value {
        Some(v) if !v.is_empty() => Ok(v.clone()),
        _ => Err(TriageRejection::MissingField(field)),
    }
}

fn require_i64(field: Field, value: Option<i64>) -> Result<i64, TriageRejection> {
    value.ok_or(TriageRejection::MissingField(field))
}

impl TaskKind {
    /// Decides which kind of task, if any, a set of triage fields describes.
    pub fn from_fields(fields: &TriageFields) -> Result<Self, TriageRejection> {
        match fields.kind.as_deref() {
            Some(POOL) => Ok(Self::Pool),
            Some(COMMITTED) => Self::committed_from(fields),
            Some(QUOTA) => Self::quota_from(fields),
            _ => Err(TriageRejection::UnknownKind(fields.kind.clone())),
        }
    }

    /// Required fields are checked before values, and in a fixed order, so a
    /// submission with several problems names the same one every time.
    fn committed_from(fields: &TriageFields) -> Result<Self, TriageRejection> {
        let deadline = require(Field::Deadline, &fields.deadline)?;
        let deadline_type = require(Field::DeadlineType, &fields.deadline_type)?;
        let priority = require(Field::Priority, &fields.priority)?;

        let deadline =
            parse_deadline_ms(&deadline).ok_or(TriageRejection::InvalidField(Field::Deadline))?;
        let deadline_type = DeadlineType::parse(&deadline_type)
            .ok_or(TriageRejection::InvalidField(Field::DeadlineType))?;
        let priority =
            Priority::parse(&priority).ok_or(TriageRejection::InvalidField(Field::Priority))?;

        Ok(Self::Committed {
            deadline,
            deadline_type,
            priority,
        })
    }

    fn quota_from(fields: &TriageFields) -> Result<Self, TriageRejection> {
        let target_count = require_i64(Field::TargetCount, fields.target_count)?;
        let target_minutes_each =
            require_i64(Field::TargetMinutesEach, fields.target_minutes_each)?;
        let period = require(Field::Period, &fields.period)?;
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
                deadline_type,
                priority,
            } => TaskAttributes {
                kind: COMMITTED,
                deadline: Some(*deadline),
                deadline_type: Some(deadline_type.as_str()),
                priority: Some(priority.as_str()),
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

    // --- parse_deadline_ms -------------------------------------------------

    #[test]
    fn parse_deadline_ms_agrees_on_the_same_instant_across_equivalent_textual_forms() {
        for text in [
            "2026-08-20T17:00:00Z",
            "2026-08-20T17:00:00.000Z",
            "2026-08-20T19:00:00+02:00",
        ] {
            assert_eq!(
                parse_deadline_ms(text),
                Some(1787245200000),
                "form {text} did not round-trip to the expected instant"
            );
        }
    }

    #[test]
    fn parse_deadline_ms_rejects_free_text() {
        assert_eq!(parse_deadline_ms("banana"), None);
    }

    #[test]
    fn parse_deadline_ms_rejects_a_syntactically_plausible_but_invalid_instant() {
        assert_eq!(parse_deadline_ms("2026-13-45T99:99:99Z"), None);
    }

    #[test]
    fn parse_deadline_ms_rejects_a_sql_injection_shaped_string() {
        assert_eq!(parse_deadline_ms("'); DROP TABLE tasks;--"), None);
    }

    // --- DeadlineType --------------------------------------------------------

    #[test]
    fn deadline_type_parses_hard_and_soft() {
        assert_eq!(DeadlineType::parse("hard"), Some(DeadlineType::Hard));
        assert_eq!(DeadlineType::parse("soft"), Some(DeadlineType::Soft));
    }

    #[test]
    fn deadline_type_rejects_values_outside_the_domain() {
        assert_eq!(DeadlineType::parse("squishy"), None);
        assert_eq!(
            DeadlineType::parse("HARD"),
            None,
            "the domain is case-sensitive"
        );
    }

    // --- Priority --------------------------------------------------------

    #[test]
    fn priority_parses_p1_through_p4() {
        assert_eq!(Priority::parse("P1"), Some(Priority::P1));
        assert_eq!(Priority::parse("P2"), Some(Priority::P2));
        assert_eq!(Priority::parse("P3"), Some(Priority::P3));
        assert_eq!(Priority::parse("P4"), Some(Priority::P4));
    }

    #[test]
    fn priority_rejects_values_outside_the_domain() {
        assert_eq!(Priority::parse("P9"), None);
        assert_eq!(Priority::parse("p1"), None, "the domain is case-sensitive");
    }

    // --- Period --------------------------------------------------------

    #[test]
    fn period_parses_week_and_month() {
        assert_eq!(Period::parse("week"), Some(Period::Week));
        assert_eq!(Period::parse("month"), Some(Period::Month));
    }

    #[test]
    fn period_rejects_values_outside_the_domain() {
        assert_eq!(Period::parse("fortnight"), None);
        assert_eq!(Period::parse("Week"), None, "the domain is case-sensitive");
    }

    // --- Field --------------------------------------------------------

    #[test]
    fn each_field_reports_the_name_the_submitter_used() {
        assert_eq!(Field::Deadline.name(), "deadline");
        assert_eq!(Field::DeadlineType.name(), "deadline_type");
        assert_eq!(Field::Priority.name(), "priority");
        assert_eq!(Field::TargetCount.name(), "target_count");
        assert_eq!(Field::TargetMinutesEach.name(), "target_minutes_each");
        assert_eq!(Field::Period.name(), "period");
    }

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
            deadline_type: Some("hard".to_string()),
            priority: Some("P1".to_string()),
            ..TriageFields::default()
        }
    }

    #[test]
    fn committed_fields_describe_a_committed_task_carrying_its_parsed_metadata() {
        assert_eq!(
            TaskKind::from_fields(&committed_fields()),
            Ok(TaskKind::Committed {
                deadline: 1787245200000,
                deadline_type: DeadlineType::Hard,
                priority: Priority::P1,
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
                deadline_type: Some("hard"),
                priority: Some("P1"),
                ..TaskAttributes::default()
            }
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
    fn committed_fields_without_a_deadline_type_are_rejected_as_missing() {
        let mut fields = committed_fields();
        fields.deadline_type = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::DeadlineType))
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

    #[test]
    fn committed_fields_with_an_invalid_deadline_type_are_rejected_naming_it_invalid() {
        let mut fields = committed_fields();
        fields.deadline_type = Some("squishy".to_string());
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(Field::DeadlineType))
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

    // --- unknown kind --------------------------------------------------------

    #[test]
    fn an_unrecognised_kind_is_rejected_and_reports_what_was_submitted() {
        assert_eq!(
            TaskKind::from_fields(&kind_named("someday")),
            Err(TriageRejection::UnknownKind(Some("someday".to_string())))
        );
    }

    #[test]
    fn an_absent_kind_is_rejected_with_nothing_to_report() {
        assert_eq!(
            TaskKind::from_fields(&TriageFields::default()),
            Err(TriageRejection::UnknownKind(None))
        );
    }
}
