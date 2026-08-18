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
//! No `store` of its own: this capability stores nothing and computes
//! everything it shows from three other capabilities' front doors plus the
//! clock, so there is no SQL that is its own to own.

pub mod http;
