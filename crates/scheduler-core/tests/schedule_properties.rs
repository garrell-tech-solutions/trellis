//! Property tests for `schedule()` (#75, M3 slice 1 of 5): invariants 1, 2,
//! 4 and 5 over >= 1000 cases each. Invariant 3 (conservation under
//! splitting) is vacuous at this slice -- nothing splits until S3.
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
use scheduler_core::free_time::Interval;
use scheduler_core::schedule::{schedule, LifeAreaWindow, ScheduleTask};
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
        let result = schedule(&tasks, &[], &windows, &[], &[], &[], NOW);
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
        let result = schedule(&tasks, &[], &windows, &[], &[], &[], NOW);
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

    /// Invariant 4: every placed task with a hard deadline finishes at or
    /// before it.
    #[test]
    #[ignore]
    fn every_placed_hard_deadline_task_finishes_by_its_deadline(
        tasks in any_tasks(),
        windows in any_windows(),
    ) {
        let result = schedule(&tasks, &[], &windows, &[], &[], &[], NOW);
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
        let result = schedule(&tasks, &[], &windows, &[], &[], &[], NOW);
        let mut seen: Vec<i64> = result.placed.iter().map(|b| b.task_id)
            .chain(result.unplaceable.iter().map(|u| u.task_id))
            .collect();
        seen.sort();
        let mut expected: Vec<i64> = tasks.iter().map(|t| t.id).collect();
        expected.sort();
        prop_assert_eq!(seen, expected);
    }
}
