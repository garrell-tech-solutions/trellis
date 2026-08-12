# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-12T15:20:56.887400372Z","feature_name":"scheduler-core stays free of async runtime and database dependencies","feature_path":"features/scheduler_core_purity.feature","background_hash":"74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b","implementation_hash":"sha256:532bcb1e7f1d0f13e5d98a643e348a3eb646fd2a122141c5bdce532567c6f66d","scenarios":[]}
# acceptance-mutation-manifest-end

# scheduler-core-purity-01: scheduler-core depends on neither tokio nor sqlx
Feature: scheduler-core stays free of async runtime and database dependencies

  Scenario: Inspecting the scheduler-core dependency tree finds none of the forbidden dependencies
    Given the workspace is checked out
    When the dependency tree for the scheduler-core crate is listed
    Then the dependency tree contains zero occurrences of "<forbidden_dependency>"

    Examples:
      | forbidden_dependency |
      | tokio                |
      | sqlx                 |
