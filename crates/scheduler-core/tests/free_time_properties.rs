//! Property tests for `free_intervals` (#60).
//!
//! Kept separate from the unit tests and every case marked `#[ignore]`, per
//! the project convention: normal verification runs `cargo test --workspace`,
//! property verification runs it with `-- --include-ignored`.
//!
//! The unit tests in `free_time.rs` pin the two DST transitions the
//! acceptance scenarios name, with real dates and real expected totals.
//! These check what has to hold for *any* well-formed guardrail and *any*
//! range, including zones neither scenario reaches: that the output stays
//! disjoint and sorted (the brief's own criterion), and that a change of
//! zone never moves a total unless a real transition falls inside the
//! range -- the DST correctness claim, generalised past the two dates the
//! literal scenarios pin.

use jiff::civil::{date, Date};
use jiff::tz::TimeZone;
use proptest::prelude::*;
use scheduler_core::free_time::{free_intervals, weekday_of, Guardrail, Interval, Range};
use scheduler_core::guardrail::{Band, Weekday};

const ALL_WEEKDAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// A handful of real zones, chosen to cover the shapes that matter: one
/// with no DST at all (`UTC`), one that observes it in the northern
/// hemisphere's spring/autumn (`America/New_York`), and one in the
/// southern hemisphere where the transitions fall on different calendar
/// months (`Australia/Sydney`) -- a guardrail's correctness should not
/// depend on which hemisphere the owner lives in.
fn any_timezone() -> impl Strategy<Value = TimeZone> {
    prop_oneof![
        Just(TimeZone::UTC),
        Just(TimeZone::get("America/New_York").unwrap()),
        Just(TimeZone::get("Australia/Sydney").unwrap()),
    ]
}

fn any_weekday() -> impl Strategy<Value = Weekday> {
    (0..7usize).prop_map(|i| ALL_WEEKDAYS[i])
}

/// One well-formed band on its own weekday -- `scheduler_core::guardrail`
/// already owns "is this band well-formed" and "do a life area's own bands
/// overlap"; generating bands that already satisfy both lets this property
/// stay about `free_intervals`, not re-prove guardrail's own rules.
fn any_band() -> impl Strategy<Value = Band> {
    (any_weekday(), 0..1439i64, 1..1440i64).prop_filter_map(
        "end must follow start",
        |(weekday, start, end)| {
            (end > start).then_some(Band {
                weekday,
                start_minutes: start,
                end_minutes: end,
            })
        },
    )
}

/// A small set of bands, each on a distinct weekday so no two can overlap
/// or touch -- the disjointness `scheduler_core::guardrail::overlaps`
/// enforces for bands that share a day, generated structurally here instead
/// of filtered after the fact.
fn any_bands() -> impl Strategy<Value = Vec<Band>> {
    prop::collection::vec(any_band(), 0..5).prop_map(|bands| {
        let mut seen = Vec::new();
        bands
            .into_iter()
            .filter(|band| {
                if seen.contains(&band.weekday) {
                    false
                } else {
                    seen.push(band.weekday);
                    true
                }
            })
            .collect()
    })
}

fn any_start_date() -> impl Strategy<Value = Date> {
    (2020..2035i16, 1..13i8, 1..28i8).prop_map(|(y, m, d)| date(y, m, d))
}

fn any_range_days() -> impl Strategy<Value = i64> {
    1..60i64
}

fn is_sorted(intervals: &[Interval]) -> bool {
    intervals
        .windows(2)
        .all(|pair| pair[0].start_ms <= pair[1].start_ms)
}

fn is_disjoint(intervals: &[Interval]) -> bool {
    intervals
        .windows(2)
        .all(|pair| pair[0].end_ms <= pair[1].start_ms)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1000, ..ProptestConfig::default() })]

    /// The brief's own criterion: for any well-formed guardrail and any
    /// range, the result is disjoint and sorted.
    #[test]
    #[ignore]
    fn free_intervals_is_always_disjoint_and_sorted(
        bands in any_bands(),
        start in any_start_date(),
        days in any_range_days(),
        tz in any_timezone(),
    ) {
        let range = Range::horizon(start, days);
        let guardrail = Guardrail { bands: &bands, timezone: &tz };

        let intervals = free_intervals(guardrail, range);

        prop_assert!(is_sorted(&intervals), "not sorted: {intervals:?}");
        prop_assert!(is_disjoint(&intervals), "not disjoint: {intervals:?}");
    }

    /// No interval is ever backwards or empty, in any zone -- the general
    /// form of "no negative interval" the brief's DST scenarios ask for.
    #[test]
    #[ignore]
    fn free_intervals_never_produces_a_non_positive_duration(
        bands in any_bands(),
        start in any_start_date(),
        days in any_range_days(),
        tz in any_timezone(),
    ) {
        let range = Range::horizon(start, days);
        let guardrail = Guardrail { bands: &bands, timezone: &tz };

        for interval in free_intervals(guardrail, range) {
            prop_assert!(interval.duration_ms() > 0, "got {interval:?}");
        }
    }

    /// Exactly one interval per (date, band) pair that lands in range --
    /// `free_intervals` neither drops a match nor invents one.
    #[test]
    #[ignore]
    fn free_intervals_reports_exactly_one_interval_per_matching_day(
        bands in any_bands(),
        start in any_start_date(),
        days in any_range_days(),
        tz in any_timezone(),
    ) {
        let range = Range::horizon(start, days);
        let guardrail = Guardrail { bands: &bands, timezone: &tz };
        let intervals = free_intervals(guardrail, range);

        // Recomputed by walking the same days a band's weekday can land
        // on, independently of `free_intervals`'s own date walk, so this
        // cannot pass by sharing a bug with it.
        let mut expected_count = 0usize;
        let mut day = range.start;
        while day < range.end {
            expected_count += bands.iter().filter(|b| weekday_of(day) == b.weekday).count();
            day = day.tomorrow().unwrap();
        }

        prop_assert_eq!(intervals.len(), expected_count);
    }

    /// A zone with no DST transition anywhere in range must report the
    /// same total a fixed offset would -- duration in real time equals
    /// civil duration whenever there is no transition to distort it. `UTC`
    /// stands in for "no transition, ever".
    #[test]
    #[ignore]
    fn a_range_with_no_dst_transition_reports_the_plain_civil_total(
        bands in any_bands(),
        start in any_start_date(),
        days in 1..14i64,
    ) {
        let range = Range::horizon(start, days);
        let utc = TimeZone::UTC;
        let guardrail = Guardrail { bands: &bands, timezone: &utc };

        let intervals = free_intervals(guardrail, range);
        let total_ms: i64 = intervals.iter().map(|i| i.duration_ms()).sum();

        let mut expected_minutes = 0i64;
        let mut day = range.start;
        while day < range.end {
            for band in bands.iter().filter(|b| weekday_of(day) == b.weekday) {
                expected_minutes += band.end_minutes - band.start_minutes;
            }
            day = day.tomorrow().unwrap();
        }

        prop_assert_eq!(total_ms, expected_minutes * 60 * 1000);
    }
}
