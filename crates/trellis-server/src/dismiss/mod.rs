//! **Dismiss** — the inbox's other exit. `POST /captures/{id}/dismiss` takes
//! a capture out of the untriaged queue without turning it into a task.
//!
//! `D-kill-means-archive`: the row stays (`captures` is never deleted). A
//! dismissed capture is not distinguished from a triaged one by a column of
//! its own — [`crate::triage::store::mark_triaged`] and this module's own
//! [`store::mark_dismissed`] write the same `captures.left_inbox_at`, so
//! "which way did it leave" is derivable from whether a `tasks` row
//! references it, and a row cannot claim both exits at once
//! (`T-archived-at-only`). `D-three-strike`: no confirmation dialog.
//! `D-inaction-archives`: dismissal is the deliberate act, not the same as
//! leaving a capture alone.

pub mod http;
pub mod store;
