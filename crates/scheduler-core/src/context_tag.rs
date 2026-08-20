//! A context tag: free text, optional, disposable
//! (`D-context-tags-are-the-taxonomy`). Not a managed set -- there is no
//! well-formedness rule beyond trimmed-and-blank-collapses-to-absent
//! (`T-empty-equals-absent`), the same reading a tag gets that
//! `WellFormedTriage`'s life-area name now gets too, since neither is
//! required any more.
//!
//! **Case identity is deliberately not here.** Whether `@HomeDepot` and
//! `@homedepot` are the same tag needs a lookup against every tag stored
//! before -- a database question, and `T-capability-owns-its-queries` puts
//! it at the adapter (`trellis_server::capture`), not in this crate. This
//! module only decides what one submitted string means on its own.

/// Trims `raw` and collapses blank (or absent) to `None`. A tag is
/// optional, so unlike a required field there is no rejection here:
/// omitting one, submitting an empty string, and submitting only
/// whitespace all mean the same thing.
pub fn normalize(raw: Option<&str>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_surrounding_whitespace() {
        assert_eq!(
            normalize(Some("  @supermarket  ")),
            Some("@supermarket".to_string())
        );
    }

    #[test]
    fn normalize_is_none_for_an_absent_tag() {
        assert_eq!(normalize(None), None);
    }

    #[test]
    fn normalize_is_none_for_an_empty_tag() {
        assert_eq!(normalize(Some("")), None);
    }

    #[test]
    fn normalize_is_none_for_a_whitespace_only_tag() {
        assert_eq!(normalize(Some("   ")), None);
    }

    #[test]
    fn normalize_keeps_a_well_formed_tag_as_is() {
        assert_eq!(
            normalize(Some("@homedepot")),
            Some("@homedepot".to_string())
        );
    }
}
