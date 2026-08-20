//! What a life-area name is.
//!
//! `T-life-areas-are-data`: a life area is a user-managed row, not a closed
//! enum, so there is no `parse`-into-a-variant the way `Priority` or `Period`
//! have. What still belongs here is the part that survives changing HTTP for
//! something else -- whether a submitted name is well-formed -- because it is
//! decidable from the string alone, with no database. Whether a *particular*
//! name currently names a real, active row needs the `life_areas` table and
//! is the adapter's job (`T-capability-owns-its-queries`).
//!
//! **Where sameness lives, and why not here.** Two names refer to the same
//! life area when they are equal once trimmed and case-folded -- "Work" and
//! "work" scattering tasks between two indistinguishable picker entries is
//! the failure this rule exists to prevent. That rule is enforced by the
//! schema: `life_areas.name` is `UNIQUE COLLATE NOCASE`, so the database
//! refuses the second row outright, and the lookups that report the
//! duplicate before the constraint fires match through the same collation.
//! A pure `same_name` here would be a second statement of it that nothing
//! calls and nothing keeps honest -- the one place it must stay true is the
//! column. Two things follow. A database swap must carry the collation
//! across (`T-sqlite-sqlx`'s "mechanical" Postgres move needs `CITEXT` or a
//! functional unique index, not just a column type), and if the rule ever
//! outgrows what a collation can express -- Unicode folding, say -- it comes
//! back here and the constraint becomes the backstop rather than the rule.

/// Why a submitted name does not describe a life area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameRejection {
    /// Present but, once trimmed, has nothing left.
    Blank,
}

/// Trims `raw` and rejects a name that is blank once trimmed. Trim runs
/// before the blank check -- whitespace-only input must be rejected, not
/// silently accepted as a name that then renders as nothing.
pub fn parse_name(raw: &str) -> Result<String, NameRejection> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Err(NameRejection::Blank)
    } else {
        Ok(trimmed.to_string())
    }
}

/// Trims `raw` and collapses blank (or absent) to `None` -- the triage-time
/// reading of a life area name, now that `T-life-area-required-at-triage`
/// is superseded by `D-context-tags-are-the-taxonomy`: a life area is still
/// validated against the active set when one is given (the adapter's job,
/// unchanged), but giving none is no longer a rejection. [`parse_name`]
/// stays the rule for a life area's own required name -- adding one still
/// demands it.
pub fn optional_name(raw: Option<&str>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_name_trims_surrounding_whitespace() {
        assert_eq!(
            optional_name(Some("  Side project  ")),
            Some("Side project".to_string())
        );
    }

    #[test]
    fn optional_name_is_none_for_an_absent_name() {
        assert_eq!(optional_name(None), None);
    }

    #[test]
    fn optional_name_is_none_for_an_empty_name() {
        assert_eq!(optional_name(Some("")), None);
    }

    #[test]
    fn optional_name_is_none_for_a_whitespace_only_name() {
        assert_eq!(optional_name(Some("   ")), None);
    }

    #[test]
    fn parse_name_trims_surrounding_whitespace() {
        assert_eq!(
            parse_name("  Side project  "),
            Ok("Side project".to_string())
        );
    }

    #[test]
    fn parse_name_rejects_a_blank_name() {
        assert_eq!(parse_name(""), Err(NameRejection::Blank));
    }

    #[test]
    fn parse_name_rejects_a_whitespace_only_name() {
        assert_eq!(parse_name("   "), Err(NameRejection::Blank));
    }

    #[test]
    fn parse_name_keeps_a_well_formed_name_as_is() {
        assert_eq!(parse_name("Work"), Ok("Work".to_string()));
    }
}
