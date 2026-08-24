# disclosures-three-buttons-01: an untriaged row offers three kind buttons, and no fields until one is chosen
# disclosures-pool-files-immediately-02: pool files on one tap and shows no fields at all
# disclosures-one-panel-at-a-time-03: choosing a kind shows that kind's fields and no other kind's
# disclosures-switching-replaces-04: choosing a second kind replaces the first kind's fields
# disclosures-rows-are-independent-05: choosing on one capture leaves every other capture alone
# disclosures-submissions-unchanged-06: every kind still submits exactly what it submitted before
#
# THE CANVAS SPEAKS HERE AND THE IMPLEMENTATION DID NOT MATCH IT. Checked
# rather than assumed, per T-canvas-is-authoritative-where-it-speaks: the
# design contains ZERO <details> elements. Its untriaged row (lines 78-94)
# draws three 44px kind buttons with the chosen one filled in
# primary-800, and a bordered gray-50 panel below (line 111) for the kind
# that needs fields. The implementation grew four independent <details>, all
# of which could be open at once.
#
# WHERE THIS SLICE FOLLOWS THE CANVAS AND WHERE IT DOES NOT, decided by the
# owner and recorded because the difference is behaviour, not taste.
#
# The canvas's own script (line 630) FILES A COMMITTED TASK ON ONE TAP,
# undated -- `at: "Unset"`, `ord: 99` -- to be dated afterwards. That is a
# better phone interaction and it is not what this slice builds: committed
# triage requires a deadline, a commitment, a priority and an estimate, which
# is BEHAVIOUR, and D-four-screens gives behaviour to the decisions log.
# Four acceptance features assert those requirements. One-tap-and-date-later
# remains a real product option and is the owner's to take deliberately, not
# something to arrive through a layout fix.
#
# SO THE PANEL FOR COMMITTED IS INVENTED. The canvas draws a panel for quota
# only; there is no committed panel anywhere in it. This is the FIFTH gap the
# canvas has left -- after the done control, the date input, a settings
# surface and an app icon -- and the largest, because the others were single
# controls and this is a whole region. Flagged, not filled silently. It
# borrows the quota panel's own drawing so the invention is as small as it
# can be.
#
# THE TRAP THE BRIEF NAMED SURVIVES THE CHANGE OF MECHANISM. lists.html
# renders one row per capture, so whatever makes a row's choice exclusive
# must be scoped to that row. 05 is that scenario and it NEEDS TWO CAPTURES:
# a single-capture fixture cannot fail this way, which is
# T-a-check-must-be-seen-to-fail in its second shape -- a check that cannot
# fail looks like coverage.
#
# WHAT MUST NOT CHANGE, and 06 pins it. Each kind submits its own form with
# its own hidden kind and its own fields. Two panels never merge their
# inputs today because two forms never merge; if this slice consolidates
# them while tidying, a cosmetic defect becomes a data defect. Pool also
# stays the cheapest path (D-pool-is-default) -- one tap, no fields, and it
# must not gain a panel to make the layout symmetrical.
Feature: A capture row offers three kinds, and shows one set of fields

  Background:
    Given the trellis server is running with an empty task list

  Scenario: An untriaged row offers three kind buttons, and no fields until one is chosen
    Given a capture with raw text "buy screws" is waiting in the untriaged queue
    When the inbox is viewed
    Then the row for "buy screws" offers the kind buttons "<buttons>"
    And the row for "buy screws" shows fields for no kind

    Examples:
      | buttons                |
      | Pool, Committed, Quota |

  # disclosures-pool-files-immediately-02: pool files on one tap and shows no fields at all
  Scenario: Pool files on one tap and shows no fields at all
    Given a capture with raw text "buy screws" is waiting in the untriaged queue
    When the kind "Pool" is chosen for "buy screws"
    And the inbox is viewed
    Then the task list shows "buy screws"
    And the inbox does not list "buy screws"

  # disclosures-one-panel-at-a-time-03: choosing a kind shows that kind's fields and no other kind's
  Scenario: Choosing a kind shows that kind's fields and no other kind's
    Given a capture with raw text "buy screws" is waiting in the untriaged queue
    When the kind "<chosen>" is chosen for "buy screws"
    Then the row for "buy screws" shows fields for "<chosen>"
    And the row for "buy screws" shows fields for no other kind
    And the row for "buy screws" marks "<chosen>" as the chosen kind

    Examples:
      | chosen    |
      | Committed |
      | Quota     |

  # disclosures-switching-replaces-04: choosing a second kind replaces the first kind's fields
  Scenario: Choosing a second kind replaces the first kind's fields
    Given a capture with raw text "buy screws" is waiting in the untriaged queue
    And the kind "Committed" is chosen for "buy screws"
    When the kind "Quota" is chosen for "buy screws"
    Then the row for "buy screws" shows fields for "<chosen>"
    And the row for "buy screws" shows fields for no other kind

    Examples:
      | chosen |
      | Quota  |

  # disclosures-rows-are-independent-05: choosing on one capture leaves every other capture alone
  Scenario: Choosing on one capture leaves every other capture alone
    Given a capture with raw text "buy screws" is waiting in the untriaged queue
    And a capture with raw text "call the dentist" is waiting in the untriaged queue
    And the kind "Committed" is chosen for "buy screws"
    When the kind "Quota" is chosen for "call the dentist"
    Then the row for "buy screws" shows fields for "<chosen>"
    And the row for "call the dentist" shows fields for "<other>"

    Examples:
      | chosen    | other |
      | Committed | Quota |

  # disclosures-submissions-unchanged-06: every kind still submits exactly what it submitted before
  Scenario: Every kind still submits exactly what it submitted before
    Given a capture with raw text "call the dentist" is waiting in the untriaged queue
    When the kind "Committed" is chosen for "call the dentist"
    And the capture is triaged as a committed task through the page with "<missing_field>" omitted
    Then the triage is rejected
    And the rejection names "<missing_field>"
    And the task list is still empty

    Examples:
      | missing_field     |
      | deadline          |
      | commitment        |
      | priority          |
      | estimated_minutes |
