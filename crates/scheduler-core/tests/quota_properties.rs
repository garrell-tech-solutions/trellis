//! Property tests for `scheduler_core::quota`'s week arithmetic (#93,
//! `D-quota-no-rollover`, `D-logging-is-retrospective-and-separate`).
//!
//! `Week` and `Weekday` decide two things the screen cannot recover from
//! getting wrong: **which days you may log against**, and **which instant a
//! logged day is stored as**. The examples pin a Tuesday in New York. What
//! they cannot pin is the interaction that makes this hard -- a timezone.
//!
//! **`day_ms` writes and `weekday_of` reads the same column**, and a day
//! that survives one and not the other is a session that silently moves.
//! The pair is exercised here across zones chosen for their transitions,
//! including three where **local midnight does not exist** on a spring-
//! forward day: São Paulo, Santiago and Beirut have all moved the clock at
//! 00:00, so `Date::at(0,0,0,0)` there is a civil time that never happened.
//! `jiff` resolves it forward to 01:00 rather than failing -- which is why
//! `Week::midnight_ms` can assert what it does -- and the round trip still
//! holds because 01:00 is the same date. That is worth a generator rather
//! than a comment.
//!
//! Kept separate and `#[ignore]`d per the project convention: normal
//! verification runs `cargo test --workspace`, property verification runs
//! it with `-- --include-ignored`.

use proptest::prelude::*;
use scheduler_core::quota::{self, Week, Weekday};
use scheduler_core::timezone;

/// Zones with awkward transitions, not a representative sample: three that
/// have sprung forward at midnight, one half-hour offset, one that has
/// changed its standard offset outright, and UTC as the trivial case.
const ZONES: [&str; 7] = [
    "America/Sao_Paulo",
    "America/Santiago",
    "Asia/Beirut",
    "Australia/Lord_Howe",
    "Pacific/Apia",
    "America/New_York",
    "UTC",
];

fn any_zone() -> impl Strategy<Value = jiff::tz::TimeZone> {
    prop::sample::select(ZONES.as_slice())
        .prop_map(|name| timezone::resolve(name).expect("a real IANA zone"))
}

/// Instants across roughly 2015-2030, so every generated week lands
/// somewhere in a span containing all the transitions above.
fn any_instant_ms() -> impl Strategy<Value = i64> {
    1_420_070_400_000i64..1_893_456_000_000i64
}

fn any_week() -> impl Strategy<Value = (Week, jiff::tz::TimeZone)> {
    (any_instant_ms(), any_zone()).prop_map(|(now, zone)| (Week::of(now, &zone), zone))
}

const ALL_DAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// The round trip that ties the writer of `day_ms` to its reader. A day
    /// stored and read back is the same day, in every zone, including one
    /// whose midnight that week did not exist.
    #[test]
    #[ignore]
    fn a_day_stored_as_an_instant_reads_back_as_the_same_day((week, zone) in any_week()) {
        for day in ALL_DAYS {
            let stored = week.day_ms(day, &zone);
            prop_assert_eq!(
                quota::weekday_of(stored, &zone), day,
                "{} did not survive the round trip", day.label()
            );
        }
    }

    /// You cannot record time you have not done yet
    /// (`quota-sessions-only-days-that-have-happened-03`). The offered days
    /// are exactly the allowed ones, contiguous from Monday, and never
    /// empty -- Monday itself always counts.
    #[test]
    #[ignore]
    fn the_offered_days_are_exactly_the_days_that_have_happened((week, zone) in any_week()) {
        let _ = &zone;
        let offered = week.days_so_far();

        prop_assert!(!offered.is_empty());
        prop_assert_eq!(offered.first().copied(), Some(Weekday::Mon));
        prop_assert_eq!(offered.last().copied(), Some(week.today()));
        for day in ALL_DAYS {
            prop_assert_eq!(
                offered.contains(&day), week.allows(day),
                "{} is offered and allowed inconsistently", day.label()
            );
        }
    }

    /// Every day of the week falls inside the week's own bounds, and the
    /// bounds are the half-open range the store's `WHERE` relies on -- so a
    /// session cannot be logged into a week that will not list it.
    #[test]
    #[ignore]
    fn every_day_of_the_week_falls_within_the_bounds_the_query_uses(
        (week, zone) in any_week()
    ) {
        let (start, end) = week.bounds_ms(&zone);

        prop_assert!(start < end);
        for day in ALL_DAYS {
            let stored = week.day_ms(day, &zone);
            prop_assert!(stored >= start, "{} is before the week starts", day.label());
            prop_assert!(stored < end, "{} is not before the week ends", day.label());
        }
    }

    /// Consecutive weeks tile without gap or overlap: one week's end is the
    /// next one's start. `D-quota-no-rollover` resets on Monday, and a
    /// boundary that drifted would either double-count a day or lose one.
    #[test]
    #[ignore]
    fn one_weeks_end_is_the_next_weeks_start((week, zone) in any_week()) {
        let (start, end) = week.bounds_ms(&zone);
        let (next_start, next_end) = Week::of(end, &zone).bounds_ms(&zone);

        prop_assert_eq!(end, next_start);
        prop_assert!(next_end > next_start);
        prop_assert!(Week::of(start, &zone).bounds_ms(&zone) == (start, end));
    }

    /// A session is accepted on exactly the days the week allows, and the
    /// day it reports back is the day submitted -- validation never
    /// silently substitutes one.
    #[test]
    #[ignore]
    fn a_session_is_accepted_on_exactly_the_days_the_week_allows(
        (week, zone) in any_week()
    ) {
        let _ = &zone;
        for day in ALL_DAYS {
            let result = quota::validate_session(Some(day.label()), Some("30"), &week);
            prop_assert_eq!(
                result.is_ok(), week.allows(day),
                "{} was accepted or refused against the week's own rule", day.label()
            );
            if let Ok(session) = result {
                prop_assert_eq!(session.day, day);
                prop_assert_eq!(session.minutes, 30);
            }
        }
    }
}
