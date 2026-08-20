//! The closed-domain vocabulary a triage submission's fields are drawn
//! from: which field a rejection can name, and the few columns
//! (`deadline_type`, `priority`, `period`) whose values are a fixed set
//! rather than free text. Split out from [`super`], which composes these
//! into the triage decision itself -- these types carry no decision logic
//! of their own, only parsing and the name each reports back.

/// A field a triage submission must supply, or supply a valid value for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Deadline,
    DeadlineType,
    Priority,
    EstimatedMinutes,
    TargetCount,
    TargetMinutesEach,
    Period,
}

/// Every field paired with the name reported back to whoever submitted the
/// triage -- a table, not a match, for the reason a fixed lookup table
/// already is one (`T-complexity-8`: seven arms of no logic beyond the
/// lookup should not cost against the threshold).
const FIELD_NAMES: [(Field, &str); 7] = [
    (Field::Deadline, "deadline"),
    (Field::DeadlineType, "deadline_type"),
    (Field::Priority, "priority"),
    (Field::EstimatedMinutes, "estimated_minutes"),
    (Field::TargetCount, "target_count"),
    (Field::TargetMinutesEach, "target_minutes_each"),
    (Field::Period, "period"),
];

impl Field {
    /// The name reported back to whoever submitted the triage.
    pub fn name(self) -> &'static str {
        FIELD_NAMES
            .iter()
            .find(|(field, _)| *field == self)
            .map(|(_, name)| *name)
            .expect("every Field variant is listed in FIELD_NAMES")
    }
}

/// `deadline_type`'s closed domain (D-guardrails-never-yield: the M3
/// scheduler branches on this field, so it cannot carry an undefined value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineType {
    Hard,
    Soft,
}

impl DeadlineType {
    /// `pub` because a stored committed task's `deadline_type` column comes
    /// back as text: #75's schedule reads it back to reconstruct the task
    /// it places, the same reason `Period::parse` is `pub` for a stored
    /// quota row's `period`.
    pub fn parse(value: &str) -> Option<Self> {
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
    /// `pub` for the same reason as [`DeadlineType::parse`]: #75's schedule
    /// reads a stored committed task's `priority` column back as text.
    pub fn parse(value: &str) -> Option<Self> {
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

/// `period`'s closed domain (T-period-closed-set): the same M8 cadence-math
/// reason as [`DeadlineType`] and [`Priority`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Week,
    Month,
}

impl Period {
    /// `pub` because a stored quota row's `period` column comes back as
    /// text: `#62`'s capacity number reads it back into this type to
    /// prorate demand, the same reason `guardrail::Weekday::parse` is
    /// `pub` for a stored band's weekday.
    pub fn parse(value: &str) -> Option<Self> {
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

/// Parses a deadline to UTC epoch milliseconds (T-jiff-epoch-millis). Rejects
/// anything that does not name a real instant — a syntactically plausible but
/// invalid timestamp (`2026-13-45T99:99:99Z`) fails the same as free text.
pub(super) fn parse_deadline_ms(value: &str) -> Option<i64> {
    value
        .parse::<jiff::Timestamp>()
        .ok()
        .map(|ts| ts.as_millisecond())
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
}
