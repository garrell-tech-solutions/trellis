# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-12T21:49:18.109725051Z","feature_name":"Committed triage validates the values of deadline, deadline type and priority, not just their presence","feature_path":"features/committed_field_domains.feature","background_hash":"213f9062faacbf3d0a2218fdb859b4c843bbee4724073795573a29ea693f9d44","implementation_hash":"sha256:cb9194cc4e83cebeb56cb950b77be5c6336c4dc534c17a3668d7a30501afdeda","scenarios":[{"index":0,"name":"A well-formed deadline round-trips to the same instant regardless of its exact textual form","scenario_hash":"401be35689e8498ce4c78d6be96172687ba4a47056db756ad6319af6dfb935c6","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-12T21:49:18.109725051Z"}]}
# acceptance-mutation-manifest-end

# committed-field-domains-deadline-round-trip-01: a well-formed deadline round-trips to the same instant regardless of its exact textual form
# committed-field-domains-invalid-deadline-02: an unparseable or invalid deadline is rejected and creates nothing
# committed-field-domains-invalid-commitment-03: a commitment outside at and by is rejected and creates nothing
# committed-field-domains-invalid-priority-04: a priority outside P1 through P4 is rejected and creates nothing
#
# `commitment` (at | by) replaced `deadline_type` (hard | soft) in #94's
# required set (D-committed-is-at-or-by); this scenario moved with it --
# it is the same "a value outside the closed domain is rejected" guarantee
# `committed_triage_validation.feature` already restated for presence.
Feature: Committed triage validates the values of deadline, commitment and priority, not just their presence

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

  # committed-field-domains-invalid-commitment-03: a commitment outside at and by is rejected and creates nothing
  Scenario: A commitment outside at and by is rejected
    When the capture is triaged as a committed task with a commitment of "<bad_commitment>"
    Then the triage is rejected
    And the rejection reports "commitment" as invalid
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | bad_commitment |
      | hard           |
      | AT              |

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
