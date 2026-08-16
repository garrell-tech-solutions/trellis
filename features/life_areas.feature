# mutation-stamp: sha256=cf684530f25fa49bdde6aac5ed237999378e5a8cae36100d48a96d314e9bf5bd
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-16T14:48:13.468156504Z","feature_name":"Life areas are user-managed rows, added and retired from the running app","feature_path":"features/life_areas.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:0aa5b45b9886c9c0f5459eb42416bd7c867229d11e2dcded7720526a8af2c90e","scenarios":[{"index":0,"name":"A fresh database offers the five seeded life areas","scenario_hash":"f235142f9ca6e277753f029ae7b0fdc538a7e1423501f9ea9aa8d8204b2958d9","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-16T14:45:56.076657693Z"},{"index":1,"name":"A life area added from the page is listed immediately","scenario_hash":"05cd50da70e88577863cf6f6acde214776bf3bc742e62224da62216cd0bf95fd","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-16T14:45:56.076657693Z"}]}
# acceptance-mutation-manifest-end

# life-areas-seed-01: a fresh database offers the five seeded life areas
# life-areas-add-02: a life area added from the page is listed immediately, without a restart
# life-areas-duplicate-03: a life area cannot be added twice, whatever the case of the name
# life-areas-trims-04: surrounding whitespace is not part of a life area's name
# life-areas-blank-05: a life area must have a name that is not just whitespace
# life-areas-archived-06: an archived life area leaves the picker but still names the tasks already in it
# life-areas-escapes-hostile-text-07: hostile text in a life area name stays escaped where the list renders it
Feature: Life areas are user-managed rows, added and retired from the running app

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A fresh database offers the five seeded life areas
    When the life areas page is viewed
    Then the life areas listed are exactly "<listed>"

    Examples:
      | listed                                |
      | Work, Fitness, Learning, Family, Home |

  # life-areas-add-02: a life area added from the page is listed immediately, without a restart
  Scenario: A life area added from the page is listed immediately
    When a life area named "<name>" is added from the life areas page
    Then the add response does not redirect the browser
    And the life areas listed are exactly "<listed>"

    Examples:
      | name         | listed                                              |
      | Side project | Work, Fitness, Learning, Family, Home, Side project |

  # life-areas-duplicate-03: a life area cannot be added twice, whatever the case of the name
  #
  # Three literal scenarios, not one Examples table varying `name`, because
  # this scenario's whole point is that the check is invariant under a case
  # change -- the one edit the Gherkin mutator's string dithering ever makes
  # to a name like "Work". A mutation that cannot help but land on a value
  # this scenario is designed to treat identically can never be caught: not
  # because the check is under-tested, but because the property under test
  # and the mutator's only move are the same axis. Naming each variant
  # directly in the step text removes it from the mutator's reach instead of
  # asserting a no-op it structurally cannot pass.
  Scenario: A life area cannot be re-added with the name spelled exactly as before
    When a life area named "Work" is added from the life areas page
    Then the add is rejected
    And the rejection says "Work" is already a life area
    And the life areas listed are exactly "Work, Fitness, Learning, Family, Home"

  # life-areas-duplicate-03: a life area cannot be added twice, whatever the case of the name
  Scenario: A life area cannot be re-added in lower case
    When a life area named "work" is added from the life areas page
    Then the add is rejected
    And the rejection says "work" is already a life area
    And the life areas listed are exactly "Work, Fitness, Learning, Family, Home"

  # life-areas-duplicate-03: a life area cannot be added twice, whatever the case of the name
  Scenario: A life area cannot be re-added in upper case
    When a life area named "WORK" is added from the life areas page
    Then the add is rejected
    And the rejection says "WORK" is already a life area
    And the life areas listed are exactly "Work, Fitness, Learning, Family, Home"

  # life-areas-trims-04: surrounding whitespace is not part of a life area's name
  Scenario: Surrounding whitespace is not part of a life area's name
    When a life area named "  Side project  " is added from the life areas page
    Then the life areas listed are exactly "Work, Fitness, Learning, Family, Home, Side project"

  # life-areas-blank-05: a life area must have a name that is not just whitespace
  Scenario: A life area must have a name that is not just whitespace
    When a life area named "   " is added from the life areas page
    Then the add is rejected
    And the rejection names "name"
    And the life areas listed are exactly "Work, Fitness, Learning, Family, Home"

  # life-areas-archived-06: an archived life area leaves the picker but still names the tasks already in it
  Scenario: An archived life area leaves the picker but still names the tasks already in it
    Given a capture with raw text "sketch the landing page" is waiting in the untriaged queue
    And the capture is triaged as a pool task in life area "Learning"
    When the life area "Learning" is archived
    And the inbox is viewed
    Then the triage life area choices are exactly "Work, Fitness, Family, Home"
    And the task list shows "sketch the landing page" tagged "Learning"

  # life-areas-escapes-hostile-text-07: hostile text in a life area name stays escaped where the list renders it
  Scenario: Hostile text in a life area name stays escaped where the list renders it
    When a life area named "<script>alert('boom')</script>" is added from the life areas page
    And the life areas page is viewed
    Then the life areas list does not contain an unescaped "<script>" tag
    And the life areas list contains the word "boom"
