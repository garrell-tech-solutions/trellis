# triage-from-page-offers-all-kinds-01: each untriaged capture offers all three triage kinds from the page
# triage-from-page-pool-02: triaging as pool through the page moves the capture into the task list without a full page reload
# triage-from-page-committed-rejected-03: triaging as committed through the page with a required field omitted is rejected the same way as the API
# triage-from-page-committed-closed-choices-04: the committed form offers deadline type and priority as fixed choices, not free text
# triage-from-page-quota-rejected-05: triaging as quota through the page with a required field omitted is rejected the same way as the API
# triage-from-page-escapes-hostile-text-06: hostile capture text stays escaped in the task list
Feature: Triage happens on the page, using the same validation as the API

  Background:
    Given the trellis server is running with an empty task list

  Scenario: Each untriaged capture offers all three triage kinds from the page
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When the inbox is viewed
    Then the inbox offers to triage "buy milk" as pool, committed and quota

  # triage-from-page-pool-02: triaging as pool through the page moves the capture into the task list without a full page reload
  Scenario: Triaging as pool through the page moves the capture into the task list without a full page reload
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When the capture is triaged as a pool task through the page
    Then the page's triage response does not redirect the browser
    When the inbox is viewed
    Then the inbox does not list "buy milk"
    And the task list shows "buy milk"

  # triage-from-page-committed-rejected-03: triaging as committed through the page with a required field omitted is rejected the same way as the API
  Scenario: Triaging as committed through the page without a required field is rejected the same way as the API
    Given a capture with raw text "call the dentist" is waiting in the untriaged queue
    When the capture is triaged as a committed task through the page with "deadline" omitted
    Then the triage is rejected
    And the rejection names "deadline"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

  # triage-from-page-committed-closed-choices-04: the committed form offers deadline type and priority as fixed choices, not free text
  Scenario: The committed form offers deadline type and priority as fixed choices, not free text
    Given a capture with raw text "call the dentist" is waiting in the untriaged queue
    When the inbox is viewed
    Then the committed form offers exactly the deadline type choices "hard" and "soft"
    And the committed form offers exactly the priority choices "P1", "P2", "P3" and "P4"

  # triage-from-page-quota-rejected-05: triaging as quota through the page with a required field omitted is rejected the same way as the API
  Scenario: Triaging as quota through the page without a required field is rejected the same way as the API
    Given a capture with raw text "go to the gym" is waiting in the untriaged queue
    When the capture is triaged as a quota task through the page with "target_count" omitted
    Then the triage is rejected
    And the rejection names "target_count"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

  # triage-from-page-escapes-hostile-text-06: hostile capture text stays escaped in the task list
  Scenario: Hostile capture text stays escaped in the task list
    Given a capture with raw text "<script>alert('boom')</script>" is waiting in the untriaged queue
    When the capture is triaged as a pool task through the page
    And the inbox is viewed
    Then the task list does not contain an unescaped "<script>" tag
    And the task list contains the word "boom"
