# unknown-kind-rejection-named-01: triaging with a kind outside pool, committed and quota is rejected and creates nothing
# unknown-kind-rejection-absent-02: triaging without naming a kind is rejected and creates nothing
Feature: Triage rejects a kind outside pool, committed and quota

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "buy milk" is waiting in the untriaged queue

  Scenario: Triaging with an unrecognised kind is rejected and creates nothing
    When the capture is triaged as kind "<bad_kind>"
    Then the triage is rejected
    And the rejection reports an unknown kind
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | bad_kind |
      | someday  |
      | later    |

  # unknown-kind-rejection-absent-02: triaging without naming a kind is rejected and creates nothing
  Scenario: Triaging without naming a kind is rejected and creates nothing
    When the capture is triaged with no kind named
    Then the triage is rejected
    And the rejection reports an unknown kind
    And the task list is still empty
    And the capture is still waiting in the untriaged queue
