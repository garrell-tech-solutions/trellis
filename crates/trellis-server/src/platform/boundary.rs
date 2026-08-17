//! The module boundary, checked rather than described.
//!
//! `T-module-boundary` wrote its own epitaph — *"a layering rule nothing
//! checks is a comment"* — and then very nearly died by it. The check it
//! shipped globbed `src/store/*.rs`; when `T-package-by-business-domain`
//! dissolved that directory the two tests would not have failed, they would
//! have stopped covering anything, which is the same rule dying quietly.
//! This module replaces them under the new packaging and is built so the
//! same thing cannot happen again:
//!
//! - It **walks** `src/` rather than naming a directory, so a new business
//!   domain is covered the moment it exists.
//! - Every check asserts a **floor** on what it found, so a walk that starts
//!   returning nothing fails loudly instead of passing vacuously.
//! - **Nothing is exempt** — not even this file. The old check skipped
//!   `store/mod.rs` because it named the forbidden words; the needles below
//!   are instead spelled in halves, so the walk can cover its own source and
//!   an exception list (the one place a real violation could hide) is not
//!   needed at all.
//!
//! What it enforces, all of it `T-module-boundary`'s dependency rule under
//! the new directory shape:
//!
//! 1. No persistence module names a delivery type. Persistence that returns
//!    a `StatusCode` cannot be reused by another delivery mechanism, and
//!    `T-sqlite-sqlx`'s "a Postgres swap stays mechanical" needs the queries
//!    to sit behind one seam.
//! 2. Nothing *but* a persistence module writes production SQL — the other
//!    half of the same seam, and the half the old check never had.
//! 3. `src/` names business domains, not technical roles.
//! 4. No capability names another capability's `store` in production.
//!    `T-one-front-door-per-capability`: a capability that other capabilities
//!    read exposes one function for it, in its `mod.rs`.
//!
//! It is a substring scan over source text, not a proof: it catches the
//! naive import and is defeated by a type alias or a macro. Treat it as a
//! lint. `scheduler_core`'s purity is a separate and stronger gate —
//! `cargo tree -p scheduler-core`, run by `scripts/qa/scheduler_core_purity.sh`.

use std::path::{Path, PathBuf};

/// Delivery vocabulary that has no business in a persistence module.
const DELIVERY_TYPES: [&str; 2] = ["axum", "StatusCode"];

/// What "this code writes SQL" looks like: the path prefix shared by the
/// `sqlx` query macros (`query`, `query_as`, `query_scalar`).
///
/// Spelled in two halves, and described without ever writing it out, so that
/// this file — which the walk below covers like any other — does not match
/// its own needle. Writing it in full here is exactly the mistake this
/// function exists to avoid, and the gate caught it once already during this
/// module's own development.
fn sql_needle() -> String {
    format!("{}::{}", "sqlx", "query")
}

/// Directory names that describe a technical role instead of a capability.
/// `T-package-by-business-domain`: a reader should learn what Trellis does
/// from `ls src/`, so these are the names that would take that away again.
/// `platform` is deliberately absent — it is the one bucket that says
/// "not a capability" out loud.
const TECHNICAL_ROLE_DIRS: [&str; 13] = [
    "api",
    "common",
    "controllers",
    "db",
    "handlers",
    "helpers",
    "http",
    "models",
    "repositories",
    "services",
    "store",
    "util",
    "utils",
];

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every Rust module in this crate, found by walking rather than by naming a
/// directory: a business domain added tomorrow is covered without anyone
/// remembering to add it here.
fn rust_sources() -> Vec<PathBuf> {
    let mut sources = Vec::new();
    collect_rust_sources(&src_dir(), &mut sources);
    sources.sort();
    sources
}

fn collect_rust_sources(dir: &Path, sources: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("crate source directory is readable");
    for entry in entries {
        let path = entry.expect("source directory entry is readable").path();
        if path.is_dir() {
            collect_rust_sources(&path, sources);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            sources.push(path);
        }
    }
}

/// The modules allowed to write SQL: every business domain's `store.rs`, plus
/// `platform/db.rs`, which owns the connection and the migrations and belongs
/// to no capability.
/// What "this module reaches into `<capability>`'s persistence" looks like as
/// a path. Spelled in halves for the reason [`sql_needle`] is: this file is
/// covered by the same walk, and a needle written out whole would match its
/// own source.
fn foreign_store_needle(capability: &str) -> String {
    format!("{}::{capability}::{}", "crate", "store")
}

