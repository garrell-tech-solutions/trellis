//! **Forward pass only** (M3 slice 1 of 5, #75): place committed tasks into
//! free time, least slack first, and say why the rest did not fit.
//!
//! `D-placed-whole-or-not-at-all`: a task that does not fit one free
//! interval whole is unplaceable, never chopped -- splitting is S3's.
//! `T-hard-refuses-soft-slips`: a hard deadline that cannot be met makes a
//! task unplaceable (`UnplaceableReason::DeadlineUnreachable`); a soft one
//! may be placed late, and its projected finish is simply its block's own
//! `end_ms`.
//!
//! [`schedule`] is the ratified seven-parameter signature
//! (`T-fact-plan-line`). This slice **takes all seven and uses four** --
//! `pins`, `facts` and `prior_plan` are accepted and empty, on purpose:
//! nothing yet produces pins, Constraints-layer facts, or a previous plan
//! to feed them, and faking a source would be worse than saying so.
//! `tasks`, `busy`, `windows` and `now` are the four this slice actually
//! reads.

use crate::free_time::Interval;
use crate::task::{DeadlineType, Priority};

/// One committed task as [`schedule`] needs it -- already resolved to a
/// life area and an estimate, both required at triage
/// (`T-capacity-never-under-reports-demand`: a task with no estimate cannot
/// occur here, and the reason enum does not grow for it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleTask {
    pub id: i64,
    pub life_area_id: i64,
    pub estimated_minutes: i64,
    pub deadline_ms: i64,
    pub deadline_type: DeadlineType,
    pub priority: Priority,
}

/// One life area's free time for the horizon -- `scheduler_core::free_time::
/// free_intervals`'s own answer, guardrail minus exceptions, read through
/// that front door rather than reprojected here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifeAreaWindow {
    pub life_area_id: i64,
    pub intervals: Vec<Interval>,
}

/// Why a task could not be placed. Closed, and the order below is the order
/// they are tried in -- more than one can be true of the same task at once,
/// so which is reported is part of the contract (#75's QA doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnplaceableReason {
    /// The life area has no hours in the horizon at all -- never scheduled,
    /// no guardrail, or every hour removed by an exception.
    NoWindow,
    /// A hard deadline falls before the task could finish even placed
    /// first -- checked against the life area's raw window, before any
    /// competition from another task.
    DeadlineUnreachable,
    /// Not enough free time remains in the task's life area after the
    /// tasks ranked above it.
    CapacityExceeded,
    /// Time remains, but no single free interval is long enough to hold the
    /// task whole (`D-placed-whole-or-not-at-all`'s own failure mode).
    ChunkPolicyUnsatisfiable,
}

/// One task placed on the schedule. `end_ms` doubles as the projected
/// finish `T-hard-refuses-soft-slips` names -- nothing splits at this
/// slice, so a task's block and its finish are the same instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedBlock {
    pub task_id: i64,
    pub start_ms: i64,
    pub end_ms: i64,
}

/// One task that did not fit, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unplaceable {
    pub task_id: i64,
    pub reason: UnplaceableReason,
}

/// [`schedule`]'s whole answer: the partition invariant 5 asserts is total
/// -- every task in `tasks` appears in exactly one of these two lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScheduleResult {
    pub placed: Vec<PlacedBlock>,
    pub unplaceable: Vec<Unplaceable>,
}

fn priority_rank(priority: Priority) -> u8 {
    match priority {
        Priority::P1 => 0,
        Priority::P2 => 1,
        Priority::P3 => 2,
        Priority::P4 => 3,
    }
}

/// Least slack first, then priority, then task id ascending -- the total
/// order `schedule-least-slack-first-02`'s own tiebreak exists for: S5's
/// regeneration property demands byte-identical placements, and two tasks
/// equal on slack and priority would otherwise be ordered by whatever the
/// caller happened to list them in.
fn slack_ms(task: &ScheduleTask, now: i64) -> i64 {
    task.deadline_ms - now - task.estimated_minutes * 60_000
}

