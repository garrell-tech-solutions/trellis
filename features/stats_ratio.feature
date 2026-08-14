# mutation-stamp: sha256=f768d7f49e091f8079feb631dbdccbd2aafc9a8711d8e178d7d6b149aff8f0d3
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-14T19:23:36.328703331Z","feature_name":"The stats page reports the committed share over a rolling fourteen-day window","feature_path":"features/stats_ratio.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:1df1c46cedc6c4418e3e68658979faaa4db487a060fc636beb7e69b04e3782d6","scenarios":[{"index":2,"name":"A task triaged just inside the fourteen-day window counts toward the share","scenario_hash":"6e86107787918c4257e30d4ff60929e76f2c33e93308780bba3ea6dfe9bfca7b","mutation_count":5,"result":{"Total":5,"Killed":5,"Survived":0,"Errors":0},"tested_at":"2026-08-14T19:23:36.328703331Z"},{"index":3,"name":"A task triaged more than fourteen days ago falls out of the window","scenario_hash":"99b8b2226b147c4a3f4b3c3f511161c106f39df5637f901bf569752bb87a6ded","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-14T19:23:36.328703331Z"},{"index":4,"name":"The page marks whether the share is over the fifty-percent line, distinguishably from the share itself","scenario_hash":"4a8fbe27b9ccc3f1db2f112a6f13ed79d5bb48fcc4c9e525603d4da4dae8db0c","mutation_count":15,"result":{"Total":15,"Killed":15,"Survived":0,"Errors":0},"tested_at":"2026-08-14T19:23:36.328703331Z"},{"index":5,"name":"Below ten tasks the page reports the counts but no share and no verdict","scenario_hash":"dfe55baa1cee28f833e9d3ebd8c4ad67f30537900f85c90ae0e744df9a3dcede","mutation_count":8,"result":{"Total":8,"Killed":8,"Survived":0,"Errors":0},"tested_at":"2026-08-14T19:23:36.328703331Z"},{"index":0,"name":"The stats page reports the committed share of everything triaged inside the window","scenario_hash":"f7ce1364c52c44cae207aaaecb221dc4aec77251176329767213c43f17c05aa0","mutation_count":12,"result":{"Total":12,"Killed":12,"Survived":0,"Errors":0},"tested_at":"2026-08-14T19:09:52.224882266Z"},{"index":1,"name":"The counts behind the share are shown, so the denominator is never in doubt","scenario_hash":"6ac791e44f21ba6e5d18ef0a7fadc91ef6b326b0f8169c45d8288303ad8d37f7","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-14T19:09:52.224882266Z"}]}
# acceptance-mutation-manifest-end

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
  #
  # Split into an inside-the-window and an outside-the-window scenario rather
  # than one Examples table varying `days_ago` and the excluded row's own
  # `committed` count: once a task falls outside the window, no page-facing
  # assertion can distinguish 3 excluded tasks from 8, or 15 days ago from 22
  # -- the count and distance are unobservable, not under-tested, so they are
  # written as literals the mutator cannot touch instead of columns whose
  # mutation nothing could ever catch.
  Scenario: A task triaged just inside the fourteen-day window counts toward the share
    Given <committed> committed tasks were triaged 13 days ago
    And <pool> pool tasks were triaged today
    And <quota> quota tasks were triaged today
    When the stats page is viewed
    Then the stats page reports <in_window> tasks in the window
    And the stats page reports a committed share of "<share>"

    Examples:
      | committed | pool | quota | in_window | share |
      | 3         | 12   | 0     | 15        | 20%   |

  # stats-ratio-window-03: only tasks triaged inside the fourteen-day window are counted
  Scenario: A task triaged more than fourteen days ago falls out of the window
    Given 1 committed tasks were triaged 15 days ago
    And <pool> pool tasks were triaged today
    And <quota> quota tasks were triaged today
    When the stats page is viewed
    Then the stats page reports <in_window> tasks in the window
    And the stats page reports a committed share of "<share>"

    Examples:
      | pool | quota | in_window | share |
      | 12   | 0     | 12        | 0%    |

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
