//! What the pool template renders — text and structure, never a raw
//! `scheduler_core::pool` type or a store row directly
//! (`T-templates-take-view-models`).

use crate::pool::store::PoolTaskRow;
use scheduler_core::pool::{self, PoolTask};
use std::collections::HashSet;

/// One item's own id and text, wherever the template needs to act on a
/// specific one rather than just list it (#97: marking it done).
pub struct PoolItemView {
    pub id: i64,
    pub text: String,
    /// Struck through, and its checkbox posts to the "undone" route rather
    /// than "done" (#122: unchecking a struck item is the direct inverse
    /// of the tap that struck it).
    pub done: bool,
}

pub struct TripView {
    pub tag: String,
    /// `"3 things"` when nothing in the trip is done yet
    /// (`pool-screen-trips-and-loose-01`'s own wording); `"3 of 5 done"`
    /// once something is (#122, `D-a-trip-survives-being-worked`'s
    /// "the group's label reports progress").
    pub count_label: String,
    /// Whether the "Clear done" control appears at all -- only once
    /// something in the trip is struck (`trip-progress-clear-control-
    /// appears-with-work-07`).
    pub offers_clear: bool,
    /// Newest-first, everything in the trip -- one list, not a visible
    /// slice plus a second hidden one (#120: the old `<details>` nested the
    /// revealed items in a second list with its own spacing, and nothing
    /// reconciled the two; the template now renders these once and lets
    /// CSS decide, under `.trip:not(.expanded)`, which ones paint).
    pub items: Vec<PoolItemView>,
    /// Whether this trip renders already expanded -- client state ridden
    /// along on the request that produced this render (#120: `expanded` is
    /// never stored, only echoed back for the one response it arrived
    /// with), never derived from anything in the database.
    pub expanded: bool,
    /// The show-more/fewer control's current text, `None` exactly when it
    /// should not render at all -- the canvas's own `hasMore: more > 0 ||
    /// open` (`Trellis.dc.html:850`): a trip already expanded keeps its
    /// control even on the render where nothing turns out to be hidden, so
    /// there is always something to tap back to collapsed.
    pub more_label: Option<String>,
    /// What the control reads once collapsed again, baked in regardless of
    /// `expanded` so the client-side toggle can restore it without a
    /// request. `None` exactly when nothing is ever hidden.
    pub collapsed_label: Option<String>,
    /// `"Complete all 8"` -- the open count, not the group's size, so the
    /// label alone tells you what a tap does before you make it (#125,
    /// `D-bulk-completion-is-explicit`).
    pub complete_label: String,
    /// Whether the complete-group control appears at all -- never once
    /// nothing is left open (`trip-controls-nothing-left-to-complete-03`):
    /// a panel offering to complete a group with nothing open in it is
    /// #103's half-pass trap wearing this slice's label.
    pub offers_complete: bool,
}

pub struct LooseItemView {
    pub id: i64,
    pub text: String,
    pub context_tag: Option<String>,
}

pub struct PoolView {
    /// `"6 waiting"`, or `"empty"` when nothing is pooled — the canvas
    /// distinguishes an empty pool from a pool of zero rather than reading
    /// `"0 waiting"` (`pool-screen-empty-07`).
    pub meta: String,
    pub empty: bool,
    pub trips: Vec<TripView>,
    pub loose: Vec<LooseItemView>,
}

/// `"3 things"` before anything is done, `"3 of 5 done"` once something is
/// (#122) -- `done_count` alone decides the format, since `0 of N done`
/// would read as the same half-pass trap `pool-screen-trips-and-loose-01`
/// already refused once, wearing a new label.
fn count_label(count: usize, done_count: usize) -> String {
    if done_count == 0 {
        format!("{count} things")
    } else {
        format!("{done_count} of {count} done")
    }
}

