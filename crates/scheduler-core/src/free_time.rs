//! `free_intervals(guardrail, range)` -- projecting a life area's weekly
//! civil guardrail across a date range into disjoint, sorted free intervals
//! (#60), then subtracting whatever dated exceptions apply (#61). Correct
//! across DST gaps and folds is the point: a naive civil subtraction hides
//! exactly the case `T-jiff-epoch-millis` exists to force into the open.
//!
//! **Exceptions are the first subtrahend; pins and calendar busy are not
//! here yet.** Pins arrive at M3, calendar busy at M4. `excluded` is shaped
//! so both are additional inputs of the same kind rather than a second
//! mechanism -- `Interval` was never named after a producer for exactly this
//! reason.

use crate::exception::DateRange;
use crate::guardrail::{Band, Weekday};
use jiff::civil::{Date, DateTime, Time};
use jiff::tz::{Disambiguation, TimeZone};
use jiff::ToSpan;

/// One free interval, as the instant span it actually covers -- UTC epoch
/// milliseconds (`T-jiff-epoch-millis`). An interval is what a scheduler
/// could place work into, and that is an instant question: two intervals
/// that are adjacent or overlapping only make sense compared as instants,
/// not as civil clock readings that a DST transition can reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub start_ms: i64,
    pub end_ms: i64,
}

impl Interval {
    pub fn duration_ms(self) -> i64 {
        self.end_ms - self.start_ms
    }
}

/// A window of civil dates to project a guardrail across: `start`
/// inclusive, `end` exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Date,
    pub end: Date,
}

impl Range {
    /// `days` civil dates starting at `start`. The horizon length is the
    /// caller's own choice, not a constant hidden in here -- see the
    /// free-time brief's open question 2: `/stats`'s fortnight and this
    /// one's are the same number by coincidence, not by rule, so neither
    /// module owns a constant the other reads.
    pub fn horizon(start: Date, days: i64) -> Self {
        Range {
            start,
            end: start
                .checked_add(days.days())
                .expect("a horizon of a few weeks stays within jiff's representable date range"),
        }
    }

    fn dates(self) -> Vec<Date> {
        let mut dates = Vec::new();
        let mut current = self.start;
        while current < self.end {
            dates.push(current);
            current = current
                .tomorrow()
                .expect("a horizon of a few weeks stays within jiff's representable date range");
        }
        dates
    }
}

/// A life area's weekly mask, the zone its civil times are read in, and the
/// dated exceptions that narrow it -- bundled together because a band with
/// no zone is not yet anything a scheduler could place work into, and this
/// is the one place `free_intervals` needs all three at once.
pub struct Guardrail<'a> {
    pub bands: &'a [Band],
    pub timezone: &'a TimeZone,
    pub excluded: &'a [DateRange],
}

/// Every `jiff` weekday paired with this crate's own -- a table, not a
/// match, for the reason `guardrail::WEEKDAYS` already is one
/// (`T-complexity-8`: seven arms of no logic beyond the lookup should not
/// cost against the threshold).
const JIFF_WEEKDAYS: [(jiff::civil::Weekday, Weekday); 7] = [
    (jiff::civil::Weekday::Monday, Weekday::Mon),
    (jiff::civil::Weekday::Tuesday, Weekday::Tue),
    (jiff::civil::Weekday::Wednesday, Weekday::Wed),
    (jiff::civil::Weekday::Thursday, Weekday::Thu),
    (jiff::civil::Weekday::Friday, Weekday::Fri),
    (jiff::civil::Weekday::Saturday, Weekday::Sat),
    (jiff::civil::Weekday::Sunday, Weekday::Sun),
];

/// Which of this crate's own [`Weekday`] variants `date` falls on. `pub`
/// because the property tests in `tests/free_time_properties.rs` need the
/// same jiff-to-`Weekday` conversion `free_intervals` uses internally to
/// independently recompute an expected count -- reimplementing it there
/// would be a second, untested statement of the same table.
pub fn weekday_of(date: Date) -> Weekday {
    JIFF_WEEKDAYS
        .iter()
        .find(|(jiff_day, _)| *jiff_day == date.weekday())
        .map(|(_, day)| *day)
        .expect("jiff names all seven weekdays, and the table lists all seven")
}

/// `minutes` (0..1440, always true of a band that passed
/// `scheduler_core::guardrail`'s own well-formedness check) as a time of
/// day.
fn time_at_minutes(minutes: i64) -> Time {
    let hour = (minutes / 60) as i8;
    let minute = (minutes % 60) as i8;
    Time::new(hour, minute, 0, 0).expect("guardrail minutes are always within a single day")
}

