# committed-field-domains-deadline-round-trip-01: a well-formed deadline round-trips to the same instant regardless of its exact textual form
# committed-field-domains-invalid-deadline-02: an unparseable or invalid deadline is rejected and creates nothing
# committed-field-domains-invalid-deadline-type-03: a deadline type outside hard and soft is rejected and creates nothing
# committed-field-domains-invalid-priority-04: a priority outside P1 through P4 is rejected and creates nothing
Feature: Committed triage validates the values of deadline, deadline type and priority, not just their presence

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "call the dentist" is waiting in the untriaged queue

  Scenario: A well-formed deadline round-trips to the same instant regardless of its exact textual form
    When the capture is triaged as a committed task with a deadline of "<submitted_deadline>"
    Then the resulting task's deadline is the instant "<expected_epoch_ms>" milliseconds since the epoch

    Examples:
      | submitted_deadline         | expected_epoch_ms |
      | 2026-08-20T17:00:00Z       | 1787245200000      |
      | 2026-08-20T17:00:00.000Z   | 1787245200000      |
      | 2026-08-20T19:00:00+02:00  | 1787245200000      |

  # committed-field-domains-invalid-deadline-02: an unparseable or invalid deadline is rejected and creates nothing
  Scenario: An unparseable or invalid deadline is rejected
    When the capture is triaged as a committed task with a deadline of "<bad_deadline>"
    Then the triage is rejected
    And the rejection reports "deadline" as invalid
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | bad_deadline               |
      | banana                     |
      | 2026-13-45T99:99:99Z       |
      | '); DROP TABLE tasks;--    |

  # committed-field-domains-invalid-deadline-type-03: a deadline type outside hard and soft is rejected and creates nothing
  Scenario: A deadline type outside hard and soft is rejected
    When the capture is triaged as a committed task with a deadline type of "<bad_deadline_type>"
    Then the triage is rejected
    And the rejection reports "deadline_type" as invalid
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | bad_deadline_type |
      | squishy            |
      | HARD                |

  # committed-field-domains-invalid-priority-04: a priority outside P1 through P4 is rejected and creates nothing
  Scenario: A priority outside P1 through P4 is rejected
    When the capture is triaged as a committed task with a priority of "<bad_priority>"
    Then the triage is rejected
    And the rejection reports "priority" as invalid
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | bad_priority |
      | P9           |
      | p1           |
