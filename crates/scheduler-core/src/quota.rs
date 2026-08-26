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

/// `value` trimmed and non-empty, or the field's own missing-field
/// rejection -- the same "required" check [`parse_name`] and
/// [`parse_weekly_target`] both start with, differing only in which
/// field is doing the asking.
fn require_nonblank(value: Option<&str>, field: Field) -> Result<&str, DefinitionRejection> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or(DefinitionRejection::MissingField(field))
}

fn parse_name(name: Option<&str>) -> Result<String, DefinitionRejection> {
    require_nonblank(name, Field::Name).map(str::to_string)
}

fn parse_weekly_target(hours: Option<&str>) -> Result<WeeklyTarget, DefinitionRejection> {
    let hours_str = require_nonblank(hours, Field::Hours)?;
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
}
