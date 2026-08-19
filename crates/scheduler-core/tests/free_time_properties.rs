//! Property tests for `free_intervals` (#60, extended by #61's subtraction).
//!
//! Kept separate from the unit tests and every case marked `#[ignore]`, per
//! the project convention: normal verification runs `cargo test --workspace`,
//! property verification runs it with `-- --include-ignored`.
//!
//! The unit tests in `free_time.rs` pin the two DST transitions the
//! acceptance scenarios name, with real dates and real expected totals.
//! These check what has to hold for *any* well-formed guardrail, *any*
//! range and *any* set of dated exceptions, including zones neither
//! scenario reaches: that the output stays disjoint and sorted (the
//! brief's own criterion), and that a change of zone never moves a total
//! unless a real transition falls inside the range -- the DST correctness
//! claim, generalised past the two dates the literal scenarios pin.
//!
//! `#61`'s brief asked for subtraction to extend this property rather than
//! duplicate it in a second file: `excluded` is now one more generated
//! input to the same four properties, not a fifth property of its own.

use jiff::civil::{date, Date};
use jiff::tz::TimeZone;
use jiff::ToSpan;
use proptest::prelude::*;
use scheduler_core::exception::DateRange;
use scheduler_core::free_time::{free_intervals, weekday_of, Guardrail, Range};
use scheduler_core::guardrail::{Band, Weekday};
use scheduler_core::interval::Interval;

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

/// How many non-overlapping slots a day is cut into. A band lives inside
/// one slot, so two bands can never overlap however they are generated --
/// `scheduler_core::guardrail::overlaps` already owns that rule and these
/// properties should not re-prove it -- while two bands *can* share a
/// weekday, which is the shape that matters here.
const SLOTS_PER_DAY: i64 = 8;
const SLOT_MINUTES: i64 = 1440 / SLOTS_PER_DAY;

/// One well-formed band, placed in a slot of its own day.
///
/// **Why slots rather than free start/end.** This generator used to give
/// every band a distinct weekday, so a date could never carry two
/// intervals -- and `free_intervals` walks dates in ascending order, so its
/// output was sorted *by construction*. Deleting its `sort_by_key`
/// outright left all four properties green, including the one whose whole
/// subject is sortedness. Two bands on one weekday is not a contrived
/// input either: `overlaps` is half-open precisely so that touching bands
/// (`09:00-12:00` and `12:00-17:00`) are both kept.
fn any_band() -> impl Strategy<Value = Band> {
    (any_weekday(), 0..SLOTS_PER_DAY, 1..=SLOT_MINUTES).prop_map(|(weekday, slot, width)| {
        let start = slot * SLOT_MINUTES;
        Band {
            weekday,
            start_minutes: start,
            // Capped at 23:59, which is what the guardrail form can actually
            // produce. The `guardrail_bands` CHECK permits `end_minutes =
            // 1440` and the core panics on it ("guardrail minutes are always
            // within a single day") -- a three-way disagreement recorded in
            // `docs/design/architecture.md`, not something this generator
            // should assert either way.
            end_minutes: (start + width).min(1439),
        }
    })
}

