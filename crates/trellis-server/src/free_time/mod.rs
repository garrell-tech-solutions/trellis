//! **Free time** -- `GET /free-time`, the first reader of both
//! `scheduler_core::guardrail` and `scheduler_core::free_time` (#60). Reads
//! every active life area's guardrail through `life_areas::guardrails` and
//! the owner's zone through `settings::current_timezone`
//! (`T-one-front-door-per-capability` -- neither store is this capability's
//! to reach into), and reports what `scheduler_core::free_time::
//! free_intervals` says about each for the next fourteen days.
//!
//! No `store` of its own: this capability stores nothing and computes
//! everything it shows from two other capabilities' front doors plus the
//! clock, so there is no SQL that is its own to own.

pub mod http;
