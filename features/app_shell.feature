# mutation-stamp: sha256=23fba9e7f94a24fa29b1a1f119f0c8d9cbf1d68c940ed8a940a2bd1fa9c54257
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-18T23:59:54.263605086Z","feature_name":"Every page carries the same navigation header","feature_path":"features/app_shell.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:1a8dff05339baa4bd40748378c6734dd7ae05c9fe31ca85b7d317c1a6bd55989","scenarios":[{"index":0,"name":"The inbox carries the shared header and marks itself current","scenario_hash":"472975bb0cbcf5b7a12c18d047d6f9898ebb84987a9ded35bdc5c21339e8f2eb","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T23:59:54.263605086Z"},{"index":1,"name":"The life areas page carries the shared header and marks itself current","scenario_hash":"6da816198ea9c585eeb9ff13fe77a6bb567aad62f62ee3c64a47b0a5e3df080e","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T23:59:54.263605086Z"},{"index":2,"name":"The stats page carries the shared header and marks itself current","scenario_hash":"7b4f00b995ddf061621d5d233e9d140bc76afc4caa82608f61268c7f267eb150","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T23:59:54.263605086Z"},{"index":3,"name":"The free time page carries the shared header and marks itself current","scenario_hash":"704671a8d1e45a94f6da105a2374368efb0b3b621dc964920e5c3d2251969b44","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T23:59:54.263605086Z"},{"index":4,"name":"The capacity page carries the shared header and marks itself current","scenario_hash":"c43084ccb28e9cae04b1baf59328ce972c6a24d20b2712233bc64b70a5751119","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T23:59:54.263605086Z"},{"index":5,"name":"Each header link points at the page it names and reaches it","scenario_hash":"36467fd2f388acc34b3a4f81316ddd883933b6fd82d03038de95c47372a1c0ae","mutation_count":10,"result":{"Total":10,"Killed":10,"Survived":0,"Errors":0},"tested_at":"2026-08-18T23:59:54.263605086Z"}]}
# acceptance-mutation-manifest-end

# app-shell-inbox-01: the inbox carries the shared header and marks itself current
# app-shell-life-areas-02: the life areas page carries the shared header and marks itself current
# app-shell-stats-03: the stats page carries the shared header and marks itself current
# app-shell-free-time-04: the free time page carries the shared header and marks itself current
# app-shell-capacity-05: the capacity page carries the shared header and marks itself current
# app-shell-links-go-where-they-say-06: following a header link reaches the page it names
# app-shell-fragment-swap-07: a fragment swap leaves the header alone and carries no header of its own
# app-shell-plain-links-08: header links are ordinary links, not htmx requests
# app-shell-renders-no-user-data-09: the header renders no capture or life area text
#
# 01 to 05 are the same three assertions made literally on each page rather
# than once over an Examples table of page names, per the life-areas-
# duplicate-03 lesson: a scenario whose subject is sameness across pages
# gives the mutator only invalid page names to produce, which error rather
# than fail. Five literal scenarios cost five copies and can each fail.
#
# 04 arrived with the free time page (#60). T-nav-is-the-site-map makes that
# automatic rather than a judgement: every page in the route table is in the
# header, so a new page necessarily changes what 01 to 03 assert. That is the
# rule working, not drift -- but it does mean this feature cannot be one of
# the "existing features pass untouched" when a page is added.
#
# The 422 assertion rides on each of them because hoisting that override into
# the shared shell extends it to the stats page, which carries no form and
# had no htmx at all. T-forms-swap-one-fragment's guard is that 422 means
# exactly "validation rejection, body is the re-rendered fragment"
# product-wide, and a rule kept in two copies has two chances to drift --
# so the override moves, and the stats page's new share of it is asserted
# here rather than left incidental.
Feature: Every page carries the same navigation header

  Background:
    Given the trellis server is running with an empty task list

  Scenario: The inbox carries the shared header and marks itself current
    When the inbox is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                               | label     |
      | Inbox, Life areas, Free time, Capacity, Stats | Inbox     |

  # app-shell-life-areas-02: the life areas page carries the shared header and marks itself current
  Scenario: The life areas page carries the shared header and marks itself current
    When the life areas page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                               | label      |
      | Inbox, Life areas, Free time, Capacity, Stats | Life areas |

  # app-shell-stats-03: the stats page carries the shared header and marks itself current
  Scenario: The stats page carries the shared header and marks itself current
    When the stats page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                               | label     |
      | Inbox, Life areas, Free time, Capacity, Stats | Stats     |

  # app-shell-free-time-04: the free time page carries the shared header and marks itself current
  Scenario: The free time page carries the shared header and marks itself current
    When the free time page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                               | label     |
      | Inbox, Life areas, Free time, Capacity, Stats | Free time |

  # app-shell-capacity-05: the capacity page carries the shared header and marks itself current
  Scenario: The capacity page carries the shared header and marks itself current
    When the capacity page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                                         | label    |
      | Inbox, Life areas, Free time, Capacity, Stats | Capacity |

  # app-shell-links-go-where-they-say-06: following a header link reaches the page it names
  Scenario: Each header link points at the page it names and reaches it
    When the inbox is viewed
    Then the header link "<label>" points at "<path>"
    When the header link "<label>" is followed
    Then the header marks "<label>" as the current page

    Examples:
      | label      | path        |
      | Inbox      | /           |
      | Life areas | /life-areas |
      | Free time  | /free-time  |
      | Capacity   | /capacity   |
      | Stats      | /stats      |

  # app-shell-fragment-swap-07: a fragment swap leaves the header alone and carries no header of its own
  Scenario: A fragment swap leaves the header alone and carries no header of its own
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When the capture is triaged as a pool task through the page
    Then the page's triage response carries no header
    When the inbox is viewed
    Then the header appears exactly once

  # app-shell-plain-links-08: header links are ordinary links, not htmx requests
  Scenario: Header links are ordinary links, not htmx requests
    When the inbox is viewed
    Then every header link is an ordinary link that loads a full page

  # app-shell-renders-no-user-data-09: the header renders no capture or life area text
  Scenario: The header renders no capture or life area text
    Given a capture with raw text "<script>alert('boom')</script>" is waiting in the untriaged queue
    And a life area named "<script>alert('boom')</script>" was added
    When the inbox is viewed
    Then the header does not contain an unescaped "<script>" tag
    And the header does not contain the word "boom"
