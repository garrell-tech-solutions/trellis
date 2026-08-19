//! Property tests for `schedule()` (#75, M3 slice 1 of 5): invariants 1-5
//! over >= 1000 cases each, plus the two #11 acceptance criteria that are
//! property-shaped without being invariants -- determinism and idempotence.
//!
//! **Invariant 3 is not vacuous here, only degenerate.** It was recorded as
//! skipped because nothing splits until S3, but "a placed task's chunks sum
//! exactly to its estimate" over a one-chunk task is the checkable claim
//! that a block is exactly as long as the work it holds, which is what
//! `block_at` could get wrong today. It is asserted below in the form this
//! slice can state, and S3 widens it to a sum rather than adding it.
//!
//! **Determinism and idempotence are separate acceptance criteria on #11**,
//! deliberately not invariants (architecture.md: an invariant is falsifiable
//! from one `schedule()` output alone, and these compare two). They are
//! still properties, and until now nothing tested them. The load-bearing
//! one is order-independence: `ordered_tasks`' tiebreak chain ends in the
//! task id precisely so two tasks equal on slack and priority do not land
//! wherever the caller happened to list them -- and `store::committed_tasks`
//! issues no `ORDER BY`, so the caller's order is genuinely SQLite's whim.
//!
//! Kept separate from the unit tests and every case marked `#[ignore]`, per
//! the project convention: normal verification runs `cargo test --workspace`,
//! property verification runs it with `-- --include-ignored`.
//!
//! **#73's lesson is the one that matters most here**: a property is only
//! as strong as the inputs its generator can produce. The generator below
//! deliberately can produce every failure each invariant exists to catch --
//! more tasks than fit comfortably and several life areas whose windows
//! overlap in clock time for invariant 1; multi-interval life areas and
//! estimates near an interval's own length for invariant 2; a mix of hard
//! and soft deadlines, some unreachable, for invariant 4; and task sets
//! large enough to exhaust the hours for invariant 5. Each was confirmed to
//! FAIL with its rule deliberately removed before being trusted (see the
//! handoff brief's gotcha 8).

use proptest::prelude::*;
use scheduler_core::interval::Interval;
use scheduler_core::schedule::{schedule, LifeAreaWindow, ScheduleResult, ScheduleTask};
use scheduler_core::task::{DeadlineType, Priority};

const NOW: i64 = 0;
const HOUR_MS: i64 = 3_600_000;

const LIFE_AREA_IDS: [i64; 4] = [1, 2, 3, 4];

/// One life area's raw window: no hours at all (`no_window` is a real,
/// generatable state), or one to four intervals -- deliberately allowed to
/// land anywhere in a shared 0..500h span so that two different life
/// areas' windows frequently overlap in clock time, which is exactly where
/// invariant 1 could fail (`D-life-area-owns-its-time` makes overlapping
/// guardrails legal).
fn any_window(life_area_id: i64) -> impl Strategy<Value = LifeAreaWindow> {
    let interval = (0..500i64, 1..8i64).prop_map(|(start_hours, duration_hours)| Interval {
        start_ms: NOW + start_hours * HOUR_MS,
        end_ms: NOW + (start_hours + duration_hours) * HOUR_MS,
    });
    prop::collection::vec(interval, 0..4).prop_map(move |mut intervals| {
        intervals.sort_by_key(|i| i.start_ms);
        LifeAreaWindow {
            life_area_id,
            intervals,
        }
    })
}

fn any_windows() -> impl Strategy<Value = Vec<LifeAreaWindow>> {
    (
        any_window(LIFE_AREA_IDS[0]),
        any_window(LIFE_AREA_IDS[1]),
        any_window(LIFE_AREA_IDS[2]),
    )
        .prop_map(|(a, b, c)| vec![a, b, c])
}

fn any_deadline_type() -> impl Strategy<Value = DeadlineType> {
    prop_oneof![Just(DeadlineType::Hard), Just(DeadlineType::Soft)]
}

fn any_priority() -> impl Strategy<Value = Priority> {
    prop_oneof![
        Just(Priority::P1),
        Just(Priority::P2),
        Just(Priority::P3),
        Just(Priority::P4),
    ]
}

/// One task, an estimate chosen to often land near a generated window
/// interval's own length (1..8h, per [`any_window`]) so placement often
/// lands near a seam rather than always comfortably inside one interval --
/// exactly where invariant 2 could fail. `life_area_id` ranges over all
/// four ids, including the fourth `any_windows` never generates a window
/// for, so `no_window` stays reachable here too. The deadline offset spans
/// negative to generous so both an unreachable hard deadline and an ample
/// one are common.
fn any_task(id: i64) -> impl Strategy<Value = ScheduleTask> {
    (
        prop::sample::select(LIFE_AREA_IDS.to_vec()),
        15..=480i64,
        -50..2000i64,
        any_deadline_type(),
        any_priority(),
    )
        .prop_map(
            move |(life_area_id, estimated_minutes, deadline_hours, deadline_type, priority)| {
                ScheduleTask {
                    id,
                    life_area_id,
                    estimated_minutes,
                    deadline_ms: NOW + deadline_hours * HOUR_MS,
                    deadline_type,
                    priority,
                }
            },
        )
}