/// The capability a module belongs to: its first path component under `src/`.
/// `lib.rs` and `main.rs` belong to none, which is why this is an `Option`
/// and not a `String` with an empty case.
fn capability_of(path: &Path) -> Option<String> {
    path.strip_prefix(src_dir())
        .ok()?
        .components()
        .next()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .filter(|first| src_dir().join(first).is_dir())
}

fn is_persistence(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "store.rs") || path.ends_with("platform/db.rs")
}

fn persistence_modules() -> Vec<PathBuf> {
    rust_sources()
        .into_iter()
        .filter(|p| is_persistence(p))
        .collect()
}

fn top_level_dirs() -> Vec<String> {
    let mut dirs: Vec<String> = std::fs::read_dir(src_dir())
        .expect("crate source directory is readable")
        .map(|entry| entry.expect("source directory entry is readable").path())
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    dirs.sort();
    dirs
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("{} is readable: {err}", path.display()))
}

/// The production half of a source file: everything outside a
/// `#[cfg(test)] mod ... { ... }` block.
///
/// Test code legitimately writes SQL anywhere — a handler test asserting what
/// landed in the table is exactly the test worth having — so scanning whole
/// files for rule 2 would be unenforceable. This mirrors what
/// `scripts/analyzers/_common.py` already means by "production" for the
/// complexity and CRAP gates, so one definition covers both.
///
/// Both ways this can be wrong are loud. Miscount and stop early, and test
/// text gets scanned *as* production — a false failure. Run off the end of
/// the file, and it panics. It never silently skips production code: a
/// `#[cfg(test)]` on anything that is not a `mod ... {` consumes exactly its
/// own line and nothing else.
fn production_source(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index].trim() == "#[cfg(test)]" && opens_a_module(lines.get(index + 1)) {
            index = end_of_module(&lines, index + 1);
        } else {
            kept.push(lines[index]);
            index += 1;
        }
    }
    kept.join("\n")
}

fn opens_a_module(line: Option<&&str>) -> bool {
    line.is_some_and(|line| line.trim_start().starts_with("mod ") && line.trim_end().ends_with('{'))
}

/// The index one past the module's closing brace, by brace counting from the
/// opening line.
fn end_of_module(lines: &[&str], opening: usize) -> usize {
    let mut depth = 0i32;
    for (offset, line) in lines[opening..].iter().enumerate() {
        depth += line.matches('{').count() as i32;
        depth -= line.matches('}').count() as i32;
        if depth <= 0 {
            return opening + offset + 1;
        }
    }
    panic!(
        "unterminated #[cfg(test)] module opening at line {}",
        opening + 1
    );
}

#[test]
fn the_walk_covers_the_whole_crate() {
    let sources = rust_sources();
    assert!(
        sources.len() >= 12,
        "the walk found only {} modules, which is fewer than this crate has: \
         it has stopped covering the tree it is supposed to police. Found {sources:?}",
        sources.len()
    );
    for expected in ["lib.rs", "main.rs"] {
        assert!(
            sources.iter().any(|p| p.ends_with(expected)),
            "the walk did not find src/{expected}: it is not looking where it thinks it is"
        );
    }
    for dir in top_level_dirs() {
        assert!(
            sources.iter().any(|p| p.starts_with(src_dir().join(&dir))),
            "src/{dir}/ contributed no module to the walk"
        );
    }
}

#[test]
fn the_crate_root_names_capabilities_not_technical_roles() {
    let dirs = top_level_dirs();
    assert!(
        !dirs.is_empty(),
        "src/ has no subdirectories at all, so this check covers nothing"
    );
    for dir in &dirs {
        assert!(
            !TECHNICAL_ROLE_DIRS.contains(&dir.as_str()),
            "src/{dir}/ names a technical role. T-package-by-business-domain: \
             `ls src/` should report what Trellis does, not what it is built \
             with. Package this under the capability it serves."
        );
    }
}

#[test]
fn every_persistence_module_is_covered_by_these_checks() {
    let modules = persistence_modules();
    assert!(
        modules.len() >= 3,
        "expected a store.rs per persisting business domain plus platform/db.rs, \
         found {modules:?}. If persistence really did move, point this check at \
         where it went — do not let it pass by covering nothing."
    );
}

