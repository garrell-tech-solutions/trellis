//! **Pool** — `GET /pool`, the first Menu tab: pool tasks grouped by where
//! they can be done (#92, `D-context-tags-are-the-taxonomy`'s first
//! consumer).
//!
//! `D-no-pool-on-calendar` makes this the *only* place pool work is
//! offered — no solver, no backward pass, no splitting, no pins
//! (`D-menu-is-a-worklist`). [`store`] owns the one query this screen
//! needs; [`view`] turns its rows into the trips-and-loose-ends shape the
//! template renders, via `scheduler_core::pool`'s pure grouping.
//!
//! **This capability writes nothing.** There is no manual reorder here —
//! the canvas draws the control and this slice deliberately does not build
//! it (`pool-screen-nothing-reorders-05`); when it comes, it belongs to
//! loose ends alone, never to a trip, whose order is derived from the data
//! and must stay that way.

pub mod http;
pub mod store;
pub mod view;
