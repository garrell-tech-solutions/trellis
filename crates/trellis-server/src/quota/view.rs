//! What the quota template renders — text and structure, never a raw store
//! row (`T-templates-take-view-models`).

use crate::quota::store::{QuotaRow, SessionRow};
use scheduler_core::quota::{self, Week};
use std::collections::HashMap;

/// One logged session, as "This week" lists it (#93, quota-sessions). The
/// list entry is always raw minutes (`"Tue 60m"`, never `"Tue 1h"`) --
/// `qa/quota_sessions.md`'s own observed procedure reads it that way, a
/// different context from the readout and note below, which do convert.
pub struct SessionRowView {
    pub id: i64,
    pub label: String,
    /// The day this session is logged under, in the same spelling
    /// `day_options` uses, so the correction form's `<select>` can mark it
    /// selected.
    pub day: String,
    pub minutes: i64,
}

/// One quota, as the screen shows it.
pub struct QuotaRowView {
    pub id: i64,
    pub name: String,
    /// `"0m / 4h"` (`quota-screen-reads-its-target-05`), or however much of
    /// the target this week's own sessions have already spent.
    pub readout: String,
    /// `"4h left this week · 0%"`.
    pub note: String,
    /// Newest fact last: Monday's sessions before Tuesday's, and within a
    /// day, in the order they were logged
    /// (`quota-sessions-this-week-lists-what-was-logged-04`).
    pub sessions: Vec<SessionRowView>,
    /// `Some("No sessions yet this week...")` exactly when `sessions` is
    /// empty (`quota-sessions-an-empty-week-says-so-05`); the two are never
    /// asserted independently; carried as a single field so the template
    /// cannot render one without the other.
    pub sessions_message: Option<String>,
    /// `"nothing logged"`, or `"N session(s) · Xh Ym"`.
    pub summary: String,
}

pub struct QuotaScreenView {
    /// `"none yet"` with nothing defined, `"1 quota"` / `"N quotas"`
    /// otherwise (`quota-screen-fourth-screen-01`,
    /// `-defined-order-08`).
    pub meta: String,
    pub empty: bool,
    pub quotas: Vec<QuotaRowView>,
    /// Monday through today, in order -- every quota's `Other…` day picker
    /// and every session's correction form share this one list
    /// (`quota-sessions-only-days-that-have-happened-03`).
    pub day_options: Vec<String>,
    /// The day picker's own default selection.
    pub today: String,
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
/// (`T-set-operations-execute-in-the-store`). `sessions` is every quota's
/// own this-week sessions in one flat list (`store::week_sessions`'s own
/// bounded query, the week boundary already applied in SQL); partitioning
/// it by `quota_id` here is display bookkeeping, not the business rule --
/// the rule is which rows the store's own `WHERE` admitted.
pub(super) fn build(
    rows: Vec<QuotaRow>,
    sessions: Vec<SessionRow>,
    week: &Week,
    zone: &jiff::tz::TimeZone,
) -> QuotaScreenView {
    let mut sessions_by_quota = group_sessions_by_quota(sessions);
    let empty = rows.is_empty();
    let meta = quota_count_meta(rows.len());
    let quotas = quota_row_views(rows, &mut sessions_by_quota, zone);
    QuotaScreenView {
        meta,
        empty,
        quotas,
        day_options: day_option_labels(week),
        today: week.today().label().to_string(),
    }
}

fn group_sessions_by_quota(sessions: Vec<SessionRow>) -> HashMap<i64, Vec<SessionRow>> {
    let mut by_quota: HashMap<i64, Vec<SessionRow>> = HashMap::new();
    for session in sessions {
        by_quota.entry(session.quota_id).or_default().push(session);
    }
    by_quota
}

fn quota_count_meta(count: usize) -> String {
    match count {
        0 => "none yet".to_string(),
        1 => "1 quota".to_string(),
        n => format!("{n} quotas"),
    }
}

fn quota_row_views(
    rows: Vec<QuotaRow>,
    sessions_by_quota: &mut HashMap<i64, Vec<SessionRow>>,
    zone: &jiff::tz::TimeZone,
) -> Vec<QuotaRowView> {
    rows.into_iter()
        .map(|row| {
            let sessions = sessions_by_quota.remove(&row.id).unwrap_or_default();
            quota_row_view(row, sessions, zone)
        })
        .collect()
}

fn day_option_labels(week: &Week) -> Vec<String> {
    week.days_so_far()
        .iter()
        .map(|day| day.label().to_string())
        .collect()
}

fn quota_row_view(
    row: QuotaRow,
    sessions: Vec<SessionRow>,
    zone: &jiff::tz::TimeZone,
) -> QuotaRowView {
    let logged_minutes: i64 = sessions.iter().map(|s| s.minutes).sum();
    let progress = quota::progress(row.weekly_target, logged_minutes);
    let sessions: Vec<SessionRowView> = sessions
        .iter()
        .map(|session| session_row_view(session, zone))
        .collect();
    let sessions_message = sessions
        .is_empty()
        .then(|| "No sessions yet this week. Log one above when you have done it.".to_string());
    let summary = session_summary(&sessions, logged_minutes);
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
        sessions,
        sessions_message,
        summary,
        name: row.name,
    }
}

fn session_row_view(row: &SessionRow, zone: &jiff::tz::TimeZone) -> SessionRowView {
    let day = quota::weekday_of(row.day_ms, zone);
    SessionRowView {
        id: row.id,
        label: format!("{} {}m", day.label(), row.minutes),
        day: day.label().to_string(),
        minutes: row.minutes,
    }
}