#[test]
fn no_persistence_module_names_a_delivery_type() {
    let modules = persistence_modules();
    assert!(
        !modules.is_empty(),
        "no persistence module was found, so this loop would assert nothing — \
         which is precisely how the check it replaces would have died"
    );
    for path in modules {
        let source = read(&path);
        for forbidden in DELIVERY_TYPES {
            assert!(
                !source.contains(forbidden),
                "{} references {forbidden}: persistence must not depend on the \
                 delivery mechanism (T-module-boundary)",
                path.display()
            );
        }
    }
}

#[test]
fn only_a_persistence_module_writes_production_sql() {
    let needle = sql_needle();
    let mut checked = 0;
    for path in rust_sources() {
        if is_persistence(&path) {
            continue;
        }
        checked += 1;
        assert!(
            !production_source(&read(&path)).contains(&needle),
            "{} writes SQL outside a persistence module. Queries belong in the \
             business domain's store.rs, behind one seam \
             (T-module-boundary, T-sqlite-sqlx)",
            path.display()
        );
    }
    assert!(
        checked >= 8,
        "only {checked} non-persistence modules were checked; the walk has \
         stopped covering the delivery side"
    );
}

/// `T-one-front-door-per-capability`, checked. A capability may name another
/// capability's *types* — `inbox::view::CaptureRow`, `life_areas::view::
/// LifeAreaOption` — but not its `store`. Reaching for a type is borrowing a
/// shape; reaching for a query is holding a copy of how that capability works,
/// and copies drift.
///
/// **Production only, on purpose.** A test that sets up "a triaged capture"
/// by calling the capability that writes one is the right fixture: retyping
/// its `INSERT` would be a second copy of the schema, which is the worse
/// failure. The rule is about what the shipped code depends on.
///
/// The failure this catches is not hypothetical. `dismiss` arrived with its
/// own `store` holding a byte-identical copy of `triage`'s "is this capture
/// still open" query and its `left_inbox_at` write, each defensible on its
/// own as `T-capability-owns-its-queries`. Both moved behind
/// `inbox::capture_is_open` and `inbox::close_capture`; this is what stops
/// the third exit from making a third copy.
#[test]
fn no_capability_names_another_capabilitys_store() {
    let capabilities = top_level_dirs();
    assert!(
        capabilities.len() >= 5,
        "only {} capabilities found, so this check covers almost nothing: {capabilities:?}",
        capabilities.len()
    );
    let mut checked = 0;
    for path in rust_sources() {
        let own = capability_of(&path);
        let source = production_source(&read(&path));
        for capability in &capabilities {
            if own.as_deref() == Some(capability.as_str()) {
                continue;
            }
            checked += 1;
            assert!(
                !source.contains(&foreign_store_needle(capability)),
                "{} names {capability}'s store. Ask that capability for what you \
                 need through its mod.rs front door, the way triage and dismiss \
                 ask inbox::capture_is_open (T-one-front-door-per-capability)",
                path.display()
            );
        }
    }
    assert!(
        checked >= 40,
        "only {checked} module/capability pairs were checked; the walk has \
         stopped covering the tree"
    );
}

#[cfg(test)]
mod production_source_tests {
    use super::production_source;

    #[test]
    fn a_cfg_test_module_is_dropped_along_with_everything_in_it() {
        let source =
            "fn kept() {}\n#[cfg(test)]\nmod tests {\n    fn dropped() {}\n}\nfn also_kept() {}\n";
        let production = production_source(source);
        assert!(production.contains("kept"));
        assert!(production.contains("also_kept"));
        assert!(!production.contains("dropped"));
    }

    #[test]
    fn nesting_inside_a_cfg_test_module_does_not_end_it_early() {
        let source =
            "#[cfg(test)]\nmod tests {\n    fn a() {\n        if x { y() }\n    }\n}\nfn kept() {}\n";
        let production = production_source(source);
        assert!(production.contains("kept"));
        assert!(!production.contains("fn a()"));
    }

    #[test]
    fn a_cfg_test_item_that_is_not_a_module_consumes_only_its_own_attribute() {
        let source = "#[cfg(test)]\nuse other::Thing;\nfn kept() {}\n";
        let production = production_source(source);
        assert!(production.contains("use other::Thing;"));
        assert!(production.contains("fn kept()"));
    }

    #[test]
    fn a_file_with_no_test_module_survives_whole() {
        let source = "fn one() {}\nfn two() {}\n";
        assert_eq!(production_source(source), "fn one() {}\nfn two() {}");
    }
}
