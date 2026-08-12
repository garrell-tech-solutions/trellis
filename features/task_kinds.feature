# task-kinds-pool-01: triaging a capture as pool creates a pool task with no deadline and no quota target
Feature: Triage creates tasks in one of three kinds

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "buy milk" is waiting in the untriaged queue

  Scenario: Triaging a capture as a pool task creates a pool task
    When the capture is triaged as a pool task
    Then the resulting task has kind "pool"
    And the resulting task has no deadline
    And the resulting task has no quota target

  # task-kinds-committed-02: triaging a capture as committed records deadline, deadline type and priority
  Scenario: Triaging a capture as a committed task records its scheduling metadata
    When the capture is triaged as a committed task with a "<deadline_type>" deadline of "<deadline>" and priority "<priority>"
    Then the resulting task has kind "committed"
    And the resulting task has a "<deadline_type>" deadline of "<deadline>"
    And the resulting task has priority "<priority>"
    And the resulting task has no quota target

    Examples:
      | deadline             | deadline_type | priority |
      | 2026-08-20T17:00:00Z | hard          | P1       |
      | 2026-08-31T09:00:00Z | soft          | P3       |

  # task-kinds-quota-03: triaging a capture as quota records the recurring target and leaves the deadline empty
  Scenario: Triaging a capture as a quota task records its recurring target
    When the capture is triaged as a quota task targeting "<target_count>" sessions of "<target_minutes_each>" minutes per "week"
    Then the resulting task has kind "quota"
    And the resulting task has a quota target of "<target_count>" sessions of "<target_minutes_each>" minutes per "week"
    And the resulting task has no deadline

    Examples:
      | target_count | target_minutes_each |
      | 3            | 45                  |
      | 1            | 90                  |
