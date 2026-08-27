# mutation-stamp: sha256=a4a58355133f3be42ac1f4786bd93977ee60fcf757d24d6059a3ee53e8e7ff69
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-17T17:57:30.316579892Z","feature_name":"The inbox renders the untriaged capture queue","feature_path":"features/inbox_view.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:ca1f4e0113105ab1427f3ed3139a4d6497b8305c4f40de2e307c8a308225aa73","scenarios":[]}

# inbox-view-list-01: the capture page lists every untriaged capture, newest first
# inbox-view-quick-add-02: a capture submitted through the quick-add box appears without a full page redirect
# inbox-view-empty-state-03: a message instead of a blank list when there is nothing to triage
# inbox-view-triaged-row-stays-04: a triaged capture stays in Recent, reading what it became, and appears once
# inbox-view-three-most-recent-05: only the three most recently triaged stay
# inbox-view-untriaged-never-drop-06: untriaged captures never fall out of Recent
# inbox-view-escapes-hostile-text-07: hostile capture text is escaped on both row states
#
# -04 IS INVERTED, NOT EDITED. It read "a triaged capture does not appear in
# the inbox" -- true only while `Tasks` existed to show it somewhere else.
# #140 deletes that list (`D-four-screens`), so a vanishing row would leave
# the page with no feedback: you tap Pool and nothing says where it went. The
# canvas answers it (`Trellis.dc.html:84-92`) -- one row template, two states.
#
# THE BRIEF DID NOT LIST THIS FILE, and its counts are low. It found `task
# list` assertions by grep: there are 25, not 24, across 10 files, not 9 --
# and 5 MORE that grep could not see, `the inbox does not list ...`, which
# invert because a triaged row now stays. Named; the next agent will trust it.
#
# "APPEARS ONCE" REPLACES "THERE IS NO TASKS LIST". Asserting an absence is
# #90's trap. A duplicate row is what a surviving `Tasks` list actually looks
# like, and today `[quota]` really is duplicated.
#
# THREE IS THE OWNER'S NUMBER, settled 2026-08-27. The canvas caps the whole
# list at 14 by recency; the owner capped the TRIAGED half at 3, so untriaged
# work can never be pushed off by confirmations of work already done (-06).
# This buys no column -- whether a capture was triaged is already durable --
# so `T-ephemeral-view-state-rides-the-request` does not bite.
Feature: The capture page is a box and what is recent

  Background:
    Given the trellis server is running with an empty task list

  Scenario: The capture page lists every untriaged capture, newest first
    Given a capture with raw text "call the dentist" is waiting in the untriaged queue
    And a capture with raw text "buy milk" is waiting in the untriaged queue
    When the inbox is viewed
    Then the inbox lists "buy milk" before "call the dentist"

  # inbox-view-quick-add-02: a capture submitted through the quick-add box appears without a full page redirect
  Scenario: A capture submitted through the quick-add box appears without a full page redirect
    When the quick-add box submits a capture with raw text "buy milk"
    Then the quick-add submission does not redirect the browser
    And the quick-add response includes "buy milk"
    When the inbox is viewed
    Then the inbox lists "buy milk"

  # inbox-view-empty-state-03: a message instead of a blank list when there is nothing to triage
  Scenario: The capture page shows a message instead of a blank list when there is nothing to triage
    When the inbox is viewed
    Then the inbox lists no captures
    And the inbox shows an empty-state message

  # inbox-view-triaged-row-stays-04: a triaged capture stays in Recent, reading what it became, and appears once
  Scenario: A triaged capture stays in Recent, reading what it became
    Given a capture with raw text "buy screws" is waiting in the untriaged queue
    When the capture is triaged as a "<kind>" task tagged "<tag>"
    And the inbox is viewed
    Then the inbox lists "buy screws"
    And the row for "buy screws" reads "<meta>"
    And the row for "buy screws" offers no kind buttons
    And the capture page shows "buy screws" once

    Examples:
      | kind      | tag        | meta                  |
      | pool      | @homedepot | Pool · @homedepot     |
      | pool      |            | Pool · no context     |
      | committed | @desk      | Committed · @desk     |

  # inbox-view-three-most-recent-05: only the three most recently triaged stay
  Scenario: Only the three most recently triaged stay
    Given the captures "<captured>" have each been triaged as pool tasks in that order
    When the inbox is viewed
    Then the inbox lists "<listed>"
    And the inbox does not list "<dropped>"

    Examples:
      | captured               | listed           | dropped |
      | one, two, three, four  | four, three, two | one     |

  # inbox-view-untriaged-never-drop-06: untriaged captures never fall out of Recent
  Scenario: Untriaged captures never fall out of Recent
    Given the captures "<captured>" have each been triaged as pool tasks in that order
    And a capture with raw text "<waiting>" is waiting in the untriaged queue
    When the inbox is viewed
    Then the inbox lists "<listed>"

    Examples:
      | captured              | waiting       | listed                          |
      | one, two, three, four | still waiting | still waiting, four, three, two |

  # inbox-view-escapes-hostile-text-07: hostile capture text is escaped on both row states
  Scenario: Hostile capture text is escaped, not interpreted, on both row states
    Given a capture with raw text "<script>alert('boom')</script>" is waiting in the untriaged queue
    And the row for it is "<state>"
    When the inbox is viewed
    Then the inbox does not contain an unescaped "<script>" tag
    And the inbox contains the word "boom"

    Examples:
      | state     |
      | untriaged |
      | triaged   |
