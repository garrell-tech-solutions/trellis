//! Property tests for `scheduler_core::context_tag` (#82,
//! `D-context-tags-are-the-taxonomy`).
//!
//! `normalize` is three lines, and the reason it earns properties rather
//! than another handful of examples is that **it is the only thing standing
//! between free text and migration `0010`'s `CHECK`**. The column says a
//! tag is `NULL`, or non-empty and equal to its own `TRIM`; `normalize` is
//! what every write path runs first. Those are two statements of one rule
//! in two languages, and the example tests cover the four spellings someone
//! thought of -- absent, empty, spaces, ordinary. The generator below
//! covers the ones nobody did: tabs, newlines, interior whitespace, and
//! non-ASCII whitespace, which `str::trim` strips and SQLite's `TRIM` does
//! not.
//!
//! The tie to the schema itself is asserted where the schema exists, in
//! `trellis_server::capture`'s own property -- a pure crate has no database
//! to check against (`T-core-no-tokio`).
//!
//! Kept separate and `#[ignore]`d per the project convention.

use proptest::prelude::*;
use scheduler_core::context_tag::normalize;

/// Text that is mostly whitespace of several kinds, so the interesting
/// cases -- leading, trailing, interior, all-of-it -- are common rather
/// than rare. `\u{a0}` and `\u{3000}` are whitespace to `str::trim` and not
/// to SQLite's `TRIM`, which is exactly the disagreement worth generating.
fn any_raw_tag() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just(" ".to_string()),
            Just("\t".to_string()),
            Just("\n".to_string()),
            Just("\u{a0}".to_string()),
            Just("\u{3000}".to_string()),
            "[a-z@#]{1,4}",
        ],
        0..6,
    )
    .prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, ..ProptestConfig::default() })]

    /// The postcondition migration `0010`'s `CHECK` is written to match:
    /// what comes out is either absent, or non-empty and already trimmed.
    /// A `Some("")` or a `Some(" x ")` is precisely what the column would
    /// refuse, and the write path has no second chance to notice.
    #[test]
    #[ignore]
    fn a_normalized_tag_is_absent_or_non_blank_and_already_trimmed(raw in any_raw_tag()) {
        let Some(tag) = normalize(Some(&raw)) else { return Ok(()) };

        prop_assert!(!tag.is_empty(), "normalize produced an empty tag from {raw:?}");
        prop_assert_eq!(
            tag.trim(), tag.as_str(),
            "normalize produced an untrimmed tag from {:?}", raw
        );
        prop_assert_eq!(
            tag.trim_matches(' '), tag.as_str(),
            "normalize left an ASCII space SQLite's TRIM would strip, from {:?}", raw
        );
    }

    /// Idempotence: feeding a normalized tag back through changes nothing.
    /// This is what makes it safe for `capture::retag` to run the same
    /// resolution over a tag that has already been through it once.
    #[test]
    #[ignore]
    fn normalizing_an_already_normalized_tag_changes_nothing(raw in any_raw_tag()) {
        let once = normalize(Some(&raw));
        let twice = normalize(once.as_deref());

        prop_assert_eq!(once, twice);
    }

    /// Nothing but surrounding whitespace is ever removed. A tag is free
    /// text (`D-context-tags-are-the-taxonomy`) -- interior spacing is the
    /// owner's, and a normalize that quietly collapsed it would make two
    /// tags the owner typed differently compare equal for the wrong reason.
    #[test]
    #[ignore]
    fn normalize_removes_only_surrounding_whitespace(raw in any_raw_tag()) {
        match normalize(Some(&raw)) {
            Some(tag) => prop_assert_eq!(tag.as_str(), raw.trim()),
            None => prop_assert!(
                raw.trim().is_empty(),
                "normalize dropped {:?}, which is not blank", raw
            ),
        }
    }

    /// An absent tag and a blank one are the same answer
    /// (`T-empty-equals-absent`). Stated over generated blanks rather than
    /// the three the example tests name.
    #[test]
    #[ignore]
    fn a_blank_tag_and_an_absent_one_agree(
        blank in prop::collection::vec(
            prop_oneof![Just(" "), Just("\t"), Just("\n"), Just("\u{a0}")], 0..6,
        ).prop_map(|parts| parts.concat()),
    ) {
        prop_assert_eq!(normalize(Some(&blank)), None);
        prop_assert_eq!(normalize(Some(&blank)), normalize(None));
    }
}
