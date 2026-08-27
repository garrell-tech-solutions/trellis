# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-23T17:53:17.610629679Z","feature_name":"A committed date is chosen, not typed, and lands on the day intended","feature_path":"features/committed_date.feature","background_hash":"ddc88cf1eb1374c2a55e04aee94532de746b37dd3da80da53dcac66b5be40a9f","implementation_hash":"sha256:1024902e2cdb2521cf49102a719ed487261d40637b3d9e8722f6f7ef2dd0a3f1","scenarios":[{"index":0,"name":"An at is dated by picking a day and a time, never by typing a timestamp","scenario_hash":"e38cecb29879683919b204a8907f790a2609054a330ba19dd70f77899a2f578b","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-23T17:53:17.610629679Z"},{"index":1,"name":"A by is dated by picking a day alone","scenario_hash":"a92b2927c07b89f6f870ca31c1f6452cca3d25b9aaac9361b9d7b93a407e69bc","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-23T17:53:17.610629679Z"},{"index":2,"name":"The owner's zone decides which day a near-midnight date lands on","scenario_hash":"be7f31ef3695463ef4f827f54dc2704ddabf961848f0f3b2cfd5d7700bff1675","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-23T17:53:17.610629679Z"},{"index":3,"name":"At and by stay an explicit choice, never inferred from what was entered","scenario_hash":"17760079f0a04caf86db5c94ccefc63217ee6c33741fed353187eb11edd9beb3","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-23T17:53:17.610629679Z"},{"index":5,"name":"A rejection is still 422 and still names the field","scenario_hash":"d7bbbebf54cbc74820deeea86d1b6c618e7f8f2138a5c34810b83413439e755d","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-23T17:53:17.610629679Z"},{"index":6,"name":"Beyond this week the date cell says which day, not just which weekday","scenario_hash":"af4eead3c108c47614d3c4b113f6c8e931ed3f2c9ff7ca32e136833826789a97","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-23T17:53:17.610629679Z"}]}
# acceptance-mutation-manifest-end

