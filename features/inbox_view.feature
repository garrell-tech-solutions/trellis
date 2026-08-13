# mutation-stamp: sha256=a4a58355133f3be42ac1f4786bd93977ee60fcf757d24d6059a3ee53e8e7ff69
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-13T18:47:49.055007414Z","feature_name":"The inbox renders the untriaged capture queue","feature_path":"features/inbox_view.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:ca1f4e0113105ab1427f3ed3139a4d6497b8305c4f40de2e307c8a308225aa73","scenarios":[]}
# acceptance-mutation-manifest-end

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
