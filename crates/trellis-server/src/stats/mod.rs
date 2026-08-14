//! **Stats** -- `GET /stats`, the rolling committed:pool ratio. Risk
//! experiment R2's instrumentation (#9 AC-6): the calendar is meant to sit
//! around 40% committed (`D-pool-is-default`), and this is the page that
//! reports whether it does, weeks before a scheduler is built around the
//! assumption.
//!
//! Two modules, because the capability is only two things once the rule is
//! where it belongs: [`store`] counts tasks by kind between two instants, and
//! [`http`] asks `scheduler_core::ratio` what those counts mean and renders
//! the answer. The window's length, the sample floor, the share and the
//! fifty-percent line are all in the core -- see `scheduler_core::ratio` for
//! why a display feature keeps its rules there.

pub mod http;
pub mod store;
