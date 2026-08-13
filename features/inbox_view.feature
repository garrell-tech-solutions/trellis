# inbox-view-list-01: the inbox lists every untriaged capture, newest first
# inbox-view-quick-add-02: a capture submitted through the quick-add box appears in the inbox without a full page redirect
# inbox-view-empty-state-03: the inbox shows a message instead of a blank list when there is nothing to triage
# inbox-view-excludes-triaged-04: a capture that has been triaged does not appear in the inbox
# inbox-view-escapes-hostile-text-05: hostile capture text is escaped, not interpreted, when rendered
Feature: The inbox renders the untriaged capture queue

  Background:
    Given the trellis server is running with an empty task list

  Scenario: The inbox lists every untriaged capture, newest first
    Given a capture with raw text "call the dentist" is waiting in the untriaged queue
    And a capture with raw text "buy milk" is waiting in the untriaged queue
    When the inbox is viewed
    Then the inbox lists "buy milk" before "call the dentist"

  # inbox-view-quick-add-02: a capture submitted through the quick-add box appears in the inbox without a full page redirect
  Scenario: A capture submitted through the quick-add box appears in the inbox without a full page redirect
    When the quick-add box submits a capture with raw text "buy milk"
    Then the quick-add submission does not redirect the browser
    And the quick-add response includes "buy milk"
    When the inbox is viewed
    Then the inbox lists "buy milk"

  # inbox-view-empty-state-03: the inbox shows a message instead of a blank list when there is nothing to triage
  Scenario: The inbox shows a message instead of a blank list when there is nothing to triage
    When the inbox is viewed
    Then the inbox lists no captures
    And the inbox shows an empty-state message

  # inbox-view-excludes-triaged-04: a capture that has been triaged does not appear in the inbox
  Scenario: A capture that has been triaged does not appear in the inbox
    Given a capture with raw text "call the dentist" is waiting in the untriaged queue
    And the capture is triaged as a pool task
    When the inbox is viewed
    Then the inbox lists no captures

  # inbox-view-escapes-hostile-text-05: hostile capture text is escaped, not interpreted, when rendered
  Scenario: Hostile capture text is escaped, not interpreted, when rendered
    Given a capture with raw text "<script>alert('boom')</script>" is waiting in the untriaged queue
    When the inbox is viewed
    Then the inbox does not contain an unescaped "<script>" tag
    And the inbox contains the word "boom"
