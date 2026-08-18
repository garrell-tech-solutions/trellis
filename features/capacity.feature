# capacity-supply-and-demand-01: the page reports hours needed against hours available, per life area
# capacity-over-is-called-out-02: over-commitment is named, not left to the reader to subtract
# capacity-pool-consumes-nothing-03: a pool backlog adds no demand at all
# capacity-quota-counts-04: quota demand counts, prorated across the horizon
# capacity-never-scheduled-05: a life area that opted out reports no capacity, rather than zero and over
# capacity-exceptions-reduce-supply-06: an exception lowers what is available, and can tip a life area over
# capacity-archived-life-area-excluded-07: an archived life area is not reported at all
#
# No scenario pins a clock except 06, and that is deliberate:
# T-free-time-horizon-fourteen-days guarantees exactly two of every weekday in
# the window, so "Sat 09:00-11:00" is 4h and "Mon-Fri 09:00-17:00" is 80h on
# whatever day the suite runs. Only the exception scenario names dates,
# because an exception is a date.
#
# Percentages are shown on every row and the warning fires only above 100%.
# D-staleness-unset's "instrument first, tune at the first reckoning" is read
# literally here: a margin picked today would be picked with no fortnight of
# real numbers behind it, so the page reports utilisation and leaves the
# owner to judge 98% for themselves until there is data to choose a threshold
# from.
#
# Two rules are specified here in prose because no affordance can drive them
# on a fresh database, and both want unit tests rather than scenarios:
#
#   - A committed task whose `estimated_minutes` is NULL predates the column
#     (T-life-area-required-at-triage's own reasoning for `life_area_id`).
#     It means "written before the estimate was required", never "takes no
#     time", so it must NOT be counted as zero -- that under-reports demand,
#     which is the exact direction of wrongness M2 exists to prevent. It is
#     excluded from the sum and surfaced as a count on its life area's row.
#     Nothing on a fresh database can create one, since triage now requires
#     the field.
#   - An archived task is excluded (T-archived-at-only). `tasks.archived_at`
#     exists but nothing writes it before M8, so there is no user-visible act
#     that archives a task at M2.
Feature: Capacity reports what each life area needs against what it has

  Background:
    Given the trellis server is running with an empty task list

  Scenario: The page reports hours needed against hours available, per life area
    Given the life area "Fitness" is saved with a guardrail band on "Sat" from "09:00" to "11:00"
    And a committed task in life area "Fitness" estimated at "<minutes>" minutes is triaged
    When the capacity page is viewed
    Then capacity reports "<needed>" hours needed and "<available>" hours available for "Fitness"
    And capacity reports "<percent>" percent used for "Fitness"
    And capacity does not warn for "Fitness"

    Examples:
      | minutes | needed | available | percent |
      | 180     | 3      | 4         | 75      |

  # capacity-over-is-called-out-02: over-commitment is named, not left to the reader to subtract
  Scenario: Over-commitment is named, not left to the reader to subtract
    Given the life area "Fitness" is saved with a guardrail band on "Sat" from "09:00" to "11:00"
    And a committed task in life area "Fitness" estimated at "180" minutes is triaged
    And a committed task in life area "Fitness" estimated at "120" minutes is triaged
    When the capacity page is viewed
    Then capacity reports "<needed>" hours needed and "<available>" hours available for "Fitness"
    And capacity reports "<percent>" percent used for "Fitness"
    And capacity warns that "Fitness" is "<over>" hours over

    Examples:
      | needed | available | percent | over |
      | 5      | 4         | 125     | 1    |

  # capacity-pool-consumes-nothing-03: a pool backlog adds no demand at all
  Scenario: A pool backlog adds no demand at all
    Given the life area "Fitness" is saved with a guardrail band on "Sat" from "09:00" to "11:00"
    And a pool task in life area "Fitness" is triaged
    And a pool task in life area "Fitness" is triaged
    When the capacity page is viewed
    Then capacity reports "<needed>" hours needed and "<available>" hours available for "Fitness"
    And capacity does not warn for "Fitness"

    Examples:
      | needed | available |
      | 0      | 4         |

  # capacity-quota-counts-04: quota demand counts, prorated across the horizon
  Scenario: Quota demand counts, prorated across the horizon
    Given the life area "Learning" is saved with a guardrail band on "Mon, Tue, Wed, Thu, Fri" from "20:00" to "22:00"
    And a quota task in life area "Learning" targeting "<count>" sessions of "<each>" minutes per "<period>" is triaged
    When the capacity page is viewed
    Then capacity reports "<needed>" hours needed and "<available>" hours available for "Learning"

    Examples:
      | count | each | period | needed | available |
      | 3     | 40   | week   | 4      | 20        |
      | 10    | 45   | month  | 3.5    | 20        |

  # capacity-never-scheduled-05: a life area that opted out reports no capacity, rather than zero and over
  Scenario: A life area that opted out reports no capacity, rather than zero and over
    Given the life area "Fitness" is saved as never scheduled
    And a committed task in life area "Fitness" estimated at "180" minutes is triaged
    When the capacity page is viewed
    Then capacity reports "Fitness" as never scheduled
    And capacity does not warn for "Fitness"

  # capacity-exceptions-reduce-supply-06: an exception lowers what is available, and can tip a life area over
  Scenario: An exception lowers what is available, and can tip a life area over
    Given the server believes it is "2026-08-17T12:00:00Z"
    And the life area "Fitness" is saved with a guardrail band on "Sat" from "09:00" to "11:00"
    And a committed task in life area "Fitness" estimated at "180" minutes is triaged
    When the dates "2026-08-22" to "2026-08-22" are marked away for all life areas
    And the capacity page is viewed
    Then capacity reports "<needed>" hours needed and "<available>" hours available for "Fitness"
    And capacity warns that "Fitness" is "<over>" hours over

    Examples:
      | needed | available | over |
      | 3      | 2         | 1    |

  # capacity-archived-life-area-excluded-07: an archived life area is not reported at all
  Scenario: An archived life area is not reported at all
    Given the life area "Fitness" is saved with a guardrail band on "Sat" from "09:00" to "11:00"
    And a committed task in life area "Fitness" estimated at "180" minutes is triaged
    When the life area "Fitness" is archived
    And the capacity page is viewed
    Then capacity does not report "Fitness"
