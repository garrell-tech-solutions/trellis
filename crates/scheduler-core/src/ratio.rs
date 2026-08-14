//! The committed:pool ratio — R2's instrumentation of `D-pool-is-default`
//! (#9 AC-6). The calendar is meant to sit around 40% committed; this decides
//! what a window of triaged tasks says about that, weeks before a scheduler
//! is built on the assumption.
//!
//! It lives in the core and not in the `stats` capability because every line
//! of it survives changing the delivery mechanism: how long the window is,
//! how many tasks make a sample worth quoting, where the line sits, and what
//! the share of a set of counts is. None of that mentions HTTP, SQL or a
//! template. `stats::store` fetches the counts and `stats::http` renders the
//! answer; the answer itself is a rule, and rules live here, behind the
//! purity gate rather than beside the adapters they are read by.
//!
//! Nothing here enforces the ratio. Display only — nothing in this product
//! can notify until M7.

use crate::task::{COMMITTED, POOL, QUOTA};

/// Fourteen days, in milliseconds: fourteen 24h days back from now, instant
/// arithmetic on epoch millis, no timezone. Civil-calendar days would make
/// this a `jiff` question and a DST one; the window is a duration, not a
/// range of dates.
pub const WINDOW_MS: i64 = 14 * 24 * 60 * 60 * 1000;

/// Below this many tasks in the window, a percentage would misrepresent a
/// handful of rows as a measurement.
pub const SAMPLE_FLOOR: i64 = 10;

/// The instant the rolling window opens for a reckoning taken at `now_ms`.
pub fn window_start(now_ms: i64) -> i64 {
    now_ms - WINDOW_MS
}

/// How many tasks of each kind a window holds.
///
/// The core's own output port for the counting query, the mirror of
/// [`crate::task::TriageFields`] on the way in: an adapter fills it, so the
/// dependency points inward and the counting adapter carries no ratio policy
/// of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KindCounts {
    pub committed: i64,
    pub pool: i64,
    pub quota: i64,
}

impl KindCounts {
    /// Records `count` tasks stored under the discriminant `kind`.
    ///
    /// The mapping from the stored string to the counter belongs beside the
    /// constants that define those strings — an adapter that spelled
    /// `"committed"` itself would be a second, silent copy of the durable
    /// contract [`crate::task`] already publishes.
    ///
    /// A discriminant outside the three is ignored rather than reported:
    /// `tasks.kind` carries a `CHECK` constraint naming exactly these, so a
    /// fourth value is not a case a caller could hit and handle — it is a
    /// database that has stopped being the one this crate was written for.
    pub fn record(&mut self, kind: &str, count: i64) {
        match kind {
            COMMITTED => self.committed = count,
            POOL => self.pool = count,
            QUOTA => self.quota = count,
            _ => {}
        }
    }

    /// Every task in the window, whatever its kind — the share's denominator.
    pub fn total(self) -> i64 {
        self.committed + self.pool + self.quota
    }
}

/// What a window of counts says about the committed share.
///
/// Below the sample floor the counts are still facts worth showing, but the
/// share and the standing are not: [`share_percent`](Self::share_percent) and
/// [`over_the_line`](Self::over_the_line) are `None` rather than a number
/// that looks measured (`D-staleness-unset`'s precedent: instrument first, be
/// honest you have not measured enough).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommittedShare {
    pub counts: KindCounts,
    pub in_window: i64,
    pub share_percent: Option<i64>,
    pub over_the_line: Option<bool>,
}

impl CommittedShare {
    /// Reckons the share from a window's counts. `over_the_line` is computed
    /// from the counts directly (`2 * committed > in_window`), not by reading
    /// back the rounded `share_percent` — the standing must be legible
    /// without the reader comparing the percentage to 50 themselves, and it
    /// must not itself be wrong because rounding moved the displayed number
    /// across the line.
    pub fn from_counts(counts: KindCounts) -> Self {
        let in_window = counts.total();
        let has_enough = in_window >= SAMPLE_FLOOR;
        CommittedShare {
            counts,
            in_window,
            share_percent: has_enough.then(|| round_percent(counts.committed, in_window)),
            over_the_line: has_enough.then(|| 2 * counts.committed > in_window),
        }
    }
}

