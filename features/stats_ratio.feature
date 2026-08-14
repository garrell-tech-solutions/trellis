# stats-ratio-share-01: the stats page reports the committed share of everything triaged inside the window
# stats-ratio-counts-02: the counts behind the share are shown, so the denominator is never in doubt
# stats-ratio-window-03: only tasks triaged inside the fourteen-day window are counted
# stats-ratio-over-the-line-04: the page marks whether the share is over the fifty-percent line, distinguishably from the share itself
# stats-ratio-small-sample-05: below ten tasks the page reports the counts but no share and no verdict, and an empty window divides nothing by zero
Feature: The stats page reports the committed share over a rolling fourteen-day window

  Background:
    Given the trellis server is running with an empty task list

  Scenario: The stats page reports the committed share of everything triaged inside the window
    Given <committed> committed tasks were triaged today
    And <pool> pool tasks were triaged today
    And <quota> quota tasks were triaged today
    When the stats page is viewed
    Then the stats page reports a committed share of "<share>"

    Examples:
      | committed | pool | quota | share |
      | 3         | 7    | 0     | 30%   |
      | 4         | 4    | 4     | 33%   |
      | 5         | 7    | 0     | 42%   |

  # stats-ratio-counts-02: the counts behind the share are shown, so the denominator is never in doubt
  Scenario: The counts behind the share are shown, so the denominator is never in doubt
    Given <committed> committed tasks were triaged today
    And <pool> pool tasks were triaged today
    And <quota> quota tasks were triaged today
    When the stats page is viewed
    Then the stats page reports <committed> committed, <pool> pool and <quota> quota tasks
    And the stats page reports <in_window> tasks in the window

    Examples:
      | committed | pool | quota | in_window |
      | 5         | 4    | 3     | 12        |

  # stats-ratio-window-03: only tasks triaged inside the fourteen-day window are counted
  Scenario: Only tasks triaged inside the fourteen-day window are counted
    Given <committed> committed tasks were triaged <days_ago> days ago
    And <pool> pool tasks were triaged today
    And <quota> quota tasks were triaged today
    When the stats page is viewed
    Then the stats page reports <in_window> tasks in the window
    And the stats page reports a committed share of "<share>"

    Examples:
      | committed | days_ago | pool | quota | in_window | share |
      | 3         | 13       | 12   | 0     | 15        | 20%   |
      | 3         | 15       | 12   | 0     | 12        | 0%    |

  # stats-ratio-over-the-line-04: the page marks whether the share is over the fifty-percent line, distinguishably from the share itself
  Scenario: The page marks whether the share is over the fifty-percent line, distinguishably from the share itself
    Given <committed> committed tasks were triaged today
    And <pool> pool tasks were triaged today
    And <quota> quota tasks were triaged today
    When the stats page is viewed
    Then the stats page reports a committed share of "<share>"
    And the stats page marks the committed share "<standing>"

    Examples:
      | committed | pool | quota | share | standing       |
      | 5         | 15   | 0     | 25%   | under the line |
      | 10        | 10   | 0     | 50%   | under the line |
      | 11        | 9    | 0     | 55%   | over the line  |

  # stats-ratio-small-sample-05: below ten tasks the page reports the counts but no share and no verdict, and an empty window divides nothing by zero
  Scenario: Below ten tasks the page reports the counts but no share and no verdict
    Given <committed> committed tasks were triaged today
    And <pool> pool tasks were triaged today
    And <quota> quota tasks were triaged today
    When the stats page is viewed
    Then the stats page reports <committed> committed, <pool> pool and <quota> quota tasks
    And the stats page reports <in_window> tasks in the window
    And the stats page reports no committed share
    And the stats page does not mark the committed share
    And the stats page says it has not measured enough yet

    Examples:
      | committed | pool | quota | in_window |
      | 4         | 5    | 0     | 9         |
      | 0         | 0    | 0     | 0         |
