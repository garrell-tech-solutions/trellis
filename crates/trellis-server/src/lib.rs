//! The trellis server, packaged by business domain: one directory per
//! capability Trellis provides, each holding the code that delivers, renders
//! and persists that capability (`T-package-by-business-domain`).
//!
//! - [`capture`] — raw text in, fast enough to use mid-thought.
//! - [`triage`] — a capture becomes a typed task.
//! - [`inbox`] — what is still waiting, and what has already become a task.
//! - [`stats`] — the rolling committed:pool ratio, R2's instrumentation.
//! - [`platform`] — deliberately *not* a capability: the route table, the
//!   database, the clock, the vendored static assets. It is named so that a
//!   reader can tell at a glance which directories are the product and which
//!   one is the machinery.
//!
//! `T-module-boundary`'s dependency rule is unchanged by the packaging; only
//! its directory shape is. Dependencies still point inward:
//!
//! - `scheduler_core` (its own crate) — the rules. Names neither adapter.
//! - a domain's `http` — delivery. Translates requests into core inputs and
//!   core decisions into responses.
//! - a domain's `store` — persistence. Translates core types into rows, and
//!   is the only place production SQL is written.
//! - a domain's `view` — what the templates render, never a store row type
//!   (`T-templates-take-view-models`).
//!
//! A handler calls its store; a store never calls back. What changed is that
//! those three roles are now leaves under a capability's name rather than
//! top-level directories of their own, so `ls src/` reports what this server
//! does instead of what it is built with.
//!
//! The rule is enforced by `platform::boundary`, which walks every module in
//! this crate: persistence must not name a delivery type, nothing outside a
//! persistence module may write production SQL, and the walk fails loudly if
//! it ever stops covering anything.

pub mod capture;
pub mod inbox;
pub mod platform;
pub mod stats;
pub mod triage;
