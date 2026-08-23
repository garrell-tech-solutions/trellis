//! Ordering committed work by date, for the third Menu tab (#94,
//! `D-four-screens`).
//!
//! **Chronological, total.** The canvas sorts by `ord`, a field whose only
//! derivation anywhere is a committed item with no time getting `ord: 99` —
//! a state committed triage's required deadline makes unreachable here, so
//! chronological order is total and `ord` is not built.
//!
//! **A past deadline still shows, marked, first.** That falls out of
//! chronological order with no special casing: past items sort earliest.
//! The one failure this screen cannot have is a missed deadline that
//! silently vanishes.

use crate::task::Commitment;

/// One committed task as this module needs it: enough to order, mark past
/// and render its date cell, nothing about how it got here.
pub struct CommittedTask {
    /// The task's own database id (#97: marking a specific one done).
    pub id: i64,
    pub text: String,
    pub context_tag: Option<String>,
    pub deadline_ms: i64,
    pub commitment: Commitment,
}

/// One row as the committed screen renders it.
pub struct CommittedRow {
    pub id: i64,
    pub text: String,
    pub context_tag: Option<String>,
    /// `"TUE 8:30"` for an *at*, `"BY THU"` for a *by* — the same 66px
    /// tabular-nums cell doing both jobs (`D-committed-is-at-or-by`'s
    /// drawing, since the canvas draws neither).
    pub date_cell: String,
    /// `deadline_ms < now_ms` — a missed deadline that vanished is the one
    /// failure this screen cannot have, so it stays in the list rather than
    /// being filtered, only marked.
    pub past: bool,
}

/// Orders `tasks` chronologically (earliest first, so a past deadline sorts
/// to the top) and marks each row past relative to `now_ms`. `tasks` may
/// arrive in any order; a stable sort keeps ties in the order they arrived.
/// `zone` is the owner's configured timezone (#110, `T-timezone-is-a-setting`)
/// -- both "past" and the date cell are read in it, not UTC, since a
/// deadline entered correctly at 23:00 local and shown on the wrong day is
/// the same bug from the other end.
pub fn order(
    mut tasks: Vec<CommittedTask>,
    now_ms: i64,
    zone: &jiff::tz::TimeZone,
) -> Vec<CommittedRow> {
    tasks.sort_by_key(|t| t.deadline_ms);
    tasks
        .into_iter()
        .map(|task| CommittedRow {
            id: task.id,
            text: task.text,
            context_tag: task.context_tag,
            date_cell: date_cell(task.deadline_ms, task.commitment, now_ms, zone),
            past: task.deadline_ms < now_ms,
        })
        .collect()
}

fn weekday_abbrev(weekday: jiff::civil::Weekday) -> &'static str {
    use jiff::civil::Weekday::*;
    match weekday {
        Monday => "MON",
        Tuesday => "TUE",
        Wednesday => "WED",
        Thursday => "THU",
        Friday => "FRI",
        Saturday => "SAT",
        Sunday => "SUN",
    }
}

const MONTH_ABBREVS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

fn month_abbrev(month: i8) -> &'static str {
    MONTH_ABBREVS[usize::try_from(month - 1).expect("jiff guarantees month is 1..=12")]
}

