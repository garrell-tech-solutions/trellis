# committed-date-at-takes-day-and-time-01: an at is dated by picking a day and a time, never by typing a timestamp
# committed-date-by-takes-a-day-02: a by is dated by picking a day alone
# committed-date-timezone-decides-the-day-03: the owner's zone decides which day a near-midnight date lands on
# committed-date-still-explicit-04: at and by stay an explicit choice, never inferred from what was entered
# committed-date-json-still-takes-an-instant-05: the JSON transport still takes an RFC 3339 instant, and a by may still carry a time
# committed-date-rejection-still-names-the-field-06: a rejection is still 422 and still names the field
# committed-date-cell-disambiguates-07: beyond this week the date cell says which day, not just which weekday
#
# THE CANVAS DRAWS NO DATE INPUT AT ALL -- verified, not paraphrased: zero
# occurrences of type="date", type="time" or type="datetime-local" in
# docs/design/Trellis.dc.html. D-four-screens makes it authoritative on
# layout, so this is a GAP of the same class as the missing done control
# #103 met, and it is flagged rather than filled silently. The owner chose a
# native date picker: it costs no typing, needs no JavaScript, and gives a
# phone its own wheel.
#
# THE CANVAS'S DAY CONTROL WAS NOT A PRECEDENT FOR THIS, and the difference
# is why the owner was asked. Its seven-entry weekday <select> (line 274)
# sits under "Log a session" -- RETROSPECTIVE, bounded to the week just gone.
# A deadline is PROSPECTIVE AND UNBOUNDED: "the tax return by the 15th" is a
# real commitment a weekday list cannot say. Its 44px touch height IS the
# precedent and is kept.
#
# A BY TAKES A DAY AND NO TIME ON THE PAGE. T-commitment-is-chosen-not-derived
# is not weakened: the owner picks at or by FIRST and the form answers, which
# is the opposite of inferring the choice from whether a time was typed. A
# by's time of day is not the commitment and the screen never shows it.
#
# BUT A BY MAY STILL CARRY A TIME over JSON, and 05 pins that -- the page
# simply stops asking. committed-screen-at-and-by-02 already asserts a by
# with a time renders as a by rather than silently becoming an at, and this
# slice must not quietly narrow the boundary to match the form.
#
# A DATE-ONLY BY IS THE END OF THAT DAY in the owner's zone. "By Thursday"
# means before Thursday is over, and the conversion from a local date to an
# instant is a business rule -- T-jiff-epoch-millis: it belongs in
# scheduler-core, not a handler and not in JavaScript.
#
# THE DISPLAY IS PART OF THIS SLICE, which the brief frames as an input
# problem. scheduler_core::committed_screen::date_cell renders UTC and says
# so in its own doc comment -- "this product has no per-user timezone applied
# to display yet". A date chosen at 23:00 local would be entered correctly
# and then shown on the wrong day, so 03 asserts the round trip rather than
# the input alone.
#
# 07 EXISTS BECAUSE THE CELL CANNOT TELL TWO THURSDAYS APART. `BY THU` is
# unambiguous only within a week; a deadline three weeks out renders
# identically to one three days out. Beyond this week the cell names the
# date instead. Both forms are kept short on purpose: the canvas gives this
# cell 66px at 10.5px uppercase, so "BY THU 27 AUG" does not fit and is not
# what is specified. qa/phone_layout.md's browser check can see a clipped
# cell for the first time in this project -- use it.
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
    And the task list is still empty

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
