//! What `/stats` renders: the committed share as a computed thing, never a
//! store row (`T-templates-take-view-models`).
//!
//! Below the sample floor the counts are still facts worth showing, but the
//! share and the fifty-percent standing are not -- `share_percent` and
//! `over_the_line` are `None` rather than a number that looks measured
//! (`D-staleness-unset`'s precedent: instrument first, be honest you have not
//! measured enough).

/// Below this many tasks in the window, a percentage would misrepresent a
/// handful of rows as a measurement.
pub const SAMPLE_FLOOR: i64 = 10;

pub struct StatsView {
    pub committed: i64,
    pub pool: i64,
    pub quota: i64,
    pub in_window: i64,
    pub share_percent: Option<i64>,
    pub over_the_line: Option<bool>,
}

impl StatsView {
    /// Builds the view from the three raw counts. `over_the_line` is computed
    /// from the counts directly (`2 * committed > in_window`), not by reading
    /// back the rounded `share_percent` -- the standing must be legible
    /// without the reader comparing the percentage to 50 themselves, and it
    /// must not itself be wrong because rounding moved the displayed number
    /// across the line.
    pub fn from_counts(committed: i64, pool: i64, quota: i64) -> Self {
        let in_window = committed + pool + quota;
        let has_enough = in_window >= SAMPLE_FLOOR;
        StatsView {
            committed,
            pool,
            quota,
            in_window,
            share_percent: has_enough.then(|| round_percent(committed, in_window)),
            over_the_line: has_enough.then(|| 2 * committed > in_window),
        }
    }
}

/// `committed` as a percentage of `in_window`, to the nearest whole percent.
/// Never called with `in_window == 0`: that case never clears the sample
/// floor, so `StatsView::from_counts` never reaches here for it.
fn round_percent(committed: i64, in_window: i64) -> i64 {
    ((committed as f64 / in_window as f64) * 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_percent_rounds_to_the_nearest_whole_percent() {
        assert_eq!(round_percent(3, 10), 30);
        assert_eq!(round_percent(4, 12), 33);
        assert_eq!(round_percent(5, 12), 42);
    }

    #[test]
    fn from_counts_below_the_sample_floor_reports_counts_but_no_share_or_standing() {
        let view = StatsView::from_counts(4, 5, 0);
        assert_eq!(view.committed, 4);
        assert_eq!(view.pool, 5);
        assert_eq!(view.quota, 0);
        assert_eq!(view.in_window, 9);
        assert_eq!(view.share_percent, None);
        assert_eq!(view.over_the_line, None);
    }

    #[test]
    fn from_counts_against_an_empty_window_does_not_divide_by_zero() {
        let view = StatsView::from_counts(0, 0, 0);
        assert_eq!(view.in_window, 0);
        assert_eq!(view.share_percent, None);
        assert_eq!(view.over_the_line, None);
    }

    #[test]
    fn from_counts_at_the_sample_floor_reports_a_share_and_a_standing() {
        let view = StatsView::from_counts(3, 7, 0);
        assert_eq!(view.in_window, 10);
        assert_eq!(view.share_percent, Some(30));
        assert_eq!(view.over_the_line, Some(false));
    }

    #[test]
    fn from_counts_includes_quota_in_the_denominator() {
        let view = StatsView::from_counts(4, 4, 4);
        assert_eq!(view.in_window, 12);
        assert_eq!(view.share_percent, Some(33));
    }

    #[test]
    fn from_counts_at_exactly_fifty_percent_is_not_over_the_line() {
        let view = StatsView::from_counts(10, 10, 0);
        assert_eq!(view.share_percent, Some(50));
        assert_eq!(view.over_the_line, Some(false));
    }

    #[test]
    fn from_counts_just_over_fifty_percent_is_over_the_line() {
        let view = StatsView::from_counts(11, 9, 0);
        assert_eq!(view.share_percent, Some(55));
        assert_eq!(view.over_the_line, Some(true));
    }

    #[test]
    fn from_counts_one_below_the_floor_still_reports_no_share() {
        let view = StatsView::from_counts(4, 4, 1);
        assert_eq!(view.in_window, 9);
        assert_eq!(view.share_percent, None);
    }
}
