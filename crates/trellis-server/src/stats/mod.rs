//! **Stats** -- `GET /stats`, the rolling committed:pool ratio. Risk
//! experiment R2's instrumentation (#9 AC-6): the calendar is meant to sit
//! around 40% committed (`D-pool-is-default`), and this is the page that
//! reports whether it does, weeks before a scheduler is built around the
//! assumption.
//!
//! The ratio itself is not a rule to enforce here -- display only, nothing in
//! this product can notify until M7. [`store`] counts tasks by kind inside
//! the fourteen-day window; [`view`] turns those counts into the share, the
//! fifty-percent standing, and the sample-floor judgment of whether either is
//! fit to show.

pub mod http;
pub mod store;
pub mod view;
