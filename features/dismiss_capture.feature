# mutation-stamp: sha256=a74bbe3ebb88fdcefdf29fd26ef2287c015f6b19c1d903f8462125694f359272
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-17T17:57:30.288312157Z","feature_name":"A capture can be dismissed, and its row is kept","feature_path":"features/dismiss_capture.feature","background_hash":"01db5b6232c439c5b108b5e869cd6178d5cdba30bf901eed9dc0738685f7b18e","implementation_hash":"sha256:3940663b77d50b9408daf9ceb23258c83423f56a888edd7da07dccd68155699a","scenarios":[{"index":2,"name":"Neither way out of the inbox deletes the capture row","scenario_hash":"a4e387170dca3db0a458e28c37b2ccee5cff4027e1cc50c5d356dad1639e425f","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-17T17:57:30.288312157Z"},{"index":3,"name":"A triaged capture cannot then be dismissed","scenario_hash":"abb7dfb5a3661b22c31b4bd61e9d72dacca9cdcded22cc44b1c609fe48fe9034","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-17T17:57:30.288312157Z"},{"index":4,"name":"A dismissed capture cannot then be triaged","scenario_hash":"0ac1b40076d821730ed397ff4f16589d7bd865a2fcb3b8e51e8e662950bc5492","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-17T17:57:30.288312157Z"},{"index":5,"name":"A dismissed capture cannot be dismissed again","scenario_hash":"ce067455f3cc62b47f56e7324227bd2ec984e08f7c22e244f10bb067d26ef267","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-17T17:57:30.288312157Z"},{"index":6,"name":"A triaged capture cannot be triaged again","scenario_hash":"098e716035dcc9e882d23ffff7f15e7937bb69ed62ac7963d19f420a63bc6dc1","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-17T17:57:30.288312157Z"}]}
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
