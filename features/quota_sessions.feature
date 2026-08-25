# quota-sessions-quick-log-counts-today-01: a quick-log tap counts against today and moves the readout
# quota-sessions-other-logs-an-earlier-day-02: an earlier day this week is logged through Other and counts the same
# quota-sessions-only-days-that-have-happened-03: the day picker offers only days that have already happened
# quota-sessions-this-week-lists-what-was-logged-04: this week lists each session and summarises them
# quota-sessions-an-empty-week-says-so-05: a week with nothing logged says so
# quota-sessions-correcting-a-session-06: correcting a session's day or minutes changes the total
# quota-sessions-deleting-a-session-07: deleting a session takes its minutes with it
# quota-sessions-nothing-carries-into-next-week-08: Monday starts again at zero
# quota-sessions-a-session-must-be-positive-09: a session of no minutes is not a session
#
# THIS FILE IS THE HALF THE OWNER NAMED AS DEFERRABLE, AND IT IS A WHOLE
# FILE ON PURPOSE. The brief's "if it does not fit" line is: the entity, the
# screen, the tab and `+ Define a new quota` land; the session surface does
# not. Splitting there means stopping is DEFERRING ONE FEATURE FILE rather
# than leaving a half-red one -- `quota_screen.feature` stands alone and
# passes without a line of this. WHAT IS NOT ACCEPTABLE IS SHIPPING PART OF
# THIS FILE: a `+30m` that writes nothing, a `This week` that cannot delete,
# or a Monday reset that is a TODO. Ship the smaller true thing.
#
# --- LOGGING IS RETROSPECTIVE, AND -03 IS WHERE THAT BECOMES A RULE -------
# Settled by the owner 2026-08-25. ON TUESDAY THE DAY PICKER OFFERS MONDAY
# AND TUESDAY, and the list grows as the week does. YOU CANNOT RECORD TIME
# YOU HAVE NOT DONE YET.
#
# `D-logging-is-retrospective-and-separate` rejected a start/stop timer
# because "an unstarted timer silently reports zero -- INACTION PRODUCING A
# FALSE NUMBER RATHER THAN A MISSING ONE". Offering Saturday on Tuesday is
# that same failure inverted: four hours logged against a day that has not
# happened fills the bar and the week reads as met. A number that looks true
# and is not is the one thing this decision exists to prevent, and it does
# not care which direction the lie runs in.
#
# THE CANVAS DRAWS ALL SEVEN DAYS AND THAT IS A GAP, NOT A STATEMENT
# (`T-canvas-is-authoritative-where-it-speaks`, checked rather than assumed).
# `days: DAYS` is a static constant beside `const TODAY = "Tue"`; the mock
# has no notion of the week passing, so it could not have drawn this
# distinction whether or not the design wanted it. That is the NINTH gap the
# canvas has left. Flagged, not filled silently.
#
# --- MONDAY, AND THE HOUR THAT HAS NOWHERE TO GO -------------------------
# `D-quota-no-rollover`: counters reset Monday and shortfalls never carry
# forward. So a week is a closed box, and -08 asserts it empties.
#
# A CONSEQUENCE WORTH SEEING BEFORE IT IS FOUND IN USE: you cannot log
# Sunday evening's practice on Monday morning. Monday is a new week, last
# week has been reckoned, and there is nowhere for that hour to go. THE
# OWNER WAS SHOWN THIS AND KEPT THE DECISION; it is recorded here so that
# whoever meets it in the shop knows it was chosen rather than missed.
#
# "MONDAY" NEEDS A TIMEZONE and this project already has one:
# `settings::current_timezone` + `scheduler_core::timezone::resolve`, the
# path `committed/body.rs:30-32` takes (`T-timezone-is-a-setting`). DO NOT
# reach for UTC and do not add a second notion of the owner's zone. KNOW THE
# TRAP YOU ARE INHERITING: #118 is open because `/timezone` is POST-only and
# reachable from no page. The live database reads `America/New_York` so the
# owner is fine today, BUT THE SCHEMA DEFAULT IS `UTC`
# (`0006_guardrails.sql:39`), and this slice makes a SECOND capability
# depend on a setting nobody can edit. That is a note for the handoff, not a
# licence to fix #118 here.
#
# --- WHAT IS NOT ASSERTED HERE -------------------------------------------
# WHICH ROW IS EXPANDED, AND WHETHER ITS `Other` PANEL IS OPEN, ARE EXACTLY
# THE STATE THAT MUST NOT BUY A COLUMN (`T-ephemeral-view-state-rides-the-
# request`, `740d224`). `expanded=<tags>` on the pool is the worked example
# and `scripts/qa/trip_controls.cjs` step 10 is the pattern; both live in
# `qa/quota_sessions.md`. A LOGGED SESSION IS THE OPPOSITE -- a durable
# consequence of a deliberate act, and it earns its table.
#
# AND READ `D-a-trip-survives-being-tidied` BESIDE IT (`cb02b3a`, settled in
# #129 the same day): a column there was PERMITTED by that test and DERIVED
# ANYWAY, because deriving cost one slice's thought and a column is
# permanent. THE TEST SAYS WHEN STORAGE IS LEGITIMATE. IT NEVER SAYS IT IS
# REQUIRED. A session is the clearest yes this product has had; the expand
# state is a clear no; decide anything between them on its own merits.
#
# ALSO NOT HERE: `Filed here` and the triage change (#138), retiring
# `TaskKind::Quota` and deciding `period` (#138), and the reorder controls
# (#139) -- whose absence is NOT asserted, because on a quota that absence
# is undecided rather than settled.

