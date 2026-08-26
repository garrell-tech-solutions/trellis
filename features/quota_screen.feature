# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-25T23:40:51.957315478Z","feature_name":"The quota screen, and defining a quota","feature_path":"features/quota_screen.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:6a7bc25388ad53cb77b3b29b96c83ab6125365eabfbcd36fbee7ef3586011740","scenarios":[{"index":0,"name":"The fourth screen exists and names itself","scenario_hash":"37ae0666897ec22a83ade2ed8ba5b68c6a6d9f50929d36a9b50adb7882a9fe88","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-25T23:40:51.957315478Z"},{"index":1,"name":"An empty quota screen says what a quota is and offers to define one","scenario_hash":"1f0e8a0f90fd8f824711c417659ea233e833b063315690479865503acac25cd5","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-25T23:40:51.957315478Z"},{"index":2,"name":"Defining a quota needs both a name and an hour target","scenario_hash":"bd0e2ca65f07376372ce4e1402bd48be5281e2bcd1bbe42b716ddcd93e9f50c7","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-25T23:40:51.957315478Z"},{"index":3,"name":"An hour target of zero or less is not a quota","scenario_hash":"44fbcfe2d89ffd54261545241546c7709e5594091fb65e3c388010ac8da6c569","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-25T23:40:51.957315478Z"},{"index":4,"name":"A newly defined quota reads its target with nothing logged yet","scenario_hash":"0c73adf4f432d8fdd129a1e23f2acf2aa7e95c6efd274fcb19a8a51ffe38b8f2","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-25T23:40:51.957315478Z"},{"index":6,"name":"A name that merely resembles one warns, and can still be created","scenario_hash":"a22fcb4c39eb0a5a735f891a4d35afb6183311338291785fce3aff28ba687ace","mutation_count":8,"result":{"Total":8,"Killed":8,"Survived":0,"Errors":0},"tested_at":"2026-08-25T23:40:51.957315478Z"},{"index":7,"name":"Quotas are listed in the order they were defined","scenario_hash":"badfce53c87b5b6cf53317935ee40f074f2a21ccabce9ce0a0b09c1d1ead3294","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-25T23:40:51.957315478Z"},{"index":8,"name":"Quota tasks from triage are a different thing and do not appear here","scenario_hash":"877094a21ff9310d2bf455f910c38637adf26357ea60740e9fe27012f1442d48","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-25T23:40:51.957315478Z"}]}
# acceptance-mutation-manifest-end