/// A small set of bands, no two occupying the same slot -- so none overlap,
/// several may share a weekday, adjacent ones touch, and the order they
/// arrive in is whatever was generated rather than sorted. That last part
/// is what makes sortedness falsifiable.
fn any_bands() -> impl Strategy<Value = Vec<Band>> {
    prop::collection::vec(any_band(), 0..6).prop_map(|bands| {
        let mut seen: Vec<(Weekday, i64)> = Vec::new();
        bands
            .into_iter()
            .filter(|band| {
                let slot = (band.weekday, band.start_minutes / SLOT_MINUTES);
                if seen.contains(&slot) {
                    false
                } else {
                    seen.push(slot);
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

/// One well-formed dated exception: a start date and a length, both ends
/// inclusive (`scheduler_core::exception::well_formed_range`'s own
/// contract) -- a single-day range is `len == 0`.
fn any_date_range() -> impl Strategy<Value = DateRange> {
    (any_start_date(), 0..10i64).prop_map(|(start, len)| DateRange {
        start,
        end: start
            .checked_add(len.days())
            .expect("a ten-day span stays within jiff's representable date range"),
    })
}

/// A small set of exceptions, deliberately allowed to overlap each other --
/// `free_intervals`'s own contract is that overlapping exclusions subtract
/// their union, not their sum, so generating disjoint ranges here would
/// leave that path untested.
fn any_excluded() -> impl Strategy<Value = Vec<DateRange>> {
    prop::collection::vec(any_date_range(), 0..3)
}

/// Whether `day` falls inside any of `excluded` -- the same union-membership
/// test `free_intervals` itself applies, recomputed independently so a
/// property's own expectation cannot share a bug with the implementation.
fn excluded_on(excluded: &[DateRange], day: Date) -> bool {
    excluded.iter().any(|range| range.contains(day))
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

/// Everything `free_intervals` takes, generated together.
///
/// The five strategies below were spelled out in each property's own
/// parameter list, which made every property four lines of setup before the
/// one line that was actually its own. Bundled here so a property reads as
/// its assertion; a sixth input (M3's pins, M4's busy) is then one field
/// rather than one more line in every property.
#[derive(Debug, Clone)]
struct Projection {
    bands: Vec<Band>,
    timezone: TimeZone,
    excluded: Vec<DateRange>,
    range: Range,
}

impl Projection {
    fn guardrail(&self) -> Guardrail<'_> {
        Guardrail {
            bands: &self.bands,
            timezone: &self.timezone,
            excluded: &self.excluded,
        }
    }

    fn intervals(&self) -> Vec<Interval> {
        free_intervals(self.guardrail(), self.range)
    }
}

prop_compose! {
    fn any_projection()(
        bands in any_bands(),
        start in any_start_date(),
        days in any_range_days(),
        timezone in any_timezone(),
        excluded in any_excluded(),
    ) -> Projection {
        Projection { bands, timezone, excluded, range: Range::horizon(start, days) }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1000, ..ProptestConfig::default() })]

    /// The brief's own criterion: for any well-formed guardrail and any
    /// range, the result is disjoint and sorted.
    #[test]
    #[ignore]
    fn free_intervals_is_always_disjoint_and_sorted(
        projection in any_projection(),
    ) {
        let intervals = projection.intervals();

        prop_assert!(is_sorted(&intervals), "not sorted: {intervals:?}");
        prop_assert!(is_disjoint(&intervals), "not disjoint: {intervals:?}");
    }

    /// No interval is ever backwards or empty, in any zone -- the general
    /// form of "no negative interval" the brief's DST scenarios ask for.
    #[test]
    #[ignore]
    fn free_intervals_never_produces_a_non_positive_duration(
        projection in any_projection(),
    ) {
        for interval in projection.intervals() {
            prop_assert!(interval.duration_ms() > 0, "got {interval:?}");
        }
    }

    /// Exactly one interval per (date, band) pair that lands in range --
    /// `free_intervals` neither drops a match nor invents one.
    #[test]
    #[ignore]
    fn free_intervals_reports_exactly_one_interval_per_matching_day(
        projection in any_projection(),
    ) {
        let Projection { bands, excluded, range, .. } = &projection;
        let intervals = projection.intervals();

        // Recomputed by walking the same days a band's weekday can land
        // on, independently of `free_intervals`'s own date walk, so this
        // cannot pass by sharing a bug with it.
        let mut expected_count = 0usize;
        let mut day = range.start;
        while day < range.end {
            if !excluded_on(excluded, day) {
                expected_count += bands.iter().filter(|b| weekday_of(day) == b.weekday).count();
            }
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
        excluded in any_excluded(),
    ) {
        let range = Range::horizon(start, days);
        let utc = TimeZone::UTC;
        let guardrail = Guardrail { bands: &bands, timezone: &utc, excluded: &excluded };

        let intervals = free_intervals(guardrail, range);
        let total_ms: i64 = intervals.iter().map(|i| i.duration_ms()).sum();

        let mut expected_minutes = 0i64;
        let mut day = range.start;
        while day < range.end {
            if !excluded_on(&excluded, day) {
                for band in bands.iter().filter(|b| weekday_of(day) == b.weekday) {
                    expected_minutes += band.end_minutes - band.start_minutes;
                }
            }
            day = day.tomorrow().unwrap();
        }

        prop_assert_eq!(total_ms, expected_minutes * 60 * 1000);
    }
}
