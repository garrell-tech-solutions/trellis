//! Supply against demand, per life area, over the free-time horizon (#62):
//! hours the owner has promised, against hours a guardrail (minus its
//! exceptions) actually offers. Every rule here survives changing HTTP --
//! what counts as demand, how quota prorates, where the warning fires --
//! so none of it lives at the adapter, the same call `scheduler_core::ratio`
//! already made for `/stats`' rules.
//!
//! **The direction that must never be wrong:** every choice below leans
//! toward reporting *less* availability rather than more, because a number
//! that is too optimistic is worse than no number at all -- that is the
//! whole reason M2 exists (`#6`).

use crate::task::Period;

/// A life area's committed demand: the sum of every task's estimate, and
/// how many carried none. An estimate-less task predates
/// `estimated_minutes` becoming required at triage (#62) -- the same
/// reading `T-life-area-required-at-triage` gave a task with no
/// `life_area_id`. It is never counted as zero, which would under-report
/// demand in the one direction this number must never be wrong; its count
/// rides along so a reader can see the total is incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommittedDemand {
    pub minutes: i64,
    pub unestimated: usize,
}

/// Sums a life area's committed task estimates, excluding (and counting)
/// any that carry none.
pub fn committed_demand(estimates: &[Option<i64>]) -> CommittedDemand {
    let mut demand = CommittedDemand::default();
    for estimate in estimates {
        match estimate {
            Some(minutes) => demand.minutes += minutes,
            None => demand.unestimated += 1,
        }
    }
    demand
}

/// A period's length in days for proration -- flat, never the calendar
/// month the horizon happens to straddle. The horizon can cross a month
/// boundary, and "which month" has no answer when it does.
fn period_days(period: Period) -> i64 {
    match period {
        Period::Week => 7,
        Period::Month => 30,
    }
}

/// One quota task's demand over `horizon_days`: `count` sessions of `each`
/// minutes per `period`, prorated by the horizon and rounded **up** to the
/// whole minute -- under-reporting demand is the direction of wrongness
/// this number exists to prevent. `D-quota-no-rollover`: no debt is ever
/// carried forward, only what the horizon itself asks for.
pub fn quota_demand_minutes(
    count: i64,
    each_minutes: i64,
    period: Period,
    horizon_days: i64,
) -> i64 {
    let numerator = horizon_days * count * each_minutes;
    let denominator = period_days(period);
    (numerator + denominator - 1) / denominator
}

/// One life area's number: needed against available, both in minutes.
/// `percent_used` is reported on every row regardless of `over_minutes`,
/// which is `Some` only strictly above 100% -- the threshold is exact, not
/// a margin (`D-staleness-unset`: instrument first, tune at the first
/// reckoning, rather than picking a cushion with no fortnight of real
/// numbers behind it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacity {
    pub needed_minutes: i64,
    pub available_minutes: i64,
    pub percent_used: i64,
    pub over_minutes: Option<i64>,
}

/// Compares demand against supply. `available_minutes == 0` with positive
/// demand is not a scenario any acceptance criterion drives (a guardrail
/// with bands but an entirely excepted horizon); `percent_used` relies on
/// Rust's saturating float-to-int cast to degrade to a large-but-finite
/// number rather than panicking, and `over_minutes` -- computed
/// independently of the percentage -- still reports correctly either way.
/// A life area with no guardrail at all (`T-guardrail-well-formedness`'s
/// "never scheduled") is not this function's caller's business: that state
/// reports no capacity, not zero-and-over, which is a decision made before
/// this function is ever called.
pub fn capacity_for(needed_minutes: i64, available_minutes: i64) -> Capacity {
    let percent_used = ((needed_minutes as f64 / available_minutes as f64) * 100.0).round() as i64;
    let over_minutes =
        (needed_minutes > available_minutes).then(|| needed_minutes - available_minutes);
    Capacity {
        needed_minutes,
        available_minutes,
        percent_used,
        over_minutes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_demand_sums_every_present_estimate() {
        let demand = committed_demand(&[Some(180), Some(120)]);
        assert_eq!(
            demand,
            CommittedDemand {
                minutes: 300,
                unestimated: 0
            }
        );
    }

    #[test]
    fn committed_demand_excludes_an_absent_estimate_from_the_sum() {
        let demand = committed_demand(&[Some(180), None]);
        assert_eq!(demand.minutes, 180);
    }

    #[test]
    fn committed_demand_counts_how_many_estimates_were_absent() {
        let demand = committed_demand(&[None, Some(60), None]);
        assert_eq!(demand.unestimated, 2);
    }

    #[test]
    fn committed_demand_of_nothing_is_zero_and_zero() {
        assert_eq!(committed_demand(&[]), CommittedDemand::default());
    }

    #[test]
    fn quota_demand_prorates_a_weekly_target_across_a_two_week_horizon() {
        // 3 sessions of 40 minutes/week, over 14 days = two full weeks.
        assert_eq!(quota_demand_minutes(3, 40, Period::Week, 14), 240);
    }

    #[test]
    fn quota_demand_prorates_a_monthly_target_by_a_flat_thirty_days() {
        // 10 sessions of 45 minutes/month = 450 minutes/month; 14/30 of that.
        assert_eq!(quota_demand_minutes(10, 45, Period::Month, 14), 210);
    }

    #[test]
    fn quota_demand_rounds_up_when_proration_does_not_divide_evenly() {
        // 14 * 10 * 41 = 5740; 5740 / 30 = 191.33..., rounds up to 192.
        assert_eq!(quota_demand_minutes(10, 41, Period::Month, 14), 192);
    }

    #[test]
    fn quota_demand_is_zero_for_zero_sessions() {
        assert_eq!(quota_demand_minutes(0, 40, Period::Week, 14), 0);
    }

    #[test]
    fn capacity_for_reports_utilisation_under_full() {
        let capacity = capacity_for(180, 240);
        assert_eq!(capacity.percent_used, 75);
        assert_eq!(capacity.over_minutes, None);
    }

    #[test]
    fn capacity_for_calls_out_the_overage_when_demand_exceeds_supply() {
        let capacity = capacity_for(300, 240);
        assert_eq!(capacity.percent_used, 125);
        assert_eq!(capacity.over_minutes, Some(60));
    }

    #[test]
    fn capacity_for_does_not_warn_at_exactly_full() {
        let capacity = capacity_for(240, 240);
        assert_eq!(capacity.percent_used, 100);
        assert_eq!(capacity.over_minutes, None);
    }

    #[test]
    fn capacity_for_does_not_warn_one_minute_under_full() {
        let capacity = capacity_for(239, 240);
        assert_eq!(capacity.over_minutes, None);
    }

    #[test]
    fn capacity_for_warns_one_minute_over_full() {
        let capacity = capacity_for(241, 240);
        assert_eq!(capacity.over_minutes, Some(1));
    }

    #[test]
    fn capacity_for_reports_zero_percent_for_no_demand_and_no_supply() {
        let capacity = capacity_for(0, 0);
        assert_eq!(capacity.percent_used, 0);
        assert_eq!(capacity.over_minutes, None);
    }
}
