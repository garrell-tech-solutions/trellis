//! A quota: a named container with a weekly hour target (#93,
//! `D-quotas-are-selected-not-typed`) — a different entity from
//! `task::TaskKind::Quota`'s triaged capture, which this module does not
//! touch and which stays exactly as it is until #138 closes the two into
//! one.

/// A validated definition, ready to write: name trimmed and non-empty,
/// target a positive whole number of minutes
/// (`quota-screen-both-fields-required-03`,
/// `quota-screen-target-must-be-positive-04`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaDefinition {
    pub name: String,
    pub weekly_target_minutes: i64,
}

/// Which required field a submission left out or gave an invalid value for
/// — the same two-shape rejection `task::TriageRejection` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Hours,
}

impl Field {
    pub fn name(self) -> &'static str {
        match self {
            Field::Name => "name",
            Field::Hours => "hours",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionRejection {
    MissingField(Field),
    InvalidField(Field),
}

impl QuotaDefinition {
    /// `name` trimmed and non-empty; `hours` a positive number, converted to
    /// whole minutes. The canvas's own `step="0.5"` on the hours input means
    /// every value it can submit already lands on a whole minute; rounding
    /// here is a safety net against float drift, not a domain rule.
    pub fn from_fields(
        name: Option<&str>,
        hours: Option<&str>,
    ) -> Result<Self, DefinitionRejection> {
        Ok(QuotaDefinition {
            name: parse_name(name)?,
            weekly_target_minutes: parse_weekly_target_minutes(hours)?,
        })
    }
}

fn parse_name(name: Option<&str>) -> Result<String, DefinitionRejection> {
    name.map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .ok_or(DefinitionRejection::MissingField(Field::Name))
}

fn parse_weekly_target_minutes(hours: Option<&str>) -> Result<i64, DefinitionRejection> {
    let hours_str = hours
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .ok_or(DefinitionRejection::MissingField(Field::Hours))?;
    let hours: f64 = hours_str
        .parse()
        .map_err(|_| DefinitionRejection::InvalidField(Field::Hours))?;
    if hours.is_nan() || hours <= 0.0 {
        return Err(DefinitionRejection::InvalidField(Field::Hours));
    }
    Ok((hours * 60.0).round() as i64)
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
/// far into it the week already is. `target_minutes` is always positive
/// (`QuotaDefinition::from_fields` refuses anything else at the door), so
/// division here never needs a guard.
pub struct Progress {
    pub remaining_minutes: i64,
    pub percent: i64,
}

pub fn progress(target_minutes: i64, logged_minutes: i64) -> Progress {
    Progress {
        remaining_minutes: (target_minutes - logged_minutes).max(0),
        percent: ((logged_minutes as f64 / target_minutes as f64) * 100.0).round() as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                weekly_target_minutes: 240,
            })
        );
    }

    #[test]
    fn from_fields_accepts_a_half_hour_target() {
        assert_eq!(
            QuotaDefinition::from_fields(Some("Piano"), Some("0.5")),
            Ok(QuotaDefinition {
                name: "Piano".to_string(),
                weekly_target_minutes: 30,
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

    #[test]
    fn progress_with_nothing_logged_is_the_full_target_at_zero_percent() {
        let p = progress(240, 0);
        assert_eq!(p.remaining_minutes, 240);
        assert_eq!(p.percent, 0);
    }

    #[test]
    fn progress_rounds_the_percent_to_the_nearest_whole_number() {
        let p = progress(240, 30);
        assert_eq!(p.remaining_minutes, 210);
        assert_eq!(p.percent, 13);
    }

    #[test]
    fn progress_never_reports_negative_remaining_once_the_target_is_exceeded() {
        let p = progress(240, 300);
        assert_eq!(p.remaining_minutes, 0);
        assert_eq!(p.percent, 125);
    }
}
