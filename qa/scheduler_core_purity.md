# QA Procedure: scheduler-core stays free of async runtime and database dependencies

Covers: `features/scheduler_core_purity.feature`

This is a release-gate check (decisions.md T4): it must be run in CI from M0
onward, and a failure blocks every later milestone.

## Procedure — repeat once per forbidden dependency

Forbidden dependencies: `tokio`, `sqlx`.

For each forbidden dependency:
1. From a checked-out workspace, list the dependency tree scoped to the
   `scheduler-core` crate (e.g. `cargo tree -p scheduler-core`) — this is a
   build-tool affordance exposed at the command line, not a call into a
   project API.
2. Search the listed dependency tree output for the forbidden dependency
   name.

### Expected Observable Outcomes
- The forbidden dependency name occurs zero times in the listed dependency
  tree, for both `tokio` and `sqlx`.

## Independent of Implementation
This procedure only depends on `cargo`'s own dependency-tree listing for the
`scheduler-core` crate — not on which specific crates `scheduler-core`
actually depends on, as long as neither forbidden dependency appears.
