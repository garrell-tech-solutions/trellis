# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-26T03:02:42.741462790Z","feature_name":"Logging hours against a quota","feature_path":"features/quota_sessions.feature","background_hash":"e529841f2d0fd9312968340601b47ea1f6598d5904d2dd216548ffe6cc46a95e","implementation_hash":"sha256:953fcc31bca500a22d47ce53960e5a79420606eebeca0129783c9015ba763149","scenarios":[{"index":0,"name":"A quick-log tap counts against today and moves the readout","scenario_hash":"052d1c7c52bf04aa3be5e45a8198ffd22dce2cdc87d301794b1812ac669d5ba0","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-26T03:02:42.741462790Z"},{"index":1,"name":"An earlier day this week is logged through Other and counts the same","scenario_hash":"586ed7158e148f4a71f61532e275e072ecaa098497b8094eb70cfeb8abece176","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-26T03:02:42.741462790Z"},{"index":2,"name":"The day picker offers only days that have already happened","scenario_hash":"0f9db7af11460bcfbafabd0cb6be5ce4aaa50524f4a5ce0d4bf1d3716e3f646e","mutation_count":8,"result":{"Total":8,"Killed":8,"Survived":0,"Errors":0},"tested_at":"2026-08-26T03:02:42.741462790Z"},{"index":3,"name":"This week lists each session and summarises them","scenario_hash":"166e7ba926bfff24f1daa3ae8fce65961dbc2e4d3dbc405ab91d398d10c606e2","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-26T03:02:42.741462790Z"},{"index":4,"name":"A week with nothing logged says so","scenario_hash":"c9bc31e962b7c66c0c03be7ddf6bc5a05b141417aca60f926dab4e1ef3e5dc3b","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-26T03:02:42.741462790Z"},{"index":6,"name":"Deleting a session takes its minutes with it","scenario_hash":"44e3daf985be9ad8af4af50f95920150e9322d6abd373a7f9a929199568b2e2e","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-26T03:02:42.741462790Z"},{"index":7,"name":"Monday starts again at zero","scenario_hash":"a1a530ac53ec66e8b36ee45159658bfb0487de9199b5a855235045da48cb364f","mutation_count":5,"result":{"Total":5,"Killed":5,"Survived":0,"Errors":0},"tested_at":"2026-08-26T03:02:42.741462790Z"}]}
# acceptance-mutation-manifest-end

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
# LOGGING IS RETROSPECTIVE, AND -03 IS WHERE THAT BECOMES A RULE. On Tuesday
# the day picker offers Monday and Tuesday, and the list grows as the week
# does: YOU CANNOT RECORD TIME YOU HAVE NOT DONE YET.
# D-logging-is-retrospective-and-separate rejected a start/stop timer because
# an unstarted one reports zero -- inaction producing a false number rather
# than a missing one. Offering Saturday on Tuesday is that failure inverted:
# four hours against a day that has not happened fills the bar and the week
# reads as met. The canvas draws all seven days, but `days: DAYS` is a static
# constant beside `const TODAY = "Tue"`, so the mock could not have drawn this
# distinction either way -- a gap under
# T-canvas-is-authoritative-where-it-speaks, not a statement.
#
# MONDAY, AND THE HOUR THAT HAS NOWHERE TO GO. Under D-quota-no-rollover a week
# is a closed box and -08 asserts it empties. The consequence, worth seeing
# before it is met in use: you cannot log Sunday evening's practice on Monday
# morning. THE OWNER WAS SHOWN THIS AND KEPT THE DECISION.
#
# "MONDAY" NEEDS A TIMEZONE and this project already has one --
# T-timezone-is-a-setting, the path committed/body.rs takes. Do not reach for
# UTC and do not add a second notion of the owner's zone. KNOW THE TRAP YOU ARE
# INHERITING: #118 is open because `/timezone` is POST-only and reachable from
# no page; the live database reads America/New_York but THE SCHEMA DEFAULT IS
# `UTC`, and this slice makes a second capability depend on a setting nobody
# can edit.
#
# WHICH ROW IS EXPANDED, AND WHETHER ITS `Other` PANEL IS OPEN, ARE NOT
# ASSERTED HERE -- exactly the state that must not buy a column
# (T-ephemeral-view-state-rides-the-request). A logged session is the opposite:
# a durable consequence of a deliberate act, and it earns its table. Read
# D-a-trip-survives-being-tidied beside it -- that test says when storage is
# legitimate, never that it is required.

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