/// `rows` need not arrive in any particular order —
/// `scheduler_core::pool::group` establishes newest-first itself.
///
/// `meta`'s count is `waiting`, not `rows.len()` (#122,
/// `mark-done-counts-exclude-04`, unchanged by this slice): a struck item
/// stays on screen but is not something still to do, so it must not count
/// toward "N waiting" even though it now counts toward a trip's own
/// threshold. `empty` stays keyed to whether any row exists at all, not to
/// `waiting`, so a trip that is entirely done does not misreport the whole
/// screen as empty out from under the panel still showing it.
///
/// `expanded_tags` names which trips render already expanded (#120) --
/// client state the caller read off *this* request and nothing this
/// function stores. `GET /pool` always passes an empty set: expand state is
/// a thing you did with your thumb during this page's own lifetime, not
/// something a fresh navigation remembers.
pub(super) fn build(rows: Vec<PoolTaskRow>, expanded_tags: &HashSet<String>) -> PoolView {
    let is_empty = rows.is_empty();
    let waiting = rows.iter().filter(|row| !row.done).count();
    let tasks = rows
        .into_iter()
        .map(|row| PoolTask {
            sequence: row.task_id,
            text: row.raw_text,
            context_tag: row.context_tag,
            done: row.done,
        })
        .collect();
    let groups = pool::group(tasks);

    let trips = groups
        .trips
        .into_iter()
        .map(|trip| {
            let expanded = expanded_tags.contains(&trip.tag);
            trip_view(trip, expanded)
        })
        .collect();
    let loose = groups
        .loose
        .into_iter()
        .map(|task| LooseItemView {
            id: task.id,
            text: task.text,
            context_tag: task.context_tag,
        })
        .collect();

    PoolView {
        meta: if is_empty {
            "empty".to_string()
        } else {
            format!("{waiting} waiting")
        },
        empty: is_empty,
        trips,
        loose,
    }
}

fn trip_view(trip: pool::Trip, expanded: bool) -> TripView {
    let more = trip.more;
    let more_label = if expanded {
        Some("Show fewer".to_string())
    } else if more > 0 {
        Some(format!("Show {more} more"))
    } else {
        None
    };
    let collapsed_label = (more > 0).then(|| format!("Show {more} more"));
    let open_count = trip.count - trip.done_count;
    TripView {
        tag: trip.tag,
        count_label: count_label(trip.count, trip.done_count),
        offers_clear: trip.done_count > 0,
        items: trip
            .visible
            .into_iter()
            .chain(trip.hidden)
            .map(pool_item_view)
            .collect(),
        expanded,
        more_label,
        collapsed_label,
        complete_label: format!("Complete all {open_count}"),
        offers_complete: open_count > 0,
    }
}

