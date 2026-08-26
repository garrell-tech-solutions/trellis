//! A quota: a named container with a weekly hour target (#93,
//! `D-quotas-are-selected-not-typed`) — a different entity from
//! `task::TaskKind::Quota`'s triaged capture, which this module does not
//! touch and which stays exactly as it is until #138 closes the two into
//! one.

/// A quota's weekly target: **always a positive whole number of minutes**,
/// and this is the type that says so rather than a comment naming whoever
/// happened to check.
///
/// The rule has three statements of it -- the `CHECK` on
/// `quotas.weekly_target_minutes`, [`QuotaDefinition::from_fields`]'s own
/// refusal, and this -- and only this one is reachable by the code that
/// *uses* a target. [`progress`] divides by it; a zero reaching that
/// division does not panic, it returns `i64::MAX` percent, which renders as
/// a number and is wrong. An unrepresentable state cannot do that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeeklyTarget(i64);

impl WeeklyTarget {
    /// `None` for a target of zero or less -- the one thing a target may
    /// not be (`quota-screen-target-must-be-positive-04`).
    pub fn from_minutes(minutes: i64) -> Option<Self> {
        (minutes > 0).then_some(WeeklyTarget(minutes))
    }

    pub fn minutes(self) -> i64 {
        self.0
    }
}

/// A validated definition, ready to write: name trimmed and non-empty,
/// target a positive whole number of minutes
/// (`quota-screen-both-fields-required-03`,
/// `quota-screen-target-must-be-positive-04`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaDefinition {
    pub name: String,
    pub weekly_target: WeeklyTarget,
}

/// Which required field a submission left out or gave an invalid value for
/// — the same two-shape rejection `task::TriageRejection` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Hours,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionRejection {
    MissingField(Field),
    InvalidField(Field),
}

impl QuotaDefinition {
    /// `name` trimmed and non-empty; `hours` a positive number, converted to
    /// whole minutes ([`to_minutes`]).
    pub fn from_fields(
        name: Option<&str>,
        hours: Option<&str>,
    ) -> Result<Self, DefinitionRejection> {
        Ok(QuotaDefinition {
            name: parse_name(name)?,
            weekly_target: parse_weekly_target(hours)?,
        })
    }
}

/// `value` trimmed and non-empty, or `err` -- the same "required" check
/// [`parse_name`], [`parse_weekly_target`] and [`validate_session`] all
/// start with, differing only in which rejection they hand back.
fn require_nonblank<E>(value: Option<&str>, err: E) -> Result<&str, E> {
    value.map(str::trim).filter(|v| !v.is_empty()).ok_or(err)
}

fn parse_name(name: Option<&str>) -> Result<String, DefinitionRejection> {
    require_nonblank(name, DefinitionRejection::MissingField(Field::Name)).map(str::to_string)
}

fn parse_weekly_target(hours: Option<&str>) -> Result<WeeklyTarget, DefinitionRejection> {
    let hours_str = require_nonblank(hours, DefinitionRejection::MissingField(Field::Hours))?;
    let hours = parse_positive_hours(hours_str)?;
    // `parse_positive_hours` already refused anything at or below zero, so
    // the only way `to_minutes` yields a non-positive number is a positive
    // target under half a minute rounding to zero -- which is out of the
    // domain for the same reason zero itself is.
    WeeklyTarget::from_minutes(to_minutes(hours))
        .ok_or(DefinitionRejection::InvalidField(Field::Hours))
}

fn parse_positive_hours(hours_str: &str) -> Result<f64, DefinitionRejection> {
    let hours: f64 = hours_str
        .parse()
        .map_err(|_| DefinitionRejection::InvalidField(Field::Hours))?;
    if hours.is_nan() || hours <= 0.0 {
        return Err(DefinitionRejection::InvalidField(Field::Hours));
    }
    Ok(hours)
}

/// The canvas's own `step="0.5"` on the hours input means every value it can
/// submit already lands on a whole minute; rounding here is a safety net
/// against float drift, not a domain rule.
fn to_minutes(hours: f64) -> i64 {
    (hours * 60.0).round() as i64
}

