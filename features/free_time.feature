# free-time-per-life-area-01: the page reports each life area's free hours over the next fourteen days
# free-time-lists-intervals-02: the page lists each free interval, not only a total
# free-time-empty-is-an-answer-03: a life area with no hours reports no free time, and that is not a failure
# free-time-overlap-both-report-04: two life areas claiming the same hours each report them
# free-time-dst-gap-05: a band spanning a spring-forward transition loses exactly the hour that does not exist
# free-time-dst-fold-06: a band spanning a fall-back transition gains the hour that happens twice
# free-time-escapes-hostile-text-07: a hostile life area name stays escaped where the free time page renders it
#
# Fourteen days is exactly two weeks, so the window holds exactly two of every
# weekday whatever day it starts on. Every total below is therefore a fixed
# number rather than something that drifts with the day the suite runs -- that
# determinism is an argument for the horizon, not a coincidence of it.
#
# The fold decision is scenario 06 and it is the whole reason that scenario
# carries a number rather than a shrug: a repeated wall-clock hour inside a
# guardrail is counted **both times**, because at both instants the owner's
# clock reads a time inside the band and the day really does have 25 hours.
# Scenario 05 is its mirror -- the hour that does not exist is not counted.
# Between them they are also the first observable consequence of the owner's
# timezone: in UTC both totals would be 6.
#
# What is deliberately not here, and belongs to the proptest this slice also
# owes: that `free_intervals` returns intervals that are disjoint, sorted, and
# each a subset of the mask minus what is taken, over >= 1000 generated cases.
# A property over arbitrary interval sets has no page to be driven from, and
# nothing at M2 supplies busy, pins or buffers -- #61 is the first.
Feature: The free time page reports when each life area is actually free

  Background:
    Given the trellis server is running with an empty task list

  Scenario: The page reports each life area's free hours over the next fourteen days
    Given the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "<end>"
    When the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"

    Examples:
      | end   | hours |
      | 17:00 | 16    |
      | 16:00 | 14    |

  # free-time-lists-intervals-02: the page lists each free interval, not only a total
  Scenario: The page lists each free interval, not only a total
    Given the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "17:00"
    When the free time page is viewed
    Then the free time page lists "<intervals>" free intervals for "Work"

    Examples:
      | intervals |
      | 2         |

  # free-time-empty-is-an-answer-03: a life area with no hours reports no free time, and that is not a failure
  Scenario: A life area with no hours reports no free time
    Given the life area "Work" is saved as never scheduled
    When the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"
    And the free time page reports "<hours>" hours free for "Home"

    Examples:
      | hours |
      | 0     |

  # free-time-overlap-both-report-04: two life areas claiming the same hours each report them
  Scenario: Two life areas claiming the same hours each report them
    Given the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "17:00"
    And the life area "Learning" is saved with a guardrail band on "Mon" from "09:00" to "17:00"
    When the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"
    And the free time page reports "<hours>" hours free for "Learning"

    Examples:
      | hours |
      | 16    |

  # free-time-dst-gap-05: a band spanning a spring-forward transition loses exactly the hour that does not exist
  Scenario: A band spanning a spring-forward transition loses exactly the hour that does not exist
    Given the server believes it is "2027-03-13T12:00:00-05:00"
    And the owner's timezone is "America/New_York"
    And the life area "Work" is saved with a guardrail band on "Sun" from "01:00" to "04:00"
    When the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"

    Examples:
      | hours |
      | 5     |

  # free-time-dst-fold-06: a band spanning a fall-back transition gains the hour that happens twice
  Scenario: A band spanning a fall-back transition gains the hour that happens twice
    Given the server believes it is "2027-11-06T12:00:00-04:00"
    And the owner's timezone is "America/New_York"
    And the life area "Work" is saved with a guardrail band on "Sun" from "01:00" to "04:00"
    When the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"

    Examples:
      | hours |
      | 7     |

  # free-time-escapes-hostile-text-07: a hostile life area name stays escaped where the free time page renders it
  Scenario: A hostile life area name stays escaped where the free time page renders it
    Given a life area named "<script>alert('boom')</script>" was added
    When the free time page is viewed
    Then the free time page does not contain an unescaped "<script>" tag
    And the free time page contains the word "boom"
