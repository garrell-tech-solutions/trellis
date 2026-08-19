//! `Interval`, and the algebra over it: the span type every producer of
//! time in this product hands to every consumer of it.
//!
//! **Why it is not in `free_time`.** It lived there because `free_time`
//! was the first module to make one, and its own doc already said the type
//! "was never named after a producer" -- but it was still *housed* by one,
//! which cost twice. `schedule` names `free_time` in its imports while
//! having nothing to do with projecting a guardrail; and `free_time`'s own
//! doc predicted M4's calendar busy would need interval-minus-interval
//! subtraction, recorded that it "does not exist here yet", and then #75
//! built exactly that as two private helpers inside `schedule` -- where the
//! second producer cannot reach it and would write it again. A value type
//! and its operations belong together, in a module named after the value.
//!
//! Everything here is closed arithmetic on `(start_ms, end_ms)` pairs. No
//! zone, no civil date, no notion of what put the interval there.

/// One span of time, as the instants it actually covers -- UTC epoch
/// milliseconds (`T-jiff-epoch-millis`), `start_ms` inclusive and `end_ms`
/// exclusive. An interval is what a scheduler could place work into, and
/// that is an instant question: two intervals that are adjacent or
/// overlapping only make sense compared as instants, not as civil clock
/// readings that a DST transition can reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub start_ms: i64,
    pub end_ms: i64,
}

impl Interval {
    pub fn duration_ms(self) -> i64 {
        self.end_ms - self.start_ms
    }

    /// `self`, minus every instant `remove` covers: zero, one or two
    /// pieces, in start order. Never a zero-length piece -- a zero-length
    /// interval is something a caller could still try to place work into,
    /// which is the same reason `free_intervals` contributes nothing at all
    /// for an excluded date rather than an empty span.
    pub fn subtract(self, remove: Interval) -> Vec<Interval> {
        if remove.end_ms <= self.start_ms || remove.start_ms >= self.end_ms {
            return vec![self];
        }
        let mut pieces = Vec::new();
        if remove.start_ms > self.start_ms {
            pieces.push(Interval {
                start_ms: self.start_ms,
                end_ms: remove.start_ms,
            });
        }
        if remove.end_ms < self.end_ms {
            pieces.push(Interval {
                start_ms: remove.end_ms,
                end_ms: self.end_ms,
            });
        }
        pieces
    }

    /// Whether `self` covers `instant`. `end_ms` is exclusive, so an
    /// interval and the one starting where it ends never both claim the
    /// same instant.
    pub fn contains(self, instant_ms: i64) -> bool {
        self.start_ms <= instant_ms && instant_ms < self.end_ms
    }
}

/// `intervals`, minus every instant any interval in `remove` covers.
///
/// Order is preserved rather than restored: each interval's own pieces come
/// out in start order and in its own place, so a sorted, disjoint input
/// stays sorted and disjoint without this function owing a sort. That
/// matters for the two callers it exists for -- `schedule`'s occupied set,
/// and (at M4) calendar busy -- because a forward pass reading "the
/// earliest interval that fits" wants the cheap answer, not a re-sort per
/// task.
pub fn subtract_all(intervals: &[Interval], remove: &[Interval]) -> Vec<Interval> {
    let mut result = intervals.to_vec();
    for cut in remove {
        result = result
            .into_iter()
            .flat_map(|interval| interval.subtract(*cut))
            .collect();
    }
    result
}

/// How much time `intervals` covers in total. Meaningful only for a
/// disjoint set, which is what every producer in this crate returns.
pub fn total_duration_ms(intervals: &[Interval]) -> i64 {
    intervals
        .iter()
        .map(|interval| interval.duration_ms())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(start_ms: i64, end_ms: i64) -> Interval {
        Interval { start_ms, end_ms }
    }

    #[test]
    fn duration_is_the_span_between_the_two_instants() {
        assert_eq!(interval(1_000, 4_000).duration_ms(), 3_000);
    }

    #[test]
    fn contains_is_start_inclusive_and_end_exclusive() {
        let span = interval(1_000, 2_000);
        assert!(span.contains(1_000));
        assert!(span.contains(1_999));
        assert!(!span.contains(2_000));
        assert!(!span.contains(999));
    }

    #[test]
    fn subtract_splits_an_interval_around_a_middle_removal() {
        assert_eq!(
            interval(0, 10).subtract(interval(4, 6)),
            vec![interval(0, 4), interval(6, 10)]
        );
    }

    #[test]
    fn subtract_leaves_a_non_overlapping_interval_untouched() {
        assert_eq!(
            interval(0, 2).subtract(interval(4, 6)),
            vec![interval(0, 2)]
        );
    }

    #[test]
    fn subtract_leaves_a_merely_adjacent_interval_untouched() {
        assert_eq!(
            interval(0, 2).subtract(interval(2, 6)),
            vec![interval(0, 2)]
        );
        assert_eq!(
            interval(2, 6).subtract(interval(0, 2)),
            vec![interval(2, 6)]
        );
    }

    #[test]
    fn subtract_consumes_an_interval_entirely_covered() {
        assert_eq!(interval(2, 4).subtract(interval(0, 10)), Vec::new());
    }

    #[test]
    fn subtract_trims_an_interval_overlapped_at_one_end() {
        assert_eq!(
            interval(0, 10).subtract(interval(0, 4)),
            vec![interval(4, 10)]
        );
        assert_eq!(
            interval(0, 10).subtract(interval(6, 20)),
            vec![interval(0, 6)]
        );
    }

    #[test]
    fn subtract_all_applies_every_removal_in_turn() {
        assert_eq!(
            subtract_all(&[interval(0, 10)], &[interval(0, 2), interval(8, 10)]),
            vec![interval(2, 8)]
        );
    }

    #[test]
    fn subtract_all_with_nothing_to_remove_is_the_input() {
        assert_eq!(subtract_all(&[interval(0, 10)], &[]), vec![interval(0, 10)]);
    }

    #[test]
    fn total_duration_sums_every_interval() {
        assert_eq!(total_duration_ms(&[interval(0, 10), interval(20, 25)]), 15);
    }

    #[test]
    fn total_duration_of_nothing_is_zero() {
        assert_eq!(total_duration_ms(&[]), 0);
    }
}
