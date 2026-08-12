# migrations-apply-to-empty-db-01: migrations apply cleanly to an empty database and enable WAL mode
Feature: Database migrations are idempotent and enable WAL mode

  Scenario: Running migrations against an empty database succeeds and enables WAL mode
    Given an empty trellis database file
    When the migration command is run
    Then the migration command exits successfully
    And the database journal mode is "wal"

  # migrations-rerun-is-noop-02: re-running migrations against an already-migrated database changes nothing
  Scenario: Re-running migrations against an already-migrated database is a no-op
    Given a trellis database that has already been migrated once
    When the migration command is run
    Then the migration command exits successfully
    And the database schema is unchanged
