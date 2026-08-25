//! Grouping pool work by where it can be done (#92,
//! `D-context-tags-are-the-taxonomy`'s first consumer).
//!
//! **A context tag becomes a trip only once three things are waiting
//! there** (`TRIP_THRESHOLD`) — fewer, and those items are strays that
//! happen to share a place rather than a reason to leave the house, so they
//! fall to loose ends, still showing their tag. An untagged task always
//! falls to loose ends; there is no threshold for having nothing to group
//! by.
//!
//! **Case identity is not this module's job.** By the time a task reaches
//! here its `context_tag` has already been canonicalized to one spelling by
//! `trellis_server::capture::resolve_tag` — the write path, not the read
//! path, is where `@HomeDepot` and `@homedepot` became the same stored
//! string. Grouping by plain equality is correct *because* that already
//! happened; a second case fold here would be a second place the rule could
//! drift from the first.
//!
//! **No priority.** `pri`-based ordering is a design the canvas draws and
//! this slice deliberately does not build (`pool-screen-nothing-reorders-05`):
//! trips are ranked by how many things they clear, derived from the data and
//! never maintained, and items within a group are newest first — what the
//! canvas's own tiebreak (`b.seq - a.seq`) degrades to once priority is
//! removed from it.

/// One pool task as this module needs it: enough to group, order and count
/// it, nothing about how it got here.
pub struct PoolTask {
    /// Newest-first ordering key. Not necessarily the task's own database
    /// id in every possible caller, but always "larger is newer" — the
    /// adapter's job is to hand these in already meaning that.
    pub sequence: i64,
    pub text: String,
    pub context_tag: Option<String>,
    /// Struck-through and still displayed (#122,
    /// `D-a-trip-survives-being-worked`) — never a task that has been
    /// cleared, which the adapter's own query excludes before this module
    /// ever sees it. A *loose* end that is done is filtered out entirely
    /// here rather than carried as `done: true`: it has no panel to hold it
    /// in place, so it leaves the screen at once, exactly as it did before
    /// this slice (`trip-progress-loose-ends-unchanged-08`).
    pub done: bool,
    /// How many pool tasks have belonged to this tag's current *run* --
    /// everything created since the run last emptied out entirely, cleared
    /// or not (#129, `D-a-trip-survives-being-tidied`). Equal to the
    /// concurrently-waiting count (`bucket_by_tag`'s own bucket size) until
    /// a clear removes something mid-run without ending it; larger than it
    /// from that point on, since the removed member still belonged to the
    /// run. The adapter's job, derived from `tasks.id` order and
    /// `cleared_at` batches -- this module only compares it to
    /// [`TRIP_THRESHOLD`], the same way it already compares the
    /// concurrently-waiting count.
    pub run_member_count: usize,
}

/// One item's own identity and text, wherever a caller needs to act on it
/// rather than just read it (#97: marking a specific one done).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolItem {
    pub id: i64,
    pub text: String,
    pub done: bool,
}

/// A context tag with enough tasks waiting to be worth a special trip.
pub struct Trip {
    pub tag: String,
    /// Everything waiting under this tag, not just what is shown —
    /// `pool-screen-truncation-06`'s "the count reads five throughout".
    /// Struck items count here too (#122): this is what stands between a
    /// panel of one open item reading "1 of 3 done" (right) and one reading
    /// "3 things" or "1 thing" (the half-pass trap #103 named, wearing a
    /// new label).
    pub count: usize,
    /// How many of `count` are struck through. Zero exactly when nothing in
    /// this trip has been marked done yet.
    pub done_count: usize,
    /// The newest [`VISIBLE_TRIP_ITEMS`], in newest-first order.
    pub visible: Vec<PoolItem>,
    /// Everything beyond `visible`, still newest-first — what a "show more"
    /// control reveals. Empty exactly when [`Trip::more`] is zero; carried
    /// separately from `visible` rather than making the adapter reconstruct
    /// it, since a native `<details>` disclosure needs the actual items to
    /// put inside it, not just a count of them.
    pub hidden: Vec<PoolItem>,
    /// How many more exist beyond `visible` — zero when nothing is hidden.
    pub more: usize,
}

/// A pool task with nowhere worth a special trip: either untagged, or
/// tagged with something too few other tasks share.
pub struct LooseTask {
    pub id: i64,
    pub text: String,
    pub context_tag: Option<String>,
}

