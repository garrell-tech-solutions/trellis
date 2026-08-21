//! What a context tag is: free text, always optional
//! (`D-context-tags-are-the-taxonomy`).
//!
//! Deliberately the opposite of a life area's `parse_name` (gone with #88,
//! but the contrast is why this module reads the way it does): a life area
//! was a name in a managed set, so an empty one was a submitter's mistake and
//! a rejection was correct. A context tag has no managed set to belong to —
//! `D-quotas-are-selected-not-typed` is the other half of that asymmetry,
//! written down so nobody reaches for a rejection here by habit. There is no
//! `TagRejection` type because there is nothing to reject: the whole of this
//! module's job is deciding what "no tag" means when it might arrive as an
//! absent field, an empty string, or whitespace (`T-empty-equals-absent`).
//!
//! **Where case identity lives, and why not here.** `@HomeDepot` and
//! `@homedepot` are one tag, shown as the spelling first typed
//! (`context-tags-case-is-one-tag-06`). Deciding *that* two names are the
//! same needs no database; deciding *which existing spelling wins* needs a
//! lookup against every tag stored before, which this crate does not have
//! (`T-core-no-tokio`). That half lives in `trellis_server::capture`, the one
//! front door both a fresh capture and a triage-time retag go through.

/// Trims `raw` and reports `None` for an absent tag, an empty string, or one
/// that is only whitespace — the three ways "no tag" arrives, all meaning
/// the same thing (`T-empty-equals-absent`).
pub fn normalize(raw: Option<&str>) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_reports_none_for_an_absent_tag() {
        assert_eq!(normalize(None), None);
    }

    #[test]
    fn normalize_reports_none_for_an_empty_tag() {
        assert_eq!(normalize(Some("")), None);
    }

    #[test]
    fn normalize_reports_none_for_a_whitespace_only_tag() {
        assert_eq!(normalize(Some("   ")), None);
    }

    #[test]
    fn normalize_trims_surrounding_whitespace() {
        assert_eq!(
            normalize(Some("  @supermarket  ")),
            Some("@supermarket".to_string())
        );
    }

    #[test]
    fn normalize_keeps_a_well_formed_tag_as_is() {
        assert_eq!(
            normalize(Some("@homedepot")),
            Some("@homedepot".to_string())
        );
    }
}
