# committed-triage-validation-missing-field-01: committed triage is rejected when a required field is absent
Feature: Committed triage requires deadline, deadline type and priority

  Scenario: Triaging as committed without a required field is rejected and creates nothing
    Given the trellis server is running with an empty task list
    And a capture with raw text "call the dentist" is waiting in the untriaged queue
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