/// Enough tasks, over enough life areas, to exhaust the hours generated for
/// at least one of them -- invariant 5 (the partition is total) is trivial
/// unless something is actually refused.
fn any_tasks() -> impl Strategy<Value = Vec<ScheduleTask>> {
    (1..=10usize).prop_flat_map(|count| (0..count as i64).map(any_task).collect::<Vec<_>>())
}

/// A task drawn from a deliberately **coarse** grid: two estimates and
/// four deadlines, so `slack_ms` takes only a handful of distinct values
/// and several tasks in a set routinely tie on it. Two life areas rather
/// than four, so the tied tasks compete for the same hours instead of
/// being separated by their windows.
///
/// **This exists because the wide generator could not falsify the property
/// it was written for.** `any_task` spreads estimates over 15..=480 minutes
/// and deadlines over -50..2000 hours; slack is
/// `deadline - now - estimate`, so an exact tie is vanishingly rare, and
/// order-independence is then trivially true of any deterministic sort.
/// Deleting the id tiebreak the property exists to protect left all 1024
/// cases green. #73's lesson, arriving a third time: a property is only as
/// strong as the inputs its generator can produce, and the input this one
/// needs is a *tie*.
fn any_tied_task(id: i64) -> impl Strategy<Value = ScheduleTask> {
    (
        prop::sample::select(vec![LIFE_AREA_IDS[0], LIFE_AREA_IDS[1]]),
        prop::sample::select(vec![60i64, 120]),
        prop::sample::select(vec![2i64, 4, 8, 24]),
        any_deadline_type(),
        prop::sample::select(vec![Priority::P1, Priority::P2]),
    )
        .prop_map(
            move |(life_area_id, estimated_minutes, deadline_hours, deadline_type, priority)| {
                ScheduleTask {
                    id,
                    life_area_id,
                    estimated_minutes,
                    deadline_ms: NOW + deadline_hours * HOUR_MS,
                    deadline_type,
                    priority,
                }
            },
        )
}

/// The same task set twice: once as generated, once permuted. Ids are
/// unique within a set (numbered `0..count`), so `ordered_tasks`' chain
/// really is a total order over it and "identical" is the honest claim,
/// not "equivalent up to ties".
fn any_tasks_and_a_permutation() -> impl Strategy<Value = (Vec<ScheduleTask>, Vec<ScheduleTask>)> {
    (2..=8usize)
        .prop_flat_map(|count| (0..count as i64).map(any_tied_task).collect::<Vec<_>>())
        .prop_flat_map(|tasks| {
            let original = tasks.clone();
            Just(tasks)
                .prop_shuffle()
                .prop_map(move |shuffled| (original.clone(), shuffled))
        })
}

/// A forward pass with the four inputs this slice has nothing to put in.
///
/// `pins`, `facts` and `prior_plan` are accepted-and-empty by design (see
/// `schedule`'s own doc), and spelling three empty slices out at every call
/// site made the one argument that *is* real -- `busy` -- the hardest one
/// to see. [`plan`] is the same call with `busy` empty too.
fn plan_against(
    tasks: &[ScheduleTask],
    busy: &[Interval],
    windows: &[LifeAreaWindow],
) -> ScheduleResult {
    schedule(tasks, busy, windows, &[], &[], &[], NOW)
}

fn plan(tasks: &[ScheduleTask], windows: &[LifeAreaWindow]) -> ScheduleResult {
    plan_against(tasks, &[], windows)
}

