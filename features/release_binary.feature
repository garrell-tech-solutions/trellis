# release-binary-static-01: the release build produces a single static musl binary
Feature: Release binary is a single static executable

  Scenario: Building the release binary for the musl target produces a static executable
    Given the workspace is checked out
    When the release binary is built for the musl target
    Then exactly one release binary is produced
    And the binary reports no dynamic executable dependencies
