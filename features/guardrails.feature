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
  Scenario: A band whose end does not follow its start is refused
    When the life area "Work" is saved with a guardrail band on "<days>" from "<start>" to "<end>"
    Then the save is rejected
    And the life area "Work" shows "no guardrail"

    Examples:
      | days | start | end   |
      | Mon  | 17:00 | 09:00 |
      | Mon  | 09:00 | 09:00 |

  # guardrails-band-no-day-refused-11: a band naming no weekday is refused
  Scenario: A band naming no weekday is refused
    When the life area "Work" is saved with a guardrail band on no weekday from "<start>" to "<end>"
    Then the save is rejected
    And the life area "Work" shows "no guardrail"

    Examples:
      | start | end   |
      | 09:00 | 17:00 |

  # guardrails-escapes-hostile-text-12: hostile text submitted to the guardrail form stays escaped
  Scenario: Hostile text submitted to the guardrail form stays escaped
    When the life area "Work" is saved with a guardrail band starting at "<script>alert('boom')</script>"
    Then the save is rejected
    And the life areas list does not contain an unescaped "<script>" tag
    And the life area "Work" shows "no guardrail"