/// Whether `candidate` collides with an existing quota — the two-tier guard
/// `D-quotas-are-selected-not-typed` requires: `Exact` once case, spaces and
/// punctuation are folded away (refused outright, never bypassable), or
/// `Similar` by edit distance or containment (warned, and still possible on
/// confirmation). Carries the existing quota's own stored spelling and
/// target, for the message to quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameMatch<'a> {
    Exact(&'a str, i64),
    Similar(&'a str, i64),
}

/// `existing` is every already-defined quota's `(name, weekly_target_minutes)`,
/// in any order — the first exact match wins outright; the first similar
/// match is kept only if no exact match is found anywhere in the list.
pub fn check_name<'a>(candidate: &str, existing: &'a [(String, i64)]) -> Option<NameMatch<'a>> {
    let candidate_norm = normalize(candidate);
    let mut similar = None;
    for (name, minutes) in existing {
        let name_norm = normalize(name);
        if candidate_norm == name_norm {
            return Some(NameMatch::Exact(name, *minutes));
        }
        if similar.is_none() && is_similar(&candidate_norm, &name_norm) {
            similar = Some(NameMatch::Similar(name.as_str(), *minutes));
        }
    }
    similar
}

/// Case, spaces and punctuation folded away — past what `COLLATE NOCASE`
/// alone can say, which is why the rule lives here rather than only in the
/// schema (`T-collation-enforces-name-identity`'s path for a rule that has
/// outgrown a collation). `"Pi-ano"` and `"pi ano"` both normalize to
/// `"piano"`.
fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Within two letters (Levenshtein distance) or one name containing the
/// other — `"Pianoo"` is a typo of `"Piano"`; `"Piano theory"` may
/// genuinely be a second quota, and both only warn rather than refuse.
fn is_similar(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a.contains(b) || b.contains(a) || levenshtein(a, b) <= 2
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, a_char) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, b_char) in b.iter().enumerate() {
            let cost = usize::from(a_char != b_char);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

/// A quota's standing against its weekly target: how much is left, and how
/// far into it the week already is.
pub struct Progress {
    pub remaining_minutes: i64,
    pub percent: i64,
}

/// Division needs no guard because [`WeeklyTarget`] cannot hold a zero --
/// the guarantee is in the argument's type rather than in a note about who
/// built it.
pub fn progress(target: WeeklyTarget, logged_minutes: i64) -> Progress {
    Progress {
        remaining_minutes: (target.minutes() - logged_minutes).max(0),
        percent: ((logged_minutes as f64 / target.minutes() as f64) * 100.0).round() as i64,
    }
}

/// A day a session can be logged against -- always Monday-first, never the
/// host locale's own week start, since `Week` below always counts from
/// Monday regardless of where the server runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

/// Monday-first order, the canonical sequence [`Weekday::index`],
/// [`Weekday::from_index`], [`Weekday::label`] and [`Weekday::parse`] all
/// derive from a single array lookup rather than a seven-or-eight-arm match
/// apiece -- past `T-complexity-8`'s cap once a wildcard arm joins the
/// other seven, and there is no logic in any of the four to extract, only
/// the same table read four ways.
const ORDERED: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];
const LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

impl Weekday {
    /// `0` for Monday through `6` for Sunday -- the ordinal `Week` compares
    /// against `today` to decide whether a day has happened yet
    /// (`quota-sessions-only-days-that-have-happened-03`).
    pub fn index(self) -> u8 {
        ORDERED
            .iter()
            .position(|&day| day == self)
            .expect("ORDERED lists every Weekday variant") as u8
    }

    fn from_index(index: u8) -> Self {
        ORDERED[usize::from(index)]
    }

    /// `jiff`'s own Monday-zero ordinal ([`jiff::civil::Weekday::
    /// to_monday_zero_offset`]) already lines up with [`ORDERED`], so this
    /// delegates to [`Weekday::from_index`] rather than a sixth,
    /// wildcard-free match to keep under `T-complexity-8`.
    fn from_jiff(weekday: jiff::civil::Weekday) -> Self {
        Weekday::from_index(weekday.to_monday_zero_offset() as u8)
    }

    /// `"Mon"` .. `"Sun"` -- the spelling every acceptance scenario and the
    /// rendered day picker share.
    pub fn label(self) -> &'static str {
        LABELS[usize::from(self.index())]
    }

    /// The inverse of [`Weekday::label`], for a submitted day name. `None`
    /// for anything else -- a full name, a lowercase spelling, or hostile
    /// text all fail the same way a missing field does.
    pub fn parse(label: &str) -> Option<Self> {
        LABELS
            .iter()
            .position(|&candidate| candidate == label)
            .map(|index| ORDERED[index])
    }
}

