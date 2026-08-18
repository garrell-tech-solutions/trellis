# mutation-stamp: sha256=eb90955e80500a431564286e8c807a21812143dcbc563ab77c5b4f81040a281b
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-18T01:09:51.855193522Z","feature_name":"Every page carries the same navigation header","feature_path":"features/app_shell.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:1a8dff05339baa4bd40748378c6734dd7ae05c9fe31ca85b7d317c1a6bd55989","scenarios":[{"index":0,"name":"The inbox carries the shared header and marks itself current","scenario_hash":"b9f4f5735054ab78e24c8e422efb7cd31adce801397aaa96cc880c273b98485a","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T01:09:51.855193522Z"},{"index":1,"name":"The life areas page carries the shared header and marks itself current","scenario_hash":"725a0e2ac06bd88050abbc04d6489a198d1f60c1c2f094dc5b54ed398e03ca68","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T01:09:51.855193522Z"},{"index":2,"name":"The stats page carries the shared header and marks itself current","scenario_hash":"38b0a3b36a54cf04ae686d1983437e2fae1b85f5cde6a04c83df5adfa82dca05","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T01:09:51.855193522Z"},{"index":3,"name":"Each header link points at the page it names and reaches it","scenario_hash":"8fb90173862c41586bf0bfa0a0e32ffc8875674147aa17cfeefb6a4f6f8d3b3c","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-18T01:09:51.855193522Z"}]}
# acceptance-mutation-manifest-end

# app-shell-inbox-01: the inbox carries the shared header and marks itself current
# app-shell-life-areas-02: the life areas page carries the shared header and marks itself current
# app-shell-stats-03: the stats page carries the shared header and marks itself current
# app-shell-links-go-where-they-say-04: following a header link reaches the page it names
# app-shell-fragment-swap-05: a fragment swap leaves the header alone and carries no header of its own
# app-shell-plain-links-06: header links are ordinary links, not htmx requests
# app-shell-renders-no-user-data-07: the header renders no capture or life area text
#
# 01 to 03 are the same three assertions made literally on each page rather
# than once over an Examples table of page names, per the life-areas-
# duplicate-03 lesson: a scenario whose subject is sameness across pages
# gives the mutator only invalid page names to produce, which error rather
# than fail. Three literal scenarios cost three copies and can each fail.
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
      | links                     | label   |
      | Inbox, Life areas, Stats  | Inbox   |

  # app-shell-life-areas-02: the life areas page carries the shared header and marks itself current
  Scenario: The life areas page carries the shared header and marks itself current
    When the life areas page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                     | label      |
      | Inbox, Life areas, Stats  | Life areas |

  # app-shell-stats-03: the stats page carries the shared header and marks itself current
  Scenario: The stats page carries the shared header and marks itself current
    When the stats page is viewed
    Then the header links are exactly "<links>"
    And the header marks "<label>" as the current page
    And the header marks exactly one link as the current page
    And the page declares the 422 swap handling

    Examples:
      | links                     | label   |
      | Inbox, Life areas, Stats  | Stats   |

  # app-shell-links-go-where-they-say-04: following a header link reaches the page it names
  Scenario: Each header link points at the page it names and reaches it
    When the inbox is viewed
    Then the header link "<label>" points at "<path>"
    When the header link "<label>" is followed
    Then the header marks "<label>" as the current page

    Examples:
      | label      | path        |
      | Inbox      | /           |
      | Life areas | /life-areas |
      | Stats      | /stats      |

  # app-shell-fragment-swap-05: a fragment swap leaves the header alone and carries no header of its own
  Scenario: A fragment swap leaves the header alone and carries no header of its own
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When the capture is triaged as a pool task through the page
    Then the page's triage response carries no header
    When the inbox is viewed
    Then the header appears exactly once

  # app-shell-plain-links-06: header links are ordinary links, not htmx requests
  Scenario: Header links are ordinary links, not htmx requests
    When the inbox is viewed
    Then every header link is an ordinary link that loads a full page

  # app-shell-renders-no-user-data-07: the header renders no capture or life area text
  Scenario: The header renders no capture or life area text
    Given a capture with raw text "<script>alert('boom')</script>" is waiting in the untriaged queue
    And a life area named "<script>alert('boom')</script>" was added
    When the inbox is viewed
    Then the header does not contain an unescaped "<script>" tag
    And the header does not contain the word "boom"
