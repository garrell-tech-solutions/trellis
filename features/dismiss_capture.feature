# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-27T04:25:03.428929569Z","feature_name":"A capture can be dismissed, and its row is kept","feature_path":"features/dismiss_capture.feature","background_hash":"01db5b6232c439c5b108b5e869cd6178d5cdba30bf901eed9dc0738685f7b18e","implementation_hash":"sha256:3940663b77d50b9408daf9ceb23258c83423f56a888edd7da07dccd68155699a","scenarios":[{"index":3,"name":"A triaged capture cannot then be dismissed","scenario_hash":"0346f546b07c0b59470947aaf4c6283d7ac5dc5a0dbba8fca2c0683eeacba656","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-27T04:25:03.428929569Z"},{"index":4,"name":"A dismissed capture cannot then be triaged","scenario_hash":"03b7af56f8fccbd528058c737d474c32646922298678995907a33149e28aa291","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-27T04:25:03.428929569Z"},{"index":5,"name":"A dismissed capture cannot be dismissed again","scenario_hash":"74612ae53e9e8344d4e79a8c95abfa72e2dccf655c9f67b6afd7a86ee2405afd","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-27T04:25:03.428929569Z"},{"index":6,"name":"A triaged capture cannot be triaged again","scenario_hash":"1cbcd7db42748056e2fe9a707f5c0c529c7f63899f3358f553291b8207ee7ba9","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-27T04:25:03.428929569Z"}]}
# acceptance-mutation-manifest-end

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
    And the pool screen lists nothing

  # dismiss-capture-keeps-the-row-03: neither way out of the inbox deletes the capture row
  Scenario: Neither way out of the inbox deletes the capture row
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When the capture is triaged as a pool task in life area "Home"
    And "asdfgh" is dismissed from the inbox
    And the inbox is viewed
    Then the inbox lists "<triaged>"
    And the inbox does not list "<dismissed>"
    And the capture row count is "<rows>"
    And the pool screen lists "<triaged>"

    Examples:
      | triaged  | dismissed | rows |
      | buy milk | asdfgh    | 2    |

  # dismiss-capture-no-dismissal-after-triage-04: a triaged capture cannot then be dismissed
  Scenario: A triaged capture cannot then be dismissed
    Given the capture is triaged as a pool task in life area "Home"
    When the capture is dismissed from the inbox
    Then the dismissal is rejected
    And the capture row count is "<rows>"
    And the pool screen lists "<pooled>"

    Examples:
      | rows | pooled |
      | 1    | asdfgh |

  # dismiss-capture-no-triage-after-dismissal-05: a dismissed capture cannot then be triaged, and the rejection says why
  Scenario: A dismissed capture cannot then be triaged
    Given the capture is dismissed from the inbox
    When the capture is triaged as a pool task in life area "Home"
    Then the triage is rejected
    And the rejection says the capture is no longer in the inbox
    And the capture row count is "<rows>"
    And the pool screen lists nothing

    Examples:
      | rows |
      | 1    |

  # dismiss-capture-no-second-dismissal-06: a dismissed capture cannot be dismissed again
  Scenario: A dismissed capture cannot be dismissed again
    Given the capture is dismissed from the inbox
    When the capture is dismissed from the inbox
    Then the dismissal is rejected
    And the capture row count is "<rows>"
    And the pool screen lists nothing

    Examples:
      | rows |
      | 1    |

  # dismiss-capture-no-second-triage-07: a triaged capture cannot be triaged again
  Scenario: A triaged capture cannot be triaged again
    Given the capture is triaged as a pool task in life area "Home"
    When the capture is triaged as a pool task in life area "Home"
    Then the triage is rejected
    And the capture row count is "<rows>"
    And the pool screen lists "<pooled>"

    Examples:
      | rows | pooled |
      | 1    | asdfgh |

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