fn window_for(windows: &[LifeAreaWindow], life_area_id: i64) -> Option<&LifeAreaWindow> {
    windows.iter().find(|w| w.life_area_id == life_area_id)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, ..ProptestConfig::default() })]

    /// Invariant 1: no two placed blocks overlap, whether they came from
    /// the same life area or two whose windows share clock time.
    #[test]
    #[ignore]
    fn no_two_placed_blocks_overlap(
        tasks in any_tasks(),
        windows in any_windows(),
    ) {
        let result = plan(&tasks, &windows);
        let mut blocks = result.placed.clone();
        blocks.sort_by_key(|b| b.start_ms);
        for pair in blocks.windows(2) {
            prop_assert!(
                pair[0].end_ms <= pair[1].start_ms,
                "overlapping blocks: {:?} and {:?}", pair[0], pair[1]
            );
        }
    }

    /// Invariant 2: every placed block lies entirely within one of its own
    /// life area's raw window intervals.
    #[test]
    #[ignore]
    fn every_placed_block_lies_within_one_of_its_life_areas_intervals(
        tasks in any_tasks(),
        windows in any_windows(),
    ) {
        let result = plan(&tasks, &windows);
        for block in &result.placed {
            let task = tasks.iter().find(|t| t.id == block.task_id).unwrap();
            let window = window_for(&windows, task.life_area_id)
                .expect("a placed task's life area has a window");
            let within_one = window
                .intervals
                .iter()
                .any(|i| i.start_ms <= block.start_ms && block.end_ms <= i.end_ms);
            prop_assert!(
                within_one,
                "block {:?} does not lie within any single interval of {:?}",
                block, window.intervals
            );
        }
    }

    /// Invariant 3, in the form this slice can state it: nothing splits, so
    /// a placed task's one chunk *is* its estimate -- exactly, never a
    /// minute either way (`D-placed-whole-or-not-at-all` is what lets this
    /// be an equality rather than a bound). S3 widens it to a sum over
    /// chunks; it does not introduce it.
    #[test]
    #[ignore]
    fn every_placed_block_is_exactly_as_long_as_the_work_it_holds(
        tasks in any_tasks(),
        windows in any_windows(),
    ) {
        let result = plan(&tasks, &windows);
        for block in &result.placed {
            let task = tasks.iter().find(|t| t.id == block.task_id).unwrap();
            prop_assert_eq!(
                block.end_ms - block.start_ms,
                task.estimated_minutes * 60_000,
                "task {} was given a block that is not its estimate", task.id
            );
        }
    }

    /// Invariant 4: every placed task with a hard deadline finishes at or
    /// before it.
    #[test]
    #[ignore]
    fn every_placed_hard_deadline_task_finishes_by_its_deadline(
        tasks in any_tasks(),
        windows in any_windows(),
    ) {
        let result = plan(&tasks, &windows);
        for block in &result.placed {
            let task = tasks.iter().find(|t| t.id == block.task_id).unwrap();
            if task.deadline_type == DeadlineType::Hard {
                prop_assert!(
                    block.end_ms <= task.deadline_ms,
                    "hard task {} finished at {} after its deadline {}",
                    task.id, block.end_ms, task.deadline_ms
                );
            }
        }
    }

    /// Invariant 5: the placed/unplaceable partition is total -- every task
    /// appears exactly once, on one side or the other.
    #[test]
    #[ignore]
    fn every_task_is_placed_or_unplaceable_exactly_once(
        tasks in any_tasks(),
        windows in any_windows(),
    ) {
        let result = plan(&tasks, &windows);
        let mut seen: Vec<i64> = result.placed.iter().map(|b| b.task_id)
            .chain(result.unplaceable.iter().map(|u| u.task_id))
            .collect();
        seen.sort();
        let mut expected: Vec<i64> = tasks.iter().map(|t| t.id).collect();
        expected.sort();
        prop_assert_eq!(seen, expected);
    }

    /// #11's determinism criterion, stated where it can actually fail: the
    /// plan does not depend on the order the tasks arrived in.
    ///
    /// *"Delete every proposed/published future block, re-run with the same
    /// facts, get byte-identical placements"* is only true if a re-read of
    /// those facts in a different row order produces the same answer, and
    /// `store::committed_tasks` promises no order at all. Deleting
    /// `ordered_tasks`' final `.then_with(|| a.id.cmp(&b.id))` -- the whole
    /// reason that tiebreak exists -- fails this and nothing else in the
    /// suite.
    #[test]
    #[ignore]
    fn the_plan_does_not_depend_on_the_order_the_tasks_arrived_in(
        (tasks, permuted) in any_tasks_and_a_permutation(),
        windows in any_windows(),
    ) {
        let first = plan(&tasks, &windows);
        let second = plan(&permuted, &windows);

        prop_assert_eq!(
            &first, &second,
            "the same tasks in a different order produced a different plan"
        );
    }

    /// #11's idempotence criterion: two consecutive runs on unchanged inputs
    /// agree byte for byte. Weak while `schedule()` is a pure function of
    /// its arguments, which is exactly why it is worth pinning -- the
    /// signature already carries `prior_plan`, and S5 is where a run starts
    /// reading its own previous answer.
    #[test]
    #[ignore]
    fn two_runs_on_unchanged_inputs_agree_byte_for_byte(
        tasks in any_tasks(),
        windows in any_windows(),
    ) {
        let first = plan(&tasks, &windows);
        let second = plan(&tasks, &windows);

        prop_assert_eq!(&first, &second);
    }

    /// Invariant 1 again, from the side `busy` will arrive on: no placed
    /// block overlaps time the caller already declared occupied. `busy` is
    /// the one of `schedule()`'s four constraint inputs that is real today,
    /// and no property covered it -- the unit test that does uses a single
    /// task against a single busy span.
    #[test]
    #[ignore]
    fn no_placed_block_overlaps_time_the_caller_declared_busy(
        tasks in any_tasks(),
        windows in any_windows(),
        busy in any_window(LIFE_AREA_IDS[0]),
    ) {
        let result = plan_against(&tasks, &busy.intervals, &windows);
        for block in &result.placed {
            for occupied in &busy.intervals {
                prop_assert!(
                    block.end_ms <= occupied.start_ms || block.start_ms >= occupied.end_ms,
                    "block {:?} overlaps busy time {:?}", block, occupied
                );
            }
        }
    }
}
