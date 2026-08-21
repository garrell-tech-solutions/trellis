# mutation-stamp: sha256=d71b55831274c760bf12ccf6207b724a4750b2c7ab27abdf2a7f9548d0c7414b
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-21T21:19:40.008367797Z","feature_name":"Committed triage requires deadline, commitment, priority and an estimate","feature_path":"features/committed_triage_validation.feature","background_hash":"213f9062faacbf3d0a2218fdb859b4c843bbee4724073795573a29ea693f9d44","implementation_hash":"sha256:2304bc384951c7673fdba860ad848ea42c847d8e7ccad566f5c2e9bd942c4736","scenarios":[{"index":0,"name":"Triaging as committed without a required field is rejected and creates nothing","scenario_hash":"fc08ec62615d29f25fe6a5900213c9d3f34d0af5a18c2896a29a012958b64c90","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-21T21:19:40.008367797Z"},{"index":1,"name":"Triaging as committed with a required field left empty is rejected the same way as omitting it","scenario_hash":"0c16c29e990ff71b1f71a15d89212d963639bed377f1e6caf51c0466f400e7ae","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-21T21:19:40.008367797Z"}]}
# acceptance-mutation-manifest-end

# committed-triage-validation-missing-field-01: committed triage is rejected when a required field is absent
# committed-triage-validation-empty-field-02: committed triage is rejected when a required field is left empty, the same way as when it is absent
#
# `commitment` (at | by) REPLACED `deadline_type` (hard | soft) in the
# required set in #94. D-committed-is-at-or-by makes a committed item one or
# the other, and deadline_type's only behaviour -- T-hard-refuses-soft-slips'
# -- lives in the scheduler D-dogfood-first paused, so it changed nothing
# while still costing a field on a form meant to be fast. When the scheduler
# returns, an `at` is hard and a `by` is soft, derived rather than typed
# twice. THE COLUMN STAYS IN THE SCHEMA, UNREAD, on #88's ground: dropping it
# throws away what the owner already typed, and unlike scheduler_core::ratio
# a hard/soft judgement is not recomputable from anything that remains.
#
# `estimated_minutes` joined the required set in #62: capacity cannot report
# what a fortnight needs if committed tasks carry no minutes, and the two
# alternatives were closed by precedent -- counting only quota demand makes
# the number wrong in the direction of "you have more time than you do", and
# defaulting the estimate is a silent wrong default of the kind
# D-manual-triage-until-llm and T-timezone-is-a-setting each refused. Only
# committed needs it: pool is never placed, and quota already carries
# target_minutes_each.
Feature: Committed triage requires deadline, commitment, priority and an estimate

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "call the dentist" is waiting in the untriaged queue

  Scenario: Triaging as committed without a required field is rejected and creates nothing
    When the capture is triaged as a committed task with "<missing_field>" omitted
    Then the triage is rejected
    And the rejection names "<missing_field>"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | missing_field     |
      | deadline          |
      | commitment        |
      | priority          |
      | estimated_minutes |

  # committed-triage-validation-empty-field-02: committed triage is rejected when a required field is left empty, the same way as when it is absent
  Scenario: Triaging as committed with a required field left empty is rejected the same way as omitting it
    When the capture is triaged as a committed task with "<empty_field>" left empty
    Then the triage is rejected
    And the rejection names "<empty_field>"
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

    Examples:
      | empty_field       |
      | deadline          |
      | commitment        |
      | priority          |
      | estimated_minutes |
