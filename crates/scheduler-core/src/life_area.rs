//! What a life-area name is, and when two of them name the same thing.
//!
//! `T-life-areas-are-data`: a life area is a user-managed row, not a closed
//! enum, so there is no `parse`-into-a-variant the way `Priority` or `Period`
//! have. What still belongs here is the part that survives changing HTTP for
//! something else -- whether a submitted name is well-formed, and whether two
//! names collide -- because both are decidable from the strings alone, with
//! no database. Whether a *particular* name currently names a real, active
//! row needs the `life_areas` table and is the adapter's job
//! (`T-capability-owns-its-queries`).

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

/// Two names refer to the same life area iff they are equal once trimmed and
/// case-folded. A user-typed name is a different situation from a wire enum
/// -- `Priority::parse("p1")` is deliberately `None`, but "Work" and "work"
/// scattering tasks between two indistinguishable picker entries is the
/// failure this project is trying to avoid, not a feature.
pub fn same_name(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn same_name_ignores_case() {
        assert!(same_name("Work", "work"));
        assert!(same_name("Work", "WORK"));
    }

    #[test]
    fn same_name_ignores_surrounding_whitespace() {
        assert!(same_name("Work", "  Work  "));
    }

    #[test]
    fn same_name_is_false_for_different_names() {
        assert!(!same_name("Work", "Fitness"));
    }
}