# quota-screen-fourth-screen-01: the fourth screen exists and names itself
# quota-screen-empty-explains-itself-02: an empty quota screen says what a quota is and offers to define one
# quota-screen-both-fields-required-03: defining a quota needs both a name and an hour target
# quota-screen-target-must-be-positive-04: an hour target of zero or less is not a quota
# quota-screen-reads-its-target-05: a newly defined quota reads its target with nothing logged yet
# quota-screen-repeated-name-refused-06: a name that repeats an existing quota is refused
# quota-screen-similar-name-warns-07: a name that merely resembles one warns, and can still be created
# quota-screen-defined-order-08: quotas are listed in the order they were defined
# quota-screen-triaged-quotas-are-elsewhere-09: quota tasks from triage are a different thing and do not appear here
#
# A QUOTA IS NOT A TASK, and that is the finding this slice turns on.
# `TaskKind::Quota { target_count, target_minutes_each, period }` makes a
# quota a TRIAGED CAPTURE counting sessions. The canvas and
# `D-quotas-are-selected-not-typed` describe something else: a NAMED
# CONTAINER WITH A WEEKLY HOUR TARGET, created from this screen without a
# capture, that sessions are logged against. So this is a new first-class
# entity, not a reshape of three columns.
#
# `TaskKind::Quota` IS NOT THIS SLICE'S TO REMOVE. It stays exactly as it
# is, the existing quota triage form keeps working, and TWO QUOTA CONCEPTS
# COEXIST until #138 closes that. `-09` is the boundary between them,
# asserted rather than assumed: a triaged quota task must not leak onto this
# screen, and nothing here migrates or displays one. Say nothing about
# `period`; its fate is #138's.
#
# --- THE NAME GUARD, SETTLED BY THE OWNER 2026-08-25 ----------------------
# `D-quotas-are-selected-not-typed` says a mistyped name must not be able to
# create a quota, and the reasoning is the whole rule: a typo does not
# mis-file an item, it CREATES A SECOND COUNTER THAT SILENTLY SPLITS THE
# WEEK'S HOURS AND MAKES BOTH WRONG. The canvas implements two tiers at
# `Trellis.dc.html:427-436` and the owner confirmed both:
#
#   REFUSED -- the same name once case, spaces and punctuation are ignored.
#     "piano", "PIANO", "Pi-ano" and "pi ano" are all "Piano" (-06).
#   WARNED, AND STILL POSSIBLE -- within two letters of an existing name, or
#     one name contained in the other. "Pianoo" is a typo; "Piano theory" may
#     genuinely be a second quota, and refusing it would be wrong (-07).
#
# THE RULE LIVES IN THE CORE AND THE COLLATION IS THE BACKSTOP, which is the
# path `T-collation-enforces-name-identity` names for exactly this case:
# "if the rule ever outgrows a collation -- Unicode folding -- it moves into
# the core and the constraint becomes the backstop". Stripping punctuation
# and measuring edit distance are past what `COLLATE NOCASE` can say, so the
# rule is Rust; `UNIQUE COLLATE NOCASE` still belongs on the column, because
# that decision's own reason stands -- A CONSTRAINT THE DATABASE ENFORCES
# CANNOT BE BYPASSED BY A WRITE PATH THAT FORGOT TO CALL SOMETHING.
#
# WHAT THIS FILE ASSERTS ABOUT THE WARNING IS WHAT HTTP SEES: that a refused
# name comes back 422 with the re-rendered form (`T-422-is-product-wide`,
# nothing new invented), that the warning names the quota it clashed with,
# and that a warned-about name goes through when the definition is repeated.
# WHETHER THE WARNING ALSO APPEARS LIVE AS YOU TYPE IS THE BROWSER TIER'S,
# and it is in `qa/quota_screen.md` --
# `T-ephemeral-view-state-rides-the-request` says to choose the tier from the
# nature of the state, and a
# warning you have not submitted yet is the definition of ephemeral. A
# scenario here asserting it would push it into a request or a column, which
# is #126's mistake with a new subject.
#
# ORDER IS THE ORDER YOU DEFINED THEM IN (-08), oldest first, new ones at the
# bottom. Read from the canvas rather than chosen: it sorts quota rows by
# `q.pri` (`Trellis.dc.html:856`) and `createQuota` appends, so with the
# priority arrows out of scope the sort is stable and degrades to exactly
# this. THE ARROWS ARE #139 AND ARE NOT DRAWN HERE -- and their absence is
# NOT asserted as a rule the way `pool-screen-nothing-reorders-05` does,
# because on a quota that absence is UNDECIDED rather than settled.
#
# ONE EXISTING SCENARIO CHANGES: `committed-screen-tabs-06` asserted three
# tabs. `nav.rs:21` has carried `ALL: [Page; 3]` and the comment "a dead
# link is worse than no link" since #92, waiting for this. It now asserts
# four, with a fourth example row. THE BRIEF ASKED FOR 23 UNTOUCHED FEATURES
# AND THERE ARE 24 -- `trip_persistence` landed as the 24th while this brief
# was being written -- SO THE COUNT IS 25 WITH THIS FILE, 26 WITH
# `quota_sessions`, AND ONE OF THEM IS EDITED. Named rather than quiet.
Feature: The quota screen, and defining a quota

  Background:
    Given the trellis server is running with an empty task list

  Scenario: The fourth screen exists and names itself
    When the "quota" screen is viewed
    Then the quota screen reports "<meta>" beside its title
    And the tab bar marks "<current>" as the current tab

    Examples:
      | meta     | current |
      | none yet | Quota   |

  # quota-screen-empty-explains-itself-02: an empty quota screen says what a quota is and offers to define one
  Scenario: An empty quota screen says what a quota is and offers to define one
    When the "quota" screen is viewed
    Then the quota screen offers no quotas
    And the quota screen notes "<note>"
    And the quota screen offers a define control named "<control>"

    Examples:
      | note                                                                                  | control              |
      | A quota is a weekly hour target you keep — practice, study, running. Define one below. | + Define a new quota |

  # quota-screen-both-fields-required-03: defining a quota needs both a name and an hour target
  Scenario: Defining a quota needs both a name and an hour target
    When a quota is defined with "<missing_field>" omitted
    Then the definition is rejected
    And the quota screen offers no quotas

    Examples:
      | missing_field |
      | name          |
      | hours         |

  # quota-screen-target-must-be-positive-04: an hour target of zero or less is not a quota
  Scenario: An hour target of zero or less is not a quota
    When a quota named "Piano" with a target of "<hours>" hours a week is defined
    Then the definition is rejected
    And the quota screen offers no quotas

    Examples:
      | hours |
      | 0     |
      | -2    |

  # quota-screen-reads-its-target-05: a newly defined quota reads its target with nothing logged yet
  Scenario: A newly defined quota reads its target with nothing logged yet
    When a quota named "Piano" with a target of "<hours>" hours a week is defined
    And the "quota" screen is viewed
    Then the quota "Piano" reads "<readout>"
    And the quota "Piano" notes "<note>"
    And the quota screen reports "<meta>" beside its title

    Examples:
      | hours | readout | note                          | meta    |
      | 4     | 0m / 4h | 4h left this week · 0%      | 1 quota |

  # quota-screen-repeated-name-refused-06: a name that repeats an existing quota is refused
  Scenario: A name that repeats an existing quota is refused
    Given a quota named "Piano" with a target of "4" hours a week
    When a quota named "<name>" with a target of "2" hours a week is defined
    Then the definition is rejected
    And the new-quota form warns "“Piano” already exists at 4 h a week. File it there instead of making a second one."
    And the quota screen offers the quotas "<quotas>"

    Examples:
      | name   | quotas |
      | piano  | Piano  |
      | PIANO  | Piano  |
      | Pi-ano | Piano  |
      | pi ano | Piano  |

  # quota-screen-similar-name-warns-07: a name that merely resembles one warns, and can still be created
  Scenario: A name that merely resembles one warns, and can still be created
    Given a quota named "Piano" with a target of "4" hours a week
    When a quota named "<name>" with a target of "<hours>" hours a week is defined
    Then the definition is rejected
    And the new-quota form warns "That reads a lot like “Piano” (4 h a week). Same thing?"
    And the new-quota form offers a create control named "<control>"
    When a quota named "<name>" with a target of "<hours>" hours a week is defined again
    And the "quota" screen is viewed
    Then the quota screen offers the quotas "<quotas>"

    Examples:
      | name         | hours | control       | quotas              |
      | Pianoo       | 2     | Create anyway | Piano, Pianoo       |
      | Piano theory | 1     | Create anyway | Piano, Piano theory |

  # quota-screen-defined-order-08: quotas are listed in the order they were defined
  Scenario: Quotas are listed in the order they were defined
    Given a quota named "Piano" with a target of "4" hours a week
    And a quota named "Running" with a target of "3" hours a week
    And a quota named "Rust" with a target of "5" hours a week
    When the "quota" screen is viewed
    Then the quota screen offers the quotas "<quotas>"
    And the quota screen reports "<meta>" beside its title

    Examples:
      | quotas               | meta     |
      | Piano, Running, Rust | 3 quotas |

  # quota-screen-triaged-quotas-are-elsewhere-09: quota tasks from triage are a different thing and do not appear here
  Scenario: Quota tasks from triage are a different thing and do not appear here
    Given a quota task "practise piano" tagged "@home"
    And a quota named "Piano" with a target of "4" hours a week
    When the "quota" screen is viewed
    Then the quota screen does not mention "practise piano"
    And the quota screen offers the quotas "<quotas>"
    And the quota screen reports "<meta>" beside its title

    Examples:
      | quotas | meta    |
      | Piano  | 1 quota |
