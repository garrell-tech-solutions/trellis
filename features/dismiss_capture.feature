# dismiss-capture-offers-01: every untriaged capture offers a dismiss action on the page
# dismiss-capture-leaves-inbox-02: dismissing a capture removes it from the inbox and creates no task
# dismiss-capture-keeps-the-row-03: neither way out of the inbox deletes the capture row
# dismiss-capture-no-dismissal-after-triage-04: a triaged capture cannot then be dismissed
# dismiss-capture-no-triage-after-dismissal-05: a dismissed capture cannot then be triaged, and the rejection says why
# dismiss-capture-no-second-dismissal-06: a dismissed capture cannot be dismissed again
# dismiss-capture-no-second-triage-07: a triaged capture cannot be triaged again
# dismiss-capture-empty-state-08: dismissing the last capture leaves the inbox showing its ordinary empty-state message
# dismiss-capture-escapes-hostile-text-09: hostile capture text stays escaped in the fragment a dismissal returns
#
# 04 to 07 are one rule seen from four sides: a capture leaves the inbox
# exactly once. Written as four literal scenarios rather than one Examples
# table over the exit taken, because an exit named in a table cell is
# step vocabulary rather than data -- a mutated cell would name an exit that
# does not exist, which errors instead of failing (the life-areas-duplicate-03
# lesson, arriving from the other direction).
Feature: A capture can be dismissed, and its row is kept

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "asdfgh" is waiting in the untriaged queue

  Scenario: Every untriaged capture offers a dismiss action on the page
    When the inbox is viewed
    Then the inbox offers to dismiss "asdfgh"

  # dismiss-capture-leaves-inbox-02: dismissing a capture removes it from the inbox and creates no task
  Scenario: Dismissing a capture removes it from the inbox and creates no task
    When the capture is dismissed from the inbox
    Then the dismissal response does not redirect the browser
    When the inbox is viewed
    Then the inbox does not list "asdfgh"
    And the task list is still empty

  # dismiss-capture-keeps-the-row-03: neither way out of the inbox deletes the capture row
  Scenario: Neither way out of the inbox deletes the capture row
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When the capture is triaged as a pool task in life area "Home"
    And "asdfgh" is dismissed from the inbox
    And the inbox is viewed
    Then the inbox lists no captures
    And the capture row count is "<rows>"
    And the task list has "<tasks>" tasks

    Examples:
      | rows | tasks |
      | 2    | 1     |

  # dismiss-capture-no-dismissal-after-triage-04: a triaged capture cannot then be dismissed
  Scenario: A triaged capture cannot then be dismissed
    Given the capture is triaged as a pool task in life area "Home"
    When the capture is dismissed from the inbox
    Then the dismissal is rejected
    And the capture row count is "<rows>"
    And the task list has "<tasks>" tasks

    Examples:
      | rows | tasks |
      | 1    | 1     |

  # dismiss-capture-no-triage-after-dismissal-05: a dismissed capture cannot then be triaged, and the rejection says why
  Scenario: A dismissed capture cannot then be triaged
    Given the capture is dismissed from the inbox
    When the capture is triaged as a pool task in life area "Home"
    Then the triage is rejected
    And the rejection says the capture is no longer in the inbox
    And the capture row count is "<rows>"
    And the task list has "<tasks>" tasks

    Examples:
      | rows | tasks |
      | 1    | 0     |

  # dismiss-capture-no-second-dismissal-06: a dismissed capture cannot be dismissed again
  Scenario: A dismissed capture cannot be dismissed again
    Given the capture is dismissed from the inbox
    When the capture is dismissed from the inbox
    Then the dismissal is rejected
    And the capture row count is "<rows>"
    And the task list has "<tasks>" tasks

    Examples:
      | rows | tasks |
      | 1    | 0     |

  # dismiss-capture-no-second-triage-07: a triaged capture cannot be triaged again
  Scenario: A triaged capture cannot be triaged again
    Given the capture is triaged as a pool task in life area "Home"
    When the capture is triaged as a pool task in life area "Home"
    Then the triage is rejected
    And the capture row count is "<rows>"
    And the task list has "<tasks>" tasks

    Examples:
      | rows | tasks |
      | 1    | 1     |

  # dismiss-capture-empty-state-08: dismissing the last capture leaves the inbox showing its ordinary empty-state message
  Scenario: Dismissing the last capture leaves the inbox showing its ordinary empty-state message
    When the capture is dismissed from the inbox
    And the inbox is viewed
    Then the inbox lists no captures
    And the inbox shows an empty-state message

  # dismiss-capture-escapes-hostile-text-09: hostile capture text stays escaped in the fragment a dismissal returns
  Scenario: Hostile capture text stays escaped in the fragment a dismissal returns
    Given a capture with raw text "<script>alert('boom')</script>" is waiting in the untriaged queue
    When "asdfgh" is dismissed from the inbox
    Then the dismissal response does not contain an unescaped "<script>" tag
    And the dismissal response contains the word "boom"