/// Which weekday `instant_ms` falls on, in `zone` -- the inverse of
/// [`Week::day_ms`], for rendering a stored session's `day_ms` back as the
/// label it was logged under.
pub fn weekday_of(instant_ms: i64, zone: &jiff::tz::TimeZone) -> Weekday {
    let date = jiff::Timestamp::from_millisecond(instant_ms)
        .expect("instant_ms is a valid instant; the store never writes anything else")
        .to_zoned(zone.clone())
        .date();
    Weekday::from_jiff(date.weekday())
}

/// The current week, Monday-anchored, as seen from `now_ms` in the owner's
/// timezone (`T-timezone-is-a-setting`) -- what decides which days a session
/// can be logged against and which sessions belong to "this week".
///
/// Logging is retrospective (`D-logging-is-retrospective-and-separate`):
/// offering a day that has not happened yet would let a session fill the bar
/// for time that was never spent, the same false-number failure a start/stop
/// timer would produce from the other direction.
pub struct Week {
    monday: jiff::civil::Date,
    today: Weekday,
}

impl Week {
    pub fn of(now_ms: i64, zone: &jiff::tz::TimeZone) -> Self {
        let today_date = jiff::Timestamp::from_millisecond(now_ms)
            .expect("now_ms is a valid instant")
            .to_zoned(zone.clone())
            .date();
        let today = Weekday::from_jiff(today_date.weekday());
        let monday = today_date
            .checked_sub(jiff::Span::new().days(i64::from(today.index())))
            .expect("subtracting at most six days from a real date stays in range");
        Week { monday, today }
    }

    pub fn today(&self) -> Weekday {
        self.today
    }

    /// Monday through today, inclusive, in order -- exactly what a day
    /// picker may offer (`quota-sessions-only-days-that-have-happened-03`).
    pub fn days_so_far(&self) -> Vec<Weekday> {
        (0..=self.today.index()).map(Weekday::from_index).collect()
    }

    /// Whether `day` has already happened this week -- the server-side half
    /// of `-03`'s guard, checked again here because a guard that only lives
    /// in the day picker's own options is not a guard: nothing stops a
    /// direct submission that skips it.
    pub fn allows(&self, day: Weekday) -> bool {
        day.index() <= self.today.index()
    }

    /// The instant `day`'s local midnight falls at, in `zone` -- what a
    /// session's `day_ms` column stores, so a stored row survives a week
    /// boundary rather than meaning only "Monday" with no year attached.
    pub fn day_ms(&self, day: Weekday, zone: &jiff::tz::TimeZone) -> i64 {
        let date = self
            .monday
            .checked_add(jiff::Span::new().days(i64::from(day.index())))
            .expect("adding at most six days to a real date stays in range");
        Self::midnight_ms(date, zone)
    }

    /// This week's own bounds as `[start_ms, end_ms)` -- the store's own
    /// `WHERE` for "this week's sessions" and "this week's total"
    /// (`T-set-operations-execute-in-the-store`), computed from `monday`'s
    /// civil date rather than by adding `7 * 86_400_000` to a millisecond
    /// count, which a daylight-saving transition would get wrong.
    pub fn bounds_ms(&self, zone: &jiff::tz::TimeZone) -> (i64, i64) {
        let next_monday = self
            .monday
            .checked_add(jiff::Span::new().weeks(1))
            .expect("adding one week to a real date stays in range");
        (
            Self::midnight_ms(self.monday, zone),
            Self::midnight_ms(next_monday, zone),
        )
    }

    fn midnight_ms(date: jiff::civil::Date, zone: &jiff::tz::TimeZone) -> i64 {
        date.at(0, 0, 0, 0)
            .to_zoned(zone.clone())
            .expect("a civil midnight always resolves to a zoned instant")
            .timestamp()
            .as_millisecond()
    }
}

/// A session's own two fields, for a rejection to name --
/// `quota-sessions-a-session-must-be-positive-09`'s own shape, one field
/// away from [`Field`]: a session has no name to guard, only a day and a
/// duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionField {
    Day,
    Minutes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRejection {
    MissingField(SessionField),
    InvalidField(SessionField),
}

