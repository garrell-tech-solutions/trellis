# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-27T04:28:47.062033641Z","feature_name":"Triage creates tasks in one of three kinds","feature_path":"features/task_kinds.feature","background_hash":"0c56ef91538254d551a330ee3bf0b84ef91c3138861767ae4fe24fedb5500548","implementation_hash":"sha256:f42eb6e18ae094316ca129abf7901128ee1ad1c94067533923abb7ac41c9b218","scenarios":[{"index":1,"name":"Triaging a capture as a committed task records its scheduling metadata","scenario_hash":"e00de524c26aff7a394439f007ef78d95dcbd1e4d088f2bf213df39ed7ad610c","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-26T17:14:14.771510908Z"}]}
# acceptance-mutation-manifest-end

# task-kinds-pool-01: triaging a capture as pool creates a pool task with no deadline and no quota
# task-kinds-committed-02: triaging a capture as committed records deadline, commitment and priority
# task-kinds-quota-03: triaging a capture as quota creates a quota carrying that weekly target
#
# STILL THREE KINDS. `T-three-task-kinds` survives this slice intact, and the
# brief expected it not to. What changed is what the quota variant CARRIES:
# `target_count`, `target_minutes_each` and `period` are retired and a single
# WEEKLY HOUR TARGET replaces them (`quota_triage_validation.feature` holds
# the reasoning and the owner's words). A quota is still a task -- the
# owner's own framing -- so `kind` stays a three-variant sum type and
# `unknown_kind_rejection.feature` needs no edit at all.
#
# "NO QUOTA TARGET" BECAME "NO QUOTA", and that is the difference between an
# assertion and a decoration. -01 and -02 used to assert that a pool or
# committed task left `target_count` and friends null. THOSE COLUMNS STILL
# EXIST -- `T-migrations-append-only` means they cannot be dropped -- but
# NOTHING WRITES THEM ANY MORE, so the old assertion would pass forever
# against any implementation whatsoever. That is #90's trap arriving by the
# back door: a step that cannot fail reads exactly like coverage. What -01
# and -02 assert now is that triaging pool or committed PUT NOTHING ON THE
# QUOTA SCREEN, which is reachable, mutable and true for a reason.
#
# -03 ASSERTS THROUGH THE SCREEN RATHER THAN THROUGH COLUMNS for the same
# reason and one more: where a quota is stored is the architect's to settle,
# and a scenario that names `quotas` or `tasks` would decide it here by
# accident. What the owner can see is that the thing they triaged is on the
# quota screen reading its target. `0.5` IS THE ROW THAT EARNS ITS KEEP --
# the canvas's hours input is `step="0.5"`, so half an hour is a value the
# product can really submit, and it is the only row where the hours-to-
# minutes conversion is visible in the readout rather than implied by it.
Feature: Triage creates tasks in one of three kinds

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "buy milk" is waiting in the untriaged queue

  Scenario: Triaging a capture as a pool task creates a pool task
    When the capture is triaged as a pool task
    Then the resulting task has kind "pool"
    And the resulting task has no deadline
    And the quota screen offers no quotas

  # task-kinds-committed-02: triaging a capture as committed records deadline, commitment and priority
  Scenario: Triaging a capture as a committed task records its scheduling metadata
    When the capture is triaged as a committed task with a "<commitment>" deadline of "<deadline>" and priority "<priority>"
    Then the resulting task has kind "committed"
    And the resulting task has a "<commitment>" deadline of "<deadline>"
    And the resulting task has priority "<priority>"
    And the quota screen offers no quotas

    Examples:
      | deadline             | commitment | priority |
      | 2026-08-20T17:00:00Z | at         | P1       |
      | 2026-08-31T09:00:00Z | by         | P3       |

  # task-kinds-quota-03: triaging a capture as quota creates a quota carrying that weekly target
  Scenario: Triaging a capture as a quota creates a quota carrying that weekly target
    When the capture is triaged as a quota named "<name>" with a target of "<hours>" hours a week
    And the "quota" screen is viewed
    Then the quota screen offers the quotas "<name>"
    And the quota "<name>" reads "<readout>"
    And the quota screen reports "1 quota" beside its title
    And the resulting task has no deadline

    Examples:
      | name    | hours | readout  |
      | Piano   | 4     | 0m / 4h  |
      | Running | 0.5   | 0m / 30m |
