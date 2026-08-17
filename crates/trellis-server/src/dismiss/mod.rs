//! **Dismiss** — the inbox's other exit. `POST /captures/{id}/dismiss` takes
//! a capture out of the untriaged queue without turning it into a task.
//!
//! `D-kill-means-archive`: the row stays (`captures` is never deleted). A
//! dismissed capture is not distinguished from a triaged one by a column of
//! its own — both exits call `inbox::close_capture`, so "which way
//! did it leave" is derivable from whether a `tasks` row references it, and a
//! row cannot claim both exits at once (`T-archived-at-only`).
//! `D-three-strike`: no confirmation dialog. `D-inaction-archives`: dismissal
//! is the deliberate act, not the same as leaving a capture alone.
//!
//! **This capability has no `store`, and that is the finding rather than an
//! omission.** Dismissal issues no SQL of its own: everything it does to the
//! database is *take this capture out of the inbox*, which is the inbox's
//! own fact and comes through the inbox's front door
//! (`T-one-front-door-per-capability`). `T-capability-owns-its-queries` gives
//! a capability the queries it issues; dismissal turns out to issue none, so
//! what is left here is exactly one HTTP handler.

pub mod http;
