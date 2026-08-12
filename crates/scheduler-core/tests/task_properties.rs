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

/// A handful of well-formed deadlines and the instant each names, since
/// deadline validity (T3) is exercised by the unit tests — this generator
/// only needs enough variety to keep the round-trip property honest.
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
    /// deadline. T17 requires all three target fields at triage, so unlike
    /// the pool case this generates only valid, complete targets.
    #[test]
    #[ignore]
    fn a_quota_task_round_trips_its_target_and_carries_no_deadline(
        fields in any_fields(),
        target_count in any::<i64>(),
        target_minutes_each in any::<i64>(),
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
}
