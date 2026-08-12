# quota-triage-validation-missing-field-01: quota triage is rejected when a required target field is absent
# quota-triage-validation-empty-period-02: quota triage is rejected when period is left empty, the same way as when it is absent
# quota-triage-validation-invalid-period-03: quota triage is rejected when period is outside week and month
Feature: Quota triage requires target count, target minutes each and period

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "go to the gym" is waiting in the untriaged queue

  Scenario: Triaging as quota without a required target field is rejected and creates nothing
    When the capture is triaged as a quota task with "<missing_field>" omitted
    Then the triage is rejected
    And the rejection names "<missing_field>"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | missing_field       |
      | target_count        |
      | target_minutes_each |
      | period              |

  # quota-triage-validation-empty-period-02: quota triage is rejected when period is left empty, the same way as when it is absent
  Scenario: Triaging as quota with period left empty is rejected the same way as omitting it
    When the capture is triaged as a quota task with period left empty
    Then the triage is rejected
    And the rejection names "period"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

  # quota-triage-validation-invalid-period-03: quota triage is rejected when period is outside week and month
  Scenario: A period outside week and month is rejected
    When the capture is triaged as a quota task with a period of "<bad_period>"
    Then the triage is rejected
    And the rejection reports "period" as invalid
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | bad_period |
      | fortnight  |
      | Week       |
