//! Which kind's triage panel a capture's row has open (#119).
//!
//! **Presentation state, and a closed domain.** It is never read by triage
//! validation — `scheduler_core::task::TaskKind` decides what a submission
//! is, and this decides only which fields the row is currently showing. But
//! it is closed all the same, to the two kinds that *have* a panel, and it
//! was previously spelled as bare string literals in three places: the
//! boundary check in [`super::http`], and one comparison per kind in
//! [`super::lists`]. Three copies of one domain, none of them naming the
//! `tasks.kind` discriminants they had to agree with.
//!
//! So the domain has an owner. The stored spellings come from
//! `scheduler_core::task`'s own constants rather than being retyped here,
//! because the column holds the same words `tasks.kind` does and a second
//! spelling of them is a second thing to keep in step.

use scheduler_core::task::{COMMITTED, QUOTA};

/// The two kinds whose triage panel a row can have open. Pool files on one
/// tap and never opens a panel (`D-pool-is-default`), so it is not a variant
/// — a row showing "the pool panel" is not a state this product has.
///
/// Being an enum is what makes `committed_open` and `quota_open` mutually
/// exclusive by construction rather than by a comment: there is no value
/// here that is both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShownKind {
    Committed,
    Quota,
}

impl ShownKind {
    /// The boundary check `POST /captures/{id}/kind` makes before writing:
    /// `None` for anything outside the domain, which the handler turns into
    /// a `400`.
    pub(super) fn parse(kind: &str) -> Option<Self> {
        match kind {
            COMMITTED => Some(Self::Committed),
            QUOTA => Some(Self::Quota),
            _ => None,
        }
    }

    /// What the `shown_kind` column stores -- the same word `tasks.kind`
    /// stores for the same kind.
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Committed => COMMITTED,
            Self::Quota => QUOTA,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_with_a_panel_parses_and_reads_back_as_what_it_was() {
        for kind in [COMMITTED, QUOTA] {
            assert_eq!(ShownKind::parse(kind).map(ShownKind::as_str), Some(kind));
        }
    }

    #[test]
    fn pool_has_no_panel_and_does_not_parse() {
        assert_eq!(ShownKind::parse(scheduler_core::task::POOL), None);
    }

    #[test]
    fn a_kind_outside_the_domain_does_not_parse() {
        assert_eq!(ShownKind::parse("nonsense"), None);
        assert_eq!(ShownKind::parse(""), None);
        assert_eq!(ShownKind::parse("Committed"), None);
    }
}
