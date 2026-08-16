//! Property tests for life-area names.
//!
//! Kept separate from the unit tests and every case marked `#[ignore]`, per
//! the project convention: normal verification runs `cargo test --workspace`,
//! property verification runs it with `-- --include-ignored`.
//!
//! `parse_name` is the one rule in this crate that runs on text the user
//! typed rather than on a value drawn from a closed domain, so "every other
//! string" is the interesting case rather than an exotic one. The unit tests
//! name the examples the acceptance criteria do -- blank, whitespace-only,
//! padded; these say what has to hold for anything at all a person can type
//! into the box.

use proptest::prelude::*;
use scheduler_core::life_area::{parse_name, NameRejection};

/// Anything at all, including strings that are entirely whitespace.
fn any_text() -> impl Strategy<Value = String> {
    ".{0,40}".prop_map(String::from)
}

/// Text deliberately padded with whitespace, so the trimming path is
/// exercised often rather than only when `any_text` happens to produce it.
fn padded_text() -> impl Strategy<Value = String> {
    ("[ \t\n]{0,4}", ".{0,20}", "[ \t\n]{0,4}")
        .prop_map(|(before, middle, after)| format!("{before}{middle}{after}"))
}

proptest! {
    /// An accepted name has no whitespace left on either end. This is what
    /// makes the stored name and the submitted name the same thing: a name
    /// that kept its padding would be a second entry in the picker that
    /// looks identical to the first.
    #[test]
    #[ignore]
    fn an_accepted_name_is_already_trimmed(raw in padded_text()) {
        if let Ok(name) = parse_name(&raw) {
            prop_assert_eq!(name.trim(), name.as_str());
        }
    }

    /// An accepted name is never empty, so nothing can reach the table that
    /// would render as a blank row in the picker.
    #[test]
    #[ignore]
    fn an_accepted_name_is_never_empty(raw in any_text()) {
        if let Ok(name) = parse_name(&raw) {
            prop_assert!(!name.is_empty());
        }
    }

    /// Parsing normalises, and normalising is finished after one pass:
    /// re-parsing an accepted name returns it unchanged. Without this a name
    /// could differ from itself depending on how many times it had been
    /// through the boundary.
    #[test]
    #[ignore]
    fn parsing_an_accepted_name_again_changes_nothing(raw in padded_text()) {
        if let Ok(name) = parse_name(&raw) {
            prop_assert_eq!(parse_name(&name), Ok(name.clone()));
        }
    }

    /// Trimming only removes: whatever survives was there, contiguously, in
    /// what the user typed. Nothing is invented and nothing is dropped from
    /// the middle.
    #[test]
    #[ignore]
    fn an_accepted_name_is_text_the_submitter_actually_typed(raw in padded_text()) {
        if let Ok(name) = parse_name(&raw) {
            prop_assert!(raw.contains(&name), "{name:?} is not contained in {raw:?}");
        }
    }

    /// Rejection is exactly the case with nothing to name: a submission is
    /// refused if and only if it holds no non-whitespace character. This is
    /// the half a "rejects an empty string" example test cannot show --
    /// that nothing *else* is refused.
    #[test]
    #[ignore]
    fn a_name_is_refused_exactly_when_it_has_no_non_whitespace_character(raw in any_text()) {
        let has_content = raw.chars().any(|c| !c.is_whitespace());

        match parse_name(&raw) {
            Ok(_) => prop_assert!(has_content, "{raw:?} was accepted with no content"),
            Err(NameRejection::Blank) => {
                prop_assert!(!has_content, "{raw:?} was refused despite having content")
            }
        }
    }
}
