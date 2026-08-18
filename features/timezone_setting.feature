# mutation-stamp: sha256=9e2e3a172f66feaaafbb54dcb90f4aac532e5fcfc6d9c288e2ab9698dc3f6b99
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-18T13:07:28.310694623Z","feature_name":"The owner's timezone is one setting for the whole product","feature_path":"features/timezone_setting.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:7f379c0ff46e1e2915b03ed572087cc94e3b2a6ff9add2bb6be8640c9ab53476","scenarios":[{"index":0,"name":"A fresh database reports the owner's timezone as UTC","scenario_hash":"1d6fa9a7dd0254d035e7c8ec758fe46600a0d8ac6ca5681904da65bbf0afa7e1","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-18T13:03:09.656031830Z"}]}
# acceptance-mutation-manifest-end

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
  #
  # No Examples table: both the acceptance and the echo below key off the
  # same submitted `<zone>`, and `scheduler_core::timezone::validate` (jiff's
  # own tzdb lookup) resolves a real zone name case-insensitively -- a
  # single-character case flip of either literal below still names the same
  # zone, so "accepted, and echoed back" stays true either way. Two literal
  # scenarios say what varies (which zone) without a column gherkin-mutator
  # could flip and never see it change anything.
  Scenario: The owner's timezone can be changed to Europe/London
    When the owner's timezone is set to "Europe/London"
    Then the change is accepted
    And the page reports the owner's timezone as "Europe/London"

  # timezone-setting-changed-02: the owner's timezone can be changed from the page
  Scenario: The owner's timezone can be changed to America/Denver
    When the owner's timezone is set to "America/Denver"
    Then the change is accepted
    And the page reports the owner's timezone as "America/Denver"

  # timezone-setting-unknown-refused-03: a name that is not a timezone is refused, and the setting is unchanged
  #
  # No Examples table for the same reason, from the other side: neither
  # literal below names a real zone under any casing, so a mutated character
  # still leaves the rejection -- and the message below, which echoes the
  # same submitted `<zone>` -- unchanged.
  Scenario: Mars/Olympus is refused, and the setting is unchanged
    When the owner's timezone is set to "Mars/Olympus"
    Then the change is rejected
    And the rejection says "Mars/Olympus" is not a timezone
    And the page reports the owner's timezone as "UTC"

  # timezone-setting-unknown-refused-03: a name that is not a timezone is refused, and the setting is unchanged
  Scenario: Europe/Londonn is refused, and the setting is unchanged
    When the owner's timezone is set to "Europe/Londonn"
    Then the change is rejected
    And the rejection says "Europe/Londonn" is not a timezone
    And the page reports the owner's timezone as "UTC"

  # timezone-setting-escapes-hostile-text-04: hostile text submitted as a timezone stays escaped
  Scenario: Hostile text submitted as a timezone stays escaped
    When the owner's timezone is set to "<script>alert('boom')</script>"
    Then the change is rejected
    And the life areas page does not contain an unescaped "<script>" tag
    And the life areas page contains the word "boom"
