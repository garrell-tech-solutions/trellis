# mutation-stamp: sha256=82bb96a579d99fa7dae1c2caefc5c5fd335952a4453188e433f36df9bb659dd5
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-21T21:20:24.853921346Z","feature_name":"Triage happens on the page, using the same validation as the API","feature_path":"features/triage_from_page.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:b6d2b9276eb4e11dbc0f53d54b6b417868310eea7acb7947131810fe9df8b364","scenarios":[{"index":7,"name":"The pool form asks for fewer inputs than committed or quota","scenario_hash":"575a2c45bf988f226723ea869edfe1dc2702d74c6c2d60e8d4a32284dd3d40cc","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-17T17:57:47.600328486Z"}]}
# acceptance-mutation-manifest-end

# triage-from-page-offers-all-kinds-01: each untriaged capture offers all three triage kinds from the page
# triage-from-page-pool-02: triaging as pool through the page moves the capture into the task list without a full page reload
# triage-from-page-committed-rejected-03: triaging as committed through the page with a required field omitted is rejected the same way as the API
# triage-from-page-committed-closed-choices-04: the committed form offers commitment and priority as fixed choices, not free text
# triage-from-page-quota-rejected-05: triaging as quota through the page with a required field omitted is rejected the same way as the API
# triage-from-page-escapes-hostile-text-06: hostile capture text stays escaped in the task list
# triage-from-page-pool-needs-no-form-07: pool triage is submitted straight from the row, with nothing to open first
# triage-from-page-pool-is-cheapest-08: the pool form asks for fewer inputs than committed or quota
#
# 07 and 08 are #9's AC-2 as amended 2026-08-17: pool is the cheapest path,
# stated as the relation it always stood for rather than a literal input
# count. Its other half is triage-from-page-pool-02 above, which pins that
# pool triage moves the capture without a full reload.
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

  # triage-from-page-committed-closed-choices-04: the committed form offers commitment and priority as fixed choices, not free text
  Scenario: The committed form offers commitment and priority as fixed choices, not free text
    Given a capture with raw text "call the dentist" is waiting in the untriaged queue
    When the inbox is viewed
    Then the committed form offers exactly the commitment choices "at" and "by"
    And the committed form offers exactly the priority choices "P1", "P2", "P3" and "P4"

  # triage-from-page-quota-rejected-05: triaging as quota through the page with a required field omitted is rejected the same way as the API
  Scenario: Triaging as quota through the page without a required field is rejected the same way as the API
    Given a capture with raw text "go to the gym" is waiting in the untriaged queue
    When the capture is triaged as a quota task through the page with "hours" omitted
    Then the triage is rejected
    And the rejection names "hours"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

  # triage-from-page-escapes-hostile-text-06: hostile capture text stays escaped in the task list
  Scenario: Hostile capture text stays escaped in the task list
    Given a capture with raw text "<script>alert('boom')</script>" is waiting in the untriaged queue
    When the capture is triaged as a pool task through the page
    And the inbox is viewed
    Then the task list does not contain an unescaped "<script>" tag
    And the task list contains the word "boom"

  # triage-from-page-pool-needs-no-form-07: pool triage is submitted straight from the row, with nothing to open first
  Scenario: Pool triage is submitted straight from the row, with nothing to open first
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When the inbox is viewed
    Then the pool triage form is not behind a control that must be opened first

  # triage-from-page-pool-is-cheapest-08: the pool form asks for fewer inputs than committed or quota
  Scenario: The pool form asks for fewer inputs than committed or quota
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When the inbox is viewed
    Then the pool triage form asks for fewer inputs than the <kind> form

    Examples:
      | kind      |
      | committed |
      | quota     |