# committed-date-at-takes-day-and-time-01: an at is dated by picking a day and a time, never by typing a timestamp
# committed-date-by-takes-a-day-02: a by is dated by picking a day alone
# committed-date-timezone-decides-the-day-03: the owner's zone decides which day a near-midnight date lands on
# committed-date-still-explicit-04: at and by stay an explicit choice, never inferred from what was entered
# committed-date-json-still-takes-an-instant-05: the JSON transport still takes an RFC 3339 instant, and a by may still carry a time
# committed-date-rejection-still-names-the-field-06: a rejection is still 422 and still names the field
# committed-date-cell-disambiguates-07: beyond this week the date cell says which day, not just which weekday
#
# THE CANVAS DRAWS NO DATE INPUT -- verified, not paraphrased: zero occurrences
# of type="date", type="time" or type="datetime-local" in
# docs/design/Trellis.dc.html. Under D-four-screens that is a GAP, flagged
# rather than filled silently; the owner chose a native picker. The canvas's
# weekday <select> is no precedent for it: that sits under "Log a session",
# RETROSPECTIVE and bounded to the week just gone, while a deadline is
# PROSPECTIVE AND UNBOUNDED. Its 44px touch height is the precedent, and kept.
#
# A BY TAKES A DAY AND NO TIME ON THE PAGE, but 05 pins that a by may still
# carry a time over JSON -- the page simply stops asking, and this slice must
# not narrow the boundary committed-screen-at-and-by-02 already asserts.
# T-commitment-is-chosen-not-derived is unweakened: at or by is picked FIRST
# and the form answers.
#
# A DATE-ONLY BY IS THE END OF THAT DAY in the owner's zone. That local-date to
# instant conversion is a business rule (T-jiff-epoch-millis): it belongs in
# scheduler-core, not a handler and not in JavaScript.
#
# DISPLAY IS IN SCOPE though the brief frames this as an input problem:
# date_cell renders UTC by its own admission, so a date picked at 23:00 local
# would be entered correctly and then shown on the wrong day -- 03 asserts the
# round trip. 07 exists because `BY THU` is unambiguous only within a week.
# Both forms stay short: the canvas gives the cell 66px at 10.5px uppercase, so
# "BY THU 27 AUG" does not fit and is not what is specified.
Feature: A committed date is chosen, not typed, and lands on the day intended

  Background:
    Given the trellis server is running with an empty task list
    And the owner's timezone is "America/New_York"

  Scenario: An at is dated by picking a day and a time, never by typing a timestamp
    Given a capture with raw text "Book the dentist" is waiting in the untriaged queue
    When the capture is triaged as a committed "at" on day "<day>" at time "<time>"
    Then the triage is accepted
    And the committed screen shows "Book the dentist" with the date cell "<cell>"

    Examples:
      | day        | time  | cell     |
      | 2026-08-25 | 08:30 | TUE 8:30 |

  # committed-date-by-takes-a-day-02: a by is dated by picking a day alone
  Scenario: A by is dated by picking a day alone
    Given a capture with raw text "File the tax return" is waiting in the untriaged queue
    When the capture is triaged as a committed "by" on day "<day>" with no time
    Then the triage is accepted
    And the committed screen shows "File the tax return" with the date cell "<cell>"
    And "File the tax return" is due at the end of "<day>" in the owner's timezone

    Examples:
      | day        | cell   |
      | 2026-08-27 | BY THU |

  # committed-date-timezone-decides-the-day-03: the owner's zone decides which day a near-midnight date lands on
  Scenario: The owner's zone decides which day a near-midnight date lands on
    Given a capture with raw text "Book the dentist" is waiting in the untriaged queue
    When the capture is triaged as a committed "at" on day "<day>" at time "<time>"
    And the committed screen is viewed
    Then the committed screen shows "Book the dentist" with the date cell "<cell>"

    Examples:
      | day        | time  | cell      |
      | 2026-08-25 | 23:30 | TUE 23:30 |
      | 2026-08-25 | 00:30 | TUE 0:30  |

  # committed-date-still-explicit-04: at and by stay an explicit choice, never inferred from what was entered
  Scenario: At and by stay an explicit choice, never inferred from what was entered
    Given a capture with raw text "File the tax return" is waiting in the untriaged queue
    When the capture is triaged as a committed "by" on day "2026-08-27" with no time
    And the committed screen is viewed
    Then the committed screen shows "File the tax return" with the date cell "<cell>"
    And the committed screen does not show "File the tax return" as an at

    Examples:
      | cell   |
      | BY THU |

  # committed-date-json-still-takes-an-instant-05: the JSON transport still takes an RFC 3339 instant, and a by may still carry a time
  Scenario: The JSON transport still takes an RFC 3339 instant, and a by may still carry a time
    Given a capture with raw text "File the tax return" is waiting in the untriaged queue
    When the capture is triaged over the API as a committed "by" with deadline "<deadline>"
    Then the triage is accepted
    And the committed screen shows "File the tax return" with the date cell "<cell>"

    Examples:
      | deadline             | cell   |
      | 2026-08-27T17:00:00Z | BY THU |

  # committed-date-rejection-still-names-the-field-06: a rejection is still 422 and still names the field
  Scenario: A rejection is still 422 and still names the field
    Given a capture with raw text "File the tax return" is waiting in the untriaged queue
    When the capture is triaged as a committed "by" with "<missing_field>" omitted
    Then the triage is rejected
    And the rejection names "<missing_field>"
    And the committed screen lists nothing
    And the capture is still waiting in the untriaged queue

    Examples:
      | missing_field |
      | deadline      |
      | commitment    |

  # committed-date-cell-disambiguates-07: beyond this week the date cell says which day, not just which weekday
  Scenario: Beyond this week the date cell says which day, not just which weekday
    Given the server believes it is "2026-08-24T09:00:00Z"
    And a capture with raw text "Renew the passport" is waiting in the untriaged queue
    When the capture is triaged as a committed "by" on day "<day>" with no time
    And the committed screen is viewed
    Then the committed screen shows "Renew the passport" with the date cell "<cell>"

    Examples:
      | day        | cell      |
      | 2026-08-27 | BY THU    |
      | 2026-09-17 | BY 17 SEP |
