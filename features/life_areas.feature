# life-areas-seed-01: a fresh database offers the five seeded life areas
# life-areas-add-02: a life area added from the page is listed immediately, without a restart
# life-areas-duplicate-03: a life area cannot be added twice, whatever the case of the name
# life-areas-trims-04: surrounding whitespace is not part of a life area's name
# life-areas-blank-05: a life area must have a name that is not just whitespace
# life-areas-archived-06: an archived life area leaves the picker but still names the tasks already in it
# life-areas-escapes-hostile-text-07: hostile text in a life area name stays escaped where the list renders it
Feature: Life areas are user-managed rows, added and retired from the running app

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A fresh database offers the five seeded life areas
    When the life areas page is viewed
    Then the life areas listed are exactly "<listed>"

    Examples:
      | listed                                |
      | Work, Fitness, Learning, Family, Home |

  # life-areas-add-02: a life area added from the page is listed immediately, without a restart
  Scenario: A life area added from the page is listed immediately
    When a life area named "<name>" is added from the life areas page
    Then the add response does not redirect the browser
    And the life areas listed are exactly "<listed>"

    Examples:
      | name         | listed                                              |
      | Side project | Work, Fitness, Learning, Family, Home, Side project |

  # life-areas-duplicate-03: a life area cannot be added twice, whatever the case of the name
  Scenario: A life area cannot be added twice, whatever the case of the name
    When a life area named "<name>" is added from the life areas page
    Then the add is rejected
    And the rejection says "<name>" is already a life area
    And the life areas listed are exactly "<listed>"

    Examples:
      | name | listed                                |
      | Work | Work, Fitness, Learning, Family, Home |
      | work | Work, Fitness, Learning, Family, Home |
      | WORK | Work, Fitness, Learning, Family, Home |

  # life-areas-trims-04: surrounding whitespace is not part of a life area's name
  Scenario: Surrounding whitespace is not part of a life area's name
    When a life area named "  Side project  " is added from the life areas page
    Then the life areas listed are exactly "Work, Fitness, Learning, Family, Home, Side project"

  # life-areas-blank-05: a life area must have a name that is not just whitespace
  Scenario: A life area must have a name that is not just whitespace
    When a life area named "   " is added from the life areas page
    Then the add is rejected
    And the rejection names "name"
    And the life areas listed are exactly "Work, Fitness, Learning, Family, Home"

  # life-areas-archived-06: an archived life area leaves the picker but still names the tasks already in it
  Scenario: An archived life area leaves the picker but still names the tasks already in it
    Given a capture with raw text "sketch the landing page" is waiting in the untriaged queue
    And the capture is triaged as a pool task in life area "Learning"
    When the life area "Learning" is archived
    And the inbox is viewed
    Then the triage life area choices are exactly "Work, Fitness, Family, Home"
    And the task list shows "sketch the landing page" tagged "Learning"

  # life-areas-escapes-hostile-text-07: hostile text in a life area name stays escaped where the list renders it
  Scenario: Hostile text in a life area name stays escaped where the list renders it
    When a life area named "<script>alert('boom')</script>" is added from the life areas page
    And the life areas page is viewed
    Then the life areas list does not contain an unescaped "<script>" tag
    And the life areas list contains the word "boom"
