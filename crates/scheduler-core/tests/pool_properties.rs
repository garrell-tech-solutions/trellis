//! Property tests for `scheduler_core::pool::group` (#92, #122, #129).
//!
//! `group` is the one function on this screen that every rule passes
//! through: it sorts, buckets, ranks, truncates, and decides trip-or-loose,
//! and each of those is asserted by examples that name one shape apiece.
//! The examples say what happens to three items at `@homedepot`. They do
//! not say what happens to *any* pool, and three of `group`'s guarantees
//! are only meaningful stated that way:
//!
//! - **Nothing is invented and nothing is silently lost.** Every task the
//!   caller hands in comes back exactly once, or is dropped for the one
//!   stated reason (a done item with no panel to hold it). A grouping
//!   function that duplicates an item into both a trip and a loose end is
//!   the sort of defect an example finds only if someone guesses it.
//! - **Input order does not matter.** `group` sorts for itself precisely so
//!   a caller cannot break it by forgetting to; the single example that
//!   checks this compares one pair of orderings out of `n!`.
//! - **The trip rule is a disjunction over two counts**
//!   (`pool.rs`'s `is_trip`), and its interesting region is exactly where
//!   the two disagree -- a bucket under the threshold whose run is over it.
//!   Examples sit on the corners; a generator sits everywhere.
//!
//! Kept separate and `#[ignore]`d per the project convention: normal
//! verification runs `cargo test --workspace`, property verification runs
//! it with `-- --include-ignored`.

use proptest::prelude::*;
use scheduler_core::pool::{self, PoolGroups, PoolItem, PoolTask, RunSizes, Trip, TRIP_THRESHOLD};

/// One generated task, before it is given a sequence. `PoolTask` is not
/// `Clone`, and the permutation property needs the same input twice.
#[derive(Debug, Clone)]
struct Spec {
    tag: Option<&'static str>,
    done: bool,
}

/// Three tags over a pool of up to a dozen tasks, so buckets collide often
/// and the threshold is crossed in both directions rather than by accident.
/// `None` is a quarter of the draw: untagged is its own rule, not a rare
/// edge.
fn any_spec() -> impl Strategy<Value = Spec> {
    (
        prop_oneof![
            Just(Some("@a")),
            Just(Some("@b")),
            Just(Some("@c")),
            Just(None),
        ],
        any::<bool>(),
    )
        .prop_map(|(tag, done)| Spec { tag, done })
}

/// Run sizes spanning both sides of [`TRIP_THRESHOLD`] for each tag, plus
/// the absent case -- a tag the adapter did not mention at all.
fn any_run_sizes() -> impl Strategy<Value = Vec<(String, usize)>> {
    prop::collection::vec(
        (prop_oneof![Just("@a"), Just("@b"), Just("@c")], 0usize..6),
        0..4,
    )
    .prop_map(|pairs| {
        pairs
            .into_iter()
            .map(|(tag, members)| (tag.to_string(), members))
            .collect()
    })
}

/// A generated pool: the tasks-to-be, and the run sizes their tags are
/// judged against.
type Pool = (Vec<Spec>, Vec<(String, usize)>);

fn any_pool() -> impl Strategy<Value = Pool> {
    (prop::collection::vec(any_spec(), 0..12), any_run_sizes())
}

/// A pool, plus a shuffled permutation of its own positions -- the second
/// order the same tasks could have arrived in.
#[allow(clippy::type_complexity)]
fn any_pool_in_two_orders() -> impl Strategy<Value = ((Vec<Spec>, Vec<(String, usize)>), Vec<usize>)>
{
    any_pool().prop_flat_map(|(specs, pairs)| {
        let order = Just((0..specs.len()).collect::<Vec<usize>>()).prop_shuffle();
        (Just((specs, pairs)), order)
    })
}

/// `PoolTask` is deliberately not `Clone` -- nothing in the product copies
/// one -- so the permutation property makes its own copy here rather than
/// widening the type's API for a test.
fn clone_task(task: &PoolTask) -> PoolTask {
    PoolTask {
        sequence: task.sequence,
        text: task.text.clone(),
        context_tag: task.context_tag.clone(),
        done: task.done,
    }
}

/// Distinct, ascending sequences by position: `group` orders on `sequence`,
/// and a tie would make "newest first" ambiguous rather than wrong, which
/// is not the property under test.
fn tasks(specs: &[Spec]) -> Vec<PoolTask> {
    specs
        .iter()
        .enumerate()
        .map(|(i, spec)| PoolTask {
            sequence: i as i64,
            text: format!("task {i}"),
            context_tag: spec.tag.map(str::to_string),
            done: spec.done,
        })
        .collect()
}

