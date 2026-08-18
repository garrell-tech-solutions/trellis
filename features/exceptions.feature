# mutation-stamp: sha256=dd6deda56ac18067f9f704875a63e0593b624e704ce15a9afd9596ad5965f9dd
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-18T18:21:09.237529453Z","feature_name":"A dated exception is how the owner says a week is not normal","feature_path":"features/exceptions.feature","background_hash":"3f8442aae7bdb91e27c54575b3c39ae37d76297e87aae2e0e8ab8b3d932330de","implementation_hash":"sha256:63be786f6f076249413a69c056a0dbec854211e6eb3913e400509b52605e5bbb","scenarios":[{"index":0,"name":"An exception removes those dates from free time","scenario_hash":"501477f6d837020a3a5a0ced156f438ee4b022427c5f87f8d87cbebcd4c55334","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-18T18:21:09.237529453Z"},{"index":3,"name":"An exception for one life area leaves the others alone","scenario_hash":"3a341545bd1bd67e95e86d332f9506c07b2d2ee5151df04e724ca99baebc5a11","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T18:21:09.237529453Z"},{"index":5,"name":"An exception whose dates have passed changes nothing and needs no cleanup","scenario_hash":"1f77a203ad1382ffacb3aab852f696ec86b3c25f3571af8b3a8d74eb31d4fa52","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-18T18:21:09.237529453Z"},{"index":6,"name":"Removing an exception gives the hours back, and the exception is gone","scenario_hash":"5e456493d33fc0040be7dba1817005c2f5b0633baf91357c50f01fbbaa19673d","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-18T18:21:09.237529453Z"},{"index":7,"name":"Overlapping exceptions subtract their union, not their sum","scenario_hash":"8c901834819a2728fb1a96008d043df8ea4339266a831f15ba6113c83a00d52d","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-18T18:21:09.237529453Z"},{"index":8,"name":"An exception crossing a DST transition leaves the transition day its true length","scenario_hash":"9716e01f3c16121d0c2852005ec16ec892851f511812c3017a531187a0e42230","mutation_count":8,"result":{"Total":8,"Killed":8,"Survived":0,"Errors":0},"tested_at":"2026-08-18T18:21:09.237529453Z"},{"index":9,"name":"A life area given bands and then marked never scheduled has no free time","scenario_hash":"54a710eb4eca6314103a39d1b19cf786a97e2546758146f841235015173882e6","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-18T18:21:09.237529453Z"}]}
# acceptance-mutation-manifest-end

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

  # exceptions-removes-days-01: an exception removes those dates from every
  # life area's free time
  #
  # No Examples table for the two rows below: each names a range that
  # overlaps nothing the guardrail claims (a weekend, or a range past the
  # 14-day horizon), so the reported hours stay 80 regardless of which
  # weekend or which distant range it is -- a mutated date is still a
  # weekend, or still outside the horizon. Two literal scenarios say what
  # they test instead of columns gherkin-mutator could never observe
  # changing.
  Scenario: A weekend-only exception changes nothing, since the guardrail never claimed those days
    When the dates "2026-08-22" to "2026-08-23" are marked away for all life areas
    And the free time page is viewed
    Then the free time page reports "80" hours free for "Work"

  # exceptions-removes-days-01: an exception removes those dates from every
  # life area's free time
  Scenario: An exception outside the horizon changes nothing
    When the dates "2026-09-10" to "2026-09-20" are marked away for all life areas
    And the free time page is viewed
    Then the free time page reports "80" hours free for "Work"

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
