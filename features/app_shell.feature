# mutation-stamp: sha256=c9c7680de5028d3af8ff88c5a396d7957ee326610eaeeb1fc5d267fa5f9a79fa
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-19T17:22:08.194658021Z","feature_name":"Every page carries the same navigation header","feature_path":"features/app_shell.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:1a8dff05339baa4bd40748378c6734dd7ae05c9fe31ca85b7d317c1a6bd55989","scenarios":[{"index":0,"name":"The inbox carries the shared header and marks itself current","scenario_hash":"7512a6f00c3da602ab0a4507325751f29feb2f645859a82b2c553045bfe30da1","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:22:08.194658021Z"},{"index":1,"name":"The life areas page carries the shared header and marks itself current","scenario_hash":"ee7143952f1afeff549c44a0c9c8f2692f029cfcea6f70abca1aaeac360225fd","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:22:08.194658021Z"},{"index":2,"name":"The stats page carries the shared header and marks itself current","scenario_hash":"dd27e553c92755a9757f0e63b754a382251f04e5f3233bec6a0f10560d9ed81f","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:22:08.194658021Z"},{"index":3,"name":"The free time page carries the shared header and marks itself current","scenario_hash":"cd0e8509754a797be3f3af64f756a8d1e3f186c7f7afb9f2361225d9197e3840","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:22:08.194658021Z"},{"index":4,"name":"The capacity page carries the shared header and marks itself current","scenario_hash":"f15e9e67c61dba3c526ee64d3c5f55cceae74cfff6d662e9305adece59c3ce49","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:22:08.194658021Z"},{"index":5,"name":"The schedule page carries the shared header and marks itself current","scenario_hash":"8aea25986288263220276ff669caadefc594c7ecdd53691bbddac202687e5e5e","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:22:08.194658021Z"},{"index":6,"name":"Each header link points at the page it names and reaches it","scenario_hash":"a5885c929df6ec907675611a011b3ced62360059f1e3e1db130c2688d10e3d90","mutation_count":12,"result":{"Total":12,"Killed":12,"Survived":0,"Errors":0},"tested_at":"2026-08-19T17:22:08.194658021Z"}]}
# acceptance-mutation-manifest-end

# app-shell-inbox-01: the inbox carries the shared header and marks itself current
# app-shell-life-areas-02: the life areas page carries the shared header and marks itself current
# app-shell-stats-03: the stats page carries the shared header and marks itself current
# app-shell-free-time-04: the free time page carries the shared header and marks itself current
# app-shell-capacity-05: the capacity page carries the shared header and marks itself current
# app-shell-schedule-06: the schedule page carries the shared header and marks itself current
# app-shell-links-go-where-they-say-07: following a header link reaches the page it names
# app-shell-fragment-swap-08: a fragment swap leaves the header alone and carries no header of its own
# app-shell-plain-links-09: header links are ordinary links, not htmx requests
# app-shell-renders-no-user-data-10: the header renders no capture or life area text
#
# 01 to 06 are the same three assertions made literally on each page rather
# than once over an Examples table of page names, per the life-areas-
# duplicate-03 lesson: a scenario whose subject is sameness across pages
# gives the mutator only invalid page names to produce, which error rather
# than fail. Six literal scenarios cost six copies and can each fail.
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
      | Inbox, Life areas, Free time, Capacity, Schedule, Stats | Inbox     |

  # app-shell-life-areas-02: the life areas page carries the shared header and marks itself current
  Scenario: The life areas page carries the shared header and marks itself current
    When the life areas page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                               | label      |
      | Inbox, Life areas, Free time, Capacity, Schedule, Stats | Life areas |

  # app-shell-stats-03: the stats page carries the shared header and marks itself current
  Scenario: The stats page carries the shared header and marks itself current
    When the stats page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                               | label     |
      | Inbox, Life areas, Free time, Capacity, Schedule, Stats | Stats     |

  # app-shell-free-time-04: the free time page carries the shared header and marks itself current
  Scenario: The free time page carries the shared header and marks itself current
    When the free time page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                               | label     |
      | Inbox, Life areas, Free time, Capacity, Schedule, Stats | Free time |

  # app-shell-capacity-05: the capacity page carries the shared header and marks itself current
  Scenario: The capacity page carries the shared header and marks itself current
    When the capacity page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                                         | label    |
      | Inbox, Life areas, Free time, Capacity, Schedule, Stats | Capacity |

  # app-shell-schedule-06: the schedule page carries the shared header and marks itself current
  Scenario: The schedule page carries the shared header and marks itself current
    When the schedule page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                                                   | label    |
      | Inbox, Life areas, Free time, Capacity, Schedule, Stats | Schedule |

  # app-shell-links-go-where-they-say-07: following a header link reaches the page it names
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
      | Schedule   | /schedule   |
      | Stats      | /stats      |

  # app-shell-fragment-swap-08: a fragment swap leaves the header alone and carries no header of its own
  Scenario: A fragment swap leaves the header alone and carries no header of its own
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When the capture is triaged as a pool task through the page
    Then the page's triage response carries no header
    When the inbox is viewed
    Then the header appears exactly once

  # app-shell-plain-links-09: header links are ordinary links, not htmx requests
  Scenario: Header links are ordinary links, not htmx requests
    When the inbox is viewed
    Then every header link is an ordinary link that loads a full page

  # app-shell-renders-no-user-data-10: the header renders no capture or life area text
  Scenario: The header renders no capture or life area text
    Given a capture with raw text "<script>alert('boom')</script>" is waiting in the untriaged queue
    And a life area named "<script>alert('boom')</script>" was added
    When the inbox is viewed
    Then the header does not contain an unescaped "<script>" tag
    And the header does not contain the word "boom"