/// `committed` as a percentage of `in_window`, to the nearest whole percent.
/// Never called with `in_window == 0`: that case never clears the sample
/// floor, so `CommittedShare::from_counts` never reaches here for it.
fn round_percent(committed: i64, in_window: i64) -> i64 {
    ((committed as f64 / in_window as f64) * 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(committed: i64, pool: i64, quota: i64) -> KindCounts {
        KindCounts {
            committed,
            pool,
            quota,
        }
    }

    #[test]
    fn the_window_opens_its_whole_length_before_the_instant_it_is_taken_at() {
        assert_eq!(window_start(WINDOW_MS), 0);
        assert_eq!(window_start(0), -WINDOW_MS);
    }

    #[test]
    fn the_window_is_fourteen_days() {
        assert_eq!(WINDOW_MS, 14 * 24 * 60 * 60 * 1000);
    }

    #[test]
    fn record_tallies_each_stored_discriminant_against_its_own_counter() {
        let mut tally = KindCounts::default();

        tally.record(COMMITTED, 3);
        tally.record(POOL, 7);
        tally.record(QUOTA, 1);

        assert_eq!(tally, counts(3, 7, 1));
    }

    #[test]
    fn record_ignores_a_discriminant_outside_the_three_kinds() {
        let mut tally = KindCounts::default();

        tally.record("banana", 5);

        assert_eq!(tally, KindCounts::default());
    }

    #[test]
    fn total_is_every_kind_in_the_window() {
        assert_eq!(counts(4, 4, 4).total(), 12);
        assert_eq!(KindCounts::default().total(), 0);
    }

    #[test]
    fn round_percent_rounds_to_the_nearest_whole_percent() {
        assert_eq!(round_percent(3, 10), 30);
        assert_eq!(round_percent(4, 12), 33);
        assert_eq!(round_percent(5, 12), 42);
    }

    #[test]
    fn below_the_sample_floor_the_counts_stand_but_the_share_and_standing_do_not() {
        let share = CommittedShare::from_counts(counts(4, 5, 0));
        assert_eq!(share.counts, counts(4, 5, 0));
        assert_eq!(share.in_window, 9);
        assert_eq!(share.share_percent, None);
        assert_eq!(share.over_the_line, None);
    }

    #[test]
    fn an_empty_window_does_not_divide_by_zero() {
        let share = CommittedShare::from_counts(KindCounts::default());
        assert_eq!(share.in_window, 0);
        assert_eq!(share.share_percent, None);
        assert_eq!(share.over_the_line, None);
    }

    #[test]
    fn at_the_sample_floor_the_share_and_standing_are_reported() {
        let share = CommittedShare::from_counts(counts(3, 7, 0));
        assert_eq!(share.in_window, 10);
        assert_eq!(share.share_percent, Some(30));
        assert_eq!(share.over_the_line, Some(false));
    }

    #[test]
    fn quota_tasks_count_toward_the_denominator() {
        let share = CommittedShare::from_counts(counts(4, 4, 4));
        assert_eq!(share.in_window, 12);
        assert_eq!(share.share_percent, Some(33));
    }

    #[test]
    fn exactly_fifty_percent_is_not_over_the_line() {
        let share = CommittedShare::from_counts(counts(10, 10, 0));
        assert_eq!(share.share_percent, Some(50));
        assert_eq!(share.over_the_line, Some(false));
    }

    #[test]
    fn just_over_fifty_percent_is_over_the_line() {
        let share = CommittedShare::from_counts(counts(11, 9, 0));
        assert_eq!(share.share_percent, Some(55));
        assert_eq!(share.over_the_line, Some(true));
    }

    #[test]
    fn one_task_below_the_floor_still_reports_no_share() {
        let share = CommittedShare::from_counts(counts(4, 4, 1));
        assert_eq!(share.in_window, 9);
        assert_eq!(share.share_percent, None);
    }
}
