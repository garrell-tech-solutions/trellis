//! **Settings** -- the owner's timezone, one value for the whole product
//! (`D-single-user`: one zone, not one per guardrail, #59).
//!
//! [`store`] holds the one row. [`http`] serves `POST /timezone`. Kept
//! deliberately by #88 even though its former reader (`free_time`) and its
//! former page (life areas) are both gone: `D-menu-is-a-worklist` gives #85
//! inline controls that read the timezone through this module, and the
//! value itself is the owner's data, not scaffolding for a page that no
//! longer exists. Until #85 lands there is no way to change it from the
//! running app -- a stated one-way door, not an oversight.

pub mod http;
pub mod store;
