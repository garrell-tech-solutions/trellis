//! What the pool template renders — text and structure, never a raw
//! `scheduler_core::pool` type or a store row directly
//! (`T-templates-take-view-models`).

use crate::pool::store::PoolTaskRow;
use scheduler_core::pool::{self, PoolTask};

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
    /// Newest-first, always shown.
    pub items: Vec<PoolItemView>,
    /// Newest-first, hidden behind `more_label` until revealed — a native
    /// `<details>` disclosure, so revealing them costs no request and no
    /// server-held state.
    pub hidden: Vec<PoolItemView>,
    /// `Some("Show 2 more")` when `hidden` is non-empty; `None` exactly when
    /// nothing is hidden (`pool-screen-truncation-06`).
    pub more_label: Option<String>,
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
pub(super) fn build(rows: Vec<PoolTaskRow>) -> PoolView {
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

    let trips = groups.trips.into_iter().map(trip_view).collect();
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

fn trip_view(trip: pool::Trip) -> TripView {
    TripView {
        tag: trip.tag,
        count_label: count_label(trip.count, trip.done_count),
        offers_clear: trip.done_count > 0,
        items: trip.visible.into_iter().map(pool_item_view).collect(),
        more_label: (trip.more > 0).then(|| format!("Show {} more", trip.more)),
        hidden: trip.hidden.into_iter().map(pool_item_view).collect(),
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
    fn a_truncated_trip_reports_how_many_more() {
        let view = build(vec![
            row(1, "a", Some("@homedepot")),
            row(2, "b", Some("@homedepot")),
            row(3, "c", Some("@homedepot")),
            row(4, "d", Some("@homedepot")),
            row(5, "e", Some("@homedepot")),
        ]);
        assert_eq!(view.trips[0].more_label.as_deref(), Some("Show 2 more"));
        assert_eq!(view.trips[0].items.len(), 3);
        let hidden: Vec<&str> = view.trips[0]
            .hidden
            .iter()
            .map(|item| item.text.as_str())
            .collect();
        assert_eq!(hidden, vec!["b", "a"]);
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
}
