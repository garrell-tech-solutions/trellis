//! **Quota** — `GET /quota`, the fourth Menu tab: a named container with a
//! weekly hour target, created directly from this screen (#93,
//! `D-quotas-are-selected-not-typed`). A different entity from
//! `scheduler_core::task::TaskKind::Quota`'s triaged capture, which stays
//! exactly as it is and does not appear here
//! (`quota-screen-triaged-quotas-are-elsewhere-09`) until #138 closes the
//! two concepts into one.
//!
//! This slice stops at the entity, the screen, the tab and defining a
//! quota. Logging sessions against one -- the quick `+30m`/`+1h` taps,
//! `Other`, and `This week`'s own edit/delete -- is `features/
//! quota_sessions.feature`, the brief's own named line to stop at, and is
//! not built here: every quota this module renders reads its target
//! against zero logged minutes, which is honest rather than a placeholder,
//! since nothing yet writes a session.
//!
//! [`store`] owns the one table this screen needs; [`view`] turns its rows
//! into the view the template renders; [`body`] is the `#quota-body`
//! fragment both `GET /quota` and a rejected `POST /quota` swap in, the same
//! shape `pool::body` and `committed::body` take.

mod body;
pub mod http;
pub mod store;
pub mod view;
