# mutation-stamp: sha256=fb7de3aabb075794468751649d612bdf7037fdbad9cb77a07b9bd3891922a1ab
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-18T13:07:28.271388138Z","feature_name":"Each life area carries its own weekly guardrail","feature_path":"features/guardrails.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:9070745b72cc0e7d1f1112f90a33d9bf2bc8b3ccc7c88c8dc5206ae56b92dcd4","scenarios":[{"index":0,"name":"A fresh database's life areas each show no guardrail","scenario_hash":"f9ef316dde82325cc24243eb235f6b49f5c25347e6f4e434b2abbafa8e344bd6","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-18T13:03:04.432817171Z"},{"index":1,"name":"A guardrail band saved for a life area is listed as it was authored","scenario_hash":"cfec5e101ba247b4a180a5cd8ad8e4bbdc0092bacc2b9fd127bde39ce56b1aa7","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-18T13:03:04.432817171Z"},{"index":2,"name":"One life area carries more than one band","scenario_hash":"7ecfc58c400fc54b5c1cbfbc18f51551230a88c25825faba8fcb3a02dea1beb0","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-18T13:03:04.432817171Z"},{"index":3,"name":"A life area marked pool-only is well-formed with no hours at all","scenario_hash":"fb1a8f05e0a6928841091d2dcc8c9309befde2446ec830bb3fc391404efd7bbb","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T13:03:04.432817171Z"},{"index":4,"name":"Saving a life area with neither a band nor the pool-only mark is refused","scenario_hash":"bf1f883e0e4cef59e2228d75137a7283216350b5d15f2d52efea296411bd052e","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-18T13:03:04.432817171Z"},{"index":5,"name":"A band overlapping the same life area's existing band is refused","scenario_hash":"c27e4d85cb98e7cd7442ef0c79794fc615283e872ba34e4ee7e1ea92d090e7c3","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-18T13:03:04.432817171Z"},{"index":6,"name":"Bands that touch without overlapping are both kept","scenario_hash":"fe996f26e3b34086060cbb2210002ed04b2aea96e0e00d9ac2234a8a7b66ba22","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-18T13:03:04.432817171Z"},{"index":8,"name":"A band can be removed, leaving the life area with no guardrail","scenario_hash":"f383a86cf8a8cc6011149bba1a8c161d6e24e31372d8fc5ac78912772b7e2f74","mutation_count":5,"result":{"Total":5,"Killed":5,"Survived":0,"Errors":0},"tested_at":"2026-08-18T13:03:04.432817171Z"}]}
# acceptance-mutation-manifest-end

