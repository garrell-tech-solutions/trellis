# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-13T21:28:30.411285268Z","feature_name":"Quota triage requires target count, target minutes each and period","feature_path":"features/quota_triage_validation.feature","background_hash":"d8cdc7c2933deb14809ad6a3cb7d7cdc60490c3c4e983e05b74bf8898673de1c","implementation_hash":"sha256:a6c5b4f9b71d7185c1b4db842c5e95b0dff8255ef6c4726f056173ffdc84597f","scenarios":[{"index":0,"name":"Triaging as quota without a required target field is rejected and creates nothing","scenario_hash":"4e761050ff966d6d4837a15cad090e29610d46fed9ceb580faea3233fc3c13a6","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-12T21:49:21.754594796Z"}]}
# acceptance-mutation-manifest-end

# quota-triage-validation-required-fields-01: quota triage is rejected when a required field is absent or left empty
# quota-triage-validation-target-must-be-positive-02: an hour target that is not a positive number of minutes is rejected
# quota-triage-validation-repeated-name-refused-03: a name that repeats an existing quota is refused
# quota-triage-validation-similar-name-warns-04: a name that merely resembles one warns, and can still be created
#
# THIS FILE CHANGED SUBJECT, and the old subject is gone rather than moved.
# It used to assert `target_count`, `target_minutes_each` and `period` --
# THE THREE FIELDS THIS SLICE RETIRES. Every one of its scenarios would
# otherwise have become a scenario asserting nothing, which is #90's exact
# trap: "if a scenario's point is that nothing happens, its parameters are
# not under test". A validation scenario whose rejected fields stop existing
# does not become a passing scenario. So the file keeps its path and its QA
# document, and states what quota triage requires NOW.
#
# --- WHAT THE OWNER SETTLED 2026-08-26, AND WHAT IT REVERSES --------------
# THE OWNER READ THE BRIEF'S MODEL AND REJECTED IT: "a quota is a task and
# should be fully creatable from the task in queuing and triage screen;
# there should not be any sort of way to create a quota in the quota screen
# -- the quota screen is only displaying the quotas that you have inserted
# as a task and triaged as a quota."
#
# So TRIAGING A CAPTURE AS A QUOTA IS WHAT CREATES THE QUOTA. That is how
# `workout` and `learning with lev` reached the owner's database in the
# first place, and the complaint that opened #138 -- "it doesn't show any of
# the things that I've triaged as a quota" -- is a complaint that the
# product forgot its own front door.
#
# THREE CONSEQUENCES, NAMED RATHER THAN QUIET:
#   1. `D-quotas-are-selected-not-typed`'s "can be done directly from the
#      Menu WITHOUT A CAPTURE" IS SUPERSEDED. There is one door and it is
#      triage. `+ Define a new quota` leaves the quota screen entirely, and
#      five of `quota_screen.feature`'s nine scenarios come here instead of
#      being deleted -- THE RULES SURVIVE, THE SURFACE MOVED.
#   2. THE BRIEF'S FILING MODEL IS DEFERRED, not built. Chips, `Filed here`
#      and choosing an existing quota at triage are all out; the owner:
#      "this seems like something that could come later (combining quotas)
#      ... having too many quotas is bounded by the amount of time
#      available." EVERY QUOTA TRIAGE MAKES ITS OWN QUOTA.
#   3. `T-three-task-kinds` AND `T-unknown-kind-rejected` DO NOT MOVE. The
#      brief called this a trap and asked whether `kind=quota` becomes an
#      unknown kind. IT DOES NOT: `quota` is still one of the three kinds
#      and it is now the verb that CREATES a quota. `unknown_kind_rejection`
#      .feature is untouched, and that is a finding, not an oversight.
#
# --- THE NAME IS THE CAPTURE'S OWN WORDS, AND YOU CAN CHANGE THEM ---------
# Settled by the owner 2026-08-26. The panel prefills the name with the
# capture's text -- so the common case is "type the hours and go" -- and
# lets you edit it. BOTH FIELDS ARE STILL REQUIRED (-01), because the
# alternative was a name that can never be missing, and then a cleared name
# box silently resurrects words you deliberately deleted.
#
# THE COST THIS BUYS OFF is why the box is editable at all: RENAMING A QUOTA
# IS #148 AND IS NOT BUILT, so whatever name triage writes is the name
# forever. A capture worded as a one-off -- "finish chapter 3" -- would
# otherwise become a permanent quota called that, and a name that collides
# with an existing quota (-03) would have no way out but re-capturing.
#
# --- THE NAME GUARD MOVES HOUSE UNCHANGED --------------------------------
# `scheduler_core::quota::check_name` already states both tiers and #147
# proved them; -03 and -04 are `quota-screen-repeated-name-refused-06` and
# `-similar-name-warns-07` with the surface swapped. The reasoning is
# untouched and is the whole rule: a typo does not mis-file an item, it
# CREATES A SECOND COUNTER THAT SILENTLY SPLITS THE WEEK'S HOURS AND MAKES
# BOTH WRONG.
#
# ONE MESSAGE HAD TO CHANGE AND IT IS A CONSEQUENCE, NOT A TIDY-UP. The
# refusal used to read "File it there instead of making a second one." WITH
# FILING DEFERRED THERE IS NOWHERE TO FILE IT, so the sentence would be
# instructing the owner to use a control that does not exist. It now names
# the two things that ARE possible: log against the existing quota, or give
# this one a different name.
#
# -01 FOLDS ABSENT AND EMPTY INTO ONE TABLE, and the IR DRY checker is why:
# the two scenarios this started as were IDENTICAL ONCE PLACEHOLDER NAMES
# WERE GENERICISED, which is the checker's own highest-confidence finding.
# `T-empty-equals-absent` says the two report identically, so a single table
# varying BOTH the field and how its value failed to arrive STATES THAT RULE
# rather than restating one scenario twice with a different variable name.
#
# -02 FOLDS FOUR REJECTIONS INTO ONE SCENARIO because they are one rule --
# `parse_weekly_target` refuses anything that is not a positive whole number
# of minutes. `0.004` IS THE ROW WORTH KEEPING: it is positive, it parses,
# and it rounds to zero minutes, so it is the only row that reaches
# `WeeklyTarget::from_minutes`'s own guard rather than
# `parse_positive_hours`'s. Drop it and that guard is untested.
Feature: Triaging a capture as a quota requires a name and an hour target

  Background:
    Given the trellis server is running with an empty task list
    And a capture with raw text "go to the gym" is waiting in the untriaged queue

  Scenario: Triaging as a quota without a usable name or hour target is rejected and creates nothing
    When the capture is triaged as a quota with "<field>" <absence>
    Then the triage is rejected
    And the rejection names "<field>"
    And the quota screen offers no quotas
    And the capture is still waiting in the untriaged queue

    Examples:
      | field | absence    |
      | name  | omitted    |
      | name  | left empty |
      | hours | omitted    |
      | hours | left empty |

  # quota-triage-validation-target-must-be-positive-02: an hour target that is not a positive number of minutes is rejected
  Scenario: An hour target that is not a positive number of minutes is rejected
    When the capture is triaged as a quota named "Gym" with a target of "<hours>" hours a week
    Then the triage is rejected
    And the rejection reports "hours" as invalid
    And the quota screen offers no quotas
    And the capture is still waiting in the untriaged queue

    Examples:
      | hours |
      | 0     |
      | -2    |
      | four  |
      | 0.004 |

  # quota-triage-validation-repeated-name-refused-03: a name that repeats an existing quota is refused
  Scenario: A name that repeats an existing quota is refused
    Given a quota named "Piano" with a target of "4" hours a week
    When the capture is triaged as a quota named "<name>" with a target of "2" hours a week
    Then the triage is rejected
    And the triage warns "“Piano” already exists at 4 h a week. Log your time against that one, or give this a different name."
    And the quota screen offers the quotas "<quotas>"
    And the capture is still waiting in the untriaged queue

    Examples:
      | name   | quotas |
      | piano  | Piano  |
      | PIANO  | Piano  |
      | Pi-ano | Piano  |
      | pi ano | Piano  |

  # quota-triage-validation-similar-name-warns-04: a name that merely resembles one warns, and can still be created
  Scenario: A name that merely resembles one warns, and can still be created
    Given a quota named "Piano" with a target of "4" hours a week
    When the capture is triaged as a quota named "<name>" with a target of "<hours>" hours a week
    Then the triage is rejected
    And the triage warns "That reads a lot like “Piano” (4 h a week). Same thing?"
    And the triage offers a create control named "<control>"
    When the capture is triaged as a quota named "<name>" with a target of "<hours>" hours a week again
    And the "quota" screen is viewed
    Then the quota screen offers the quotas "<quotas>"

    Examples:
      | name         | hours | control       | quotas              |
      | Pianoo       | 2     | Create anyway | Piano, Pianoo       |
      | Piano theory | 1     | Create anyway | Piano, Piano theory |
