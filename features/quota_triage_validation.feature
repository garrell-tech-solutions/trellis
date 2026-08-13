# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-13T21:28:30.411285268Z","feature_name":"Quota triage requires target count, target minutes each and period","feature_path":"features/quota_triage_validation.feature","background_hash":"d8cdc7c2933deb14809ad6a3cb7d7cdc60490c3c4e983e05b74bf8898673de1c","implementation_hash":"sha256:a6c5b4f9b71d7185c1b4db842c5e95b0dff8255ef6c4726f056173ffdc84597f","scenarios":[{"index":0,"name":"Triaging as quota without a required target field is rejected and creates nothing","scenario_hash":"4e761050ff966d6d4837a15cad090e29610d46fed9ceb580faea3233fc3c13a6","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-12T21:49:21.754594796Z"}]}
# acceptance-mutation-manifest-end

# quota-triage-validation-missing-field-01: quota triage is rejected when a required target field is absent
# quota-triage-validation-empty-period-02: quota triage is rejected when period is left empty, the same way as when it is absent
# quota-triage-validation-invalid-period-03: quota triage is rejected when period is outside week and month
# quota-triage-validation-nonpositive-target-count-04: quota triage is rejected when target_count is not positive
# quota-triage-validation-nonpositive-target-minutes-05: quota triage is rejected when target_minutes_each is not positive
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

  # quota-triage-validation-nonpositive-target-count-04: quota triage is rejected when target_count is not positive
  Scenario: A non-positive target_count is rejected
    When the capture is triaged as a quota task with a target_count of "<bad_target_count>"
    Then the triage is rejected
    And the rejection reports "target_count" as invalid
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | bad_target_count |
      | 0                 |
      | -1                |

  # quota-triage-validation-nonpositive-target-minutes-05: quota triage is rejected when target_minutes_each is not positive
  Scenario: A non-positive target_minutes_each is rejected
    When the capture is triaged as a quota task with a target_minutes_each of "<bad_target_minutes_each>"
    Then the triage is rejected
    And the rejection reports "target_minutes_each" as invalid
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | bad_target_minutes_each |
      | 0                        |
      | -5                       |
