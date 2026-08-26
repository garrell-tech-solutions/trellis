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

// --- Quota identity, and the definition that has to pass it (#138) --------
//
// Triage is now the only door that creates a quota, and `check_name` is the
// guard on it (`D-quotas-are-selected-not-typed`). Two of its three tiers are
// beyond what examples reach comfortably: `normalize` folds case, spaces and
// punctuation, so the set of spellings that mean one quota is unbounded, and
// `is_similar` is a *relation over pairs*, which an example pins at one pair
// at a time. The properties below are about the shape of the answer rather
// than about any pair: a respelling is never a second quota, an exact match
// beats a resemblance no matter where in the list it sits, and which of two
// names is the candidate does not change the verdict.

/// Characters that mean nothing to quota identity: `normalize` drops every
/// one of them.
const SEPARATORS: [&str; 6] = ["-", " ", "_", ".", "  ", "'"];

/// The runs in order, with a separator between each adjacent pair. Both
/// spellings below are built this way, from the same runs, which is what
/// makes them respellings rather than two names.
fn spell(runs: &[String], separators: &[&str]) -> String {
    let mut spelling = runs[0].clone();
    for (index, run) in runs.iter().enumerate().skip(1) {
        spelling.push_str(separators[(index - 1) % separators.len()]);
        spelling.push_str(run);
    }
    spelling
}

/// Uppercase or lowercase, never both -- `check_name` must not care which.
fn recase(spelling: String, shout: bool) -> String {
    if shout {
        spelling.to_uppercase()
    } else {
        spelling.to_lowercase()
    }
}

/// Two spellings of one name: the same alphanumeric runs in the same order,
/// separated differently and cased differently. Restricted to ASCII
/// alphanumerics on purpose -- `to_uppercase` expands some characters into
/// several (`ß` becomes `SS`), which would change the letters rather than
/// only their case, and this generator is about respelling, not about
/// Unicode case folding.
fn any_respelling() -> impl Strategy<Value = (String, String)> {
    (
        prop::collection::vec("[a-zA-Z0-9]{1,6}", 1..4),
        prop::collection::vec(prop::sample::select(SEPARATORS.as_slice()), 1..4),
        prop::collection::vec(prop::sample::select(SEPARATORS.as_slice()), 1..4),
        any::<bool>(),
    )
        .prop_map(|(runs, first, second, shout)| {
            (spell(&runs, &first), recase(spell(&runs, &second), shout))
        })
}

/// A name with at least one alphanumeric character, so it never normalizes
/// to the empty string.
fn any_name() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{1,8}[-a-zA-Z0-9 ]{0,8}"
}

/// Submitted name text, weighted towards the shapes a submission actually
/// takes -- padded, blank, or ordinary -- rather than towards arbitrary
/// characters. `.{0,30}` alone is *almost never accepted*, which would leave
/// the property below asserting nothing on nearly every case
/// (`T-a-check-must-be-seen-to-fail`: the breakage that proves it must
/// reach the assertion, not skip past it).
fn any_submitted_name() -> impl Strategy<Value = String> {
    prop_oneof![
        r"[ \t]{0,3}[a-zA-Z0-9][a-zA-Z0-9 ]{0,10}[ \t]{0,3}",
        r"[ \t]{0,4}",
        ".{0,30}",
    ]
}

