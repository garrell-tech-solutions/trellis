# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-19T17:45:11.666641671Z","feature_name":"The schedule places committed work into free time, and says what did not fit","feature_path":"features/schedule.feature","background_hash":"fc646056c403d34a6add9f3a42769199a6f816aa8f487fe4018296d7d3bb9cdc","implementation_hash":"sha256:548ccf214449bfec4a1447320ef704ca08a01045777f550b3bae93c757df0d0e","scenarios":[{"index":0,"name":"A committed task is placed inside its own life area's hours","scenario_hash":"2aeb2acc000cc15ee3a266a96d2d5d6b8a2ec511d136c388c4c7fad44276ff7d","mutation_count":5,"result":{"Total":5,"Killed":5,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:21:03.642810213Z"},{"index":1,"name":"The tighter deadline goes first, and a soft deadline may be overrun","scenario_hash":"17cd817240152e463c5d9eba03fc9ee136bd3c3cd9dad8e988968bc876340a38","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:21:03.642810213Z"},{"index":2,"name":"A task longer than any single free interval is unplaceable, never split","scenario_hash":"21ed5ac889ab69947264718b3f121b1c80a025f34d9b558e562165b4da06e5c8","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:21:03.642810213Z"},{"index":4,"name":"When the hours run out, the tasks that did not fit say so","scenario_hash":"77dfd580e44dbaa71db628d83b8985d5035f492fae7f0f2610a45beed77f3670","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:21:03.642810213Z"},{"index":5,"name":"A life area with no hours places nothing and says why","scenario_hash":"ecc3899ad32153e520902b02c07ed23bf754fb1cf4034a2bd917f28114c85d38","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:21:03.642810213Z"},{"index":6,"name":"Pool and quota work is neither placed nor reported as unplaceable","scenario_hash":"bd3ebfe4737a16a060e75c109bfcad4ca92abfc8cee952519068f1f8f1897b09","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:21:03.642810213Z"},{"index":7,"name":"The plan does not change until it is regenerated","scenario_hash":"f569f7a0fb4ff0cf6bd5ec6db14de153975ba4ef4571c96f145eea96568386a7","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:21:03.642810213Z"},{"index":8,"name":"With nothing to schedule the page says so rather than showing an empty box","scenario_hash":"c94db166d338e422375e6ffcf97a14ae4dc90c8ae9ff95c03ceec1286d0ad446","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:21:03.642810213Z"}]}
# acceptance-mutation-manifest-end