fn ordered_tasks(tasks: &[ScheduleTask], now: i64) -> Vec<&ScheduleTask> {
    let mut ordered: Vec<&ScheduleTask> = tasks.iter().collect();
    ordered.sort_by(|a, b| {
        slack_ms(a, now)
            .cmp(&slack_ms(b, now))
            .then_with(|| priority_rank(a.priority).cmp(&priority_rank(b.priority)))
            .then_with(|| a.id.cmp(&b.id))
    });
    ordered
}

/// Drops `interval` once it has fully ended before `now` -- a forward pass
/// never places work in an interval that is already over. An interval still
/// open at `now` keeps its own start rather than being clamped forward to
/// `now` exactly: `Clock::pinned_at` keeps advancing for as long as a
/// server runs, so two reads of "now" a request apart routinely differ by a
/// millisecond, and clamping an already-open interval's start to whichever
/// reading happened to land would erode its usable duration by that same
/// millisecond -- turning an interval exactly as long as a task's estimate
/// unusable for that task, the way a 2h guardrail band and a 120-minute
/// task are meant to fit exactly (`schedule-places-committed-work-01`,
/// `schedule-capacity-exceeded-05`). Whether `now` falls a millisecond
/// before or after an interval's own start changes nothing about which
/// interval it is.
fn drop_if_past(interval: Interval, now: i64) -> Option<Interval> {
    (interval.end_ms > now).then_some(interval)
}

fn window_for(windows: &[LifeAreaWindow], life_area_id: i64) -> &[Interval] {
    windows
        .iter()
        .find(|window| window.life_area_id == life_area_id)
        .map(|window| window.intervals.as_slice())
        .unwrap_or(&[])
}

/// `intervals`, minus every instant any interval in `remove` covers.
/// General interval subtraction: each original interval may be split into
/// zero, one or two pieces per subtrahend.
fn subtract_all(intervals: &[Interval], remove: &[Interval]) -> Vec<Interval> {
    let mut result = intervals.to_vec();
    for cut in remove {
        result = result
            .into_iter()
            .flat_map(|interval| subtract_one(interval, *cut))
            .collect();
    }
    result
}

fn subtract_one(interval: Interval, remove: Interval) -> Vec<Interval> {
    if remove.end_ms <= interval.start_ms || remove.start_ms >= interval.end_ms {
        return vec![interval];
    }
    let mut pieces = Vec::new();
    if remove.start_ms > interval.start_ms {
        pieces.push(Interval {
            start_ms: interval.start_ms,
            end_ms: remove.start_ms,
        });
    }
    if remove.end_ms < interval.end_ms {
        pieces.push(Interval {
            start_ms: remove.end_ms,
            end_ms: interval.end_ms,
        });
    }
    pieces
}

/// The earliest interval in `intervals` that can hold `estimate_ms` whole,
/// finishing at or before `deadline_ms` when one applies
/// (`D-placed-whole-or-not-at-all`: whole or not at all, never a partial
/// interval).
fn earliest_fit(
    intervals: &[Interval],
    estimate_ms: i64,
    deadline_ms: Option<i64>,
) -> Option<Interval> {
    intervals
        .iter()
        .filter(|interval| interval.duration_ms() >= estimate_ms)
        .filter(|interval| {
            deadline_ms.is_none_or(|deadline| interval.start_ms + estimate_ms <= deadline)
        })
        .min_by_key(|interval| interval.start_ms)
        .copied()
}

/// `task`'s own life area's raw window, with anything already past dropped
/// -- the state every reason but [`UnplaceableReason::NoWindow`] presumes
/// is non-empty.
fn raw_window_for(task: &ScheduleTask, windows: &[LifeAreaWindow], now: i64) -> Vec<Interval> {
    window_for(windows, task.life_area_id)
        .iter()
        .filter_map(|interval| drop_if_past(*interval, now))
        .collect()
}