/// A civil `(date, minutes)` reading resolved to the instant it names, under
/// `disambiguation` for the rare reading a DST transition makes ambiguous.
fn to_instant_ms(tz: &TimeZone, date: Date, minutes: i64, disambiguation: Disambiguation) -> i64 {
    let dt = DateTime::from_parts(date, time_at_minutes(minutes));
    tz.to_ambiguous_zoned(dt)
        .disambiguate(disambiguation)
        .expect("a datetime built from a valid date and time always resolves to some instant")
        .timestamp()
        .as_millisecond()
}

/// One band projected onto one civil date.
///
/// **The fold decision lives here, and only here.** A civil reading inside
/// a fall-back fold names two real instants; this resolves a band's start to
/// the *earlier* of the two and its end to the *later* of the two, so the
/// interval spans the full repeated hour rather than picking one occurrence
/// arbitrarily. Recorded in `docs/decisions.md` because `T-jiff-epoch-millis`
/// says a fold is exactly the kind of thing that must be an explicit
/// decision, not an accident of whichever offset a library tries first — and
/// once made, it is invisible in the numbers this function returns.
///
/// A gap needs no matching decision: `Disambiguation::Earlier` and `::Later`
/// each resolve a gap the same way (the reading that would have existed
/// slides to the valid instant nearest it), so the interval's start and end
/// land on real instants regardless, and subtracting them as instants -- not
/// as civil minutes -- is what makes the missing hour disappear on its own.
fn band_interval(tz: &TimeZone, date: Date, band: Band) -> Interval {
    Interval {
        start_ms: to_instant_ms(tz, date, band.start_minutes, Disambiguation::Earlier),
        end_ms: to_instant_ms(tz, date, band.end_minutes, Disambiguation::Later),
    }
}

