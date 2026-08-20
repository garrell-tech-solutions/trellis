# mutation-stamp: sha256=53cc8d82e4b030d2a8962df5f8334b377e9d63f7719578186656e712a2099acc
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-17T17:57:45.238312126Z","feature_name":"Triage tags a task with a life area","feature_path":"features/life_area_triage.feature","background_hash":"7bc51985dd2897894c3d5dfc3b919840d9cc397ff48878073315f59557a78164","implementation_hash":"sha256:5dc73b4c014ebc91ff211ecb0be4ac092c07a1fcd31c8ad69a5f553b3d0f90a5","scenarios":[{"index":6,"name":"The life area picker preselects nothing, whatever the kind","scenario_hash":"269ec50258ef81c0d01f718a4b072e7162db2dee0103e98dd4e8e296ad52b3bc","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-17T17:57:45.238312126Z"},{"index":8,"name":"Triage submitted from the page with no life area chosen is refused on that row","scenario_hash":"292702427bcd445259f4db59efe3aa69e10399dcc2e5ee0b9852db424370fcb6","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-17T17:57:45.238312126Z"},{"index":2,"name":"Triage without a life area is rejected, whatever the kind","scenario_hash":"efb3dabfa3455242837cf698fb2401c958c1177cbc861fa2d8be483536b01e30","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-16T14:46:11.972912466Z"}]}
# acceptance-mutation-manifest-end

# life-area-triage-offers-01: triage offers every life area, including one added since the server started
# life-area-triage-tags-02: triage tags the task with the chosen life area, and the task list shows it
# life-area-triage-optional-03: triage without a life area succeeds, whatever the kind
# life-area-triage-unknown-04: triage naming a life area that does not exist is rejected
# life-area-triage-archived-05: triage into an archived life area is rejected at the boundary, not only hidden from the picker
# life-area-triage-escapes-hostile-text-06: a hostile life area name stays escaped where the task row renders it
# life-area-triage-no-preselection-07: the life area picker preselects nothing, whatever the kind
# life-area-triage-quick-add-picker-08: a row rendered by the quick-add box carries the same unselected picker
# life-area-triage-page-optional-09: triage submitted from the page with no life area chosen succeeds
#
# The picker's unselected placeholder is not a life area, so "the triage life
# area choices are exactly ..." above keeps naming only real life areas.
# D-manual-triage-until-llm: nothing chooses for the user, and a default that
# is merely first-by-id is that same silent wrong default one layer down.
#
# 03 AND 09 WERE INVERTED BY #82, not deleted. They asserted that triage
# without a life area is refused; T-life-area-required-at-triage is superseded
# by D-context-tags-are-the-taxonomy, which makes context tags the product's
# only taxonomy and drops the requirement. Inverting keeps the same two
# transports under test and pins the new behaviour: deleting them would have
# left "triage succeeds with no life area" asserted nowhere, on exactly the
# path T-required-fields-are-specified-per-transport was written about. The
# picker itself is unchanged -- it still offers every active life area and
# still preselects nothing.
Feature: Triage tags a task with a life area

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "sketch the landing page" is waiting in the untriaged queue

  Scenario: Triage offers every life area, including one added since the server started
    Given a life area named "Side project" was added
    When the inbox is viewed
    Then the triage life area choices are exactly "Work, Fitness, Learning, Family, Home, Side project"

  # life-area-triage-tags-02: triage tags the task with the chosen life area, and the task list shows it
  Scenario: Triage tags the task with the chosen life area, and the task list shows it
    Given a capture with raw text "buy milk" is waiting in the untriaged queue
    When "sketch the landing page" is triaged as a pool task in life area "Learning"
    And "buy milk" is triaged as a pool task in life area "Home"
    And the inbox is viewed
    Then the task list shows "sketch the landing page" tagged "Learning"
    And the task list shows "buy milk" tagged "Home"

  # life-area-triage-optional-03: triage without a life area succeeds, whatever the kind
  Scenario: Triage without a life area succeeds, whatever the kind
    When the capture is triaged as a <kind> task with "life_area" omitted
    Then the triage succeeds
    And the inbox is viewed
    And the task list shows "sketch the landing page" with no life area

    Examples:
      | kind      |
      | pool      |
      | committed |
      | quota     |

  # life-area-triage-unknown-04: triage naming a life area that does not exist is rejected
  Scenario: Triage naming a life area that does not exist is rejected
    When the capture is triaged as a pool task in life area "Gardening"
    Then the triage is rejected
    And the rejection says "Gardening" is not a life area
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

  # life-area-triage-archived-05: triage into an archived life area is rejected at the boundary, not only hidden from the picker
  Scenario: Triage into an archived life area is rejected at the boundary, not only hidden from the picker
    Given the life area "Learning" is archived
    When the capture is triaged as a pool task in life area "Learning"
    Then the triage is rejected
    And the task list is still empty
    And the capture is still waiting in the untriaged queue

  # life-area-triage-escapes-hostile-text-06: a hostile life area name stays escaped where the task row renders it
  Scenario: A hostile life area name stays escaped where the task row renders it
    Given a life area named "<script>alert('boom')</script>" was added
    When the capture is triaged as a pool task in life area "<script>alert('boom')</script>"
    And the inbox is viewed
    Then the task list does not contain an unescaped "<script>" tag
    And the task list contains the word "boom"

  # life-area-triage-no-preselection-07: the life area picker preselects nothing, whatever the kind
  Scenario: The life area picker preselects nothing, whatever the kind
    When the inbox is viewed
    Then the <kind> triage form preselects no life area

    Examples:
      | kind      |
      | pool      |
      | committed |
      | quota     |

  # life-area-triage-quick-add-picker-08: a row rendered by the quick-add box carries the same unselected picker
  Scenario: A row rendered by the quick-add box carries the same unselected picker
    When the quick-add box submits a capture with raw text "buy milk"
    Then the quick-add response's pool triage form preselects no life area

  # life-area-triage-page-optional-09: triage submitted from the page with no life area chosen succeeds
  Scenario: Triage submitted from the page with no life area chosen succeeds
    When the capture is triaged as a <kind> task through the page with no life area chosen
    Then the triage succeeds
    And the task list shows "sketch the landing page" with no life area

    Examples:
      | kind      |
      | pool      |
      | committed |
      | quota     |