pub struct PoolGroups {
    /// Ranked by how many things they clear, ties broken alphabetically by
    /// tag (`pool-screen-trip-order-02`) — derived from the data, and never
    /// a stored order a caller could disagree with.
    pub trips: Vec<Trip>,
    /// Untagged tasks and every task from a below-threshold tag, all
    /// together, newest first.
    pub loose: Vec<LooseTask>,
}

/// A context tag needs at least this many waiting tasks to become a trip
/// (`pool-screen-trips-and-loose-01`). The canvas's own default
/// (`tripThreshold || 3`), confirmed by the owner rather than the brief's
/// contradicting demo.
pub const TRIP_THRESHOLD: usize = 3;

/// A trip shows its newest items up to this many before truncating
/// (`pool-screen-truncation-06`).
const VISIBLE_TRIP_ITEMS: usize = 3;

/// Groups `tasks` into trips and loose ends. `tasks` may arrive in any
/// order; this function establishes newest-first itself rather than trusting
/// the caller to have sorted already, since a caller that forgot would fail
/// silently rather than loudly (both orders type-check).
///
/// Whether a bucket earns its trip panel is [`classify_group`]'s call, not
/// this function's -- FORMATION (#122) and PERSISTENCE (#129) are two rules,
/// not one, and [`is_trip`]'s own doc has both.
pub fn group(mut tasks: Vec<PoolTask>) -> PoolGroups {
    tasks.sort_by_key(|t| std::cmp::Reverse(t.sequence));
    let (groups, mut loose) = bucket_by_tag(tasks);

    let mut trips = Vec::new();
    for (tag, list) in groups {
        classify_group(tag, list, &mut trips, &mut loose);
    }

    loose.sort_by_key(|t| std::cmp::Reverse(t.sequence));
    let loose = loose.into_iter().map(LooseTask::from).collect();

    PoolGroups { trips, loose }
}

/// Routes one tag's bucket to `trips` once it earns a panel ([`is_trip`]),
/// or spreads its still-open tasks into `loose` otherwise -- a
/// below-threshold, un-persisted group's own struck items are dropped here
/// rather than shown as loose ends: a loose end has no panel to hold a
/// completed item in place, so it leaves at once, exactly as it did before
/// #122 (`trip-progress-loose-ends-unchanged-08`).
fn classify_group(
    tag: String,
    list: Vec<PoolTask>,
    trips: &mut Vec<Trip>,
    loose: &mut Vec<PoolTask>,
) {
    if is_trip(&list) {
        trips.push(trip_from_group(tag, list));
    } else {
        loose.extend(list.into_iter().filter(|t| !t.done));
    }
}

/// Whether one tag's bucket has earned a trip panel: FORMATION (#122) needs
/// [`TRIP_THRESHOLD`] tasks waiting there now; PERSISTENCE (#129) needs the
/// same threshold met by the tag's *run* instead, which can hold after a
/// clear has dropped the concurrently-waiting count below it. Every task in
/// one bucket carries the same tag-level `run_member_count` -- the adapter's
/// fact about the tag, not the task -- so the first one speaks for the whole
/// bucket.
fn is_trip(list: &[PoolTask]) -> bool {
    let run_member_count = list.first().map_or(0, |t| t.run_member_count);
    list.len() >= TRIP_THRESHOLD || run_member_count >= TRIP_THRESHOLD
}

/// Splits `tasks` (already newest-first) into tag buckets, ranked by size
/// then alphabetically (`pool-screen-trip-order-02`), and the untagged
/// remainder. Every bucket's own relative order is preserved from `tasks`,
/// so it stays newest-first without a second sort.
fn bucket_by_tag(tasks: Vec<PoolTask>) -> (Vec<(String, Vec<PoolTask>)>, Vec<PoolTask>) {
    let mut buckets: std::collections::HashMap<String, Vec<PoolTask>> =
        std::collections::HashMap::new();
    let mut loose = Vec::new();
    for task in tasks {
        bucket_one(task, &mut buckets, &mut loose);
    }

    let mut groups: Vec<(String, Vec<PoolTask>)> = buckets.into_iter().collect();
    groups.sort_by(by_size_then_tag);
    (groups, loose)
}