/// Submitted hours text, weighted the same way: the control's own half-hour
/// steps, then numbers a hand-typed submission might carry, then anything.
fn any_submitted_hours() -> impl Strategy<Value = String> {
    prop_oneof![
        (1i64..2_000).prop_map(|halves| (halves as f64 / 2.0).to_string()),
        r"[ \t]{0,2}-?[0-9]{1,4}(\.[0-9]{1,3})?[ \t]{0,2}",
        // Positive, and small enough that a whole number of minutes rounds it
        // away -- the one route by which a zero can reach
        // `WeeklyTarget::from_minutes`, and the case `parse_weekly_target`'s
        // own comment names. Without it the "target is positive" assertion
        // below is reachable only by luck.
        (1i64..=99).prop_map(|hundredths| format!("0.00{hundredths:02}")),
        ".{0,12}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// **Punctuation, spacing and case are not what makes a quota.** However
    /// the owner respells a name they already have, triage must refuse it as
    /// the same one -- that is the whole of what `Exact` means, and it is
    /// stronger than the `UNIQUE COLLATE NOCASE` column behind it, which
    /// would let `"Pi-ano"` and `"pi ano"` both exist
    /// (`T-collation-enforces-name-identity`'s path for a rule that has
    /// outgrown a collation).
    #[test]
    #[ignore]
    fn a_respelling_of_an_existing_name_is_the_same_quota(
        (existing, respelling) in any_respelling(),
        minutes in 1i64..10_000,
    ) {
        let quotas = [(existing.clone(), minutes)];
        prop_assert_eq!(
            quota::check_name(&respelling, &quotas),
            Some(quota::NameMatch::Exact(existing.as_str(), minutes)),
            "{:?} against {:?}", respelling, existing
        );
    }

    /// **An exact match anywhere outranks a resemblance found earlier.** The
    /// two tiers are not the same product decision: `Exact` is refused
    /// outright and `Similar` can be confirmed past, so returning the wrong
    /// one lets a duplicate quota be created. `check_name` scans once and
    /// keeps a `Similar` only provisionally, and an example can only ever pin
    /// one arrangement of the list -- this pins every arrangement, with the
    /// exact match placed last, behind entries that resemble it.
    #[test]
    #[ignore]
    fn an_exact_match_outranks_a_resemblance_found_before_it(
        (existing, respelling) in any_respelling(),
        earlier in prop::collection::vec(any_name(), 0..4),
        minutes in 1i64..10_000,
    ) {
        let mut quotas: Vec<(String, i64)> =
            earlier.into_iter().map(|name| (name, 7)).collect();
        quotas.push((existing.clone(), minutes));

        prop_assert!(
            matches!(
                quota::check_name(&respelling, &quotas),
                Some(quota::NameMatch::Exact(_, _))
            ),
            "{:?} against {:?}", respelling, quotas
        );
    }

    /// **Which name is the candidate does not change the verdict.** Every
    /// tier of the comparison is symmetric -- folded equality, containment
    /// checked both ways round, and edit distance -- so "is this new one like
    /// that old one" and "is that old one like this new one" are the same
    /// question. An asymmetry would mean the order two quotas were created in
    /// decided whether the second was allowed, which nothing in the product
    /// says.
    #[test]
    #[ignore]
    fn a_resemblance_does_not_depend_on_which_name_is_the_candidate(
        one in any_name(),
        other in any_name(),
        minutes in 1i64..10_000,
    ) {
        let just_one = [(one.clone(), minutes)];
        let just_other = [(other.clone(), minutes)];
        let forward = quota::check_name(&one, &just_other);
        let backward = quota::check_name(&other, &just_one);

        let tier = |verdict: Option<quota::NameMatch<'_>>| match verdict {
            None => "free",
            Some(quota::NameMatch::Exact(_, _)) => "exact",
            Some(quota::NameMatch::Similar(_, _)) => "similar",
        };
        prop_assert_eq!(
            tier(forward), tier(backward),
            "{:?} and {:?} disagreed depending on which was asked about", one, other
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// **Whatever `from_fields` accepts is safe to write.** `quotas.name` is
    /// `UNIQUE COLLATE NOCASE` and `weekly_target_minutes` carries
    /// `CHECK (> 0)`; a definition that satisfies neither reaches the store
    /// as a constraint violation -- a 500 on a submission the owner could
    /// have been told about. The type is what keeps that from happening, and
    /// this is that claim over arbitrary text rather than over the handful of
    /// spellings the examples name.
    #[test]
    #[ignore]
    fn an_accepted_definition_is_trimmed_non_empty_and_positive(
        name in any_submitted_name(),
        hours in any_submitted_hours(),
    ) {
        if let Ok(definition) =
            quota::QuotaDefinition::from_fields(Some(&name), Some(&hours))
        {
            prop_assert_eq!(definition.name.trim(), definition.name.as_str());
            prop_assert!(!definition.name.is_empty());
            prop_assert!(definition.weekly_target.minutes() > 0);
        }
    }

    /// **Every value the hours input can submit lands on a whole minute.**
    /// The canvas gives that field `step="0.5"`, and `to_minutes` calls its
    /// own rounding "a safety net against float drift, not a domain rule" --
    /// a claim about the whole step sequence, which is what this checks.
    /// `0.5 h` is the smallest submission the control offers and must survive
    /// the conversion as 30 minutes rather than rounding away.
    #[test]
    #[ignore]
    fn every_target_the_hours_step_can_submit_is_a_whole_number_of_minutes(
        half_hours in 1i64..=400,
    ) {
        let hours = half_hours as f64 / 2.0;
        let definition =
            quota::QuotaDefinition::from_fields(Some("Piano"), Some(&hours.to_string()))
                .expect("the hours control's own step is a valid target");

        prop_assert_eq!(definition.weekly_target.minutes(), half_hours * 30);
    }

    /// **Logging time never moves a quota backwards.** `progress` divides,
    /// rounds and clamps, and each of those can invert an ordering on its
    /// own. The readout is the screen's whole point, so a percentage that
    /// falls when a session is added -- or a remainder that grows -- is
    /// visible to the owner immediately and explicable by nothing.
    #[test]
    #[ignore]
    fn logging_more_time_never_lowers_the_percentage_or_raises_what_is_left(
        target_minutes in 1i64..10_000,
        logged in 0i64..20_000,
        extra in 0i64..20_000,
    ) {
        let target = quota::WeeklyTarget::from_minutes(target_minutes)
            .expect("a positive target");

        let before = quota::progress(target, logged);
        let after = quota::progress(target, logged + extra);

        prop_assert!(
            after.percent >= before.percent,
            "{}% fell to {}% after {} more minutes", before.percent, after.percent, extra
        );
        prop_assert!(
            after.remaining_minutes <= before.remaining_minutes,
            "{} minutes left rose to {} after {} more",
            before.remaining_minutes, after.remaining_minutes, extra
        );
        prop_assert!(after.remaining_minutes >= 0);
    }
}
