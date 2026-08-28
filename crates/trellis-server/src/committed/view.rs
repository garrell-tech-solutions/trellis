//! What the committed template renders — text and structure, never a raw
//! `scheduler_core::committed_screen` type or a store row directly
//! (`T-templates-take-view-models`).

use crate::committed::store::CommittedTaskRow;
use scheduler_core::committed_screen::{self, CommittedTask};
use scheduler_core::task::Commitment;

pub struct CommittedRowView {
    pub id: i64,
    pub text: String,
    pub context_tag: Option<String>,
    pub date_cell: String,
    pub past: bool,
}

/// [`crate::pool::view::WayBackView`]'s identical shape for this screen
/// (#111) -- not shared between the two capabilities because each
/// template's own undo route differs (`/pool/.../undone` vs
/// `/committed/.../undone`), and a shared type would still need each caller
/// to know which path prefix its own `id` belongs behind.
pub struct WayBackView {
    pub id: i64,
    pub text: String,
}

pub struct CommittedView {
    /// `"3 dated"`, or `"nothing dated"` when empty -- the canvas
    /// distinguishes an empty screen from a screen of zero rather than
    /// reading `"0 dated"` (`committed-screen-empty-05`).
    pub meta: String,
    pub empty: bool,
    pub rows: Vec<CommittedRowView>,
}

/// `rows` need not arrive in any particular order --
/// `scheduler_core::committed_screen::order` establishes chronological order
/// itself. `zone` is the owner's configured timezone, already resolved
/// (#110, `T-timezone-is-a-setting`): both "past" and each row's date cell
/// are read in it, never UTC.
///
/// It arrives as a `TimeZone` rather than as its name because resolving a
/// name can fail, and the layer that read the name is the layer that should
/// answer for it. Taking a `&str` put two `expect`s inside `scheduler-core`,
/// resting on a doc comment asserting `settings` had validated the value --
/// an invariant held across two capabilities by prose, which is what
/// `T-cross-capability-invariants-need-an-owner` is about, and a panic path
/// in the one crate meant to be pure and total. Resolved here the core
/// cannot be handed a zone it cannot use, so there is nothing left to tie:
/// the shape the deleted `free_time` already used, where `free_intervals`
/// took a `&TimeZone` and `today_and_zone` did the resolving.
pub(super) fn build(
    rows: Vec<CommittedTaskRow>,
    now_ms: i64,
    zone: &jiff::tz::TimeZone,
) -> CommittedView {
    let total = rows.len();
    let tasks = rows
        .into_iter()
        .map(|row| CommittedTask {
            id: row.task_id,
            text: row.raw_text,
            context_tag: row.context_tag,
            deadline_ms: row.deadline,
            commitment: Commitment::parse(&row.commitment)
                .expect("every committed row's commitment was validated at triage"),
        })
        .collect();
    let ordered = committed_screen::order(tasks, now_ms, zone);

    let rows = ordered
        .into_iter()
        .map(|row| CommittedRowView {
            id: row.id,
            text: row.text,
            context_tag: row.context_tag,
            date_cell: row.date_cell,
            past: row.past,
        })
        .collect();

    CommittedView {
        meta: if total == 0 {
            "nothing dated".to_string()
        } else {
            format!("{total} dated")
        },
        empty: total == 0,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(name: &str) -> jiff::tz::TimeZone {
        scheduler_core::timezone::resolve(name).expect("test zones are real IANA names")
    }

    fn row(text: &str, tag: Option<&str>, deadline: i64, commitment: &str) -> CommittedTaskRow {
        CommittedTaskRow {
            task_id: 1,
            raw_text: text.to_string(),
            context_tag: tag.map(str::to_string),
            deadline,
            commitment: commitment.to_string(),
        }
    }

    const NOW_MS: i64 = 1787562000000; // 2026-08-24T09:00:00Z

    #[test]
    fn an_empty_committed_screen_reports_nothing_dated_and_the_empty_flag() {
        let view = build(vec![], NOW_MS, &zone("UTC"));
        assert_eq!(view.meta, "nothing dated");
        assert!(view.empty);
    }

    #[test]
    fn meta_counts_every_committed_task() {
        let view = build(
            vec![
                row("a", None, 1787646600000, "at"),
                row("b", None, 1787850000000, "by"),
                row("c", None, 1787922000000, "at"),
            ],
            NOW_MS,
            &zone("UTC"),
        );
        assert_eq!(view.meta, "3 dated");
        assert!(!view.empty);
    }

    #[test]
    fn a_row_carries_its_text_tag_date_cell_and_past_flag() {
        let view = build(
            vec![row("book the dentist", Some("@phone"), 1787646600000, "at")],
            NOW_MS,
            &zone("UTC"),
        );
        assert_eq!(view.rows[0].text, "book the dentist");
        assert_eq!(view.rows[0].context_tag.as_deref(), Some("@phone"));
        assert_eq!(view.rows[0].date_cell, "TUE 8:30");
        assert!(!view.rows[0].past);
    }

    #[test]
    fn a_row_with_no_tag_carries_none() {
        let view = build(
            vec![row("furnace service window", None, 1787922000000, "at")],
            NOW_MS,
            &zone("UTC"),
        );
        assert_eq!(view.rows[0].context_tag, None);
    }

    #[test]
    fn rows_come_back_in_chronological_order() {
        let view = build(
            vec![
                row("Q3 planning doc", None, 1787850000000, "at"),
                row("Book the dentist", None, 1787646600000, "at"),
                row("Furnace service window", None, 1787922000000, "at"),
            ],
            NOW_MS,
            &zone("UTC"),
        );
        let order: Vec<&str> = view.rows.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(
            order,
            vec![
                "Book the dentist",
                "Q3 planning doc",
                "Furnace service window"
            ]
        );
    }

    #[test]
    fn a_by_row_reads_its_by_cell() {
        let view = build(
            vec![row("File the tax return", None, 1787850000000, "by")],
            NOW_MS,
            &zone("UTC"),
        );
        assert_eq!(view.rows[0].date_cell, "BY THU");
    }

    #[test]
    fn a_rows_id_is_its_tasks_id() {
        let mut with_id = row("book the dentist", None, 1787646600000, "at");
        with_id.task_id = 42;
        let view = build(vec![with_id], NOW_MS, &zone("UTC"));
        assert_eq!(view.rows[0].id, 42);
    }

    #[test]
    fn the_date_cell_renders_in_the_given_zone_not_utc() {
        // 2026-08-25T08:30:00Z read in America/New_York is 04:30 the same
        // day -- a different cell than reading it in UTC would produce.
        let view = build(
            vec![row("book the dentist", None, 1787646600000, "at")],
            NOW_MS,
            &zone("America/New_York"),
        );
        assert_eq!(view.rows[0].date_cell, "TUE 4:30");
    }
}