/// `"TUE 8:30"` for an *at* — the day and time, hour unpadded, minute
/// zero-padded, in `zone`. `"BY THU"` for a *by* within the next six days
/// of `now_ms`, since a *by*'s time of day is not the commitment
/// (`committed-screen-at-and-by-02`: a *by* with a time still renders as a
/// *by*, not silently becomes an *at*). Beyond that window `"BY THU"` can no
/// longer tell two Thursdays apart (`committed-date-cell-disambiguates-07`),
/// so a *by* further out reads `"BY 17 SEP"` instead — still short enough
/// for the cell's 66px, unlike a form that keeps the weekday too.
fn date_cell(
    deadline_ms: i64,
    commitment: Commitment,
    now_ms: i64,
    zone: &jiff::tz::TimeZone,
) -> String {
    let zoned = jiff::Timestamp::from_millisecond(deadline_ms)
        .expect("deadline_ms is a valid instant; the triage boundary already validated it")
        .to_zoned(zone.clone());
    let day = weekday_abbrev(zoned.weekday());
    match commitment {
        Commitment::At => format!("{day} {}:{:02}", zoned.hour(), zoned.minute()),
        Commitment::By => {
            let today = jiff::Timestamp::from_millisecond(now_ms)
                .expect("now_ms is a valid instant")
                .to_zoned(zone.clone())
                .date();
            let days_out = zoned
                .date()
                .since(today)
                .expect("both dates are in jiff's representable range")
                .get_days();
            if days_out < 7 {
                format!("BY {day}")
            } else {
                format!("BY {} {}", zoned.day(), month_abbrev(zoned.month()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(name: &str) -> jiff::tz::TimeZone {
        crate::timezone::resolve(name).expect("test zones are real IANA names")
    }

    fn task(text: &str, deadline_ms: i64, commitment: Commitment) -> CommittedTask {
        CommittedTask {
            id: 1,
            text: text.to_string(),
            context_tag: None,
            deadline_ms,
            commitment,
        }
    }

    // 2026-08-25T08:30:00Z is a Tuesday; 2026-08-27T17:00:00Z a Thursday;
    // 2026-08-28T13:00:00Z a Friday -- the same three dates the feature's
    // Background pins "today" against (2026-08-24T09:00:00Z, a Monday).
    const DENTIST_MS: i64 = 1787646600000; // 2026-08-25T08:30:00Z
    const TAX_RETURN_MS: i64 = 1787850000000; // 2026-08-27T17:00:00Z
    const FURNACE_MS: i64 = 1787922000000; // 2026-08-28T13:00:00Z
    const NOW_MS: i64 = 1787562000000; // 2026-08-24T09:00:00Z (Monday)

    #[test]
    fn an_at_cell_reads_the_weekday_and_unpadded_time() {
        assert_eq!(
            date_cell(DENTIST_MS, Commitment::At, NOW_MS, &zone("UTC")),
            "TUE 8:30"
        );
    }

    #[test]
    fn a_by_cell_reads_only_the_weekday_prefixed_by() {
        assert_eq!(
            date_cell(TAX_RETURN_MS, Commitment::By, NOW_MS, &zone("UTC")),
            "BY THU"
        );
    }

    #[test]
    fn a_by_with_a_time_still_renders_as_a_by_not_an_at() {
        // 17:00 is a real time, and it must not leak into the by cell.
        assert_eq!(
            date_cell(TAX_RETURN_MS, Commitment::By, NOW_MS, &zone("UTC")),
            "BY THU"
        );
    }

    #[test]
    fn tasks_order_chronologically_regardless_of_input_order() {
        let rows = order(
            vec![
                task("Q3 planning doc", TAX_RETURN_MS, Commitment::At),
                task("Book the dentist", DENTIST_MS, Commitment::At),
                task("Furnace service window", FURNACE_MS, Commitment::At),
            ],
            NOW_MS,
            &zone("UTC"),
        );
        let order: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
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
    fn a_deadline_before_now_is_marked_past() {
        let rows = order(
            vec![task("Renew the passport", NOW_MS - 1, Commitment::By)],
            NOW_MS,
            &zone("UTC"),
        );
        assert!(rows[0].past);
    }

    #[test]
    fn a_deadline_at_or_after_now_is_not_marked_past() {
        let rows = order(
            vec![task("Book the dentist", DENTIST_MS, Commitment::At)],
            NOW_MS,
            &zone("UTC"),
        );
        assert!(!rows[0].past);
    }

    /// The boundary itself: a deadline exactly at `now_ms` is not yet past.
    /// `a_deadline_at_or_after_now_is_not_marked_past` uses a deadline well
    /// after `now_ms`, which cannot distinguish `<` from `<=`.
    #[test]
    fn a_deadline_exactly_at_now_is_not_marked_past() {
        let rows = order(
            vec![task("Renew the passport", NOW_MS, Commitment::By)],
            NOW_MS,
            &zone("UTC"),
        );
        assert!(!rows[0].past);
    }

    #[test]
    fn a_past_deadline_sorts_first() {
        let rows = order(
            vec![
                task("Book the dentist", DENTIST_MS, Commitment::At),
                task("Renew the passport", NOW_MS - 1, Commitment::By),
            ],
            NOW_MS,
            &zone("UTC"),
        );
        assert_eq!(rows[0].text, "Renew the passport");
        assert!(rows[0].past);
        assert!(!rows[1].past);
    }

    #[test]
    fn context_tag_is_preserved_and_none_stays_none() {
        let mut with_tag = task("buy screws", DENTIST_MS, Commitment::At);
        with_tag.context_tag = Some("@desk".to_string());
        let rows = order(vec![with_tag], NOW_MS, &zone("UTC"));
        assert_eq!(rows[0].context_tag.as_deref(), Some("@desk"));

        let rows = order(
            vec![task("no tag", DENTIST_MS, Commitment::At)],
            NOW_MS,
            &zone("UTC"),
        );
        assert_eq!(rows[0].context_tag, None);
    }

    #[test]
    fn an_empty_list_orders_to_nothing() {
        assert!(order(vec![], NOW_MS, &zone("UTC")).is_empty());
    }

    #[test]
    fn a_rows_id_is_its_tasks_id() {
        let mut with_id = task("book the dentist", DENTIST_MS, Commitment::At);
        with_id.id = 42;
        let rows = order(vec![with_id], NOW_MS, &zone("UTC"));
        assert_eq!(rows[0].id, 42);
    }

    // --- timezone-aware rendering (#110) -----------------------------------

    const NY_NAME: &str = "America/New_York";
    // 2026-08-25T08:30:00Z in America/New_York (2026-08-25T08:30 NY was
    // computed as this instant by scheduler_core::task::fields::
    // local_deadline_ms -- rendering it back in the same zone must
    // round-trip to the same wall-clock day and time).
    const DENTIST_NY_AT_0830_MS: i64 = 1787661000000;
    const DENTIST_NY_AT_2330_MS: i64 = 1787715000000;
    const DENTIST_NY_AT_0030_MS: i64 = 1787632200000;
    const TAX_RETURN_NY_BY_MS: i64 = 1787889599999; // end of 2026-08-27 NY
    const PASSPORT_NY_FAR_BY_MS: i64 = 1789703999999; // end of 2026-09-17 NY

    #[test]
    fn an_at_cell_renders_in_the_given_zone_not_utc() {
        assert_eq!(
            date_cell(
                DENTIST_NY_AT_0830_MS,
                Commitment::At,
                NOW_MS,
                &zone(NY_NAME)
            ),
            "TUE 8:30"
        );
    }

    #[test]
    fn near_midnight_local_times_render_on_the_day_they_were_chosen() {
        assert_eq!(
            date_cell(
                DENTIST_NY_AT_2330_MS,
                Commitment::At,
                NOW_MS,
                &zone(NY_NAME)
            ),
            "TUE 23:30"
        );
        assert_eq!(
            date_cell(
                DENTIST_NY_AT_0030_MS,
                Commitment::At,
                NOW_MS,
                &zone(NY_NAME)
            ),
            "TUE 0:30"
        );
    }

    #[test]
    fn a_by_within_the_next_six_days_reads_the_weekday() {
        // NOW_MS is 2026-08-24T09:00:00Z; the by is three days out.
        assert_eq!(
            date_cell(TAX_RETURN_NY_BY_MS, Commitment::By, NOW_MS, &zone(NY_NAME)),
            "BY THU"
        );
    }

    #[test]
    fn a_by_beyond_this_week_reads_the_date_instead() {
        // 24 days out -- "BY THU" could not tell it apart from a closer one.
        assert_eq!(
            date_cell(
                PASSPORT_NY_FAR_BY_MS,
                Commitment::By,
                NOW_MS,
                &zone(NY_NAME)
            ),
            "BY 17 SEP"
        );
    }
}
