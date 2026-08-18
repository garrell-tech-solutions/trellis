# timezone-setting-default-01: a fresh database reports the owner's timezone as UTC
# timezone-setting-changed-02: the owner's timezone can be changed from the page
# timezone-setting-unknown-refused-03: a name that is not a timezone is refused, and the setting is unchanged
# timezone-setting-escapes-hostile-text-04: hostile text submitted as a timezone stays escaped
#
# The owner's timezone is one value for the whole product (D-single-user):
# not one per life area, not one per guardrail. It exists because a civil
# wall-clock band is not an instant until you know where the owner is --
# but nothing in this slice converts a band to an instant, so the zone has
# no effect here beyond being stored and shown. #60 is the first reader.
#
# UTC is the default because it is the only honest one. Guessing the owner's
# zone from the host clock would mean a guardrail silently meaning a
# different hour than the one displayed, which is the class of silent wrong
# default D-manual-triage-until-llm rejected for the life area picker.
#
# It lives on the life areas page rather than a settings page of its own:
# that is where the times it governs are authored, and T-nav-is-the-site-map
# would put a whole route in the header for a single field.
Feature: The owner's timezone is one setting for the whole product

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A fresh database reports the owner's timezone as UTC
    When the life areas page is viewed
    Then the page reports the owner's timezone as "<zone>"

    Examples:
      | zone |
      | UTC  |

  # timezone-setting-changed-02: the owner's timezone can be changed from the page
  Scenario: The owner's timezone can be changed from the page
    When the owner's timezone is set to "<zone>"
    Then the change is accepted
    And the page reports the owner's timezone as "<zone>"

    Examples:
      | zone           |
      | Europe/London  |
      | America/Denver |

  # timezone-setting-unknown-refused-03: a name that is not a timezone is refused, and the setting is unchanged
  Scenario: A name that is not a timezone is refused, and the setting is unchanged
    When the owner's timezone is set to "<zone>"
    Then the change is rejected
    And the rejection says "<zone>" is not a timezone
    And the page reports the owner's timezone as "UTC"

    Examples:
      | zone           |
      | Mars/Olympus   |
      | Europe/Londonn |

  # timezone-setting-escapes-hostile-text-04: hostile text submitted as a timezone stays escaped
  Scenario: Hostile text submitted as a timezone stays escaped
    When the owner's timezone is set to "<script>alert('boom')</script>"
    Then the change is rejected
    And the life areas page does not contain an unescaped "<script>" tag
    And the life areas page contains the word "boom"
