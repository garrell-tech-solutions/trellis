# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-27T04:29:00.078079685Z","feature_name":"Triage rejects a kind outside pool, committed and quota","feature_path":"features/unknown_kind_rejection.feature","background_hash":"0c56ef91538254d551a330ee3bf0b84ef91c3138861767ae4fe24fedb5500548","implementation_hash":"sha256:4b4809c877479bced618ae3873df3499e07b73b46333a4302b6a79552fc0d349","scenarios":[]}
# acceptance-mutation-manifest-end

# unknown-kind-rejection-named-01: triaging with a kind outside pool, committed and quota is rejected and creates nothing
# unknown-kind-rejection-absent-02: triaging without naming a kind is rejected and creates nothing
# unknown-kind-rejection-wrong-type-03: a kind submitted as the wrong JSON type is rejected and echoes exactly what was submitted
Feature: Triage rejects a kind outside pool, committed and quota

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "buy milk" is waiting in the untriaged queue

  Scenario: Triaging with an unrecognised kind is rejected and creates nothing
    When the capture is triaged as kind "<bad_kind>"
    Then the triage is rejected
    And the rejection reports an unknown kind
    And the pool screen lists nothing
    And the committed screen lists nothing
    And the capture is still waiting in the untriaged queue

    Examples:
      | bad_kind |
      | someday  |
      | later    |

  # unknown-kind-rejection-absent-02: triaging without naming a kind is rejected and creates nothing
  Scenario: Triaging without naming a kind is rejected and creates nothing
    When the capture is triaged with no kind named
    Then the triage is rejected
    And the rejection reports an unknown kind
    And the pool screen lists nothing
    And the committed screen lists nothing
    And the capture is still waiting in the untriaged queue

  # unknown-kind-rejection-wrong-type-03: a kind submitted as the wrong JSON type is rejected and echoes exactly what was submitted
  Scenario: A kind submitted as the wrong JSON type is rejected and echoes exactly what was submitted
    When the capture is triaged with kind submitted as the number 7
    Then the triage is rejected
    And the rejection reports the unknown kind as the number 7
    And the pool screen lists nothing
    And the committed screen lists nothing
    And the capture is still waiting in the untriaged queue