# schedule-places-committed-work-01: a committed task is placed inside its own life area's hours
# schedule-least-slack-first-02: the tighter deadline goes first, and a soft deadline may be overrun
# schedule-whole-or-not-at-all-03: a task longer than any single free interval is unplaceable, never split
# schedule-hard-deadline-unreachable-04: a hard deadline that cannot be met makes the task unplaceable
# schedule-capacity-exceeded-05: when the hours run out, the tasks that did not fit say so
# schedule-no-window-06: a life area with no hours places nothing and says why
# schedule-only-committed-07: pool and quota work is neither placed nor reported as unplaceable
# schedule-plan-is-stable-08: the plan does not change until it is regenerated
# schedule-empty-state-09: with nothing to schedule the page says so rather than showing an empty box
# schedule-escapes-hostile-text-10: hostile task text stays escaped on the schedule page
#
# Every scenario pins the clock. A schedule is meaningless without a fixed
# today, and "now" is one of schedule()'s seven parameters. Today is Monday
# 2026-08-17 09:00 UTC, which is exactly when Work's Mon-Fri 09:00-17:00
# guardrail opens -- so the first free interval starts at "now" and every
# expected instant below is arithmetic rather than an accident of the day the
# suite runs.
#
# THE REASON ENUM IS CLOSED AND ITS PRECEDENCE IS SPECIFIED. `no_window`,
# `capacity_exceeded`, `deadline_unreachable` and `chunk_policy_unsatisfiable`
# are the four, and more than one can be true of the same task, so the order
# they are tried is part of the contract rather than an implementation
# detail -- otherwise this feature would be asserting an outcome several
# causes collapse into, which is the #69 trap:
#
#   1. no_window                   the life area has no hours in the horizon
#                                  at all -- never scheduled, no guardrail, or
#                                  every hour removed by an exception
#   2. deadline_unreachable        a HARD deadline falls before the task could
#                                  finish even placed first
#   3. capacity_exceeded           not enough free time remains after the
#                                  tasks ranked above it
#   4. chunk_policy_unsatisfiable  time remains, but no single free interval
#                                  is long enough to hold it whole
#
# 4 is what D-placed-whole-or-not-at-all produces at this slice: the chunk
# policy here is "one whole chunk", and a task that cannot meet it is
# unplaceable rather than half-booked. When S3 adds splitting the same task
# may become placeable; the reason is honest about the capability, not about
# the task.
#
# ORDERING IS TOTAL, and that matters before S5 rather than at it. Least
# slack first -- deadline minus now minus estimate -- then priority, then
# task id ascending. The last tiebreak looks like pedantry and is not: S5's
# regeneration property demands byte-identical placements from identical
# inputs, and two tasks with equal slack and equal priority would otherwise
# be ordered by whatever the query returned. A committed task always has a
# deadline (required at triage since #29), so "slack of a task with no
# deadline" is a state that cannot occur and gets no rule, exactly as
# T-capacity-never-under-reports-demand refused to grow the enum for an
# estimate that cannot be missing.
#
# TWO STEP SHAPES FOR A COMMITTED TASK, deliberately. The short form implies
# a soft deadline at P2, which is the uninteresting case; the long form is
# used only in 01 and 04, where deadline_type or priority is the thing under
# test. And scenario 08 needs two counts of the same step in one scenario --
# that the plan did not change, then that it did -- so "<placed>" carries the
# primary value throughout the file and "<after>" is the single deliberate
# second name.
#
# WHAT IS NOT HERE, and belongs to the proptests this slice owes -- invariants
# 1, 2, 4 and 5 over >= 1000 cases (3 is vacuous until S3 splits anything).
# #73's lesson is the one that matters most here, because this slice is
# mostly proptests: A PROPERTY TEST IS ONLY AS STRONG AS THE INPUTS ITS
# GENERATOR CAN PRODUCE. That sortedness property could not fail, over 1000
# cases, because every generated band had a distinct weekday. So the
# generator for these must be able to produce the failures:
#   - invariant 1 (no two blocks overlap) needs more tasks than fit
#     comfortably, and several life areas whose guardrails overlap in clock
#     time -- D-life-area-owns-its-time makes that legal and it is exactly
#     where two blocks could collide
#   - invariant 2 (a block lies within one free interval) needs guardrails
#     with more than one band on a day, and tasks whose estimates approach an
#     interval's length, or nothing ever lands near a seam
#   - invariant 4 (hard deadlines hold) needs hard deadlines that fall inside
#     the horizon, and a mix of hard and soft, or it passes vacuously
#   - invariant 5 (the partition is total) needs task sets large enough to
#     exhaust the hours, or nothing is ever unplaceable
# Each should be confirmed to FAIL with the relevant rule removed, the way
# the sortedness property was only trusted once deleting the sort broke it.
Feature: The schedule places committed work into free time, and says what did not fit

  Background:
    Given the server believes it is "2026-08-17T09:00:00Z"
    And the trellis server is running with an empty task list

  Scenario: A committed task is placed inside its own life area's hours
    Given the life area "Work" is saved with a guardrail band on "Mon, Tue, Wed, Thu, Fri" from "09:00" to "17:00"
    And a committed task "write the Q3 deck" in life area "Work" estimated "120" minutes due "<deadline>" as "<deadline_type>" with priority "P2"
    When the schedule is generated
    And the schedule page is viewed
    Then the schedule places "write the Q3 deck" starting "<start>" and ending "<end>"
    And the schedule reports "<placed>" placed blocks

    Examples:
      | deadline             | deadline_type | start                | end                  | placed |
      | 2026-08-21T17:00:00Z | hard          | 2026-08-17T09:00:00Z | 2026-08-17T11:00:00Z | 1      |

  # schedule-least-slack-first-02: the tighter deadline goes first, and a soft deadline may be overrun
  Scenario: The tighter deadline goes first, and a soft deadline may be overrun
    Given the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "11:00"
    And a committed task "renew the passport" in life area "Work" needs "120" minutes by "2026-08-21T17:00:00Z"
    And a committed task "file the return" in life area "Work" needs "120" minutes by "2026-08-17T11:00:00Z"
    When the schedule is generated
    And the schedule page is viewed
    Then the schedule places "file the return" starting "<first_start>" and ending "<first_end>"
    And the schedule places "renew the passport" starting "<second_start>" and ending "<second_end>"
    And the schedule reports "renew the passport" as finishing after its deadline

    Examples:
      | first_start          | first_end            | second_start         | second_end           |
      | 2026-08-17T09:00:00Z | 2026-08-17T11:00:00Z | 2026-08-24T09:00:00Z | 2026-08-24T11:00:00Z |

  # schedule-whole-or-not-at-all-03: a task longer than any single free interval is unplaceable, never split
  Scenario: A task longer than any single free interval is unplaceable, never split
    Given the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "11:00"
    And a committed task "rebuild the deck" in life area "Work" needs "180" minutes by "2026-08-28T17:00:00Z"
    When the schedule is generated
    And the schedule page is viewed
    Then the schedule reports "<placed>" placed blocks
    And the schedule reports "rebuild the deck" as unplaceable because "<reason>"

    Examples:
      | placed | reason                      |
      | 0      | chunk_policy_unsatisfiable  |

  # schedule-hard-deadline-unreachable-04: a hard deadline that cannot be met makes the task unplaceable
  Scenario: A hard deadline that cannot be met makes the task unplaceable
    Given the life area "Work" is saved with a guardrail band on "Mon, Tue, Wed, Thu, Fri" from "09:00" to "17:00"
    And a committed task "call the notary" in life area "Work" estimated "120" minutes due "<deadline>" as "hard" with priority "P1"
    When the schedule is generated
    And the schedule page is viewed
    Then the schedule reports "<placed>" placed blocks
    And the schedule reports "call the notary" as unplaceable because "<reason>"

    Examples:
      | deadline             | placed | reason               |
      | 2026-08-17T10:00:00Z | 0      | deadline_unreachable |

  # schedule-capacity-exceeded-05: when the hours run out, the tasks that did not fit say so
  Scenario: When the hours run out, the tasks that did not fit say so
    Given the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "11:00"
    And a committed task "first" in life area "Work" needs "120" minutes by "2026-08-17T11:00:00Z"
    And a committed task "second" in life area "Work" needs "120" minutes by "2026-08-24T11:00:00Z"
    And a committed task "third" in life area "Work" needs "120" minutes by "2026-08-28T17:00:00Z"
    When the schedule is generated
    And the schedule page is viewed
    Then the schedule reports "<placed>" placed blocks
    And the schedule reports "third" as unplaceable because "<reason>"

    Examples:
      | placed | reason             |
      | 2      | capacity_exceeded  |

  # schedule-no-window-06: a life area with no hours places nothing and says why
  Scenario: A life area with no hours places nothing and says why
    Given the life area "Fitness" is saved as never scheduled
    And a committed task "swim a mile" in life area "Fitness" needs "60" minutes by "2026-08-28T17:00:00Z"
    When the schedule is generated
    And the schedule page is viewed
    Then the schedule reports "<placed>" placed blocks
    And the schedule reports "swim a mile" as unplaceable because "<reason>"

    Examples:
      | placed | reason    |
      | 0      | no_window |

  # schedule-only-committed-07: pool and quota work is neither placed nor reported as unplaceable
  Scenario: Pool and quota work is neither placed nor reported as unplaceable
    Given the life area "Work" is saved with a guardrail band on "Mon, Tue, Wed, Thu, Fri" from "09:00" to "17:00"
    And a pool task "read the spec" in life area "Work" is triaged
    And a quota task "practise piano" in life area "Work" targeting "3" sessions of "40" minutes per "week" is triaged
    When the schedule is generated
    And the schedule page is viewed
    Then the schedule reports "<placed>" placed blocks
    And the schedule does not mention "read the spec"
    And the schedule does not mention "practise piano"

    Examples:
      | placed |
      | 0      |

  # schedule-plan-is-stable-08: the plan does not change until it is regenerated
  Scenario: The plan does not change until it is regenerated
    Given the life area "Work" is saved with a guardrail band on "Mon, Tue, Wed, Thu, Fri" from "09:00" to "17:00"
    And a committed task "write the Q3 deck" in life area "Work" needs "120" minutes by "2026-08-21T17:00:00Z"
    And the schedule is generated
    When a committed task "book the venue" in life area "Work" needs "60" minutes by "2026-08-21T17:00:00Z" is triaged
    And the schedule page is viewed
    Then the schedule reports "<placed>" placed blocks
    And the schedule does not mention "book the venue"
    When the schedule is generated
    And the schedule page is viewed
    Then the schedule reports "<after>" placed blocks

    Examples:
      | placed | after |
      | 1      | 2     |

  # schedule-empty-state-09: with nothing to schedule the page says so rather than showing an empty box
  Scenario: With nothing to schedule the page says so rather than showing an empty box
    Given the life area "Work" is saved with a guardrail band on "Mon, Tue, Wed, Thu, Fri" from "09:00" to "17:00"
    When the schedule page is viewed
    Then the schedule reports "<placed>" placed blocks
    And the schedule shows an empty-state message

    Examples:
      | placed |
      | 0      |

  # schedule-escapes-hostile-text-10: hostile task text stays escaped on the schedule page
  Scenario: Hostile task text stays escaped on the schedule page
    Given the life area "Work" is saved with a guardrail band on "Mon, Tue, Wed, Thu, Fri" from "09:00" to "17:00"
    And a committed task "<script>alert('boom')</script>" in life area "Work" needs "120" minutes by "2026-08-21T17:00:00Z"
    When the schedule is generated
    And the schedule page is viewed
    Then the schedule page does not contain an unescaped "<script>" tag
    And the schedule page contains the word "boom"