/// `task`'s deadline, but only when it is a feasibility constraint --
/// `T-hard-refuses-soft-slips`: a soft deadline never gates placement.
fn hard_deadline_of(task: &ScheduleTask) -> Option<i64> {
    match task.deadline_type {
        DeadlineType::Hard => Some(task.deadline_ms),
        DeadlineType::Soft => None,
    }
}

/// Whether a hard deadline could be met at all, ignoring every other task --
/// `UnplaceableReason::DeadlineUnreachable`'s own definition, "even placed
/// first". Vacuously true for a soft deadline, which this reason never
/// applies to.
fn reachable_even_placed_first(
    raw_window: &[Interval],
    estimate_ms: i64,
    hard_deadline: Option<i64>,
) -> bool {
    hard_deadline.is_none() || earliest_fit(raw_window, estimate_ms, hard_deadline).is_some()
}

fn total_free_ms(free: &[Interval]) -> i64 {
    free.iter().map(|interval| interval.duration_ms()).sum()
}

fn block_at(task: &ScheduleTask, interval: Interval, estimate_ms: i64) -> PlacedBlock {
    PlacedBlock {
        task_id: task.id,
        start_ms: interval.start_ms,
        end_ms: interval.start_ms + estimate_ms,
    }
}

/// The reason a placement search that reached this point still failed:
/// competition left real capacity but no interval was long enough
/// (`ChunkPolicyUnsatisfiable`), or a hard deadline ruled out every interval
/// long enough (`DeadlineUnreachable`) -- [`reachable_even_placed_first`]
/// only ruled out the best case, not this one.
fn reason_when_no_fit(hard_deadline: Option<i64>) -> UnplaceableReason {
    if hard_deadline.is_some() {
        UnplaceableReason::DeadlineUnreachable
    } else {
        UnplaceableReason::ChunkPolicyUnsatisfiable
    }
}

fn place_one(
    task: &ScheduleTask,
    occupied: &[Interval],
    windows: &[LifeAreaWindow],
    now: i64,
) -> Result<PlacedBlock, UnplaceableReason> {
    let raw_window = raw_window_for(task, windows, now);
    if raw_window.is_empty() {
        return Err(UnplaceableReason::NoWindow);
    }

    let estimate_ms = task.estimated_minutes * 60_000;
    let hard_deadline = hard_deadline_of(task);
    if !reachable_even_placed_first(&raw_window, estimate_ms, hard_deadline) {
        return Err(UnplaceableReason::DeadlineUnreachable);
    }

    let free = subtract_all(&raw_window, occupied);
    if total_free_ms(&free) < estimate_ms {
        return Err(UnplaceableReason::CapacityExceeded);
    }

    earliest_fit(&free, estimate_ms, hard_deadline)
        .map(|interval| block_at(task, interval, estimate_ms))
        .ok_or_else(|| reason_when_no_fit(hard_deadline))
}

