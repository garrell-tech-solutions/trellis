//! Persistence adapters: the only place in the crate that writes SQL.
//!
//! Everything here speaks `scheduler_core` types and `sqlx::Error`. It must
//! not know that an HTTP server exists — mapping a failed write onto a status
//! code is the delivery layer's job, and a store that returns `StatusCode`
//! cannot be reused by any other delivery mechanism (T-sqlite-sqlx's
//! "a Postgres swap stays mechanical" has the same shape: keep the queries in
//! one layer that nothing above it reaches around).

pub mod capture;
pub mod task;

#[cfg(test)]
mod boundary_tests {
    use std::path::{Path, PathBuf};

    /// Delivery vocabulary that has no business in a persistence adapter.
    const FORBIDDEN: [&str; 2] = ["axum", "StatusCode"];

    fn store_sources() -> Vec<PathBuf> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/store");
        let mut sources: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("store module directory is readable")
            .map(|entry| entry.expect("store directory entry is readable").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
            .collect();
        sources.sort();
        sources
    }

    #[test]
    fn every_store_module_is_covered_by_this_check() {
        let sources = store_sources();
        assert!(
            sources.len() >= 3,
            "expected at least mod.rs, capture.rs and task.rs, found {sources:?}"
        );
    }

    #[test]
    fn the_store_layer_names_no_delivery_types() {
        for path in store_sources() {
            if path.ends_with("mod.rs") {
                continue; // this check itself names the forbidden words
            }
            let source = std::fs::read_to_string(&path).expect("store module is readable");
            for forbidden in FORBIDDEN {
                assert!(
                    !source.contains(forbidden),
                    "{} references {forbidden}: persistence must not depend on the delivery mechanism",
                    path.display()
                );
            }
        }
    }
}
