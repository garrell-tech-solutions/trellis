//! What the quota template renders — text and structure, never a raw store
//! row (`T-templates-take-view-models`).

use crate::quota::store::QuotaRow;
use scheduler_core::quota;

/// One quota, as the screen shows it. No expand, no quick-log controls and
/// no per-session list here on purpose: this slice stops at the entity, the
/// screen and defining a quota (docs/plans/2026-08-25-quota-screen-brief.md
/// -- "the line to stop at"), and drawing a `+30m` that writes nothing is
/// exactly the half-built surface the brief forbids. `logged_minutes` is
/// therefore always `0`: there is nowhere yet for a session to come from.
pub struct QuotaRowView {
    pub id: i64,
    pub name: String,
    /// `"0m / 4h"` (`quota-screen-reads-its-target-05`).
    pub readout: String,
    /// `"4h left this week · 0%"`.
    pub note: String,
}

pub struct QuotaScreenView {
    /// `"none yet"` with nothing defined, `"1 quota"` / `"N quotas"`
    /// otherwise (`quota-screen-fourth-screen-01`,
    /// `-defined-order-08`).
    pub meta: String,
    pub empty: bool,
    pub quotas: Vec<QuotaRowView>,
}

/// `minutes` formatted the way every duration on this screen reads:
/// `"0m"`, `"30m"`, `"1h"`, `"1h 30m"`. Zero is a special case rather than
/// falling out of the general rule, since `0 % 60 == 0` would otherwise
/// print `"0h"` and every tested zero on this screen reads `"0m"`
/// (`quota-screen-reads-its-target-05`'s `"0m / 4h"`).
fn format_duration(minutes: i64) -> String {
    if minutes == 0 {
        return "0m".to_string();
    }
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// `rows` already arrive oldest-first (`store::list_quotas`'s own
/// `ORDER BY`); this function does not re-sort them
/// (`T-set-operations-execute-in-the-store`).
pub(super) fn build(rows: Vec<QuotaRow>) -> QuotaScreenView {
    let empty = rows.is_empty();
    let meta = match rows.len() {
        0 => "none yet".to_string(),
        1 => "1 quota".to_string(),
        n => format!("{n} quotas"),
    };
    let quotas = rows.into_iter().map(quota_row_view).collect();
    QuotaScreenView {
        meta,
        empty,
        quotas,
    }
}

fn quota_row_view(row: QuotaRow) -> QuotaRowView {
    let logged_minutes = 0;
    let progress = quota::progress(row.weekly_target, logged_minutes);
    QuotaRowView {
        id: row.id,
        readout: format!(
            "{} / {}",
            format_duration(logged_minutes),
            format_duration(row.weekly_target.minutes())
        ),
        note: format!(
            "{} left this week · {}%",
            format_duration(progress.remaining_minutes),
            progress.percent
        ),
        name: row.name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i64, name: &str, weekly_target_minutes: i64) -> QuotaRow {
        QuotaRow {
            id,
            name: name.to_string(),
            weekly_target: quota::WeeklyTarget::from_minutes(weekly_target_minutes)
                .expect("a positive target"),
        }
    }

    #[test]
    fn an_empty_quota_screen_reports_none_yet_and_the_empty_flag() {
        let view = build(vec![]);
        assert_eq!(view.meta, "none yet");
        assert!(view.empty);
        assert!(view.quotas.is_empty());
    }

    #[test]
    fn meta_reads_one_quota_in_the_singular() {
        let view = build(vec![row(1, "Piano", 240)]);
        assert_eq!(view.meta, "1 quota");
        assert!(!view.empty);
    }

    #[test]
    fn meta_reads_several_quotas_in_the_plural() {
        let view = build(vec![row(1, "Piano", 240), row(2, "Running", 180)]);
        assert_eq!(view.meta, "2 quotas");
    }

    #[test]
    fn a_freshly_defined_quota_reads_its_target_with_nothing_logged_yet() {
        let view = build(vec![row(1, "Piano", 240)]);
        assert_eq!(view.quotas[0].readout, "0m / 4h");
        assert_eq!(view.quotas[0].note, "4h left this week · 0%");
    }

    #[test]
    fn rows_keep_the_stores_own_order() {
        let view = build(vec![row(1, "Piano", 240), row(2, "Running", 180)]);
        let names: Vec<&str> = view.quotas.iter().map(|q| q.name.as_str()).collect();
        assert_eq!(names, vec!["Piano", "Running"]);
    }

    #[test]
    fn format_duration_reads_minutes_under_an_hour() {
        assert_eq!(format_duration(30), "30m");
    }

    #[test]
    fn format_duration_reads_a_whole_number_of_hours() {
        assert_eq!(format_duration(60), "1h");
        assert_eq!(format_duration(240), "4h");
    }

    #[test]
    fn format_duration_reads_hours_and_minutes_together() {
        assert_eq!(format_duration(90), "1h 30m");
    }

    #[test]
    fn format_duration_reads_zero_as_zero_minutes() {
        assert_eq!(format_duration(0), "0m");
    }
}
