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
}

pub struct TripView {
    pub tag: String,
    /// `"3 things"` — `pool-screen-trips-and-loose-01`'s own wording.
    pub count_label: String,
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

/// `rows` need not arrive in any particular order —
/// `scheduler_core::pool::group` establishes newest-first itself.
pub(super) fn build(rows: Vec<PoolTaskRow>) -> PoolView {
    let total = rows.len();
    let tasks = rows
        .into_iter()
        .map(|row| PoolTask {
            sequence: row.task_id,
            text: row.raw_text,
            context_tag: row.context_tag,
        })
        .collect();
    let groups = pool::group(tasks);

    let trips = groups
        .trips
        .into_iter()
        .map(|trip| TripView {
            tag: trip.tag,
            count_label: format!("{} things", trip.count),
            items: trip.visible.into_iter().map(pool_item_view).collect(),
            more_label: (trip.more > 0).then(|| format!("Show {} more", trip.more)),
            hidden: trip.hidden.into_iter().map(pool_item_view).collect(),
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
        meta: if total == 0 {
            "empty".to_string()
        } else {
            format!("{total} waiting")
        },
        empty: total == 0,
        trips,
        loose,
    }
}

fn pool_item_view(item: pool::PoolItem) -> PoolItemView {
    PoolItemView {
        id: item.id,
        text: item.text,
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
}
