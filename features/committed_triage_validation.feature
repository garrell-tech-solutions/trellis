# mutation-stamp: sha256=88f09b4ed53e6b6f84deec0435d4e86cbbbd3c33572a7bd1b517b749992b8116
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-12T18:23:54.837066046Z","feature_name":"Committed triage requires deadline, deadline type and priority","feature_path":"features/committed_triage_validation.feature","background_hash":"74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b","implementation_hash":"sha256:2304bc384951c7673fdba860ad848ea42c847d8e7ccad566f5c2e9bd942c4736","scenarios":[{"index":0,"name":"Triaging as committed without a required field is rejected and creates nothing","scenario_hash":"4f2a560b6ccc4405734d76d806bd2cb3014c57b5c9800a31b0e9afe9827cbe64","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-12T18:23:54.837066046Z"}]}
# acceptance-mutation-manifest-end

# committed-triage-validation-missing-field-01: committed triage is rejected when a required field is absent
# committed-triage-validation-empty-field-02: committed triage is rejected when a required field is left empty, the same way as when it is absent
Feature: Committed triage requires deadline, deadline type and priority

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "call the dentist" is waiting in the untriaged queue

  Scenario: Triaging as committed without a required field is rejected and creates nothing
    When the capture is triaged as a committed task with "<missing_field>" omitted
    Then the triage is rejected
    And the rejection names "<missing_field>"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | missing_field |
      | deadline      |
      | deadline_type |
      | priority      |

  # committed-triage-validation-empty-field-02: committed triage is rejected when a required field is left empty, the same way as when it is absent
  Scenario: Triaging as committed with a required field left empty is rejected the same way as omitting it
    When the capture is triaged as a committed task with "<empty_field>" left empty
    Then the triage is rejected
    And the rejection names "<empty_field>"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | empty_field   |
      | deadline      |
      | deadline_type |
      | priority      |
