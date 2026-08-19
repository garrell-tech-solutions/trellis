//! **Free time** -- `GET /free-time`, the first reader of both
//! `scheduler_core::guardrail` and `scheduler_core::free_time` (#60). Reads
//! every active life area's guardrail through `life_areas::guardrails`,
//! the owner's zone through `settings::current_timezone`, and each life
//! area's applicable dated exceptions through `exceptions::for_life_area`
//! (`T-one-front-door-per-capability` -- no other capability's store is
//! this one's to reach into), and reports what `scheduler_core::free_time::
//! free_intervals` says about each for the next fourteen days.
//!
//! Also renders the exceptions list itself and its add form (#61) --
//! `exceptions::http` owns the writes, this capability only reads them
//! back through `exceptions::list`.
//!
//! [`free_time_by_life_area`] is this capability's own front door for a
//! second reader: `#62`'s capacity page needs the identical "guardrail
//! minus exceptions, projected over the horizon" computation this page
//! renders, and the only way to guarantee the two pages agree about the
//! same fortnight is one computation both call, not two that happen to
//! agree today (`T-one-front-door-per-capability`).
//!
//! No `store` of its own: this capability stores nothing and computes
//! everything it shows from three other capabilities' front doors plus the
//! clock, so there is no SQL that is its own to own.

pub mod http;

use crate::platform::clock::Clock;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::Timestamp;
use scheduler_core::free_time::{free_intervals, Guardrail, Range};
use scheduler_core::interval::Interval;
use sqlx::SqlitePool;

/// The look-ahead every reader of this capability projects across. Fourteen
/// days by rule, not a coincidence (`T-free-time-horizon-fourteen-days`):
/// the window holds exactly two of every weekday whatever day it starts
/// on, and `#62`'s capacity horizon is the same number by rule too, because
/// its supply is the sum of these very intervals.
pub(crate) const HORIZON_DAYS: i64 = 14;

/// One active life area's projected free time over the horizon: its
/// guardrail's bands, minus whatever exceptions apply to it, as disjoint
/// sorted intervals. `pool_only` rides along because a reader may need to
/// tell "never scheduled" apart from "walled, currently offering nothing"
/// (`T-guardrail-well-formedness`) -- this module resolves bands for that
/// distinction already; a second reader should not have to ask again.
pub(crate) struct LifeAreaFreeTime {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) pool_only: bool,
    pub(crate) intervals: Vec<Interval>,
}

/// The owner's zone and today's civil date in it -- resolved once, read by
/// every life area's own projection.
///
/// Both `expect`s state an invariant this module does not itself enforce:
/// `settings::set_timezone` never persists a zone `scheduler_core::
/// timezone::resolve` would refuse, and `Clock::now_ms` always reports an
/// instant `jiff::Timestamp` can represent (both callers -- the system
/// clock and `trellis serve --now`'s parsed offset -- already went through
/// that same conversion once to construct the clock at all).
async fn today_and_zone(pool: &SqlitePool, clock: &Clock) -> Result<(Date, TimeZone), sqlx::Error> {
    let zone_name = crate::settings::current_timezone(pool).await?;
    let tz = scheduler_core::timezone::resolve(&zone_name)
        .expect("settings::set_timezone validates a zone before storing it");
    let now = Timestamp::from_millisecond(clock.now_ms())
        .expect("Clock::now_ms always reports a representable instant");
    Ok((now.to_zoned(tz.clone()).date(), tz))
}

/// Every active life area's projected free time over the horizon, and the
/// zone it was projected in (a second reader formatting an interval needs
/// the same zone to read it back in civil terms).
pub(crate) async fn free_time_by_life_area(
    pool: &SqlitePool,
    clock: &Clock,
) -> Result<(Vec<LifeAreaFreeTime>, TimeZone), sqlx::Error> {
    let (today, tz) = today_and_zone(pool, clock).await?;
    let range = Range::horizon(today, HORIZON_DAYS);
    let guardrails = crate::life_areas::guardrails(pool).await?;
    let mut life_areas = Vec::with_capacity(guardrails.len());
    for area in guardrails {
        let excluded = crate::exceptions::for_life_area(pool, area.id).await?;
        let guardrail = Guardrail {
            bands: &area.bands,
            timezone: &tz,
            excluded: &excluded,
        };
        life_areas.push(LifeAreaFreeTime {
            id: area.id,
            name: area.name,
            pool_only: area.pool_only,
            intervals: free_intervals(guardrail, range),
        });
    }
    Ok((life_areas, tz))
}
