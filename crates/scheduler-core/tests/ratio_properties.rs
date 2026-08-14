//! Property tests for the committed:pool ratio.
//!
//! Kept separate from the unit tests and every case marked `#[ignore]`, per
//! the project convention: normal verification runs `cargo test --workspace`,
//! property verification runs it with `-- --include-ignored`.
//!
//! The unit tests pin the ratio at the values the acceptance criteria name —
//! the floor, exactly fifty percent, one over. These check what has to hold
//! for every other window, and in particular the one claim example tests
//! cannot reach: that the rounded percentage the page prints and the standing
//! printed beside it never disagree. `share_percent` rounds and
//! `over_the_line` does not, so "55%, under the line" is a page that could
//! exist unless something says it cannot.

use proptest::prelude::*;
use scheduler_core::ratio::{window_start, CommittedShare, KindCounts, SAMPLE_FLOOR, WINDOW_MS};
use scheduler_core::task::{COMMITTED, POOL, QUOTA};

/// Counts are row tallies from `COUNT(*)`, so the interesting range is small
/// non-negative numbers. Bounded rather than `any::<i64>()` on purpose: a
/// tally near `i64::MAX` overflows the sum, and a database that produced one
/// is not a case this rule is being asked to survive.
fn any_counts() -> impl Strategy<Value = KindCounts> {
    (0..10_000i64, 0..10_000i64, 0..10_000i64).prop_map(|(committed, pool, quota)| KindCounts {
        committed,
        pool,
        quota,
    })
}

/// Epoch milliseconds across a few centuries either side of now — wide enough
/// to be arbitrary, narrow enough that subtracting a fortnight cannot
/// overflow.
fn any_instant_ms() -> impl Strategy<Value = i64> {
    -10_000_000_000_000i64..10_000_000_000_000i64
}

proptest! {
    /// Conservation: the denominator is every task in the window and nothing
    /// else, so no kind is dropped and none is counted twice.
    #[test]
    #[ignore]
    fn the_window_total_is_every_kind_in_it(counts in any_counts()) {
        let share = CommittedShare::from_counts(counts);

        prop_assert_eq!(
            share.in_window,
            counts.committed + counts.pool + counts.quota
        );
    }

    /// The counts pass through untouched. The page reports them below the
    /// sample floor, where the share and standing are withheld, so they must
    /// survive the reckoning whatever it decides.
    #[test]
    #[ignore]
    fn the_counts_survive_the_reckoning_unchanged(counts in any_counts()) {
        prop_assert_eq!(CommittedShare::from_counts(counts).counts, counts);
    }

    /// The share and the standing appear together or not at all, exactly on
    /// the sample floor. The template nests one inside the other, so a window
    /// with a percentage but no standing would render half an answer.
    #[test]
    #[ignore]
    fn the_share_and_the_standing_are_withheld_together_below_the_floor(counts in any_counts()) {
        let share = CommittedShare::from_counts(counts);
        let enough = share.in_window >= SAMPLE_FLOOR;

        prop_assert_eq!(share.share_percent.is_some(), enough);
        prop_assert_eq!(share.over_the_line.is_some(), enough);
    }

    /// A share is a share: committed is part of the window it is measured
    /// against, so the percentage can never leave 0..=100.
    #[test]
    #[ignore]
    fn a_reported_share_is_a_percentage(counts in any_counts()) {
        if let Some(percent) = CommittedShare::from_counts(counts).share_percent {
            prop_assert!((0..=100).contains(&percent), "share_percent={percent}");
        }
    }

    /// The standing is a strict majority of the window, stated without the
    /// `2 * committed` arithmetic that computes it.
    #[test]
    #[ignore]
    fn the_standing_is_whether_committed_holds_a_strict_majority(counts in any_counts()) {
        let share = CommittedShare::from_counts(counts);

        if let Some(over_the_line) = share.over_the_line {
            let rest = counts.pool + counts.quota;
            prop_assert_eq!(over_the_line, counts.committed > rest);
        }
    }

    /// The one the rounding comment claims and no example test can establish:
    /// the printed percentage never sits on the far side of fifty from the
    /// standing printed beside it. A window at 50.4% rounds to 50 and reads
    /// "over the line", which is honest; 55% marked "under the line" would
    /// not be.
    #[test]
    #[ignore]
    fn the_printed_share_never_contradicts_the_standing(counts in any_counts()) {
        let share = CommittedShare::from_counts(counts);

        match (share.share_percent, share.over_the_line) {
            (Some(percent), Some(true)) => prop_assert!(
                percent >= 50,
                "{percent}% was marked over the line"
            ),
            (Some(percent), Some(false)) => prop_assert!(
                percent <= 50,
                "{percent}% was marked under the line"
            ),
            _ => {}
        }
    }

    /// The instrument points the right way: triaging one more committed task
    /// never moves the reported share down. R2 reads this page to find out
    /// whether the committed load is creeping up, which a measure that could
    /// fall as commitment rises would not tell it.
    #[test]
    #[ignore]
    fn one_more_committed_task_never_lowers_the_share(counts in any_counts()) {
        let before = CommittedShare::from_counts(counts);
        let after = CommittedShare::from_counts(KindCounts {
            committed: counts.committed + 1,
            ..counts
        });

        if let (Some(before), Some(after)) = (before.share_percent, after.share_percent) {
            prop_assert!(after >= before, "share fell from {before}% to {after}%");
        }
    }

    /// The window is a duration: whenever it is taken, it is exactly a
    /// fortnight wide, and a later reckoning looks at a later window.
    #[test]
    #[ignore]
    fn the_window_is_a_fortnight_wide_wherever_it_is_taken(
        now_ms in any_instant_ms(),
        later_by in 1..1_000_000_000i64,
    ) {
        prop_assert_eq!(now_ms - window_start(now_ms), WINDOW_MS);
        prop_assert!(window_start(now_ms + later_by) > window_start(now_ms));
    }

    /// The stored discriminants are the only three that count. Anything else
    /// leaves the tally exactly as it was, so a row this crate does not
    /// recognise cannot silently inflate a kind.
    #[test]
    #[ignore]
    fn only_the_three_stored_discriminants_move_the_tally(
        kind in ".{0,20}",
        count in 1..1_000i64,
    ) {
        let mut tally = KindCounts::default();

        tally.record(&kind, count);

        let known = [COMMITTED, POOL, QUOTA].contains(&kind.as_str());
        prop_assert_eq!(tally != KindCounts::default(), known, "kind={:?}", kind);
        prop_assert_eq!(tally.total(), if known { count } else { 0 });
    }
}
