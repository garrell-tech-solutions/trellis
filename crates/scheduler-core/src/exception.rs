//! A dated exception: how the owner narrows a guardrail for specific civil
//! dates -- never widens it (`D-guardrails-never-yield`, #61). Whether a
//! submitted range is well-formed lives here for the same reason
//! `scheduler_core::guardrail` owns a band's: it survives changing HTTP, and
//! `#60`'s free-interval algebra reads what this module writes.

use jiff::civil::Date;

/// A closed civil-date range, both ends inclusive -- the owner names
/// "24 August to 28 August" as five whole days, not a half-open span
/// (`exceptions` brief, open question 4: whole dates only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub start: Date,
    pub end: Date,
}

impl DateRange {
    /// Whether `date` falls within this range, both ends inclusive.
    pub fn contains(self, date: Date) -> bool {
        self.start <= date && date <= self.end
    }
}

/// Why an exception submission was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionRejection {
    /// The last day named precedes the first.
    Backwards,
}

/// The only way a dated exception can be malformed: its last day precedes
/// its first. `start == end` is a well-formed single-day exception.
pub fn well_formed_range(start: Date, end: Date) -> Result<DateRange, ExceptionRejection> {
    if end < start {
        return Err(ExceptionRejection::Backwards);
    }
    Ok(DateRange { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn well_formed_range_accepts_a_single_day() {
        let day = date(2026, 8, 24);
        assert_eq!(
            well_formed_range(day, day),
            Ok(DateRange {
                start: day,
                end: day
            })
        );
    }

    #[test]
    fn well_formed_range_accepts_an_ordinary_span() {
        let start = date(2026, 8, 24);
        let end = date(2026, 8, 28);
        assert_eq!(well_formed_range(start, end), Ok(DateRange { start, end }));
    }

    #[test]
    fn well_formed_range_rejects_an_end_before_its_start() {
        let start = date(2026, 8, 28);
        let end = date(2026, 8, 24);
        assert_eq!(
            well_formed_range(start, end),
            Err(ExceptionRejection::Backwards)
        );
    }

    #[test]
    fn contains_is_true_at_both_inclusive_ends() {
        let range = DateRange {
            start: date(2026, 8, 24),
            end: date(2026, 8, 28),
        };
        assert!(range.contains(date(2026, 8, 24)));
        assert!(range.contains(date(2026, 8, 28)));
        assert!(range.contains(date(2026, 8, 26)));
    }

    #[test]
    fn contains_is_false_just_outside_either_end() {
        let range = DateRange {
            start: date(2026, 8, 24),
            end: date(2026, 8, 28),
        };
        assert!(!range.contains(date(2026, 8, 23)));
        assert!(!range.contains(date(2026, 8, 29)));
    }

    #[test]
    fn a_single_day_ranges_contains_is_true_only_for_that_day() {
        let day = date(2026, 8, 24);
        let range = DateRange {
            start: day,
            end: day,
        };
        assert!(range.contains(day));
        assert!(!range.contains(date(2026, 8, 23)));
        assert!(!range.contains(date(2026, 8, 25)));
    }
}
