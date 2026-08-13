# mutation-stamp: sha256=b8b5bd460969e2640490df306480fd709d2a91efa027844f59e84c3dcf052204
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-12T21:49:27.075864932Z","feature_name":"Committed triage requires deadline, deadline type and priority","feature_path":"features/committed_triage_validation.feature","background_hash":"213f9062faacbf3d0a2218fdb859b4c843bbee4724073795573a29ea693f9d44","implementation_hash":"sha256:2304bc384951c7673fdba860ad848ea42c847d8e7ccad566f5c2e9bd942c4736","scenarios":[{"index":0,"name":"Triaging as committed without a required field is rejected and creates nothing","scenario_hash":"babbbfe44c6eaf6d1cf1d9bba2935d677455b14cbd7db0072da0fa22927d7654","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-12T21:49:27.075864932Z"},{"index":1,"name":"Triaging as committed with a required field left empty is rejected the same way as omitting it","scenario_hash":"ebdf7071db862250ef1cc32a30d6d968da4527fd475a69337b84f5557f00f1a4","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-12T21:49:27.075864932Z"}]}
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
