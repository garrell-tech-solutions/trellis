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
}

/// A context tag with enough tasks waiting to be worth a special trip.
pub struct Trip {
    pub tag: String,
    /// Everything waiting under this tag, not just what is shown —
    /// `pool-screen-truncation-06`'s "the count reads five throughout".
    pub count: usize,
    /// The newest [`VISIBLE_TRIP_ITEMS`], in newest-first order.
    pub visible: Vec<String>,
    /// Everything beyond `visible`, still newest-first — what a "show more"
    /// control reveals. Empty exactly when [`Trip::more`] is zero; carried
    /// separately from `visible` rather than making the adapter reconstruct
    /// it, since a native `<details>` disclosure needs the actual items to
    /// put inside it, not just a count of them.
    pub hidden: Vec<String>,
    /// How many more exist beyond `visible` — zero when nothing is hidden.
    pub more: usize,
}

/// A pool task with nowhere worth a special trip: either untagged, or
/// tagged with something too few other tasks share.
pub struct LooseTask {
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
pub fn group(mut tasks: Vec<PoolTask>) -> PoolGroups {
    tasks.sort_by_key(|t| std::cmp::Reverse(t.sequence));
    let (groups, mut loose) = bucket_by_tag(tasks);

    let mut trips = Vec::new();
    for (tag, list) in groups {
        if list.len() >= TRIP_THRESHOLD {
            trips.push(trip_from_group(tag, list));
        } else {
            loose.extend(list);
        }
    }

    loose.sort_by_key(|t| std::cmp::Reverse(t.sequence));
    let loose = loose.into_iter().map(LooseTask::from).collect();

    PoolGroups { trips, loose }
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
        match &task.context_tag {
            Some(tag) => buckets.entry(tag.clone()).or_default().push(task),
            None => loose.push(task),
        }
    }

    let mut groups: Vec<(String, Vec<PoolTask>)> = buckets.into_iter().collect();
    groups.sort_by(|(a_tag, a_list), (b_tag, b_list)| {
        b_list
            .len()
            .cmp(&a_list.len())
            .then_with(|| a_tag.cmp(b_tag))
    });
    (groups, loose)
}

/// A group at or over [`TRIP_THRESHOLD`], split into what a trip panel
/// shows and what it hides behind a "show more" control.
fn trip_from_group(tag: String, list: Vec<PoolTask>) -> Trip {
    let count = list.len();
    let visible: Vec<String> = list
        .iter()
        .take(VISIBLE_TRIP_ITEMS)
        .map(|t| t.text.clone())
        .collect();
    let hidden: Vec<String> = list
        .iter()
        .skip(VISIBLE_TRIP_ITEMS)
        .map(|t| t.text.clone())
        .collect();
    let more = hidden.len();
    Trip {
        tag,
        count,
        visible,
        hidden,
        more,
    }
}

impl From<PoolTask> for LooseTask {
    fn from(task: PoolTask) -> Self {
        LooseTask {
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

        assert_eq!(groups.trips[0].visible, vec!["third", "second", "first"]);
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
        assert_eq!(
            groups.trips[0].hidden,
            vec!["b".to_string(), "a".to_string()]
        );
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
}
