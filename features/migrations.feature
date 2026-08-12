# mutation-stamp: sha256=aaf90a7ef81b70a720b8e1f1f324d6c3a84cfc7bf4abaea2da4273838c01b162
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-12T15:15:17.237499288Z","feature_name":"Database migrations are idempotent and enable WAL mode","feature_path":"features/migrations.feature","background_hash":"74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b","implementation_hash":"sha256:ddb048be63fd0a6d6498571f6aa5fc1b249c8ee401c4cad1cc06d5530269207e","scenarios":[]}
# acceptance-mutation-manifest-end

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