/// Routes a single task to its tag's bucket, or to `loose` when it is
/// untagged -- a done, untagged task is dropped here rather than carried
/// into `loose`: it has no tag to ever reach [`TRIP_THRESHOLD`] with, so it
/// is unconditionally a loose end, and a done loose end leaves the screen at
/// once (#122).
fn bucket_one(
    task: PoolTask,
    buckets: &mut std::collections::HashMap<String, Vec<PoolTask>>,
    loose: &mut Vec<PoolTask>,
) {
    match &task.context_tag {
        Some(tag) => buckets.entry(tag.clone()).or_default().push(task),
        None if !task.done => loose.push(task),
        None => {}
    }
}

/// Ranks buckets by size then alphabetically (`pool-screen-trip-order-02`).
fn by_size_then_tag(
    (a_tag, a_list): &(String, Vec<PoolTask>),
    (b_tag, b_list): &(String, Vec<PoolTask>),
) -> std::cmp::Ordering {
    b_list
        .len()
        .cmp(&a_list.len())
        .then_with(|| a_tag.cmp(b_tag))
}

/// A group at or over [`TRIP_THRESHOLD`], split into what a trip panel
/// shows and what it hides behind a "show more" control.
fn trip_from_group(tag: String, list: Vec<PoolTask>) -> Trip {
    let count = list.len();
    let done_count = list.iter().filter(|t| t.done).count();
    let visible: Vec<PoolItem> = list
        .iter()
        .take(VISIBLE_TRIP_ITEMS)
        .map(PoolItem::from_task)
        .collect();
    let hidden: Vec<PoolItem> = list
        .iter()
        .skip(VISIBLE_TRIP_ITEMS)
        .map(PoolItem::from_task)
        .collect();
    let more = hidden.len();
    Trip {
        tag,
        count,
        done_count,
        visible,
        hidden,
        more,
    }
}

impl PoolItem {
    fn from_task(task: &PoolTask) -> Self {
        PoolItem {
            id: task.sequence,
            text: task.text.clone(),
            done: task.done,
        }
    }
}