Feature: Logging hours against a quota

  Background:
    Given the trellis server is running with an empty task list
    And the owner's timezone is "America/New_York"
    And the server believes it is "2026-08-25T14:00:00Z"
    And a quota named "Piano" with a target of "4" hours a week

  Scenario: A quick-log tap counts against today and moves the readout
    When "<control>" is tapped on the quota "Piano"
    And the "quota" screen is viewed
    Then the quota "Piano" reads "<readout>"
    And the quota "Piano" notes "<note>"

    Examples:
      | control | readout   | note                                |
      | +30m    | 30m / 4h  | 3h 30m left this week · 13%        |
      | +1h     | 1h / 4h   | 3h left this week · 25%            |

  # quota-sessions-other-logs-an-earlier-day-02: an earlier day this week is logged through Other and counts the same
  Scenario: An earlier day this week is logged through Other and counts the same
    When a session of "<mins>" minutes on "<day>" is logged against "Piano"
    And the "quota" screen is viewed
    Then the quota "Piano" reads "<readout>"
    And this week for "Piano" lists "<sessions>"

    Examples:
      | mins | day | readout  | sessions |
      | 20   | Mon | 20m / 4h | Mon 20m  |

  # quota-sessions-only-days-that-have-happened-03: the day picker offers only days that have already happened
  Scenario: The day picker offers only days that have already happened
    Given the server believes it is "<now>"
    When the "quota" screen is viewed
    Then logging a session against "Piano" offers the days "<days>"

    Examples:
      | now                  | days                              |
      | 2026-08-24T14:00:00Z | Mon                               |
      | 2026-08-25T14:00:00Z | Mon, Tue                          |
      | 2026-08-28T14:00:00Z | Mon, Tue, Wed, Thu, Fri           |
      | 2026-08-30T14:00:00Z | Mon, Tue, Wed, Thu, Fri, Sat, Sun |

  # quota-sessions-this-week-lists-what-was-logged-04: this week lists each session and summarises them
  Scenario: This week lists each session and summarises them
    Given a session of "25" minutes on "Mon" is logged against "Piano"
    And a session of "35" minutes on "Tue" is logged against "Piano"
    When the "quota" screen is viewed
    Then this week for "Piano" lists "<sessions>"
    And this week for "Piano" summarises "<summary>"
    And the quota "Piano" reads "<readout>"

    Examples:
      | sessions         | summary             | readout |
      | Mon 25m, Tue 35m | 2 sessions · 1h    | 1h / 4h |

  # quota-sessions-an-empty-week-says-so-05: a week with nothing logged says so
  Scenario: A week with nothing logged says so
    When the "quota" screen is viewed
    Then this week for "Piano" says "<message>"
    And this week for "Piano" summarises "<summary>"

    Examples:
      | message                                                    | summary        |
      | No sessions yet this week. Log one above when you have done it. | nothing logged |

  # quota-sessions-correcting-a-session-06: correcting a session's day or minutes changes the total
  Scenario: Correcting a session's day or minutes changes the total
    Given a session of "<mins>" minutes on "<day>" is logged against "Piano"
    When that session is corrected to "<new_mins>" minutes on "<new_day>"
    And the "quota" screen is viewed
    Then this week for "Piano" lists "<sessions>"
    And the quota "Piano" reads "<readout>"

    Examples:
      | mins | day | new_mins | new_day | sessions | readout  |
      | 25   | Mon | 45       | Tue     | Tue 45m  | 45m / 4h |

  # quota-sessions-deleting-a-session-07: deleting a session takes its minutes with it
  Scenario: Deleting a session takes its minutes with it
    Given a session of "25" minutes on "Mon" is logged against "Piano"
    And a session of "35" minutes on "Tue" is logged against "Piano"
    When the session on "<day>" for "Piano" is deleted
    And the "quota" screen is viewed
    Then this week for "Piano" lists "<sessions>"
    And the quota "Piano" reads "<readout>"

    Examples:
      | day | sessions | readout  |
      | Mon | Tue 35m  | 35m / 4h |

  # quota-sessions-nothing-carries-into-next-week-08: Monday starts again at zero
  Scenario: Monday starts again at zero
    Given a session of "25" minutes on "Mon" is logged against "Piano"
    And a session of "35" minutes on "Tue" is logged against "Piano"
    When the "quota" screen is viewed
    Then the quota "Piano" reads "<readout>"
    When the server believes it is "<next_week>"
    And the "quota" screen is viewed
    Then the quota "Piano" reads "<next_readout>"
    And this week for "Piano" says "<message>"
    And the quota screen offers the quotas "<quotas>"

    Examples:
      | readout | next_week            | next_readout | message                                                    | quotas |
      | 1h / 4h | 2026-08-31T14:00:00Z | 0m / 4h      | No sessions yet this week. Log one above when you have done it. | Piano  |

  # quota-sessions-a-session-must-be-positive-09: a session of no minutes is not a session
  Scenario: A session of no minutes is not a session
    When a session of "<mins>" minutes on "Mon" is logged against "Piano"
    Then the session is rejected
    When the "quota" screen is viewed
    Then this week for "Piano" summarises "<summary>"

    Examples:
      | mins | summary        |
      | 0    | nothing logged |
      | -30  | nothing logged |