/// `"nothing logged"` with nothing this week, otherwise `"N session(s) ·
/// Xh Ym"` (`quota-sessions-this-week-lists-what-was-logged-04`,
/// `-an-empty-week-says-so-05`).
fn session_summary(sessions: &[SessionRowView], logged_minutes: i64) -> String {
    match sessions.len() {
        0 => "nothing logged".to_string(),
        1 => format!("1 session · {}", format_duration(logged_minutes)),
        n => format!("{n} sessions · {}", format_duration(logged_minutes)),
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

    fn zone() -> jiff::tz::TimeZone {
        scheduler_core::timezone::resolve("America/New_York").expect("a real IANA zone")
    }

    /// A Tuesday -- `quota_sessions.feature`'s own Background instant.
    fn tuesday_week() -> Week {
        Week::of(
            "2026-08-25T14:00:00Z"
                .parse::<jiff::Timestamp>()
                .unwrap()
                .as_millisecond(),
            &zone(),
        )
    }

    fn session(quota_id: i64, day_ms: i64, minutes: i64) -> SessionRow {
        SessionRow {
            id: 1,
            quota_id,
            day_ms,
            minutes,
        }
    }

    fn build_with(rows: Vec<QuotaRow>) -> QuotaScreenView {
        build(rows, vec![], &tuesday_week(), &zone())
    }

    #[test]
    fn an_empty_quota_screen_reports_none_yet_and_the_empty_flag() {
        let view = build_with(vec![]);
        assert_eq!(view.meta, "none yet");
        assert!(view.empty);
        assert!(view.quotas.is_empty());
    }

    #[test]
    fn meta_reads_one_quota_in_the_singular() {
        let view = build_with(vec![row(1, "Piano", 240)]);
        assert_eq!(view.meta, "1 quota");
        assert!(!view.empty);
    }

    #[test]
    fn meta_reads_several_quotas_in_the_plural() {
        let view = build_with(vec![row(1, "Piano", 240), row(2, "Running", 180)]);
        assert_eq!(view.meta, "2 quotas");
    }

    #[test]
    fn a_freshly_defined_quota_reads_its_target_with_nothing_logged_yet() {
        let view = build_with(vec![row(1, "Piano", 240)]);
        assert_eq!(view.quotas[0].readout, "0m / 4h");
        assert_eq!(view.quotas[0].note, "4h left this week · 0%");
    }

    #[test]
    fn rows_keep_the_stores_own_order() {
        let view = build_with(vec![row(1, "Piano", 240), row(2, "Running", 180)]);
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

    // --- sessions (#93, quota-sessions) -----------------------------------

    #[test]
    fn day_options_offers_monday_through_today() {
        let view = build_with(vec![row(1, "Piano", 240)]);
        assert_eq!(view.day_options, vec!["Mon", "Tue"]);
        assert_eq!(view.today, "Tue");
    }

    #[test]
    fn a_quotas_readout_reflects_this_weeks_logged_minutes() {
        let week = tuesday_week();
        let monday_ms = week.day_ms(scheduler_core::quota::Weekday::Mon, &zone());
        let view = build(
            vec![row(1, "Piano", 240)],
            vec![session(1, monday_ms, 20)],
            &week,
            &zone(),
        );
        assert_eq!(view.quotas[0].readout, "20m / 4h");
        assert_eq!(view.quotas[0].note, "3h 40m left this week · 8%");
    }

    #[test]
    fn sessions_are_scoped_to_their_own_quota() {
        let week = tuesday_week();
        let monday_ms = week.day_ms(scheduler_core::quota::Weekday::Mon, &zone());
        let view = build(
            vec![row(1, "Piano", 240), row(2, "Running", 180)],
            vec![session(1, monday_ms, 20)],
            &week,
            &zone(),
        );
        assert_eq!(view.quotas[0].readout, "20m / 4h");
        assert_eq!(view.quotas[1].readout, "0m / 3h");
    }

    #[test]
    fn a_sessions_label_reads_its_day_and_raw_minutes() {
        let week = tuesday_week();
        let monday_ms = week.day_ms(scheduler_core::quota::Weekday::Mon, &zone());
        let view = build(
            vec![row(1, "Piano", 240)],
            vec![session(1, monday_ms, 60)],
            &week,
            &zone(),
        );
        assert_eq!(view.quotas[0].sessions[0].label, "Mon 60m");
        assert_eq!(view.quotas[0].sessions[0].day, "Mon");
    }

    #[test]
    fn an_empty_week_says_so_and_summarises_nothing_logged() {
        let view = build_with(vec![row(1, "Piano", 240)]);
        assert_eq!(
            view.quotas[0].sessions_message.as_deref(),
            Some("No sessions yet this week. Log one above when you have done it.")
        );
        assert_eq!(view.quotas[0].summary, "nothing logged");
    }

    #[test]
    fn a_week_with_sessions_carries_no_message_and_summarises_them() {
        let week = tuesday_week();
        let zone = zone();
        let monday_ms = week.day_ms(scheduler_core::quota::Weekday::Mon, &zone);
        let tuesday_ms = week.day_ms(scheduler_core::quota::Weekday::Tue, &zone);
        let view = build(
            vec![row(1, "Piano", 240)],
            vec![session(1, monday_ms, 25), session(1, tuesday_ms, 35)],
            &week,
            &zone,
        );
        assert_eq!(view.quotas[0].sessions_message, None);
        assert_eq!(view.quotas[0].summary, "2 sessions · 1h");
    }

    #[test]
    fn a_single_session_summarises_in_the_singular() {
        let week = tuesday_week();
        let zone = zone();
        let monday_ms = week.day_ms(scheduler_core::quota::Weekday::Mon, &zone);
        let view = build(
            vec![row(1, "Piano", 240)],
            vec![session(1, monday_ms, 20)],
            &week,
            &zone,
        );
        assert_eq!(view.quotas[0].summary, "1 session · 20m");
    }
}