/// Places every committed task it can into free time, least slack first,
/// and reports why the rest did not fit. Forward pass only: no splitting,
/// no pins, no backward pass (all later slices).
///
/// `busy`, `pins`, `facts` and `prior_plan` are every other source of
/// already-occupied or already-decided time (`T-fact-plan-line`); this
/// slice honestly has none of the latter three to offer, so callers pass
/// empty slices for them. `busy` is real input -- it seeds the occupied set
/// every subsequent placement subtracts from, alongside every block this
/// very call places, which is what keeps two tasks from landing on the same
/// instant even across two life areas whose guardrails overlap in clock
/// time (`D-life-area-owns-its-time`).
pub fn schedule(
    tasks: &[ScheduleTask],
    busy: &[Interval],
    windows: &[LifeAreaWindow],
    pins: &[Interval],
    facts: &[Interval],
    prior_plan: &[PlacedBlock],
    now: i64,
) -> ScheduleResult {
    let _ = (pins, facts, prior_plan);

    let mut occupied: Vec<Interval> = busy.to_vec();
    let mut result = ScheduleResult::default();

    for task in ordered_tasks(tasks, now) {
        match place_one(task, &occupied, windows, now) {
            Ok(block) => {
                occupied.push(Interval {
                    start_ms: block.start_ms,
                    end_ms: block.end_ms,
                });
                result.placed.push(block);
            }
            Err(reason) => result.unplaceable.push(Unplaceable {
                task_id: task.id,
                reason,
            }),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR_MS: i64 = 3_600_000;
    const NOW: i64 = 1_755_421_200_000; // 2026-08-17T09:00:00Z (a Monday)

    fn interval(start_hours: i64, end_hours: i64) -> Interval {
        Interval {
            start_ms: NOW + start_hours * HOUR_MS,
            end_ms: NOW + end_hours * HOUR_MS,
        }
    }

    fn window(life_area_id: i64, intervals: Vec<Interval>) -> LifeAreaWindow {
        LifeAreaWindow {
            life_area_id,
            intervals,
        }
    }

    fn task(
        id: i64,
        estimated_minutes: i64,
        deadline_hours: i64,
        deadline_type: DeadlineType,
    ) -> ScheduleTask {
        ScheduleTask {
            id,
            life_area_id: 1,
            estimated_minutes,
            deadline_ms: NOW + deadline_hours * HOUR_MS,
            deadline_type,
            priority: Priority::P2,
        }
    }

    fn soft(id: i64, estimated_minutes: i64, deadline_hours: i64) -> ScheduleTask {
        task(id, estimated_minutes, deadline_hours, DeadlineType::Soft)
    }

    fn hard(id: i64, estimated_minutes: i64, deadline_hours: i64) -> ScheduleTask {
        task(id, estimated_minutes, deadline_hours, DeadlineType::Hard)
    }

    // --- schedule-places-committed-work-01 ----------------------------------

    #[test]
    fn a_task_is_placed_at_the_start_of_its_life_areas_first_free_interval() {
        let windows = [window(1, vec![interval(0, 8)])];
        let result = schedule(&[hard(1, 120, 96)], &[], &windows, &[], &[], &[], NOW);

        assert_eq!(
            result.placed,
            vec![PlacedBlock {
                task_id: 1,
                start_ms: NOW,
                end_ms: NOW + 2 * HOUR_MS,
            }]
        );
        assert!(result.unplaceable.is_empty());
    }

    // --- schedule-least-slack-first-02 --------------------------------------

    #[test]
    fn the_tighter_deadline_is_placed_into_the_earlier_interval() {
        let windows = [window(1, vec![interval(0, 2), interval(168, 170)])];
        let tighter = soft(1, 120, 2);
        let looser = soft(2, 120, 200);

        let result = schedule(&[looser, tighter], &[], &windows, &[], &[], &[], NOW);

        let placed_for = |id: i64| result.placed.iter().find(|b| b.task_id == id).unwrap();
        assert_eq!(placed_for(1).start_ms, NOW);
        assert_eq!(placed_for(2).start_ms, NOW + 168 * HOUR_MS);
    }

    #[test]
    fn reversing_which_task_has_the_tighter_deadline_reverses_the_order_placed() {
        let windows = [window(1, vec![interval(0, 2), interval(168, 170)])];
        let tighter = soft(1, 120, 200);
        let looser = soft(2, 120, 2);

        let result = schedule(&[tighter, looser], &[], &windows, &[], &[], &[], NOW);

        let placed_for = |id: i64| result.placed.iter().find(|b| b.task_id == id).unwrap();
        assert_eq!(
            placed_for(2).start_ms,
            NOW,
            "the tighter deadline (task 2) should go first"
        );
        assert_eq!(placed_for(1).start_ms, NOW + 168 * HOUR_MS);
    }

    #[test]
    fn a_soft_deadline_may_be_placed_late_and_its_block_end_is_the_projected_finish() {
        let windows = [window(1, vec![interval(0, 2), interval(168, 170)])];
        let tighter = soft(1, 120, 2);
        let looser = soft(2, 120, 2); // same deadline, placed second by priority/id tiebreak but shares the tighter's slack

        let result = schedule(&[tighter, looser], &[], &windows, &[], &[], &[], NOW);

        let overrun = result.placed.iter().find(|b| b.task_id == 2).unwrap();
        assert_eq!(overrun.end_ms, NOW + 170 * HOUR_MS);
        assert!(
            overrun.end_ms > NOW + 2 * HOUR_MS,
            "task 2's deadline was two hours out"
        );
    }

    #[test]
    fn equal_slack_is_broken_by_priority_then_by_task_id() {
        let windows = [window(1, vec![interval(0, 2), interval(2, 4)])];
        let mut low = soft(2, 60, 4);
        low.priority = Priority::P3;
        let mut high = soft(1, 60, 4);
        high.priority = Priority::P1;

        let result = schedule(&[low, high], &[], &windows, &[], &[], &[], NOW);

        let placed_for = |id: i64| result.placed.iter().find(|b| b.task_id == id).unwrap();
        assert_eq!(
            placed_for(1).start_ms,
            NOW,
            "P1 should be placed before P3 at equal slack"
        );
        assert_eq!(placed_for(2).start_ms, NOW + HOUR_MS);
    }

    // --- schedule-whole-or-not-at-all-03 ------------------------------------

    #[test]
    fn a_task_longer_than_any_single_free_interval_is_unplaceable_never_split() {
        let windows = [window(1, vec![interval(0, 2), interval(168, 170)])];
        let result = schedule(&[soft(1, 180, 300)], &[], &windows, &[], &[], &[], NOW);

        assert!(result.placed.is_empty());
        assert_eq!(
            result.unplaceable,
            vec![Unplaceable {
                task_id: 1,
                reason: UnplaceableReason::ChunkPolicyUnsatisfiable,
            }]
        );
    }

    // --- schedule-hard-deadline-unreachable-04 ------------------------------

    #[test]
    fn a_hard_deadline_that_cannot_be_met_even_placed_first_is_unplaceable() {
        let windows = [window(1, vec![interval(0, 8)])];
        let result = schedule(&[hard(1, 120, 1)], &[], &windows, &[], &[], &[], NOW);

        assert!(result.placed.is_empty());
        assert_eq!(
            result.unplaceable,
            vec![Unplaceable {
                task_id: 1,
                reason: UnplaceableReason::DeadlineUnreachable,
            }]
        );
    }

    #[test]
    fn the_same_deadline_placed_as_soft_instead_of_hard_is_placed_and_overruns() {
        let windows = [window(1, vec![interval(0, 8)])];
        let result = schedule(&[soft(1, 120, 1)], &[], &windows, &[], &[], &[], NOW);

        assert_eq!(result.unplaceable, Vec::new());
        let block = result.placed.first().unwrap();
        assert_eq!(block.start_ms, NOW);
        assert!(
            block.end_ms > NOW + HOUR_MS,
            "the deadline was one hour out"
        );
    }

    // --- schedule-capacity-exceeded-05 --------------------------------------

    #[test]
    fn when_the_hours_run_out_the_task_ranked_last_by_slack_is_the_one_refused() {
        let windows = [window(1, vec![interval(0, 2), interval(168, 170)])];
        let first = soft(1, 120, 2);
        let second = soft(2, 120, 200);
        let third = soft(3, 120, 300);

        let result = schedule(&[first, second, third], &[], &windows, &[], &[], &[], NOW);

        assert_eq!(result.placed.len(), 2);
        assert!(result.placed.iter().any(|b| b.task_id == 1));
        assert!(result.placed.iter().any(|b| b.task_id == 2));
        assert_eq!(
            result.unplaceable,
            vec![Unplaceable {
                task_id: 3,
                reason: UnplaceableReason::CapacityExceeded,
            }]
        );
    }

    // --- schedule-no-window-06 ----------------------------------------------

    #[test]
    fn a_life_area_with_no_window_at_all_places_nothing_and_says_why() {
        let result = schedule(&[soft(1, 60, 300)], &[], &[], &[], &[], &[], NOW);

        assert!(result.placed.is_empty());
        assert_eq!(
            result.unplaceable,
            vec![Unplaceable {
                task_id: 1,
                reason: UnplaceableReason::NoWindow,
            }]
        );
    }

    // --- invariant 1: no two blocks overlap, even across life areas --------

    #[test]
    fn two_life_areas_whose_guardrails_overlap_in_clock_time_never_double_book() {
        let windows = [
            window(1, vec![interval(0, 4)]),
            window(2, vec![interval(0, 4)]),
        ];
        let mut in_area_two = soft(2, 240, 300);
        in_area_two.life_area_id = 2;
        let in_area_one = soft(1, 240, 200); // tighter slack, placed first

        let result = schedule(
            &[in_area_two, in_area_one],
            &[],
            &windows,
            &[],
            &[],
            &[],
            NOW,
        );

        assert_eq!(
            result.placed.len(),
            1,
            "only one task can occupy 09:00-13:00"
        );
        assert_eq!(result.unplaceable.len(), 1);
    }

    // --- busy is real input, not a stub -------------------------------------

    #[test]
    fn busy_time_is_subtracted_from_every_life_areas_free_time() {
        let windows = [window(1, vec![interval(0, 4)])];
        let busy = [interval(0, 2)];
        let result = schedule(&[soft(1, 120, 200)], &busy, &windows, &[], &[], &[], NOW);

        let block = result.placed.first().unwrap();
        assert_eq!(
            block.start_ms,
            NOW + 2 * HOUR_MS,
            "the first two hours are busy"
        );
    }

    // --- the empty partition -------------------------------------------------

    #[test]
    fn no_tasks_is_an_empty_but_total_partition() {
        let result = schedule(&[], &[], &[], &[], &[], &[], NOW);
        assert_eq!(result, ScheduleResult::default());
    }

    // --- forward pass never places in the past --------------------------------

    /// An interval already open at `now` is not clamped forward to `now`
    /// exactly -- it keeps its own start, which is what stops a millisecond
    /// of clock drift from eroding an exact-length interval below an
    /// exact-length task's estimate (see [`drop_if_past`]'s own doc).
    #[test]
    fn a_window_already_open_at_now_keeps_its_own_start() {
        let windows = [window(1, vec![interval(-2, 4)])];
        let result = schedule(&[soft(1, 60, 200)], &[], &windows, &[], &[], &[], NOW);

        assert_eq!(result.placed.first().unwrap().start_ms, NOW - 2 * HOUR_MS);
    }

    #[test]
    fn a_window_that_ends_before_now_contributes_nothing() {
        let windows = [window(1, vec![interval(-4, -2)])];
        let result = schedule(&[soft(1, 60, 200)], &[], &windows, &[], &[], &[], NOW);

        assert_eq!(
            result.unplaceable,
            vec![Unplaceable {
                task_id: 1,
                reason: UnplaceableReason::NoWindow,
            }]
        );
    }

    // --- subtract_all / subtract_one -----------------------------------------

    #[test]
    fn subtract_one_splits_an_interval_around_a_middle_removal() {
        let pieces = subtract_one(interval(0, 10), interval(4, 6));
        assert_eq!(pieces, vec![interval(0, 4), interval(6, 10)]);
    }

    #[test]
    fn subtract_one_leaves_a_non_overlapping_interval_untouched() {
        let pieces = subtract_one(interval(0, 2), interval(4, 6));
        assert_eq!(pieces, vec![interval(0, 2)]);
    }

    #[test]
    fn subtract_one_consumes_an_interval_entirely_covered() {
        let pieces = subtract_one(interval(2, 4), interval(0, 10));
        assert_eq!(pieces, Vec::new());
    }

    #[test]
    fn subtract_all_applies_every_removal_in_turn() {
        let result = subtract_all(&[interval(0, 10)], &[interval(0, 2), interval(8, 10)]);
        assert_eq!(result, vec![interval(2, 8)]);
    }
}
