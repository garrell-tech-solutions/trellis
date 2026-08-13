//! Property tests for the task model.
//!
//! Kept separate from the unit tests and every case marked `#[ignore]`, per
//! the project convention: normal verification runs `cargo test --workspace`,
//! property verification runs it with `-- --include-ignored`.
//!
//! The unit tests check the three kinds against the example values the
//! acceptance criteria name. These check the same invariants against
//! everything else — in particular that "a pool task has no deadline" holds
//! for a payload that supplied one, which is exactly what a sum type buys and
//! what an example-based test cannot show.

use proptest::prelude::*;
use scheduler_core::task::{
    Field, TaskAttributes, TaskKind, TriageFields, TriageRejection, COMMITTED, POOL, QUOTA,
};

/// Every field a caller could send, chosen independently of the kind.
fn any_fields() -> impl Strategy<Value = TriageFields> {
    (
        proptest::option::of(".{0,40}"),
        proptest::option::of(".{0,40}"),
        proptest::option::of(".{0,40}"),
        proptest::option::of(any::<i64>()),
        proptest::option::of(any::<i64>()),
        proptest::option::of(".{0,40}"),
    )
        .prop_map(
            |(deadline, deadline_type, priority, target_count, target_minutes_each, period)| {
                TriageFields {
                    kind: None,
                    deadline,
                    deadline_type,
                    priority,
                    target_count,
                    target_minutes_each,
                    period,
                }
            },
        )
}

fn with_kind(fields: TriageFields, kind: &str) -> TriageFields {
    TriageFields {
        kind: Some(kind.to_string()),
        ..fields
    }
}

fn has_deadline(attributes: &TaskAttributes) -> bool {
    attributes.deadline.is_some()
        || attributes.deadline_type.is_some()
        || attributes.priority.is_some()
}

fn has_quota_target(attributes: &TaskAttributes) -> bool {
    attributes.target_count.is_some()
        || attributes.target_minutes_each.is_some()
        || attributes.period.is_some()
}

/// The closed domains, restated here on purpose. The core parses these
/// strings but does not enumerate them, so writing the membership out is what
/// lets a property check the domain from outside rather than against itself.
const DEADLINE_TYPES: [&str; 2] = ["hard", "soft"];
const PRIORITIES: [&str; 4] = ["P1", "P2", "P3", "P4"];
const PERIODS: [&str; 2] = ["week", "month"];

const VALID_DEADLINE: &str = "2026-08-20T17:00:00Z";

/// A required string field, paired with the submission it belongs to and the
/// way that submission carries it.
///
/// A table rather than a `match` arm per field: a property that varies "some
/// required field" then names each field once, and adding a field to the core
/// means adding one row here instead of an arm in every helper.
#[derive(Debug, Clone, Copy)]
struct RequiredField {
    field: Field,
    /// The closed set of values this field accepts, where it has one.
    /// `deadline` has none — it names an instant, not a member of a set.
    domain: Option<&'static [&'static str]>,
    base: fn() -> TriageFields,
    set: fn(&mut TriageFields, Option<String>),
}

const REQUIRED_STRING_FIELDS: [RequiredField; 4] = [
    RequiredField {
        field: Field::Deadline,
        domain: None,
        base: valid_committed,
        set: |fields, value| fields.deadline = value,
    },
    RequiredField {
        field: Field::DeadlineType,
        domain: Some(&DEADLINE_TYPES),
        base: valid_committed,
        set: |fields, value| fields.deadline_type = value,
    },
    RequiredField {
        field: Field::Priority,
        domain: Some(&PRIORITIES),
        base: valid_committed,
        set: |fields, value| fields.priority = value,
    },
    RequiredField {
        field: Field::Period,
        domain: Some(&PERIODS),
        base: valid_quota,
        set: |fields, value| fields.period = value,
    },
];

impl RequiredField {
    /// An otherwise-valid submission carrying `value` for this field.
    fn submission(self, value: Option<String>) -> TriageFields {
        let mut fields = (self.base)();
        (self.set)(&mut fields, value);
        fields
    }
}

fn closed_domain_fields() -> Vec<RequiredField> {
    REQUIRED_STRING_FIELDS
        .into_iter()
        .filter(|entry| entry.domain.is_some())
        .collect()
}