fn pool_item_view(item: pool::PoolItem) -> PoolItemView {
    PoolItemView {
        id: item.id,
        text: item.text,
        done: item.done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(task_id: i64, text: &str, tag: Option<&str>) -> PoolTaskRow {
        PoolTaskRow {
            task_id,
            raw_text: text.to_string(),
            context_tag: tag.map(str::to_string),
            done: false,
        }
    }

    fn done_row(task_id: i64, text: &str, tag: Option<&str>) -> PoolTaskRow {
        PoolTaskRow {
            task_id,
            raw_text: text.to_string(),
            context_tag: tag.map(str::to_string),
            done: true,
        }
    }

    /// Every test in this module but the #120 ones below is indifferent to
    /// expand state -- shadows the real `build` with the empty set baked
    /// in, so those tests do not have to carry a `&HashSet::new()` they do
    /// not care about.
    fn build(rows: Vec<PoolTaskRow>) -> PoolView {
        super::build(rows, &no_expanded())
    }

    fn build_expanded(rows: Vec<PoolTaskRow>, expanded: &HashSet<String>) -> PoolView {
        super::build(rows, expanded)
    }

    #[test]
    fn an_empty_pool_reports_empty_meta_and_the_empty_flag() {
        let view = build(vec![]);
        assert_eq!(view.meta, "empty");
        assert!(view.empty);
    }

    #[test]
    fn meta_counts_every_pooled_task_trips_and_loose_together() {
        let view = build(vec![
            row(1, "a", Some("@homedepot")),
            row(2, "b", Some("@homedepot")),
            row(3, "c", Some("@homedepot")),
            row(4, "d", Some("@supermarket")),
            row(5, "e", Some("@supermarket")),
            row(6, "f", None),
        ]);
        assert_eq!(view.meta, "6 waiting");
        assert!(!view.empty);
    }

    /// Exactly [`scheduler_core::pool::TRIP_THRESHOLD`] rows at one tag --
    /// the smallest input that forms a trip with nothing truncated.
    fn three_task_trip() -> Vec<PoolTaskRow> {
        vec![
            row(1, "a", Some("@homedepot")),
            row(2, "b", Some("@homedepot")),
            row(3, "c", Some("@homedepot")),
        ]
    }

    #[test]
    fn a_trips_count_label_reads_things() {
        let view = build(three_task_trip());
        assert_eq!(view.trips[0].count_label, "3 things");
    }

    #[test]
    fn a_trip_with_nothing_hidden_has_no_more_label() {
        let view = build(three_task_trip());
        assert_eq!(view.trips[0].more_label, None);
    }

    #[test]
    fn a_truncated_trip_reports_how_many_more_and_holds_all_of_them_newest_first() {
        let view = build(vec![
            row(1, "a", Some("@homedepot")),
            row(2, "b", Some("@homedepot")),
            row(3, "c", Some("@homedepot")),
            row(4, "d", Some("@homedepot")),
            row(5, "e", Some("@homedepot")),
        ]);
        assert_eq!(view.trips[0].more_label.as_deref(), Some("Show 2 more"));
        let texts: Vec<&str> = view.trips[0]
            .items
            .iter()
            .map(|item| item.text.as_str())
            .collect();
        assert_eq!(texts, vec!["e", "d", "c", "b", "a"]);
    }

    #[test]
    fn a_loose_item_keeps_its_tag() {
        let view = build(vec![row(1, "milk", Some("@supermarket"))]);
        assert_eq!(view.loose[0].context_tag.as_deref(), Some("@supermarket"));
    }

    #[test]
    fn a_loose_item_with_no_tag_carries_none() {
        let view = build(vec![row(1, "fix the door latch", None)]);
        assert_eq!(view.loose[0].context_tag, None);
    }

    #[test]
    fn a_loose_items_id_is_its_task_id() {
        let view = build(vec![row(42, "fix the door latch", None)]);
        assert_eq!(view.loose[0].id, 42);
    }

    #[test]
    fn a_trip_items_id_is_its_task_id() {
        let view = build(three_task_trip());
        assert_eq!(view.trips[0].items[0].id, 3);
    }

    // --- #122: a trip survives being worked ---------------------------

    #[test]
    fn a_trips_label_reports_progress_once_something_is_done() {
        let view = build(vec![
            done_row(1, "a", Some("@homedepot")),
            done_row(2, "b", Some("@homedepot")),
            row(3, "c", Some("@homedepot")),
            row(4, "d", Some("@homedepot")),
            row(5, "e", Some("@homedepot")),
        ]);
        assert_eq!(view.trips[0].count_label, "2 of 5 done");
    }

    #[test]
    fn a_trips_label_stays_things_when_nothing_in_it_is_done() {
        let view = build(three_task_trip());
        assert_eq!(view.trips[0].count_label, "3 things");
    }

    #[test]
    fn a_trip_offers_no_clear_control_until_something_is_done() {
        let view = build(three_task_trip());
        assert!(!view.trips[0].offers_clear);
    }

    #[test]
    fn a_trip_offers_a_clear_control_once_something_is_done() {
        let view = build(vec![
            done_row(1, "a", Some("@homedepot")),
            row(2, "b", Some("@homedepot")),
            row(3, "c", Some("@homedepot")),
        ]);
        assert!(view.trips[0].offers_clear);
    }

    #[test]
    fn a_trip_item_carries_whether_it_is_done() {
        let view = build(vec![
            done_row(1, "a", Some("@homedepot")),
            row(2, "b", Some("@homedepot")),
            row(3, "c", Some("@homedepot")),
        ]);
        let done_flags: Vec<bool> = view.trips[0].items.iter().map(|item| item.done).collect();
        // newest first: id 3, 2, 1 -- only id 1 is done
        assert_eq!(done_flags, vec![false, false, true]);
    }

    #[test]
    fn meta_excludes_done_tasks_from_the_waiting_count() {
        let view = build(vec![
            done_row(1, "a", Some("@homedepot")),
            row(2, "b", Some("@homedepot")),
            row(3, "c", Some("@homedepot")),
        ]);
        assert_eq!(view.meta, "2 waiting");
    }

    #[test]
    fn a_fully_done_trip_does_not_report_the_screen_as_empty() {
        let view = build(vec![
            done_row(1, "a", Some("@homedepot")),
            done_row(2, "b", Some("@homedepot")),
            done_row(3, "c", Some("@homedepot")),
        ]);
        assert!(!view.empty);
        assert_eq!(view.meta, "0 waiting");
        assert_eq!(view.trips[0].count_label, "3 of 3 done");
    }

    // --- #120: the expand control is client state, ridden along ---------

    fn no_expanded() -> std::collections::HashSet<String> {
        std::collections::HashSet::new()
    }

    fn expanded_at(tag: &str) -> std::collections::HashSet<String> {
        std::collections::HashSet::from([tag.to_string()])
    }

    #[test]
    fn a_trips_items_are_one_list_holding_everything() {
        let view = build_expanded(
            vec![
                row(1, "a", Some("@homedepot")),
                row(2, "b", Some("@homedepot")),
                row(3, "c", Some("@homedepot")),
                row(4, "d", Some("@homedepot")),
                row(5, "e", Some("@homedepot")),
            ],
            &no_expanded(),
        );
        assert_eq!(view.trips[0].items.len(), 5);
    }

    #[test]
    fn a_trip_of_exactly_three_offers_no_more_control() {
        let view = build_expanded(three_task_trip(), &no_expanded());
        assert_eq!(view.trips[0].more_label, None);
    }

    #[test]
    fn a_truncated_collapsed_trip_offers_a_more_control_naming_the_count() {
        let view = build_expanded(
            vec![
                row(1, "a", Some("@homedepot")),
                row(2, "b", Some("@homedepot")),
                row(3, "c", Some("@homedepot")),
                row(4, "d", Some("@homedepot")),
                row(5, "e", Some("@homedepot")),
            ],
            &no_expanded(),
        );
        assert_eq!(view.trips[0].more_label.as_deref(), Some("Show 2 more"));
        assert!(!view.trips[0].expanded);
    }

    #[test]
    fn a_trip_named_in_the_expanded_set_renders_expanded_with_a_fewer_label() {
        let view = build_expanded(
            vec![
                row(1, "a", Some("@homedepot")),
                row(2, "b", Some("@homedepot")),
                row(3, "c", Some("@homedepot")),
                row(4, "d", Some("@homedepot")),
                row(5, "e", Some("@homedepot")),
            ],
            &expanded_at("@homedepot"),
        );
        assert!(view.trips[0].expanded);
        assert_eq!(view.trips[0].more_label.as_deref(), Some("Show fewer"));
        assert_eq!(
            view.trips[0].collapsed_label.as_deref(),
            Some("Show 2 more"),
            "the label to restore once collapsed again must survive being expanded"
        );
    }

    #[test]
    fn a_different_trips_expanded_state_is_independent() {
        let view = build_expanded(
            vec![
                row(1, "a", Some("@homedepot")),
                row(2, "b", Some("@homedepot")),
                row(3, "c", Some("@homedepot")),
                row(4, "d", Some("@homedepot")),
                row(5, "e", Some("@homedepot")),
                row(6, "f", Some("@supermarket")),
                row(7, "g", Some("@supermarket")),
                row(8, "h", Some("@supermarket")),
                row(9, "i", Some("@supermarket")),
                row(10, "j", Some("@supermarket")),
            ],
            &expanded_at("@homedepot"),
        );
        let homedepot = view.trips.iter().find(|t| t.tag == "@homedepot").unwrap();
        let supermarket = view.trips.iter().find(|t| t.tag == "@supermarket").unwrap();
        assert!(homedepot.expanded);
        assert!(!supermarket.expanded);
    }

    // --- #125: the complete-group control ---------------------------------

    #[test]
    fn a_trip_with_nothing_done_offers_to_complete_the_whole_group() {
        let view = build_expanded(
            (1..=8)
                .map(|n| row(n, "item", Some("@homedepot")))
                .collect(),
            &no_expanded(),
        );
        assert!(view.trips[0].offers_complete);
        assert_eq!(view.trips[0].complete_label, "Complete all 8");
    }

    #[test]
    fn a_partly_done_trip_names_only_the_open_count() {
        let rows: Vec<PoolTaskRow> = (1..=8)
            .map(|n| {
                if n <= 5 {
                    done_row(n, "item", Some("@homedepot"))
                } else {
                    row(n, "item", Some("@homedepot"))
                }
            })
            .collect();
        let view = build_expanded(rows, &no_expanded());
        assert!(view.trips[0].offers_complete);
        assert_eq!(view.trips[0].complete_label, "Complete all 3");
    }

    #[test]
    fn a_fully_done_trip_offers_no_complete_control() {
        let view = build_expanded(
            vec![
                done_row(1, "a", Some("@homedepot")),
                done_row(2, "b", Some("@homedepot")),
                done_row(3, "c", Some("@homedepot")),
            ],
            &no_expanded(),
        );
        assert!(!view.trips[0].offers_complete);
    }
}