fn run_sizes(pairs: &[(String, usize)]) -> RunSizes {
    RunSizes::new(pairs.iter().cloned())
}

fn members(pairs: &[(String, usize)], tag: &str) -> usize {
    pairs
        .iter()
        .filter(|(t, _)| t == tag)
        .map(|(_, m)| *m)
        .next_back()
        .unwrap_or(0)
}

/// Every sequence `group` handed back, trips and loose ends together.
fn returned(groups: &PoolGroups) -> Vec<i64> {
    let mut ids: Vec<i64> = groups
        .trips
        .iter()
        .flat_map(|trip| trip.visible.iter().chain(trip.hidden.iter()))
        .map(|item| item.id)
        .chain(groups.loose.iter().map(|task| task.id))
        .collect();
    ids.sort_unstable();
    ids
}

/// One panel as `shape` compares it: its tag, its visible items and its
/// hidden ones, all in placement order.
type Panel<'a> = (&'a str, Vec<i64>, Vec<i64>);

/// The whole result as an ordered value: every panel's tag with its
/// visible and hidden items *in the order they were placed*, then the loose
/// ends in theirs. `returned` deliberately sorts, which makes it blind to
/// ordering -- the property about arrival order needs the opposite.
fn shape(groups: &PoolGroups) -> (Vec<Panel<'_>>, Vec<i64>) {
    (
        groups.trips.iter().map(panel).collect(),
        groups.loose.iter().map(|task| task.id).collect(),
    )
}

fn panel(trip: &Trip) -> Panel<'_> {
    (trip.tag.as_str(), ids(&trip.visible), ids(&trip.hidden))
}

fn ids(items: &[PoolItem]) -> Vec<i64> {
    items.iter().map(|item| item.id).collect()
}

/// Which sequences `group` is contracted to return: everything, less the
/// done items that have no panel to hold them -- a done task at an untagged
/// or below-threshold-and-un-persisted tag (`pool.rs`'s `bucket_one` and
/// `classify_group`).
fn expected(specs: &[Spec], pairs: &[(String, usize)]) -> Vec<i64> {
    let mut kept: Vec<i64> = specs
        .iter()
        .enumerate()
        .filter(|(_, spec)| survives(spec, specs, pairs))
        .map(|(i, _)| i as i64)
        .collect();
    kept.sort_unstable();
    kept
}

/// An open task always comes back. A done one comes back only inside a
/// panel: `bucket_one` drops a done untagged task, and `classify_group`
/// drops a done one whose tag did not earn a panel.
fn survives(spec: &Spec, specs: &[Spec], pairs: &[(String, usize)]) -> bool {
    !spec.done || spec.tag.is_some_and(|tag| is_trip(specs, pairs, tag))
}