fn valid_committed() -> TriageFields {
    TriageFields {
        kind: Some(COMMITTED.to_string()),
        deadline: Some(VALID_DEADLINE.to_string()),
        deadline_type: Some("hard".to_string()),
        priority: Some("P1".to_string()),
        ..TriageFields::default()
    }
}

fn valid_quota() -> TriageFields {
    TriageFields {
        kind: Some(QUOTA.to_string()),
        target_count: Some(3),
        target_minutes_each: Some(45),
        period: Some("week".to_string()),
        ..TriageFields::default()
    }
}

/// A handful of well-formed deadlines and the instant each names, since
/// deadline validity (T-jiff-epoch-millis) is exercised by the unit tests —
/// this generator only needs enough variety to keep the round-trip property
/// honest.
fn valid_deadline() -> impl Strategy<Value = (&'static str, i64)> {
    prop::sample::select(vec![
        ("2026-08-20T17:00:00Z", 1787245200000i64),
        ("2026-08-20T17:00:00.000Z", 1787245200000i64),
        ("2026-08-20T19:00:00+02:00", 1787245200000i64),
        ("2000-01-01T00:00:00Z", 946684800000i64),
        ("2099-12-31T23:59:59Z", 4102444799000i64),
    ])
}

proptest! {
    /// A pool task carries nothing, however much the caller sent.
    #[test]
    #[ignore]
    fn a_pool_task_never_carries_a_deadline_or_a_quota_target(fields in any_fields()) {
        let kind = TaskKind::from_fields(&with_kind(fields, POOL)).unwrap();
        let attributes = kind.attributes();

        prop_assert_eq!(attributes.kind, POOL);
        prop_assert!(!has_deadline(&attributes));
        prop_assert!(!has_quota_target(&attributes));
    }

    /// A committed task reports back exactly the metadata it was given, and
    /// never a quota target. `deadline_type`/`priority` are drawn from their
    /// closed domains (validity itself is the unit tests' job); the deadline
    /// is reported as the instant it names, not the text that named it.
    #[test]
    #[ignore]
    fn a_committed_task_round_trips_its_metadata_and_carries_no_quota_target(
        fields in any_fields(),
        deadline in valid_deadline(),
        deadline_type in prop::sample::select(vec!["hard", "soft"]),
        priority in prop::sample::select(vec!["P1", "P2", "P3", "P4"]),
    ) {
        let (deadline_text, deadline_ms) = deadline;
        let fields = TriageFields {
            deadline: Some(deadline_text.to_string()),
            deadline_type: Some(deadline_type.to_string()),
            priority: Some(priority.to_string()),
            ..with_kind(fields, COMMITTED)
        };

        let kind = TaskKind::from_fields(&fields).unwrap();
        let attributes = kind.attributes();

        prop_assert_eq!(attributes.kind, COMMITTED);
        prop_assert_eq!(attributes.deadline, Some(deadline_ms));
        prop_assert_eq!(attributes.deadline_type, Some(deadline_type));
        prop_assert_eq!(attributes.priority, Some(priority));
        prop_assert!(!has_quota_target(&attributes));
    }

    /// A quota task reports back exactly the target it was given, and never a
    /// deadline. T-quota-targets-required requires all three target fields at
    /// triage, so unlike the pool case this generates only valid, complete
    /// targets — and a target must be positive (folded in from the PR #31
    /// review), so the generator only draws positive counts.
    #[test]
    #[ignore]
    fn a_quota_task_round_trips_its_target_and_carries_no_deadline(
        fields in any_fields(),
        target_count in 1i64..=1_000_000,
        target_minutes_each in 1i64..=1_000_000,
        period in prop::sample::select(vec!["week", "month"]),
    ) {
        let fields = TriageFields {
            target_count: Some(target_count),
            target_minutes_each: Some(target_minutes_each),
            period: Some(period.to_string()),
            ..with_kind(fields, QUOTA)
        };

        let kind = TaskKind::from_fields(&fields).unwrap();
        let attributes = kind.attributes();

        prop_assert_eq!(attributes.kind, QUOTA);
        prop_assert_eq!(attributes.target_count, Some(target_count));
        prop_assert_eq!(attributes.target_minutes_each, Some(target_minutes_each));
        prop_assert_eq!(attributes.period, Some(period));
        prop_assert!(!has_deadline(&attributes));
    }

    /// Whichever kind is accepted, at most one attribute group is populated.
    /// This is the invariant the three-variant sum type exists to guarantee.
    /// Every field required by any kind is supplied and valid, so the
    /// outcome turns only on which `kind` was named.
    #[test]
    #[ignore]
    fn an_accepted_task_never_populates_both_attribute_groups(
        fields in any_fields(),
        kind_index in 0usize..3,
    ) {
        let name = [POOL, COMMITTED, QUOTA][kind_index];
        let fields = TriageFields {
            deadline: Some("2026-08-20T17:00:00Z".to_string()),
            deadline_type: Some("hard".to_string()),
            priority: Some("P1".to_string()),
            target_count: Some(3),
            target_minutes_each: Some(45),
            period: Some("week".to_string()),
            ..with_kind(fields, name)
        };

        let kind = TaskKind::from_fields(&fields).unwrap();
        let attributes = kind.attributes();

        prop_assert_eq!(attributes.kind, name);
        prop_assert!(!(has_deadline(&attributes) && has_quota_target(&attributes)));
    }

    /// A committed submission is rejected naming the first required field it
    /// left out — the same field every time, whichever others are also
    /// absent, so the caller can fix them one at a time.
    #[test]
    #[ignore]
    fn a_committed_submission_is_rejected_naming_the_first_required_field_it_omits(
        fields in any_fields(),
        present in proptest::collection::vec(any::<bool>(), 3..=3),
    ) {
        let fields = TriageFields {
            deadline: present[0].then(|| "2026-08-20T17:00:00Z".to_string()),
            deadline_type: present[1].then(|| "hard".to_string()),
            priority: present[2].then(|| "P1".to_string()),
            ..with_kind(fields, COMMITTED)
        };

        let expected = [Field::Deadline, Field::DeadlineType, Field::Priority]
            .into_iter()
            .zip(&present)
            .find(|(_, supplied)| !**supplied)
            .map(|(field, _)| field);

        match expected {
            Some(field) => prop_assert_eq!(
                TaskKind::from_fields(&fields),
                Err(TriageRejection::MissingField(field))
            ),
            None => prop_assert!(TaskKind::from_fields(&fields).is_ok()),
        }
    }

    /// Anything that is not one of the three kinds is rejected, and the
    /// rejection repeats what was submitted so the caller can see the typo.
    #[test]
    #[ignore]
    fn a_kind_outside_the_three_is_rejected_and_reported_back(name in ".{0,40}") {
        prop_assume!(![POOL, COMMITTED, QUOTA].contains(&name.as_str()));

        let fields = TriageFields {
            kind: Some(name.clone()),
            ..TriageFields::default()
        };

        prop_assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::UnknownKind(Some(name)))
        );
    }

    /// Every value inside a closed domain survives triage unchanged. Paired
    /// with the rejection property below, the two pin each domain's exact
    /// membership from outside the core: everything listed is accepted and
    /// stored verbatim, everything else is refused.
    #[test]
    #[ignore]
    fn every_value_inside_a_closed_domain_round_trips(
        fields in any_fields(),
        deadline_type in prop::sample::select(&DEADLINE_TYPES[..]),
        priority in prop::sample::select(&PRIORITIES[..]),
        period in prop::sample::select(&PERIODS[..]),
    ) {
        let committed = TaskKind::from_fields(&TriageFields {
            deadline: Some(VALID_DEADLINE.to_string()),
            deadline_type: Some(deadline_type.to_string()),
            priority: Some(priority.to_string()),
            ..with_kind(fields.clone(), COMMITTED)
        })
        .unwrap()
        .attributes();
        prop_assert_eq!(committed.deadline_type, Some(deadline_type));
        prop_assert_eq!(committed.priority, Some(priority));

        let quota = TaskKind::from_fields(&TriageFields {
            target_count: Some(3),
            target_minutes_each: Some(45),
            period: Some(period.to_string()),
            ..with_kind(fields, QUOTA)
        })
        .unwrap()
        .attributes();
        prop_assert_eq!(quota.period, Some(period));
    }

    /// A present value outside its domain is rejected as invalid — naming
    /// that field, not some other one.
    #[test]
    #[ignore]
    fn a_value_outside_a_closed_domain_is_rejected_as_invalid(
        value in ".{1,40}",
        entry in prop::sample::select(closed_domain_fields()),
    ) {
        let domain = entry.domain.expect("only closed-domain fields are selected");
        prop_assume!(!domain.contains(&value.as_str()));

        prop_assert_eq!(
            TaskKind::from_fields(&entry.submission(Some(value))),
            Err(TriageRejection::InvalidField(entry.field))
        );
    }

    /// A non-positive quota target is rejected as invalid, whichever of the
    /// two count fields carries it — the reckoning's `count(done)/target`
    /// has no meaning at zero or below (folded in from the PR #31 review).
    #[test]
    #[ignore]
    fn a_non_positive_quota_target_is_rejected_as_invalid(
        bad_value in i64::MIN..=0,
        vary_target_count in any::<bool>(),
    ) {
        let mut fields = valid_quota();
        let expected_field = if vary_target_count {
            fields.target_count = Some(bad_value);
            Field::TargetCount
        } else {
            fields.target_minutes_each = Some(bad_value);
            Field::TargetMinutesEach
        };

        prop_assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::InvalidField(expected_field))
        );
    }

    /// A deadline that does not name a real instant is rejected as invalid
    /// (T-jiff-epoch-millis), however plausible its shape.
    #[test]
    #[ignore]
    fn a_deadline_that_names_no_instant_is_rejected_as_invalid(text in ".{1,40}") {
        prop_assume!(text.parse::<jiff::Timestamp>().is_err());

        prop_assert_eq!(
            TaskKind::from_fields(&REQUIRED_STRING_FIELDS[0].submission(Some(text))),
            Err(TriageRejection::InvalidField(Field::Deadline))
        );
    }

    /// T-empty-equals-absent: absent and empty report identically. Written as
    /// a comparison rather than as an expected value, so it stays true if the
    /// rejection for an absent field ever changes — the point is that the two
    /// agree.
    #[test]
    #[ignore]
    fn an_empty_required_field_is_rejected_exactly_like_an_absent_one(
        entry in prop::sample::select(REQUIRED_STRING_FIELDS.to_vec()),
    ) {
        let absent = TaskKind::from_fields(&entry.submission(None));
        let empty = TaskKind::from_fields(&entry.submission(Some(String::new())));

        prop_assert_eq!(absent, empty);
    }

    /// Required-before-valid: a submission that both omits one field and
    /// botches another names the omission. Reporting the invalid value first
    /// would send the caller to fix a field while a required one is still
    /// missing, so the next attempt fails again.
    #[test]
    #[ignore]
    fn a_missing_required_field_is_reported_before_an_invalid_one(garbage in ".{1,20}") {
        prop_assume!(!PRIORITIES.contains(&garbage.as_str()));

        let fields = TriageFields {
            kind: Some(COMMITTED.to_string()),
            deadline: None,
            deadline_type: Some("hard".to_string()),
            priority: Some(garbage),
            ..TriageFields::default()
        };

        prop_assert_eq!(
            TaskKind::from_fields(&fields),
            Err(TriageRejection::MissingField(Field::Deadline))
        );
    }

    /// Equivalent spellings of one instant store one deadline. The stored
    /// value is the instant, so timezone offset and sub-second precision are
    /// details of the text, not of the task.
    #[test]
    #[ignore]
    fn equivalent_textual_forms_of_an_instant_store_the_same_deadline(
        first in valid_deadline(),
        second in valid_deadline(),
    ) {
        prop_assume!(first.1 == second.1);

        let deadline_of = |text: &str| {
            TaskKind::from_fields(&TriageFields {
                kind: Some(COMMITTED.to_string()),
                deadline: Some(text.to_string()),
                deadline_type: Some("hard".to_string()),
                priority: Some("P1".to_string()),
                ..TriageFields::default()
            })
            .map(|kind| kind.attributes().deadline)
        };

        prop_assert_eq!(deadline_of(first.0), Ok(Some(first.1)));
        prop_assert_eq!(deadline_of(first.0), deadline_of(second.0));
    }
}
