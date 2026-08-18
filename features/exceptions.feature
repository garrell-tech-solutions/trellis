# exceptions-removes-days-01: an exception removes those dates from every life area's free time
# exceptions-scoped-to-one-life-area-02: an exception for one life area leaves the others alone
# exceptions-scope-is-visible-03: the page says which life areas each exception applies to
# exceptions-past-stops-mattering-04: an exception whose dates have passed changes nothing and needs no cleanup
# exceptions-removed-restores-05: removing an exception gives the hours back, and the exception is gone
# exceptions-overlap-no-double-subtraction-06: overlapping exceptions subtract their union, not their sum
# exceptions-dst-07: an exception crossing a DST transition leaves the transition day its true length
# exceptions-never-scheduled-after-bands-08: a life area given bands and then marked never scheduled has no free time
# exceptions-backwards-range-refused-09: an exception whose last day precedes its first is refused
# exceptions-escapes-hostile-text-10: a hostile exception label stays escaped where the list renders it
#
# Every total below is computed from a pinned clock, not from the day the
# suite runs. Today is Monday 2026-08-17, so the fourteen-day horizon
# (T-free-time-horizon-fourteen-days) covers 17-30 August and holds exactly
# ten weekdays: 80h of a Mon-Fri 09:00-17:00 guardrail. An exception on
# 24-28 August removes exactly five of them, which is the brief's 80h -> 40h
# with dates that actually land that way -- its own 20-24 August is a
# Thursday-to-Monday in 2026 and would remove three.
#
# Exceptions only ever REMOVE hours. "Working this Saturday" is not an
# exception; it is a different guardrail. Two reasons, and the second is the
# one that outlives this slice: a wall that can only shrink for a day cannot
# be argued into yielding (D-guardrails-never-yield), and M3's pins and M4's
# calendar busy are both subtractive, so one subtrahend shape serves all
# three rather than exceptions being a per-day mask override nothing else
# wants.
#
# Scenario 08 is owed from #70. Its own free-time scenario marked a life
# area never-scheduled from a clean state, so it read 0h whether or not the
# rule was honoured; the defect needed bands FIRST and the mark AFTER. The
# lesson generalises past this case: a scenario that reaches the right end
# state by the wrong path is green against an implementation that enforces
# nothing.
#
# Two dry-checker findings in here are deliberate, so they are not mistaken
# for drift later. Scenario 02 needs two different totals through one step in
# one state -- that one life area changed and another did not is the whole
# assertion -- so "<hours>" carries the primary value everywhere in this file
# and "<fitness_hours>" is the single deliberate second name. And scenario 09
# reads as a near-identical twin of scenario 01's setup step by design: the
# only difference between them is the order of the two dates, which is
# precisely what it tests.
#
# Not here, and belonging to the proptest this slice extends rather than
# duplicates: that subtraction still yields disjoint, sorted, positive-length
# intervals over generated inputs. crates/scheduler-core/tests/
# free_time_properties.rs already asserts that of projection; subtraction
# joins the same property.
Feature: A dated exception is how the owner says a week is not normal

  Background:
    Given the server believes it is "2026-08-17T12:00:00Z"
    And the trellis server is running with an empty task list
    And the life area "Work" is saved with a guardrail band on "Mon, Tue, Wed, Thu, Fri" from "09:00" to "17:00"

  Scenario: An exception removes those dates from free time
    When the dates "<from>" to "<to>" are marked away for all life areas
    And the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"

    Examples:
      | from       | to         | hours |
      | 2026-08-24 | 2026-08-28 | 40    |
      | 2026-08-24 | 2026-08-24 | 72    |
      | 2026-08-22 | 2026-08-23 | 80    |
      | 2026-09-10 | 2026-09-20 | 80    |

  # exceptions-scoped-to-one-life-area-02: an exception for one life area leaves the others alone
  Scenario: An exception for one life area leaves the others alone
    Given the life area "Fitness" is saved with a guardrail band on "Mon, Tue, Wed, Thu, Fri" from "06:00" to "07:00"
    When the dates "2026-08-24" to "2026-08-28" are marked away for the life area "Fitness"
    And the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"
    And the free time page reports "<fitness_hours>" hours free for "Fitness"

    Examples:
      | hours | fitness_hours |
      | 80    | 5             |

  # exceptions-scope-is-visible-03: the page says which life areas each exception applies to
  Scenario: The page says which life areas each exception applies to
    When the dates "2026-08-24" to "2026-08-28" are marked away for all life areas
    And the dates "2026-09-01" to "2026-09-02" are marked away for the life area "Work"
    Then the exceptions list shows "All life areas" for the exception starting "2026-08-24"
    And the exceptions list shows "Work" for the exception starting "2026-09-01"

  # exceptions-past-stops-mattering-04: an exception whose dates have passed changes nothing and needs no cleanup
  Scenario: An exception whose dates have passed changes nothing and needs no cleanup
    When the dates "2026-08-10" to "2026-08-14" are marked away for all life areas
    And the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"
    And the exceptions list shows "All life areas" for the exception starting "2026-08-10"

    Examples:
      | hours |
      | 80    |

  # exceptions-removed-restores-05: removing an exception gives the hours back, and the exception is gone
  Scenario: Removing an exception gives the hours back, and the exception is gone
    Given the dates "2026-08-24" to "2026-08-28" are marked away for all life areas
    When the exception starting "2026-08-24" is removed
    And the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"
    And the exceptions list is empty

    Examples:
      | hours |
      | 80    |

  # exceptions-overlap-no-double-subtraction-06: overlapping exceptions subtract their union, not their sum
  Scenario: Overlapping exceptions subtract their union, not their sum
    Given the dates "2026-08-24" to "2026-08-28" are marked away for all life areas
    When the dates "2026-08-26" to "2026-08-29" are marked away for all life areas
    And the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"

    Examples:
      | hours |
      | 40    |

  # exceptions-dst-07: an exception crossing a DST transition leaves the transition day its true length
  Scenario: An exception crossing a DST transition leaves the transition day its true length
    Given the server believes it is "<now>"
    And the owner's timezone is "America/New_York"
    And the life area "Learning" is saved with a guardrail band on "Sun" from "01:00" to "04:00"
    When the dates "<from>" to "<to>" are marked away for all life areas
    And the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Learning"

    Examples:
      | now                       | from       | to         | hours |
      | 2027-03-13T12:00:00-05:00 | 2027-03-21 | 2027-03-21 | 2     |
      | 2027-11-06T12:00:00-04:00 | 2027-11-14 | 2027-11-14 | 4     |

  # exceptions-never-scheduled-after-bands-08: a life area given bands and then marked never scheduled has no free time
  Scenario: A life area given bands and then marked never scheduled has no free time
    When the life area "Work" is saved as never scheduled
    And the free time page is viewed
    Then the free time page reports "<hours>" hours free for "Work"

    Examples:
      | hours |
      | 0     |

  # exceptions-backwards-range-refused-09: an exception whose last day precedes its first is refused
  Scenario: An exception whose last day precedes its first is refused
    When the dates "2026-08-28" to "2026-08-24" are marked away for all life areas
    Then the exception is rejected
    And the rejection says the last day precedes the first
    And the exceptions list is empty

  # exceptions-escapes-hostile-text-10: a hostile exception label stays escaped where the list renders it
  Scenario: A hostile exception label stays escaped where the list renders it
    When the dates "2026-08-24" to "2026-08-28" are marked away for all life areas labelled "<script>alert('boom')</script>"
    Then the exceptions list does not contain an unescaped "<script>" tag
    And the exceptions list contains the word "boom"
