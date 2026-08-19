//! Property tests for `scheduler_core::interval` -- the algebra `schedule()`
//! subtracts its occupied set with today, and the one M4's calendar busy
//! and M3's later slices will subtract theirs with tomorrow.
//!
//! It arrived at #75 as two private helpers inside `schedule`, covered only
//! by four example tests that walk one interval against one cut. That is
//! enough to show the three cases someone thought of; it is not enough for
//! a function whose whole job is to be right about every arrangement of two
//! spans, which is exactly the shape a property test is for.
//!
//! **The load-bearing property is [`subtraction_removes_exactly_the_instants
//! _the_subtrahend_covers`]**: it states set subtraction itself -- an
//! instant survives if and only if it was in the input and in nothing
//! removed -- rather than any consequence of it. The rest pin the
//! *representation* choices the callers depend on and the equality above
//! cannot see: order, disjointness, and no zero-length pieces.
//!
//! Kept separate and `#[ignore]`d per the project convention: normal
//! verification runs `cargo test --workspace`, property verification runs
//! it with `-- --include-ignored`.

use proptest::prelude::*;
use scheduler_core::interval::{subtract_all, total_duration_ms, Interval};

/// The span every generator here works inside, and [`SPAN`] is the probe
/// range too, so a probed instant is as likely to land inside a subtracted
/// hole as outside one.
const SPAN: i64 = 200;

/// Deliberately cramped: starts across the whole span with lengths of
/// 1..20, so a generated subtrahend lands on a generated set constantly. A
/// generator spread thinly enough that the subtrahend usually misses would
/// make every property below pass by never asking the function to cut
/// anything (#73's lesson).
fn any_interval() -> impl Strategy<Value = Interval> {
    (0..SPAN, 1..20i64).prop_map(|(start_ms, length)| Interval {
        start_ms,
        end_ms: start_ms + length,
    })
}

/// A sorted, disjoint set -- what every producer in this crate returns, and
/// therefore the only input shape `subtract_all`'s ordering guarantee is
/// claimed for.
///
/// Generated as *gap-then-length* from a moving cursor, so it is sorted and
/// disjoint by construction: no sort, no rejection, no dropped samples, and
/// nothing branching on what came before.
fn any_disjoint_set() -> impl Strategy<Value = Vec<Interval>> {
    prop::collection::vec((0..15i64, 1..20i64), 0..6).prop_map(|steps| {
        let mut cursor = 0;
        steps
            .into_iter()
            .map(|(gap, length)| {
                let start_ms = cursor + gap;
                cursor = start_ms + length;
                Interval {
                    start_ms,
                    end_ms: cursor,
                }
            })
            .collect()
    })
}

/// Free-form: overlapping and unsorted are both legal for the subtrahend,
/// and `schedule`'s occupied set is exactly that -- blocks pushed in
/// placement order across several life areas.
fn any_set() -> impl Strategy<Value = Vec<Interval>> {
    prop::collection::vec(any_interval(), 0..6)
}

fn covers(intervals: &[Interval], instant_ms: i64) -> bool {
    intervals
        .iter()
        .any(|interval| interval.contains(instant_ms))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, ..ProptestConfig::default() })]

    /// Subtraction, stated as subtraction: an instant is in the result
    /// exactly when it was in the input and in nothing removed. Every other
    /// property here is a consequence of this one or a statement about the
    /// representation; this is the definition.
    #[test]
    #[ignore]
    fn subtraction_removes_exactly_the_instants_the_subtrahend_covers(
        intervals in any_disjoint_set(),
        remove in any_set(),
        probe in 0..SPAN,
    ) {
        let result = subtract_all(&intervals, &remove);

        prop_assert_eq!(
            covers(&result, probe),
            covers(&intervals, probe) && !covers(&remove, probe),
            "instant {} disagreed: input {:?}, removed {:?}, result {:?}",
            probe, intervals, remove, result
        );
    }

    /// The ordering guarantee the doc comment makes and `schedule`'s
    /// `earliest_fit` relies on: a sorted, disjoint input comes back sorted
    /// and disjoint without `subtract_all` owing a sort.
    #[test]
    #[ignore]
    fn subtracting_from_a_sorted_disjoint_set_leaves_it_sorted_and_disjoint(
        intervals in any_disjoint_set(),
        remove in any_set(),
    ) {
        let result = subtract_all(&intervals, &remove);

        for pair in result.windows(2) {
            prop_assert!(
                pair[0].end_ms <= pair[1].start_ms,
                "out of order or overlapping: {:?} then {:?}", pair[0], pair[1]
            );
        }
    }

    /// No zero-length piece ever comes out. A zero-length interval is
    /// something a caller could still try to place work into -- the same
    /// reason `free_intervals` contributes nothing at all for an excluded
    /// date rather than an empty span.
    #[test]
    #[ignore]
    fn subtraction_never_produces_a_zero_length_piece(
        intervals in any_disjoint_set(),
        remove in any_set(),
    ) {
        for piece in subtract_all(&intervals, &remove) {
            prop_assert!(
                piece.duration_ms() > 0,
                "zero-length piece {:?} from {:?} minus {:?}", piece, intervals, remove
            );
        }
    }

    /// Subtraction is idempotent: removing the same set again finds nothing
    /// left to remove. `schedule` subtracts a *growing* occupied set once
    /// per task, re-removing everything it removed the task before, so this
    /// is the shape its inner loop actually runs in.
    #[test]
    #[ignore]
    fn subtracting_the_same_set_twice_changes_nothing_the_second_time(
        intervals in any_disjoint_set(),
        remove in any_set(),
    ) {
        let once = subtract_all(&intervals, &remove);
        let twice = subtract_all(&once, &remove);

        prop_assert_eq!(once, twice);
    }

    /// Subtracting nothing is the identity -- the empty-`busy`, empty-`pins`
    /// case every call `schedule()` makes today takes.
    #[test]
    #[ignore]
    fn subtracting_nothing_returns_the_input_unchanged(
        intervals in any_disjoint_set(),
    ) {
        prop_assert_eq!(subtract_all(&intervals, &[]), intervals);
    }

    /// Time only ever leaves. `place_one` compares `total_duration_ms` of
    /// the survivors against an estimate to tell `CapacityExceeded` from
    /// `ChunkPolicyUnsatisfiable`, so a subtraction that could invent
    /// duration would report the wrong reason rather than crash.
    #[test]
    #[ignore]
    fn subtraction_never_increases_the_total_duration(
        intervals in any_disjoint_set(),
        remove in any_set(),
    ) {
        let before = total_duration_ms(&intervals);
        let after = total_duration_ms(&subtract_all(&intervals, &remove));

        prop_assert!(after <= before, "{} became {}", before, after);
        prop_assert!(after >= 0);
    }
}
