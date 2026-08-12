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

/// A field a committed task cannot do without.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommittedField {
    Deadline,
    DeadlineType,
    Priority,
}

impl CommittedField {
    /// The name reported back to whoever submitted the triage.
    pub fn name(self) -> &'static str {
        match self {
            Self::Deadline => "deadline",
            Self::DeadlineType => "deadline_type",
            Self::Priority => "priority",
        }
    }
}

/// Why a set of triage fields does not describe a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriageRejection {
    /// A committed task was submitted without one of the fields it requires.
    MissingField(CommittedField),
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
        deadline: String,
        deadline_type: String,
        priority: String,
    },
    /// T11 leaves the quota fields nullable: quota *scheduling* lands at M8,
    /// so nothing yet can say which of them are required.
    Quota {
        target_count: Option<i64>,
        target_minutes_each: Option<i64>,
        period: Option<String>,
    },
}

/// A task kind flattened into the nullable, row-shaped projection a
/// row-oriented consumer needs. At most one attribute group is ever
/// populated — that exclusivity is the invariant [`TaskKind`] exists to
/// guarantee, and expressing the projection here keeps it provable without
/// a database.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TaskAttributes<'a> {
    pub kind: &'static str,
    pub deadline: Option<&'a str>,
    pub deadline_type: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub target_count: Option<i64>,
    pub target_minutes_each: Option<i64>,
    pub period: Option<&'a str>,
}

fn require(field: CommittedField, value: &Option<String>) -> Result<String, TriageRejection> {
    value.clone().ok_or(TriageRejection::MissingField(field))
}

impl TaskKind {
    /// Decides which kind of task, if any, a set of triage fields describes.
    pub fn from_fields(fields: &TriageFields) -> Result<Self, TriageRejection> {
        match fields.kind.as_deref() {
            Some(POOL) => Ok(Self::Pool),
            Some(COMMITTED) => Self::committed_from(fields),
            Some(QUOTA) => Ok(Self::Quota {
                target_count: fields.target_count,
                target_minutes_each: fields.target_minutes_each,
                period: fields.period.clone(),
            }),
            _ => Err(TriageRejection::UnknownKind(fields.kind.clone())),
        }
    }

    /// The required fields are checked in a fixed order so a submission
    /// missing several of them names the same one every time.
    fn committed_from(fields: &TriageFields) -> Result<Self, TriageRejection> {
        let deadline = require(CommittedField::Deadline, &fields.deadline)?;
        let deadline_type = require(CommittedField::DeadlineType, &fields.deadline_type)?;
        let priority = require(CommittedField::Priority, &fields.priority)?;
        Ok(Self::Committed {
            deadline,
            deadline_type,
            priority,
        })
    }

    pub fn attributes(&self) -> TaskAttributes<'_> {
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
                deadline: Some(deadline),
                deadline_type: Some(deadline_type),
                priority: Some(priority),
                ..TaskAttributes::default()
            },
            Self::Quota {
                target_count,
                target_minutes_each,
                period,
            } => TaskAttributes {
                kind: QUOTA,
                target_count: *target_count,
                target_minutes_each: *target_minutes_each,
                period: period.as_deref(),
                ..TaskAttributes::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed_fields() -> TriageFields {
        TriageFields {
            kind: Some(COMMITTED.to_string()),
            deadline: Some("2026-08-20T17:00:00Z".to_string()),
            deadline_type: Some("hard".to_string()),
            priority: Some("P1".to_string()),
            ..TriageFields::default()
        }
    }

    fn quota_fields() -> TriageFields {
        TriageFields {
            kind: Some(QUOTA.to_string()),
            target_count: Some(3),
            target_minutes_each: Some(45),
            period: Some("week".to_string()),
            ..TriageFields::default()
        }
    }

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
        let attributes = TaskKind::Pool.attributes();
        assert_eq!(
            attributes,
            TaskAttributes {
                kind: POOL,
                ..TaskAttributes::default()
            }
        );
    }

    #[test]
    fn committed_fields_describe_a_committed_task_carrying_its_metadata() {
        assert_eq!(
            TaskKind::from_fields(&committed_fields()),
            Ok(TaskKind::Committed {
                deadline: "2026-08-20T17:00:00Z".to_string(),
                deadline_type: "hard".to_string(),
                priority: "P1".to_string(),
            })
        );
    }

    #[test]
    fn a_committed_task_reports_its_metadata_and_no_quota_target() {
        let kind = TaskKind::from_fields(&committed_fields()).unwrap();
        let attributes = kind.attributes();
        assert_eq!(
            attributes,
            TaskAttributes {
                kind: COMMITTED,
                deadline: Some("2026-08-20T17:00:00Z"),
                deadline_type: Some("hard"),
                priority: Some("P1"),
                ..TaskAttributes::default()
            }
        );
    }

    #[test]
    fn committed_fields_without_a_deadline_are_rejected_naming_the_deadline() {
        let mut fields = committed_fields();
        fields.deadline = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(CommittedField::Deadline))
        );
    }

    #[test]
    fn committed_fields_without_a_deadline_type_are_rejected_naming_the_deadline_type() {
        let mut fields = committed_fields();
        fields.deadline_type = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(CommittedField::DeadlineType))
        );
    }

    #[test]
    fn committed_fields_without_a_priority_are_rejected_naming_the_priority() {
        let mut fields = committed_fields();
        fields.priority = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(CommittedField::Priority))
        );
    }

    #[test]
    fn a_submission_missing_several_required_fields_names_the_first_of_them() {
        let mut fields = committed_fields();
        fields.deadline = None;
        fields.priority = None;
        assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(CommittedField::Deadline))
        );
    }

    #[test]
    fn quota_fields_describe_a_quota_task_carrying_its_target() {
        assert_eq!(
            TaskKind::from_fields(&quota_fields()),
            Ok(TaskKind::Quota {
                target_count: Some(3),
                target_minutes_each: Some(45),
                period: Some("week".to_string()),
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
    fn quota_fields_are_nullable_because_quota_scheduling_lands_at_m8() {
        assert_eq!(
            TaskKind::from_fields(&kind_named(QUOTA)),
            Ok(TaskKind::Quota {
                target_count: None,
                target_minutes_each: None,
                period: None,
            })
        );
    }

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

    #[test]
    fn each_committed_field_reports_the_name_the_submitter_used() {
        assert_eq!(CommittedField::Deadline.name(), "deadline");
        assert_eq!(CommittedField::DeadlineType.name(), "deadline_type");
        assert_eq!(CommittedField::Priority.name(), "priority");
    }
}