/// Projects `guardrail`'s weekly mask across `range`, subtracting whichever
/// whole civil dates `excluded` names, and returning the result as disjoint,
/// sorted intervals.
///
/// **Subtraction is whole-date, not instant-ranged**: an excluded date
/// contributes no interval at all rather than a zero-length one -- a
/// zero-length interval is something a scheduler could still try to place
/// work into, and whole dates is this slice's own scope (`exceptions`
/// brief, open question 4). A date excluded by more than one overlapping
/// range is still skipped exactly once: membership in the union of
/// `excluded`, not a count, is all `free_intervals` ever asks.
///
/// Disjoint and sorted both follow from what is guaranteed upstream rather
/// than being enforced here: `scheduler_core::guardrail::overlaps` already
/// refuses a life area's own bands touching-but-not-overlapping is allowed,
/// so two bands can never produce intervals on the same date that overlap,
/// and different dates never overlap by construction. The one thing this
/// function still owes is the sort, since bands are not walked in start-time
/// order.
pub fn free_intervals(guardrail: Guardrail<'_>, range: Range) -> Vec<Interval> {
    let mut intervals: Vec<Interval> = Vec::new();
    for date in range.dates() {
        if guardrail
            .excluded
            .iter()
            .any(|excluded| excluded.contains(date))
        {
            continue;
        }
        let weekday = weekday_of(date);
        for band in guardrail
            .bands
            .iter()
            .filter(|band| band.weekday == weekday)
        {
            intervals.push(band_interval(guardrail.timezone, date, *band));
        }
    }
    intervals.sort_by_key(|interval| interval.start_ms);
    intervals
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn band(weekday: Weekday, start: i64, end: i64) -> Band {
        Band {
            weekday,
            start_minutes: start,
            end_minutes: end,
        }
    }

    #[test]
    fn range_horizon_spans_exactly_the_requested_days() {
        let range = Range::horizon(date(2026, 1, 1), 14);
        assert_eq!(range.dates().len(), 14);
        assert_eq!(range.dates()[0], date(2026, 1, 1));
        assert_eq!(range.dates()[13], date(2026, 1, 14));
    }

    #[test]
    fn free_intervals_is_empty_for_no_bands() {
        let tz = TimeZone::UTC;
        let guardrail = Guardrail {
            bands: &[],
            timezone: &tz,
            excluded: &[],
        };
        let range = Range::horizon(date(2026, 8, 24), 14);

        assert_eq!(free_intervals(guardrail, range), Vec::new());
    }

    #[test]
    fn free_intervals_reports_one_interval_per_matching_date_in_range() {
        let tz = TimeZone::UTC;
        let bands = [band(Weekday::Mon, 9 * 60, 17 * 60)];
        let guardrail = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &[],
        };
        // 2026-08-24 is a Monday; a 14-day horizon holds exactly two.
        let range = Range::horizon(date(2026, 8, 24), 14);

        let intervals = free_intervals(guardrail, range);

        assert_eq!(intervals.len(), 2);
        let total_ms: i64 = intervals.iter().map(|i| i.duration_ms()).sum();
        assert_eq!(total_ms, 2 * 8 * 60 * 60 * 1000);
    }

    #[test]
    fn free_intervals_is_sorted_by_start() {
        let tz = TimeZone::UTC;
        let bands = [
            band(Weekday::Wed, 9 * 60, 10 * 60),
            band(Weekday::Mon, 9 * 60, 10 * 60),
        ];
        let guardrail = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &[],
        };
        let range = Range::horizon(date(2026, 8, 24), 7);

        let intervals = free_intervals(guardrail, range);

        let starts: Vec<i64> = intervals.iter().map(|i| i.start_ms).collect();
        let mut sorted = starts.clone();
        sorted.sort();
        assert_eq!(starts, sorted);
    }

    #[test]
    fn utc_reports_the_same_totals_a_zone_with_no_dst_transition_that_week_would() {
        // A guardrail entirely in UTC has no DST to distort, so a change of
        // zone away from UTC to one with no transition in range must not
        // change the total.
        let tz_utc = TimeZone::UTC;
        let tz_tokyo = TimeZone::get("Asia/Tokyo").unwrap(); // observes no DST
        let bands = [band(Weekday::Mon, 9 * 60, 17 * 60)];
        let range = Range::horizon(date(2026, 8, 24), 14);

        let utc_total: i64 = free_intervals(
            Guardrail {
                bands: &bands,
                timezone: &tz_utc,
                excluded: &[],
            },
            range,
        )
        .iter()
        .map(|i| i.duration_ms())
        .sum();
        let tokyo_total: i64 = free_intervals(
            Guardrail {
                bands: &bands,
                timezone: &tz_tokyo,
                excluded: &[],
            },
            range,
        )
        .iter()
        .map(|i| i.duration_ms())
        .sum();

        assert_eq!(utc_total, tokyo_total);
    }

    /// `2027-03-14` is the US spring-forward transition; `01:00-04:00`
    /// spans it and loses exactly the hour `02:00-03:00` that never
    /// happens.
    #[test]
    fn a_band_spanning_a_spring_forward_transition_loses_the_missing_hour() {
        let tz = TimeZone::get("America/New_York").unwrap();
        let bands = [band(Weekday::Sun, 60, 4 * 60)];
        let guardrail = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &[],
        };
        let range = Range::horizon(date(2027, 3, 14), 1);

        let intervals = free_intervals(guardrail, range);

        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].duration_ms(), 2 * 60 * 60 * 1000);
    }

    /// `2027-11-07` is the US fall-back transition; `01:00-04:00` spans it
    /// and gains the `01:00-02:00` hour that happens twice.
    #[test]
    fn a_band_spanning_a_fall_back_transition_gains_the_repeated_hour() {
        let tz = TimeZone::get("America/New_York").unwrap();
        let bands = [band(Weekday::Sun, 60, 4 * 60)];
        let guardrail = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &[],
        };
        let range = Range::horizon(date(2027, 11, 7), 1);

        let intervals = free_intervals(guardrail, range);

        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].duration_ms(), 4 * 60 * 60 * 1000);
    }

    #[test]
    fn no_interval_has_a_negative_or_zero_duration_across_either_transition() {
        let tz = TimeZone::get("America/New_York").unwrap();
        let bands = [band(Weekday::Sun, 60, 4 * 60)];
        for start in [date(2027, 3, 14), date(2027, 11, 7)] {
            let guardrail = Guardrail {
                bands: &bands,
                timezone: &tz,
                excluded: &[],
            };
            let range = Range::horizon(start, 1);
            for interval in free_intervals(guardrail, range) {
                assert!(interval.duration_ms() > 0, "got {interval:?} for {start}");
            }
        }
    }

    #[test]
    fn an_excluded_range_removes_exactly_the_dates_it_covers() {
        let tz = TimeZone::UTC;
        let bands = [band(Weekday::Mon, 9 * 60, 17 * 60)];
        // 2026-08-24 is a Monday; a 14-day horizon holds two (24 and 31).
        let range = Range::horizon(date(2026, 8, 24), 14);
        let excluded = [DateRange {
            start: date(2026, 8, 24),
            end: date(2026, 8, 24),
        }];
        let guardrail = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &excluded,
        };

        let intervals = free_intervals(guardrail, range);

        assert_eq!(intervals.len(), 1);
        let total_ms: i64 = intervals.iter().map(|i| i.duration_ms()).sum();
        assert_eq!(total_ms, 8 * 60 * 60 * 1000);
    }

    /// The excluded day contributes no interval at all -- a zero-length one
    /// would still be something a scheduler could try to place work into.
    #[test]
    fn an_excluded_date_produces_no_interval_rather_than_a_zero_length_one() {
        let tz = TimeZone::UTC;
        let bands = [band(Weekday::Mon, 9 * 60, 17 * 60)];
        let range = Range::horizon(date(2026, 8, 24), 1);
        let excluded = [DateRange {
            start: date(2026, 8, 24),
            end: date(2026, 8, 24),
        }];
        let guardrail = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &excluded,
        };

        assert_eq!(free_intervals(guardrail, range), Vec::new());
    }

    /// An exception on days the guardrail does not cover removes nothing --
    /// not an error, not a no-op to reject, simply a true statement about
    /// empty days (`exceptions` QA doc).
    #[test]
    fn an_excluded_range_covering_no_guardrail_day_changes_nothing() {
        let tz = TimeZone::UTC;
        let bands = [band(Weekday::Mon, 9 * 60, 17 * 60)];
        let range = Range::horizon(date(2026, 8, 24), 14);
        // 2026-08-22/23 is the weekend before the Monday band.
        let excluded = [DateRange {
            start: date(2026, 8, 22),
            end: date(2026, 8, 23),
        }];
        let with_exclusion = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &excluded,
        };
        let without_exclusion = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &[],
        };

        assert_eq!(
            free_intervals(with_exclusion, range),
            free_intervals(without_exclusion, range)
        );
    }

    /// An exception entirely outside the horizon changes nothing -- it is
    /// still a real, stored exception, just not one this projection reaches.
    #[test]
    fn an_excluded_range_entirely_outside_the_horizon_changes_nothing() {
        let tz = TimeZone::UTC;
        let bands = [band(Weekday::Mon, 9 * 60, 17 * 60)];
        let range = Range::horizon(date(2026, 8, 24), 14);
        let excluded = [DateRange {
            start: date(2026, 9, 10),
            end: date(2026, 9, 20),
        }];
        let with_exclusion = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &excluded,
        };
        let without_exclusion = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &[],
        };

        assert_eq!(
            free_intervals(with_exclusion, range),
            free_intervals(without_exclusion, range)
        );
    }

    /// Two overlapping exceptions subtract their union, not their sum --
    /// 24-28 and 26-29 August overlap on 26-28, so together they remove
    /// 24-29 (six days, of which 24-28 are the five weekdays a Mon-Fri
    /// band covers; 29 is a Saturday it does not).
    #[test]
    fn overlapping_excluded_ranges_subtract_their_union_not_twice() {
        let tz = TimeZone::UTC;
        let bands = [band(Weekday::Mon, 9 * 60, 17 * 60)];
        let range = Range::horizon(date(2026, 8, 24), 14);
        let excluded = [
            DateRange {
                start: date(2026, 8, 24),
                end: date(2026, 8, 28),
            },
            DateRange {
                start: date(2026, 8, 26),
                end: date(2026, 8, 29),
            },
        ];
        let guardrail = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &excluded,
        };

        let intervals = free_intervals(guardrail, range);

        // Only 31 August's Monday band survives.
        assert_eq!(intervals.len(), 1);
        let total_ms: i64 = intervals.iter().map(|i| i.duration_ms()).sum();
        assert_eq!(total_ms, 8 * 60 * 60 * 1000);
    }

    /// Excluding the *ordinary* Sunday leaves the transition Sunday's own
    /// odd length intact -- excluding the transition day instead would
    /// leave a plain three-hour Sunday and prove nothing about subtraction
    /// interacting with the fold correctly (`exceptions` QA doc).
    #[test]
    fn excluding_the_ordinary_sunday_leaves_the_fall_back_transition_day_at_its_true_length() {
        let tz = TimeZone::get("America/New_York").unwrap();
        let bands = [band(Weekday::Sun, 60, 4 * 60)];
        // 2027-11-07 is the transition Sunday; 2027-11-14 is the ordinary one.
        let range = Range::horizon(date(2027, 11, 6), 14);
        let excluded = [DateRange {
            start: date(2027, 11, 14),
            end: date(2027, 11, 14),
        }];
        let guardrail = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &excluded,
        };

        let intervals = free_intervals(guardrail, range);

        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].duration_ms(), 4 * 60 * 60 * 1000);
    }

    /// The spring-forward mirror of the above: excluding the ordinary
    /// Sunday leaves the transition Sunday's two-hour span intact.
    #[test]
    fn excluding_the_ordinary_sunday_leaves_the_spring_forward_transition_day_at_its_true_length() {
        let tz = TimeZone::get("America/New_York").unwrap();
        let bands = [band(Weekday::Sun, 60, 4 * 60)];
        // 2027-03-14 is the transition Sunday; 2027-03-21 is the ordinary one.
        let range = Range::horizon(date(2027, 3, 13), 14);
        let excluded = [DateRange {
            start: date(2027, 3, 21),
            end: date(2027, 3, 21),
        }];
        let guardrail = Guardrail {
            bands: &bands,
            timezone: &tz,
            excluded: &excluded,
        };

        let intervals = free_intervals(guardrail, range);

        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].duration_ms(), 2 * 60 * 60 * 1000);
    }
}
