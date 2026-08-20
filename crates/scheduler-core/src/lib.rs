//! Pure scheduling algorithm core. No async runtime, no database.
//! Enforced by scheduler_core_purity.feature: this crate must never
//! depend on tokio or sqlx, directly or transitively.
//!
//! This is where the product's rules live. Delivery mechanisms (HTTP today)
//! and persistence adapters depend on this crate; it depends on neither.

pub mod capacity;
pub mod context_tag;
pub mod exception;
pub mod free_time;
pub mod guardrail;
pub mod interval;
pub mod life_area;
pub mod ratio;
pub mod schedule;
pub mod task;
pub mod timezone;

/// Trims `raw` and collapses blank (or absent) to `None` --
/// `T-empty-equals-absent`, shared by every optional free-text field that
/// reads this way: a context tag ([`context_tag::normalize`]) and, at
/// triage, a life area name ([`life_area::optional_name`]).
pub(crate) fn trim_or_absent(raw: Option<&str>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}