/// A validated session, ready to write: a real day within `week`, and a
/// positive whole number of minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoggedSession {
    pub day: Weekday,
    pub minutes: i64,
}

/// `day` a name [`Weekday::parse`] recognizes and that `week` allows
/// (`quota-sessions-only-days-that-have-happened-03`'s server-side half: a
/// guard that only lives in the day picker's own options is not a guard),
/// `minutes` a positive whole number
/// (`quota-sessions-a-session-must-be-positive-09`).
pub fn validate_session(
    day: Option<&str>,
    minutes: Option<&str>,
    week: &Week,
) -> Result<LoggedSession, SessionRejection> {
    let day = parse_day(day, week)?;
    let minutes = parse_positive_minutes(minutes)?;
    Ok(LoggedSession { day, minutes })
}

fn parse_day(day: Option<&str>, week: &Week) -> Result<Weekday, SessionRejection> {
    let day_str = require_nonblank(day, SessionRejection::MissingField(SessionField::Day))?;
    let day = Weekday::parse(day_str).ok_or(SessionRejection::InvalidField(SessionField::Day))?;
    if week.allows(day) {
        Ok(day)
    } else {
        Err(SessionRejection::InvalidField(SessionField::Day))
    }
}

fn parse_positive_minutes(minutes: Option<&str>) -> Result<i64, SessionRejection> {
    let minutes_str = require_nonblank(
        minutes,
        SessionRejection::MissingField(SessionField::Minutes),
    )?;
    let minutes: i64 = minutes_str
        .parse()
        .map_err(|_| SessionRejection::InvalidField(SessionField::Minutes))?;
    if minutes > 0 {
        Ok(minutes)
    } else {
        Err(SessionRejection::InvalidField(SessionField::Minutes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A target these tests already know is valid.
    fn target(minutes: i64) -> WeeklyTarget {
        WeeklyTarget::from_minutes(minutes).expect("a positive target")
    }

    #[test]
    fn from_fields_rejects_a_missing_name() {
        assert_eq!(
            QuotaDefinition::from_fields(None, Some("4")),
            Err(DefinitionRejection::MissingField(Field::Name))
        );
    }

    #[test]
    fn from_fields_rejects_a_blank_name() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("   "), Some("4")),
            Err(DefinitionRejection::MissingField(Field::Name))
        );
    }

    #[test]
    fn from_fields_rejects_a_missing_hours() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("Piano"), None),
            Err(DefinitionRejection::MissingField(Field::Hours))
        );
    }

    #[test]
    fn from_fields_rejects_hours_of_zero() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("Piano"), Some("0")),
            Err(DefinitionRejection::InvalidField(Field::Hours))
        );
    }

    #[test]
    fn from_fields_rejects_negative_hours() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("Piano"), Some("-2")),
            Err(DefinitionRejection::InvalidField(Field::Hours))
        );
    }

    #[test]
    fn from_fields_rejects_unparseable_hours() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("Piano"), Some("many")),
            Err(DefinitionRejection::InvalidField(Field::Hours))
        );
    }

    #[test]
    fn from_fields_accepts_a_whole_number_of_hours() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("Piano"), Some("4")),
            Ok(QuotaDefinition {
                name: "Piano".to_string(),
                weekly_target: target(240),
            })
        );
    }

    #[test]
    fn from_fields_accepts_a_half_hour_target() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("Piano"), Some("0.5")),
            Ok(QuotaDefinition {
                name: "Piano".to_string(),
                weekly_target: target(30),
            })
        );
    }

    #[test]
    fn from_fields_trims_the_name() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("  Piano  "), Some("4"))
                .unwrap()
                .name,
            "Piano"
        );
    }

    #[test]
    fn check_name_finds_an_exact_match_once_case_is_folded() {
        let existing = vec![("Piano".to_string(), 240)];
        assert_eq!(
            check_name("piano", &existing),
            Some(NameMatch::Exact("Piano", 240))
        );
        assert_eq!(
            check_name("PIANO", &existing),
            Some(NameMatch::Exact("Piano", 240))
        );
    }

    #[test]
    fn check_name_finds_an_exact_match_once_punctuation_and_spaces_are_folded() {
        let existing = vec![("Piano".to_string(), 240)];
        assert_eq!(
            check_name("Pi-ano", &existing),
            Some(NameMatch::Exact("Piano", 240))
        );
        assert_eq!(
            check_name("pi ano", &existing),
            Some(NameMatch::Exact("Piano", 240))
        );
    }

    #[test]
    fn check_name_warns_on_a_name_one_edit_away() {
        let existing = vec![("Piano".to_string(), 240)];
        assert_eq!(
            check_name("Pianoo", &existing),
            Some(NameMatch::Similar("Piano", 240))
        );
    }

    #[test]
    fn check_name_warns_when_one_name_contains_the_other() {
        let existing = vec![("Piano".to_string(), 240)];
        assert_eq!(
            check_name("Piano theory", &existing),
            Some(NameMatch::Similar("Piano", 240))
        );
    }

    #[test]
    fn check_name_is_silent_on_an_unrelated_name() {
        let existing = vec![("Piano".to_string(), 240)];
        assert_eq!(check_name("Running", &existing), None);
    }

    #[test]
    fn check_name_prefers_an_exact_match_over_a_similar_one_elsewhere_in_the_list() {
        let existing = vec![("Pianoo".to_string(), 120), ("Piano".to_string(), 240)];
        assert_eq!(
            check_name("Piano", &existing),
            Some(NameMatch::Exact("Piano", 240))
        );
    }

    /// The containing side can be either name -- `"Piano theory"` warns
    /// against `"Piano"` (candidate contains existing) and this is the other
    /// direction: an existing `"Piano reading"` warns against a candidate of
    /// just `"Reading"` (existing contains candidate), too far apart in edit
    /// distance for that tier to catch it alone.
    #[test]
    fn check_name_warns_when_an_existing_name_contains_the_candidate() {
        let existing = vec![("Piano reading".to_string(), 240)];
        assert_eq!(
            check_name("Reading", &existing),
            Some(NameMatch::Similar("Piano reading", 240))
        );
    }

    /// A candidate that is nothing but punctuation normalizes to the empty
    /// string, and an empty string is a substring of everything -- without
    /// its own guard, `is_similar` would warn a punctuation-only candidate
    /// against every existing quota rather than staying silent.
    #[test]
    fn check_name_is_silent_on_a_candidate_that_normalizes_to_nothing() {
        let existing = vec![("Piano".to_string(), 240)];
        assert_eq!(check_name("---", &existing), None);
    }

    /// The invariant the type exists for: the one value that would make
    /// [`progress`] divide by zero cannot be built at all.
    #[test]
    fn a_weekly_target_cannot_be_zero_or_negative() {
        assert_eq!(WeeklyTarget::from_minutes(0), None);
        assert_eq!(WeeklyTarget::from_minutes(-30), None);
        assert_eq!(
            WeeklyTarget::from_minutes(1).map(WeeklyTarget::minutes),
            Some(1)
        );
    }

    /// A positive number of hours too small to round to a whole minute is
    /// out of the domain for the same reason zero is, and reports as the
    /// same rejection rather than reaching the target type.
    #[test]
    fn from_fields_rejects_a_target_too_small_to_be_a_whole_minute() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("Piano"), Some("0.001")),
            Err(DefinitionRejection::InvalidField(Field::Hours))
        );
    }

    #[test]
    fn progress_with_nothing_logged_is_the_full_target_at_zero_percent() {
        let p = progress(target(240), 0);
        assert_eq!(p.remaining_minutes, 240);
        assert_eq!(p.percent, 0);
    }

    #[test]
    fn progress_rounds_the_percent_to_the_nearest_whole_number() {
        let p = progress(target(240), 30);
        assert_eq!(p.remaining_minutes, 210);
        assert_eq!(p.percent, 13);
    }

    #[test]
    fn progress_never_reports_negative_remaining_once_the_target_is_exceeded() {
        let p = progress(target(240), 300);
        assert_eq!(p.remaining_minutes, 0);
        assert_eq!(p.percent, 125);
    }

    // --- parse_positive_hours and levenshtein: exercised directly ---------
    //
    // `parse_weekly_target` re-checks the result through
    // `WeeklyTarget::from_minutes`, which refuses anything non-positive on
    // its own -- so every input that reaches `from_fields` gets the same
    // `InvalidField(Hours)` whether `parse_positive_hours` catches it or the
    // downstream guard does. That redundancy is exactly why a defect in
    // `parse_positive_hours` alone needs a direct test to be visible at all.
    // `levenshtein`'s only external signal is `is_similar`'s `<= 2`
    // threshold, which likewise hides most wrong distances that still land
    // on the same side of 2; a direct test on known distances is what
    // actually pins the arithmetic down.

    #[test]
    fn parse_positive_hours_rejects_a_value_that_parses_as_not_a_number() {
        assert_eq!(
            parse_positive_hours("NaN"),
            Err(DefinitionRejection::InvalidField(Field::Hours))
        );
    }

    #[test]
    fn levenshtein_of_identical_strings_is_zero() {
        assert_eq!(levenshtein("piano", "piano"), 0);
    }

    #[test]
    fn levenshtein_against_an_empty_string_is_the_others_length() {
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("", "abc"), 3);
    }

    #[test]
    fn levenshtein_of_the_classic_kitten_sitting_pair_is_three() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn levenshtein_of_a_single_insertion_is_one() {
        assert_eq!(levenshtein("ab", "abc"), 1);
        assert_eq!(levenshtein("abc", "ab"), 1);
    }

    #[test]
    fn levenshtein_does_not_credit_a_transposition() {
        assert_eq!(levenshtein("ab", "ba"), 2);
    }

    // --- Weekday / Week (#93, quota-sessions) ----------------------------

    fn ny() -> jiff::tz::TimeZone {
        crate::timezone::resolve("America/New_York").expect("a real IANA zone")
    }

    fn ms(rfc3339: &str) -> i64 {
        rfc3339.parse::<jiff::Timestamp>().unwrap().as_millisecond()
    }

    #[test]
    fn weekday_label_and_parse_round_trip() {
        for day in [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ] {
            assert_eq!(Weekday::parse(day.label()), Some(day));
        }
    }

    #[test]
    fn weekday_parse_rejects_anything_else() {
        assert_eq!(Weekday::parse("Monday"), None);
        assert_eq!(Weekday::parse("mon"), None);
        assert_eq!(Weekday::parse(""), None);
    }

    // 2026-08-25T14:00:00Z is 10:00 America/New_York on a Tuesday --
    // quota_sessions.feature's own Background instant.
    fn tuesday_ms() -> i64 {
        ms("2026-08-25T14:00:00Z")
    }

    #[test]
    fn week_of_a_tuesday_reports_tuesday_as_today() {
        let week = Week::of(tuesday_ms(), &ny());
        assert_eq!(week.today(), Weekday::Tue);
    }

    #[test]
    fn days_so_far_on_monday_offers_only_monday() {
        let week = Week::of(ms("2026-08-24T14:00:00Z"), &ny());
        assert_eq!(week.days_so_far(), vec![Weekday::Mon]);
    }

    #[test]
    fn days_so_far_on_tuesday_offers_monday_and_tuesday() {
        let week = Week::of(ms("2026-08-25T14:00:00Z"), &ny());
        assert_eq!(week.days_so_far(), vec![Weekday::Mon, Weekday::Tue]);
    }

    #[test]
    fn days_so_far_on_friday_offers_the_first_five_days() {
        let week = Week::of(ms("2026-08-28T14:00:00Z"), &ny());
        assert_eq!(
            week.days_so_far(),
            vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri
            ]
        );
    }

    #[test]
    fn days_so_far_on_sunday_offers_all_seven() {
        let week = Week::of(ms("2026-08-30T14:00:00Z"), &ny());
        assert_eq!(
            week.days_so_far(),
            vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
                Weekday::Sat,
                Weekday::Sun,
            ]
        );
    }

    #[test]
    fn allows_refuses_a_day_that_has_not_happened_yet() {
        let week = Week::of(tuesday_ms(), &ny());
        assert!(week.allows(Weekday::Mon));
        assert!(week.allows(Weekday::Tue));
        assert!(!week.allows(Weekday::Wed));
        assert!(!week.allows(Weekday::Sun));
    }

    #[test]
    fn day_ms_of_today_is_todays_own_local_midnight() {
        let week = Week::of(tuesday_ms(), &ny());
        let zone = ny();
        let expected = ms("2026-08-25T04:00:00Z"); // 2026-08-25T00:00 America/New_York
        assert_eq!(week.day_ms(Weekday::Tue, &zone), expected);
    }

    #[test]
    fn day_ms_of_an_earlier_day_is_that_days_own_local_midnight() {
        let week = Week::of(tuesday_ms(), &ny());
        let zone = ny();
        let expected = ms("2026-08-24T04:00:00Z"); // 2026-08-24T00:00 America/New_York
        assert_eq!(week.day_ms(Weekday::Mon, &zone), expected);
    }

    #[test]
    fn bounds_ms_spans_exactly_this_monday_through_next_monday() {
        let week = Week::of(tuesday_ms(), &ny());
        let zone = ny();
        let (start, end) = week.bounds_ms(&zone);
        assert_eq!(start, ms("2026-08-24T04:00:00Z")); // Monday 00:00 NY
        assert_eq!(end, ms("2026-08-31T04:00:00Z")); // next Monday 00:00 NY
    }

    #[test]
    fn a_different_week_has_disjoint_bounds() {
        let this_week = Week::of(tuesday_ms(), &ny());
        let next_week = Week::of(ms("2026-08-31T14:00:00Z"), &ny());
        let zone = ny();
        let (_, this_end) = this_week.bounds_ms(&zone);
        let (next_start, _) = next_week.bounds_ms(&zone);
        assert_eq!(this_end, next_start);
    }

    #[test]
    fn weekday_of_is_the_inverse_of_week_day_ms() {
        let week = tuesday_week();
        let zone = ny();
        for day in week.days_so_far() {
            assert_eq!(weekday_of(week.day_ms(day, &zone), &zone), day);
        }
    }

    // --- validate_session (#93, quota-sessions) --------------------------

    fn tuesday_week() -> Week {
        Week::of(tuesday_ms(), &ny())
    }

    #[test]
    fn validate_session_rejects_a_missing_day() {
        assert_eq!(
            validate_session(None, Some("20"), &tuesday_week()),
            Err(SessionRejection::MissingField(SessionField::Day))
        );
    }

    #[test]
    fn validate_session_rejects_a_day_name_it_does_not_recognize() {
        assert_eq!(
            validate_session(Some("Someday"), Some("20"), &tuesday_week()),
            Err(SessionRejection::InvalidField(SessionField::Day))
        );
    }

    #[test]
    fn validate_session_rejects_a_day_that_has_not_happened_yet() {
        assert_eq!(
            validate_session(Some("Wed"), Some("20"), &tuesday_week()),
            Err(SessionRejection::InvalidField(SessionField::Day)),
            "Wednesday has not happened yet against a Tuesday week -- not by the picker, and not by posting one directly"
        );
    }

    #[test]
    fn validate_session_accepts_today() {
        assert_eq!(
            validate_session(Some("Tue"), Some("20"), &tuesday_week()),
            Ok(LoggedSession {
                day: Weekday::Tue,
                minutes: 20
            })
        );
    }

    #[test]
    fn validate_session_accepts_an_earlier_day_this_week() {
        assert_eq!(
            validate_session(Some("Mon"), Some("20"), &tuesday_week()),
            Ok(LoggedSession {
                day: Weekday::Mon,
                minutes: 20
            })
        );
    }

    #[test]
    fn validate_session_rejects_a_missing_minutes() {
        assert_eq!(
            validate_session(Some("Mon"), None, &tuesday_week()),
            Err(SessionRejection::MissingField(SessionField::Minutes))
        );
    }

    #[test]
    fn validate_session_rejects_zero_minutes() {
        assert_eq!(
            validate_session(Some("Mon"), Some("0"), &tuesday_week()),
            Err(SessionRejection::InvalidField(SessionField::Minutes))
        );
    }

    #[test]
    fn validate_session_rejects_negative_minutes() {
        assert_eq!(
            validate_session(Some("Mon"), Some("-30"), &tuesday_week()),
            Err(SessionRejection::InvalidField(SessionField::Minutes))
        );
    }

    #[test]
    fn validate_session_rejects_unparseable_minutes() {
        assert_eq!(
            validate_session(Some("Mon"), Some("many"), &tuesday_week()),
            Err(SessionRejection::InvalidField(SessionField::Minutes))
        );
    }
}