/// The rule under test, restated: a bucket earns a panel on either count.
fn is_trip(specs: &[Spec], pairs: &[(String, usize)], tag: &str) -> bool {
    let waiting = specs.iter().filter(|s| s.tag == Some(tag)).count();
    waiting >= TRIP_THRESHOLD || members(pairs, tag) >= TRIP_THRESHOLD
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, ..ProptestConfig::default() })]

    /// Conservation. Every task comes back exactly once, or is dropped for
    /// the one stated reason -- never duplicated into two places, never
    /// invented, never quietly swallowed.
    #[test]
    #[ignore]
    fn every_task_is_returned_exactly_once_or_dropped_for_the_stated_reason(
        (specs, pairs) in any_pool()
    ) {
        let groups = pool::group(tasks(&specs), &run_sizes(&pairs));

        prop_assert_eq!(returned(&groups), expected(&specs, &pairs));
    }

    /// Input order is not a contract the caller can break. `group` sorts
    /// for itself; two orderings of one pool must be indistinguishable in
    /// the result, down to item order inside every panel.
    #[test]
    #[ignore]
    fn grouping_is_indifferent_to_the_order_tasks_arrive_in(
        ((specs, pairs), order) in any_pool_in_two_orders()
    ) {
        // Each task keeps its own sequence and moves position, so the
        // second run is the same pool arriving differently ordered rather
        // than a different pool.
        let all = tasks(&specs);
        let mut reordered_input = tasks(&specs);
        for (slot, from) in order.iter().enumerate() {
            reordered_input[slot] = PoolTask { ..clone_task(&all[*from]) };
        }

        let forward = pool::group(all, &run_sizes(&pairs));
        let reordered = pool::group(reordered_input, &run_sizes(&pairs));

        prop_assert_eq!(shape(&forward), shape(&reordered));
    }

    /// A trip's own shape. The panel splits its bucket at
    /// `VISIBLE_TRIP_ITEMS` and reports the halves consistently -- `count`
    /// is the whole bucket, `more` is exactly what is hidden, and the two
    /// lists never share an item.
    #[test]
    #[ignore]
    fn a_trips_counts_agree_with_the_items_it_carries((specs, pairs) in any_pool()) {
        let groups = pool::group(tasks(&specs), &run_sizes(&pairs));

        for trip in &groups.trips {
            prop_assert_eq!(trip.count, trip.visible.len() + trip.hidden.len());
            prop_assert_eq!(trip.more, trip.hidden.len());
            prop_assert!(trip.done_count <= trip.count);
            prop_assert!(!trip.visible.is_empty());
            prop_assert!(trip.hidden.is_empty() || trip.visible.len() == 3);

            let ids: Vec<i64> = trip.visible.iter().chain(&trip.hidden).map(|i| i.id).collect();
            let mut sorted = ids.clone();
            sorted.sort_unstable_by(|a, b| b.cmp(a));
            prop_assert_eq!(&ids, &sorted, "a trip's items must be newest first");
        }
    }

    /// Ranking is derived, not stored (`T-trips-are-derived-not-ranked`):
    /// bigger panels first, ties alphabetical, and loose ends newest first.
    #[test]
    #[ignore]
    fn trips_rank_by_size_then_tag_and_loose_ends_stay_newest_first(
        (specs, pairs) in any_pool()
    ) {
        let groups = pool::group(tasks(&specs), &run_sizes(&pairs));

        let ranked: Vec<(usize, &str)> = groups
            .trips
            .iter()
            .map(|trip| (trip.count, trip.tag.as_str()))
            .collect();
        for pair in ranked.windows(2) {
            let ((left_count, left_tag), (right_count, right_tag)) = (pair[0], pair[1]);
            prop_assert!(
                left_count > right_count || (left_count == right_count && left_tag < right_tag),
                "{:?} is not ranked by size then tag", ranked
            );
        }

        let loose: Vec<i64> = groups.loose.iter().map(|task| task.id).collect();
        let mut sorted = loose.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        prop_assert_eq!(loose, sorted);
    }

    /// The trip rule itself, over both counts at once: a tag is a panel
    /// exactly when its bucket or its run reaches the threshold, and every
    /// other tagged task is a loose end still wearing its tag.
    #[test]
    #[ignore]
    fn a_tag_forms_a_panel_exactly_when_either_count_reaches_the_threshold(
        (specs, pairs) in any_pool()
    ) {
        let groups = pool::group(tasks(&specs), &run_sizes(&pairs));
        let panels: Vec<&str> = groups.trips.iter().map(|trip| trip.tag.as_str()).collect();

        for tag in ["@a", "@b", "@c"] {
            let present = specs.iter().any(|spec| spec.tag == Some(tag));
            prop_assert_eq!(
                panels.contains(&tag),
                present && is_trip(&specs, &pairs, tag),
                "{} panelled wrongly among {:?}", tag, panels
            );
        }

        for task in &groups.loose {
            if let Some(tag) = task.context_tag.as_deref() {
                prop_assert!(!is_trip(&specs, &pairs, tag), "{} is loose and a trip", tag);
            }
        }
    }

    /// Persistence only ever adds panels. Raising a tag's run size can turn
    /// a loose end into a trip; it can never take a panel away -- the two
    /// halves of `is_trip` are an `or`, and this is what that means for the
    /// screen (#129).
    #[test]
    #[ignore]
    fn a_larger_run_never_costs_a_tag_its_panel((specs, pairs) in any_pool()) {
        let smaller = pool::group(tasks(&specs), &run_sizes(&pairs));
        let raised: Vec<(String, usize)> = pairs
            .iter()
            .map(|(tag, members)| (tag.clone(), members + 1))
            .collect();
        let larger = pool::group(tasks(&specs), &run_sizes(&raised));

        for trip in &smaller.trips {
            prop_assert!(
                larger.trips.iter().any(|t| t.tag == trip.tag),
                "{} lost its panel to a larger run", trip.tag
            );
        }
    }
}