# guardrails-none-initially-01: a fresh database's life areas each show no guardrail
# guardrails-band-saved-02: a guardrail band saved for a life area is listed as it was authored
# guardrails-two-bands-03: one life area carries more than one band
# guardrails-pool-only-04: a life area marked pool-only is well-formed with no hours at all
# guardrails-neither-refused-05: saving a life area with neither a band nor the pool-only mark is refused
# guardrails-own-overlap-refused-06: a band overlapping the same life area's existing band is refused
# guardrails-touching-allowed-07: bands that touch without overlapping are both kept
# guardrails-cross-overlap-allowed-08: two life areas may claim the same hours
# guardrails-band-removed-09: a band can be removed, leaving the life area with no guardrail
# guardrails-band-times-refused-10: a band whose end does not follow its start is refused
# guardrails-band-no-day-refused-11: a band naming no weekday is refused
# guardrails-escapes-hostile-text-12: hostile text submitted to the guardrail form stays escaped
#
# Every scenario acts on "Work" unless it needs a second life area, and every
# band is 09:00-17:00 unless the scenario is about the times themselves. The
# first draft varied both freely and the DRY checker reported 117 findings
# against a repo norm of single figures -- all the same step shape carrying
# different literals, which is exactly the noise a real drift would hide in.
#
# "Pool-only" is the concept's name in docs/decisions.md and stays the slug.
# On the page it reads "never scheduled - menu only", because "pool-only"
# reads as "only holds pool tasks" and that is not what it means: a walled
# life area holds pool tasks too, and pool is the default kind. The mark says
# the life area has no hours, so its work is never placed and only ever
# surfaces in the menu. The step vocabulary below keeps "pool-only" for the
# act; the assertions use the words the owner sees.
#
# Bands are civil wall-clock (T-jiff-epoch-millis). Nothing here converts one
# to an instant -- that is #60's -- so no scenario asserts a timezone, and the
# owner's zone is features/timezone_setting.feature's subject.
#
# Two acceptance criteria are deliberately absent because the page cannot
# show them:
#   - Guardrails survive a restart. Acceptance runs in-process, so restart
#     persistence is qa/guardrails.md's, as it has been since life-areas.
#   - An archived life area keeps its guardrail. `list_active` filters
#     archived rows out of the management list and there is no un-archive, so
#     an archived life area renders nowhere at all. The row surviving is
#     durable state, and only QA's read-only sqlite3 can see it.
Feature: Each life area carries its own weekly guardrail

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A fresh database's life areas each show no guardrail
    When the life areas page is viewed
    Then every life area shows "<state>"

    Examples:
      | state        |
      | no guardrail |

  # guardrails-band-saved-02: a guardrail band saved for a life area is listed as it was authored
  Scenario: A guardrail band saved for a life area is listed as it was authored
    When the life area "Work" is saved with a guardrail band on "<days>" from "<start>" to "<end>"
    Then the save is accepted
    And the life area "Work" shows the guardrail band "<listed>"

    Examples:
      | days                    | start | end   | listed                              |
      | Mon, Tue, Wed, Thu, Fri | 09:00 | 17:00 | Mon, Tue, Wed, Thu, Fri 09:00-17:00 |

  # guardrails-two-bands-03: one life area carries more than one band
  Scenario: One life area carries more than one band
    Given the life area "Work" is saved with a guardrail band on "Mon, Wed, Fri" from "06:00" to "07:00"
    When the life area "Work" is saved with a guardrail band on "Sat" from "09:00" to "11:00"
    Then the life area "Work" shows the guardrail band "Mon, Wed, Fri 06:00-07:00"
    And the life area "Work" shows the guardrail band "Sat 09:00-11:00"
    And the life area "Work" shows "<bands>" guardrail bands

    Examples:
      | bands |
      | 2     |

  # guardrails-pool-only-04: a life area marked pool-only is well-formed with no hours at all
  Scenario: A life area marked pool-only is well-formed with no hours at all
    When the life area "Work" is saved as pool-only
    Then the save is accepted
    And the life area "Work" shows "<state>"
    And the life area "Work" shows "<bands>" guardrail bands

    Examples:
      | state                       | bands |
      | never scheduled - menu only | 0     |

  # guardrails-neither-refused-05: saving a life area with neither a band nor the pool-only mark is refused
  Scenario: Saving a life area with neither a band nor the pool-only mark is refused
    When the life area "Work" is saved with neither a guardrail band nor the pool-only mark
    Then the save is rejected
    And the rejection message on the life area's row mentions "<option>"
    And the life area "Work" shows "no guardrail"

    Examples:
      | option          |
      | guardrail band  |
      | never scheduled |

  # guardrails-own-overlap-refused-06: a band overlapping the same life area's existing band is refused
  Scenario: A band overlapping the same life area's existing band is refused
    Given the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "12:00"
    When the life area "Work" is saved with a guardrail band on "Mon" from "11:00" to "17:00"
    Then the save is rejected
    And the rejection says the band overlaps one the life area already has
    And the life area "Work" shows "<bands>" guardrail bands

    Examples:
      | bands |
      | 1     |

  # guardrails-touching-allowed-07: bands that touch without overlapping are both kept
  Scenario: Bands that touch without overlapping are both kept
    Given the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "12:00"
    When the life area "Work" is saved with a guardrail band on "Mon" from "12:00" to "17:00"
    Then the save is accepted
    And the life area "Work" shows "<bands>" guardrail bands

    Examples:
      | bands |
      | 2     |

  # guardrails-cross-overlap-allowed-08: two life areas may claim the same hours
  Scenario: Two life areas may claim the same hours
    Given the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "17:00"
    When the life area "Learning" is saved with a guardrail band on "Mon" from "09:00" to "17:00"
    Then the life area "Work" shows the guardrail band "Mon 09:00-17:00"
    And the life area "Learning" shows the guardrail band "Mon 09:00-17:00"

  # guardrails-band-removed-09: a band can be removed, leaving the life area with no guardrail
  Scenario: A band can be removed, leaving the life area with no guardrail
    Given the life area "Work" is saved with a guardrail band on "<days>" from "<start>" to "<end>"
    When the guardrail band "<listed>" is removed from the life area "Work"
    Then the life area "Work" shows "<state>"

    Examples:
      | days | start | end   | listed        | state        |
      | Mon  | 09:00 | 17:00 | Mon 09:00-17:00 | no guardrail |

  # guardrails-band-times-refused-10: a band whose end does not follow its start is refused
  #
  # No Examples table: an invalid end -- before its start, or equal to it --
  # is rejected the same way an unparseable time string is
  # (`well_formed_times` collapses both into `InvalidTimes`), and "the save
  # is rejected" can't tell those apart. A mutated day or a mutated time
  # would still be rejected, just for a different reason, so a data table
  # here would only be columns whose values gherkin-mutator could never
  # observe changing. Two literal scenarios say what they test instead.
  Scenario: A band whose end is before its start is refused
    When the life area "Work" is saved with a guardrail band on "Mon" from "17:00" to "09:00"
    Then the save is rejected
    And the life area "Work" shows "no guardrail"

  # guardrails-band-times-refused-10: a band whose end does not follow its start is refused
  Scenario: A band whose end equals its start is refused
    When the life area "Work" is saved with a guardrail band on "Mon" from "09:00" to "09:00"
    Then the save is rejected
    And the life area "Work" shows "no guardrail"

  # guardrails-band-no-day-refused-11: a band naming no weekday is refused
  #
  # No Examples table for the same reason: `from_fields` rejects on an empty
  # weekday list before it ever looks at start/end, so no start/end value --
  # mutated or not -- changes this scenario's outcome.
  Scenario: A band naming no weekday is refused
    When the life area "Work" is saved with a guardrail band on no weekday from "09:00" to "17:00"
    Then the save is rejected
    And the life area "Work" shows "no guardrail"

  # guardrails-escapes-hostile-text-12: hostile text submitted to the guardrail form stays escaped
  Scenario: Hostile text submitted to the guardrail form stays escaped
    When the life area "Work" is saved with a guardrail band starting at "<script>alert('boom')</script>"
    Then the save is rejected
    And the life areas list does not contain an unescaped "<script>" tag
    And the life area "Work" shows "no guardrail"
