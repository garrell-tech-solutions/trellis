# committed-screen-date-order-01: committed items list in date order with their date, text and context tag
# committed-screen-at-and-by-02: an at shows its time; a by shows that it is a by
# committed-screen-past-still-shows-03: a deadline that has passed is still listed, and marked
# committed-screen-only-committed-04: pool and quota work does not appear
# committed-screen-empty-05: with nothing dated the screen says that is allowed
# committed-screen-tabs-06: the tab bar offers three tabs and marks the current one
# committed-screen-triage-takes-at-or-by-07: committed triage asks at or by, on every transport
# committed-screen-escapes-hostile-text-08: hostile text stays escaped on this screen
#
# AT AND BY ARE AN EXPLICIT CHOICE AT TRIAGE, not derived from how precisely
# the deadline was typed. D-committed-is-at-or-by says a committed item is one
# or the other and that they behave differently; the canvas draws neither, and
# D-four-screens makes the canvas authoritative on layout and the decisions
# log on behaviour. So the distinction is real and the drawing of it is ours:
# a `by` carries a BY prefix in the same 66px cell an `at` uses for its time.
#
# The owner chose the explicit control over deriving it from whether a time
# was given, because a derived rule cannot express a HARD BY -- "the tax
# return, by Jan 31, and that one cannot slip" -- which is a real commitment
# and would have been silently unrepresentable.
#
# AT/BY REPLACES deadline_type ON THE FORM. `deadline_type`'s only behaviour
# was T-hard-refuses-soft-slips', which lives in the scheduler D-dogfood-first
# paused, so it currently changes nothing -- the exact state U2 complained of
# before that decision fixed it. When the scheduler returns, an `at` is hard
# (a fixed block cannot slip) and a `by` is soft (that is what slack means),
# so the behaviour is derived rather than typed twice.
#
# THE COLUMN STAYS, UNREAD. #88 kept `tasks.life_area_id` on exactly this
# ground: dropping a column throws away what the owner already typed, and
# unread columns cost SQLite nothing. This is not the `scheduler_core::ratio`
# case -- that was a derivable number, recomputable from rows that stayed;
# a hard/soft judgement is not recoverable once dropped.
#
# ORDER IS CHRONOLOGICAL. The canvas sorts by `ord`, a bare sort key whose
# fixture happens to agree with its dates (3/1/4 against Thu/Tue/Fri) and
# whose only derivation anywhere is line 637 -- a committed item with no time
# gets `at: "Unset"` and `ord: 99`, sorting last. Trellis requires a deadline
# for committed triage, so that state cannot arise here and chronological is
# total. `ord` is not built.
#
# A PAST DEADLINE STILL SHOWS, marked. A commitments screen that silently
# drops what you missed is the one failure it cannot have -- the product's
# value is that the owner trusts what it shows. It falls out of chronological
# order without special casing: past items are simply first.
#
# THE GAP THIS EXPOSES, raised rather than resolved: NOTHING IN TRELLIS CAN
# MARK A TASK DONE. So past items accumulate with no way to clear them, and
# this screen is the first place that becomes visible daily. Not this slice's
# to fix; worth an issue before the fortnight of dogfooding fills it.
Feature: The committed screen lists what has a date on it

  Background:
    Given the server believes it is "2026-08-24T09:00:00Z"
    And the trellis server is running with an empty task list

  Scenario: Committed items list in date order with their date, text and context tag
    Given a committed task "Q3 planning doc" tagged "@desk" due "2026-08-27T17:00:00Z" as an "at"
    And a committed task "Book the dentist" tagged "@phone" due "2026-08-25T08:30:00Z" as an "at"
    And a committed task "Furnace service window" with no context tag due "2026-08-28T13:00:00Z" as an "at"
    When the committed screen is viewed
    Then the committed screen lists "<order>"
    And the committed row "Book the dentist" shows the context "<ctx>"
    And the committed row "Furnace service window" shows no context
    And the committed screen reports "<meta>" beside its title

    Examples:
      | order                                                    | ctx    | meta    |
      | Book the dentist, Q3 planning doc, Furnace service window | @phone | 3 dated |

  # committed-screen-at-and-by-02: an at shows its time; a by shows that it is a by
  Scenario: An at shows its time; a by shows that it is a by
    Given a committed task "Book the dentist" with no context tag due "2026-08-25T08:30:00Z" as an "at"
    And a committed task "File the tax return" with no context tag due "2026-08-27T17:00:00Z" as a "by"
    When the committed screen is viewed
    Then the committed row "Book the dentist" shows the date "<at_cell>"
    And the committed row "File the tax return" shows the date "<by_cell>"

    Examples:
      | at_cell  | by_cell |
      | TUE 8:30 | BY THU  |

  # committed-screen-past-still-shows-03: a deadline that has passed is still listed, and marked
  Scenario: A deadline that has passed is still listed, and marked
    Given a committed task "Renew the passport" with no context tag due "2026-08-20T09:00:00Z" as a "by"
    And a committed task "Book the dentist" with no context tag due "2026-08-25T08:30:00Z" as an "at"
    When the committed screen is viewed
    Then the committed screen lists "<order>"
    And the committed row "Renew the passport" is marked as past
    And the committed row "Book the dentist" is not marked as past

    Examples:
      | order                                  |
      | Renew the passport, Book the dentist   |

  # committed-screen-only-committed-04: pool and quota work does not appear
  Scenario: Pool and quota work does not appear
    Given a committed task "Book the dentist" with no context tag due "2026-08-25T08:30:00Z" as an "at"
    And a pool task "buy screws" tagged "@homedepot"
    And a quota task "practise piano" tagged "@desk"
    When the committed screen is viewed
    Then the committed screen does not mention "buy screws"
    And the committed screen does not mention "practise piano"
    And the committed screen reports "<meta>" beside its title

    Examples:
      | meta    |
      | 1 dated |

  # committed-screen-empty-05: with nothing dated the screen says that is allowed
  Scenario: With nothing dated the screen says that is allowed
    When the committed screen is viewed
    Then the committed screen shows the message "<message>"
    And the committed screen offers a way back to Capture
    And the committed screen reports "<meta>" beside its title

    Examples:
      | message                                       | meta          |
      | Nothing with a time on it. That is allowed.   | nothing dated |

  # committed-screen-tabs-06: the tab bar offers three tabs and marks the current one
  Scenario: The tab bar offers three tabs and marks the current one
    When the "<screen>" screen is viewed
    Then the tab bar offers exactly "<tabs>"
    And the tab bar marks "<current>" as the current tab

    Examples:
      | screen    | tabs                      | current   |
      | capture   | Capture, Pool, Committed  | Capture   |
      | pool      | Capture, Pool, Committed  | Pool      |
      | committed | Capture, Pool, Committed  | Committed |

  # committed-screen-triage-takes-at-or-by-07: committed triage asks at or by, on every transport
  Scenario: Committed triage asks at or by, on every transport
    Given a capture with raw text "File the tax return" is waiting in the untriaged queue
    When the capture is triaged as a committed task through the "<transport>" with "commitment" omitted
    Then the triage is rejected
    And the rejection names "commitment"
    And the task list is still empty

    Examples:
      | transport |
      | page      |
      | api       |

  # committed-screen-escapes-hostile-text-08: hostile text stays escaped on this screen
  Scenario: Hostile text stays escaped on this screen
    Given a committed task "<script>alert('boom')</script>" tagged "@desk" due "2026-08-25T08:30:00Z" as an "at"
    When the committed screen is viewed
    Then the committed screen does not contain an unescaped "<script>" tag
    And the committed screen contains the word "boom"
