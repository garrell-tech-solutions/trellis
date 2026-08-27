# mutation-stamp: sha256=3c50b374e258cb15325eb990367bcddfd29b5cb4bb6626e0d9cf6b85a549a921
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-27T04:26:46.417420996Z","feature_name":"The quota screen shows what you triaged as a quota","feature_path":"features/quota_screen.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:6a7bc25388ad53cb77b3b29b96c83ab6125365eabfbcd36fbee7ef3586011740","scenarios":[{"index":0,"name":"The fourth screen exists and names itself","scenario_hash":"37ae0666897ec22a83ade2ed8ba5b68c6a6d9f50929d36a9b50adb7882a9fe88","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-26T17:12:08.066627455Z"},{"index":1,"name":"An empty quota screen says what a quota is and points at Capture","scenario_hash":"97590824a00148e79a394a6225928241155bbd87b156826984106c56b11029eb","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-26T17:12:08.066627455Z"},{"index":2,"name":"A triaged quota reads its target with nothing logged yet","scenario_hash":"0c0cdd36fd656b1ad4da67629bde7c1a8abc9d20fd3ac1f67236b9b3c24b2866","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-26T17:12:08.066627455Z"},{"index":3,"name":"Quotas are listed in the order they were triaged","scenario_hash":"98a19d60e6f6ba4e135abe18fa86710938e7a56d616ebf3e8e4b0d18f2f6e17f","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-26T17:12:08.066627455Z"}]}
# acceptance-mutation-manifest-end

# quota-screen-fourth-screen-01: the fourth screen exists and names itself
# quota-screen-empty-explains-itself-02: an empty quota screen says what a quota is and points at Capture
# quota-screen-reads-its-target-03: a triaged quota reads its target with nothing logged yet
# quota-screen-triaged-order-04: quotas are listed in the order they were triaged
#
# --- THIS SCREEN NO LONGER CREATES ANYTHING ------------------------------
# Settled by the owner 2026-08-26, and it reverses the larger half of #147:
# "there should not be any sort of way to create a quota in the quota screen
# -- the quota screen is only displaying the quotas that you have inserted
# as a task and triaged as a quota."
#
# `+ Define a new quota` AND ITS WHOLE FORM LEAVE THIS SCREEN. Five of the
# nine scenarios this file shipped with go with them, and NOT ONE OF THEM IS
# DELETED: `-03` both-fields-required, `-04` target-must-be-positive, `-06`
# repeated-name-refused and `-07` similar-name-warns are now
# `quota_triage_validation.feature`'s `-01`/`-02`, `-03`, `-04` and `-05`
# respectively. THE RULES SURVIVED, THE SURFACE MOVED, and the guard that
# `D-quotas-are-selected-not-typed` exists for is enforced at the one door
# that remains. That decision's own "can be done directly from the Menu
# WITHOUT A CAPTURE" clause is superseded; the PM places the dated row.
#
# `-09` IS THE ONE SCENARIO THAT IS GONE OUTRIGHT, and it is gone because it
# has been INVERTED rather than weakened. It asserted "quota tasks from
# triage are a different thing and do not appear here" -- the boundary
# between two coexisting quota concepts, asserted deliberately while that
# state was knowingly accepted. #138 CLOSES THAT STATE: a triaged quota is
# now the ONLY thing this screen can show, so the assertion's replacement is
# every remaining scenario in this file, all of which now arrive by triage.
# Keeping a narrowed version would be asserting the opposite of the product.
#
# --- WHAT IS LEFT IS A SCREEN THAT READS -------------------------------
# Display and session logging, nothing else. `quota_sessions.feature` is
# UNTOUCHED by this slice: not one of its scenarios cares how a quota came
# to exist, so its `Given a quota named "Piano" with a target of "4" hours a
# week` keeps its wording and changes only which door its handler goes
# through. THAT IS THE TEST OF WHETHER A GIVEN WAS WRITTEN AS STATE OR AS
# IMPLEMENTATION, and it passed.
#
# -02 SAYS "POINT AT CAPTURE" AND MEANS IT LITERALLY. With no define control
# here, an empty quota screen that only explains itself is a dead end, so it
# takes the shape `pool-screen-empty-07` already established for exactly
# this situation. THE EMPTY-STATE NOTE HAD TO CHANGE TOO: "Define one below"
# named a control that no longer exists, which is the same failure as the
# refusal message in `quota_triage_validation.feature` and is corrected for
# the same reason -- TEXT THAT INSTRUCTS THE OWNER TO USE SOMETHING THAT IS
# NOT THERE.
#
# THE ABSENCE OF THE DEFINE CONTROL IS ASSERTED, and that is legitimate here
# where the reorder arrows' absence still is not. The difference is not
# style: the owner settled this one today, so it is a rule; on the arrows
# (#139) the absence remains UNDECIDED, exactly as `quota_sessions.feature`
# and `qa/quota_screen.md` both say. -02 carries it rather than becoming a
# scenario of its own, so it rides alongside parameters that genuinely vary
# and never becomes #90's "the point is that nothing happens".
#
# ORDER IS THE ORDER YOU TRIAGED THEM IN (-04), oldest first. Unchanged in
# substance from `-08` -- the store's `ORDER BY id ASC` still degrades to
# exactly this with the priority arrows out of scope -- but its Given now
# names the act that really produces a quota.
Feature: The quota screen shows what you triaged as a quota

  Background:
    Given the trellis server is running with an empty task list

  Scenario: The fourth screen exists and names itself
    When the "quota" screen is viewed
    Then the quota screen reports "<meta>" beside its title
    And the tab bar marks "<current>" as the current tab

    Examples:
      | meta     | current |
      | none yet | Quota   |

  # quota-screen-empty-explains-itself-02: an empty quota screen says what a quota is and points at Capture
  Scenario: An empty quota screen says what a quota is and points at Capture
    When the "quota" screen is viewed
    Then the quota screen offers no quotas
    And the quota screen notes "<note>"
    And the quota screen offers a way back to Capture
    And the quota screen offers no way to define a quota

    Examples:
      | note                                                                                          |
      | A quota is a weekly hour target you keep — practice, study, running. Capture one and triage it. |

  # quota-screen-reads-its-target-03: a triaged quota reads its target with nothing logged yet
  Scenario: A triaged quota reads its target with nothing logged yet
    Given a quota named "Piano" with a target of "<hours>" hours a week
    When the "quota" screen is viewed
    Then the quota "Piano" reads "<readout>"
    And the quota "Piano" notes "<note>"
    And the quota screen reports "<meta>" beside its title

    Examples:
      | hours | readout | note                     | meta    |
      | 4     | 0m / 4h | 4h left this week · 0% | 1 quota |

  # quota-screen-triaged-order-04: quotas are listed in the order they were triaged
  Scenario: Quotas are listed in the order they were triaged
    Given a quota named "Piano" with a target of "4" hours a week
    And a quota named "Running" with a target of "3" hours a week
    And a quota named "Rust" with a target of "5" hours a week
    When the "quota" screen is viewed
    Then the quota screen offers the quotas "<quotas>"
    And the quota screen reports "<meta>" beside its title

    Examples:
      | quotas               | meta     |
      | Piano, Running, Rust | 3 quotas |
