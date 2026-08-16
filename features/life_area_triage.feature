# life-area-triage-offers-01: triage offers every life area, including one added since the server started
# life-area-triage-tags-02: triage tags the task with the chosen life area, and the task list shows it
# life-area-triage-required-03: triage without a life area is rejected, whatever the kind
# life-area-triage-unknown-04: triage naming a life area that does not exist is rejected
# life-area-triage-archived-05: triage into an archived life area is rejected at the boundary, not only hidden from the picker
# life-area-triage-escapes-hostile-text-06: a hostile life area name stays escaped where the task row renders it
Feature: Triage tags a task with a life area

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "sketch the landing page" is waiting in the untriaged queue

  Scenario: Triage offers every life area, including one added since the server started
    Given a life area named "Side project" was added
    When the inbox is viewed
    Then the triage life area choices are exactly "Work, Fitness, Learning, Family, Home, Side project"

  # life-area-triage-tags-02: triage tags the task with the chosen life area, and the task list shows it
  Scenario: Triage tags the task with the chosen life area, and the task list shows it
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When "sketch the landing page" is triaged as a pool task in life area "Learning"
    And "buy milk" is triaged as a pool task in life area "Home"
    And the inbox is viewed
    Then the task list shows "sketch the landing page" tagged "Learning"
    And the task list shows "buy milk" tagged "Home"

  # life-area-triage-required-03: triage without a life area is rejected, whatever the kind
  Scenario: Triage without a life area is rejected, whatever the kind
    When the capture is triaged as a <kind> task with "life_area" omitted
    Then the triage is rejected
    And the rejection names "life_area"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | kind      |
      | pool      |
      | committed |
      | quota     |

  # life-area-triage-unknown-04: triage naming a life area that does not exist is rejected
  Scenario: Triage naming a life area that does not exist is rejected
    When the capture is triaged as a pool task in life area "Gardening"
    Then the triage is rejected
    And the rejection says "Gardening" is not a life area
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

  # life-area-triage-archived-05: triage into an archived life area is rejected at the boundary, not only hidden from the picker
  Scenario: Triage into an archived life area is rejected at the boundary, not only hidden from the picker
    Given the life area "Learning" is archived
    When the capture is triaged as a pool task in life area "Learning"
    Then the triage is rejected
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

  # life-area-triage-escapes-hostile-text-06: a hostile life area name stays escaped where the task row renders it
  Scenario: A hostile life area name stays escaped where the task row renders it
    Given a life area named "<script>alert('boom')</script>" was added
    When the capture is triaged as a pool task in life area "<script>alert('boom')</script>"
    And the inbox is viewed
    Then the task list does not contain an unescaped "<script>" tag
    And the task list contains the word "boom"
