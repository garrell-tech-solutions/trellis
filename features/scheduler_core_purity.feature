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
