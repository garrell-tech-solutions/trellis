//! **Life areas** -- user-managed rows a task is tagged with at triage
//! (`T-life-areas-are-data`, #47). Not a Rust enum and not a config file: a
//! fresh database seeds five (Work, Fitness, Learning, Family, Home), and
//! from then on the set belongs to the user, editable from the running app
//! with no rebuild.
//!
//! [`store`] holds the capability's own reads and writes: listing what is
//! still active, finding a name to refuse a duplicate, adding, and
//! archiving. [`view`] is what the management page and the triage picker
//! both render. [`http`] serves `/life-areas` and its archive control.
//!
//! What is deliberately *not* here: resolving a triage submission's life
//! area name to an id is triage's own query, in `triage::store`
//! (`T-capability-owns-its-queries`), and whether a submitted name is
//! well-formed or collides with another is `scheduler_core::life_area`'s
//! rule, not this module's -- both survive changing HTTP for something else.

pub mod http;
pub mod store;
pub mod view;