impl From<PoolTask> for LooseTask {
    fn from(task: PoolTask) -> Self {
        LooseTask {
            id: task.sequence,
            text: task.text,
            context_tag: task.context_tag,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(sequence: i64, text: &str, tag: Option<&str>) -> PoolTask {
        PoolTask {
            sequence,
            text: text.to_string(),
            context_tag: tag.map(str::to_string),
            done: false,
            run_member_count: 0,
        }
    }

    fn done_task(sequence: i64, text: &str, tag: Option<&str>) -> PoolTask {
        PoolTask {
            sequence,
            text: text.to_string(),
            context_tag: tag.map(str::to_string),
            done: true,
            run_member_count: 0,
        }
    }

    /// A below-threshold task whose tag's *run* has already reached
    /// [`TRIP_THRESHOLD`] -- the store's own answer to "is this tag still
    /// inside a run that once formed a trip" (#129).
    fn run_task(sequence: i64, text: &str, tag: &str, run_member_count: usize) -> PoolTask {
        PoolTask {
            sequence,
            text: text.to_string(),
            context_tag: Some(tag.to_string()),
            done: false,
            run_member_count,
        }
    }

    #[test]
    fn three_tasks_at_one_tag_become_a_trip() {
        let groups = group(vec![
            task(1, "buy screws", Some("@homedepot")),
            task(2, "return the drill", Some("@homedepot")),
            task(3, "pick up trim", Some("@homedepot")),
        ]);

        assert_eq!(groups.trips.len(), 1);
        assert_eq!(groups.trips[0].tag, "@homedepot");
        assert_eq!(groups.trips[0].count, 3);
        assert!(groups.loose.is_empty());
    }

    #[test]
    fn two_tasks_at_one_tag_stay_loose_and_keep_their_tag() {
        let groups = group(vec![
            task(1, "milk", Some("@supermarket")),
            task(2, "coffee", Some("@supermarket")),
        ]);

        assert!(groups.trips.is_empty());
        assert_eq!(groups.loose.len(), 2);
        assert!(groups
            .loose
            .iter()
            .all(|t| t.context_tag.as_deref() == Some("@supermarket")));
    }

    #[test]
    fn an_untagged_task_is_always_loose_regardless_of_threshold() {
        let groups = group(vec![task(1, "fix the door latch", None)]);

        assert!(groups.trips.is_empty());
        assert_eq!(groups.loose.len(), 1);
        assert_eq!(groups.loose[0].context_tag, None);
    }

    #[test]
    fn a_third_task_at_a_tag_promotes_the_whole_group_to_a_trip() {
        let two = group(vec![
            task(1, "milk", Some("@supermarket")),
            task(2, "coffee", Some("@supermarket")),
        ]);
        assert!(two.trips.is_empty());

        let three = group(vec![
            task(1, "milk", Some("@supermarket")),
            task(2, "coffee", Some("@supermarket")),
            task(3, "bread", Some("@supermarket")),
        ]);
        assert_eq!(three.trips.len(), 1);
        assert!(three.loose.is_empty());
    }

    #[test]
    fn trips_are_ranked_by_size_then_alphabetically() {
        let groups = group(vec![
            task(1, "a1", Some("@bakery")),
            task(2, "a2", Some("@bakery")),
            task(3, "a3", Some("@bakery")),
            task(4, "a4", Some("@bakery")),
            task(5, "b1", Some("@attic")),
            task(6, "b2", Some("@attic")),
            task(7, "b3", Some("@attic")),
            task(8, "c1", Some("@cellar")),
            task(9, "c2", Some("@cellar")),
            task(10, "c3", Some("@cellar")),
        ]);

        let order: Vec<&str> = groups.trips.iter().map(|t| t.tag.as_str()).collect();
        assert_eq!(order, vec!["@bakery", "@attic", "@cellar"]);
    }

    #[test]
    fn trip_items_are_newest_first() {
        let groups = group(vec![
            task(1, "first", Some("@homedepot")),
            task(2, "second", Some("@homedepot")),
            task(3, "third", Some("@homedepot")),
        ]);

        let texts: Vec<&str> = groups.trips[0]
            .visible
            .iter()
            .map(|item| item.text.as_str())
            .collect();
        assert_eq!(texts, vec!["third", "second", "first"]);
    }

    #[test]
    fn loose_ends_are_newest_first_across_untagged_and_below_threshold_tags() {
        let groups = group(vec![
            task(1, "milk", Some("@supermarket")),
            task(2, "fix the door latch", None),
            task(3, "coffee", Some("@supermarket")),
        ]);

        let order: Vec<&str> = groups.loose.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(order, vec!["coffee", "fix the door latch", "milk"]);
    }

    #[test]
    fn a_trip_shows_at_most_three_and_reports_the_rest_as_more() {
        let groups = group(vec![
            task(1, "a", Some("@homedepot")),
            task(2, "b", Some("@homedepot")),
            task(3, "c", Some("@homedepot")),
            task(4, "d", Some("@homedepot")),
            task(5, "e", Some("@homedepot")),
        ]);

        assert_eq!(groups.trips[0].visible.len(), 3);
        assert_eq!(groups.trips[0].more, 2);
        assert_eq!(groups.trips[0].count, 5);
        let hidden: Vec<&str> = groups.trips[0]
            .hidden
            .iter()
            .map(|item| item.text.as_str())
            .collect();
        assert_eq!(hidden, vec!["b", "a"]);
    }

    #[test]
    fn a_trip_of_exactly_three_offers_nothing_more() {
        let groups = group(vec![
            task(1, "a", Some("@homedepot")),
            task(2, "b", Some("@homedepot")),
            task(3, "c", Some("@homedepot")),
        ]);

        assert_eq!(groups.trips[0].more, 0);
        assert!(groups.trips[0].hidden.is_empty());
    }

    #[test]
    fn group_accepts_tasks_in_any_input_order() {
        let forward = group(vec![
            task(1, "first", Some("@homedepot")),
            task(2, "second", Some("@homedepot")),
            task(3, "third", Some("@homedepot")),
        ]);
        let shuffled = group(vec![
            task(2, "second", Some("@homedepot")),
            task(3, "third", Some("@homedepot")),
            task(1, "first", Some("@homedepot")),
        ]);

        assert_eq!(forward.trips[0].visible, shuffled.trips[0].visible);
    }

    #[test]
    fn an_empty_pool_groups_to_nothing() {
        let groups = group(vec![]);
        assert!(groups.trips.is_empty());
        assert!(groups.loose.is_empty());
    }

    #[test]
    fn a_trip_items_id_is_its_tasks_sequence() {
        let groups = group(vec![
            task(1, "buy screws", Some("@homedepot")),
            task(2, "return the drill", Some("@homedepot")),
            task(3, "pick up trim", Some("@homedepot")),
        ]);

        assert_eq!(groups.trips[0].visible[0].id, 3);
        assert_eq!(groups.trips[0].visible[2].id, 1);
    }

    #[test]
    fn a_loose_tasks_id_is_its_sequence() {
        let groups = group(vec![task(42, "fix the door latch", None)]);

        assert_eq!(groups.loose[0].id, 42);
    }

    // --- #122: a trip survives being worked ---------------------------

    #[test]
    fn a_trip_holds_while_some_of_it_is_done() {
        let groups = group(vec![
            done_task(1, "buy screws", Some("@homedepot")),
            done_task(2, "return the drill", Some("@homedepot")),
            task(3, "pick up trim", Some("@homedepot")),
            task(4, "grab a tarp", Some("@homedepot")),
            task(5, "buy screws again", Some("@homedepot")),
        ]);

        assert_eq!(groups.trips.len(), 1);
        assert_eq!(groups.trips[0].count, 5);
        assert_eq!(groups.trips[0].done_count, 2);
        assert!(groups.loose.is_empty());
    }

    #[test]
    fn a_trip_holds_when_every_item_in_it_is_done() {
        let groups = group(vec![
            done_task(1, "buy screws", Some("@homedepot")),
            done_task(2, "return the drill", Some("@homedepot")),
            done_task(3, "pick up trim", Some("@homedepot")),
        ]);

        assert_eq!(groups.trips.len(), 1);
        assert_eq!(groups.trips[0].count, 3);
        assert_eq!(groups.trips[0].done_count, 3);
    }

    #[test]
    fn done_items_are_still_visible_and_carry_the_done_flag() {
        let groups = group(vec![
            done_task(1, "buy screws", Some("@homedepot")),
            task(2, "return the drill", Some("@homedepot")),
            task(3, "pick up trim", Some("@homedepot")),
        ]);

        let done_flags: Vec<bool> = groups.trips[0]
            .visible
            .iter()
            .map(|item| item.done)
            .collect();
        // newest first: sequence 3, 2, 1 -- only the last (sequence 1) is done
        assert_eq!(done_flags, vec![false, false, true]);
    }

    #[test]
    fn a_below_threshold_groups_open_items_stay_loose_and_its_done_items_vanish() {
        let groups = group(vec![
            done_task(1, "buy screws", Some("@homedepot")),
            task(2, "return the drill", Some("@homedepot")),
        ]);

        assert!(groups.trips.is_empty());
        assert_eq!(groups.loose.len(), 1);
        assert_eq!(groups.loose[0].text, "return the drill");
    }

    #[test]
    fn a_done_untagged_task_vanishes_rather_than_appearing_loose() {
        let groups = group(vec![done_task(1, "fix the door latch", None)]);

        assert!(groups.loose.is_empty());
    }

    #[test]
    fn an_open_untagged_task_still_appears_loose() {
        let groups = group(vec![task(1, "fix the door latch", None)]);

        assert_eq!(groups.loose.len(), 1);
    }

    // --- #129: a trip survives being tidied -----------------------------

    #[test]
    fn a_below_threshold_group_persists_as_a_trip_when_its_run_has_reached_the_threshold() {
        let groups = group(vec![
            run_task(4, "grab a tarp", "@homedepot", 5),
            run_task(5, "buy screws again", "@homedepot", 5),
        ]);

        assert_eq!(groups.trips.len(), 1);
        assert_eq!(groups.trips[0].tag, "@homedepot");
        assert_eq!(groups.trips[0].count, 2);
        assert!(groups.loose.is_empty());
    }

    #[test]
    fn a_below_threshold_group_stays_loose_when_its_run_has_never_reached_the_threshold() {
        let groups = group(vec![
            run_task(1, "milk", "@supermarket", 2),
            run_task(2, "coffee", "@supermarket", 2),
        ]);

        assert!(groups.trips.is_empty());
        assert_eq!(groups.loose.len(), 2);
    }

    #[test]
    fn a_persisted_trips_done_items_stay_visible_and_struck() {
        let mut done = run_task(1, "buy screws", "@homedepot", 5);
        done.done = true;
        let groups = group(vec![done, run_task(2, "return the drill", "@homedepot", 5)]);

        assert_eq!(groups.trips.len(), 1);
        assert_eq!(groups.trips[0].count, 2);
        assert_eq!(groups.trips[0].done_count, 1);
    }
}
