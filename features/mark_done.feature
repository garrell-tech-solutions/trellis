# mutation-stamp: sha256=9c34354d76771a0d2030c171faa20893d184b7ea17064068e8032b7444e35f0a
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-27T04:25:51.162977063Z","feature_name":"A task you have done leaves the screen it lives on","feature_path":"features/mark_done.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:26fc8b699cf0eb72110cd6f3ef06909d93b9a3c7798e3af4684142e54e6438c9","scenarios":[{"index":1,"name":"A committed task marked done leaves the Committed screen","scenario_hash":"065ccca448a023f9e1f2011fc130e4b246bde4d6b4bc12f2d6ef29bbeadcfde6","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-24T14:44:39.359880531Z"},{"index":2,"name":"A done task is in no count any screen shows","scenario_hash":"b2c0ac4bc1f73d3a638ab3c8f394db9e4dbb71f1b69662e9526a3910770673df","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-24T14:44:39.359880531Z"}]}
# acceptance-mutation-manifest-end

# mark-done-leaves-pool-01: a pool task marked done leaves the Pool screen
# mark-done-leaves-committed-03: a committed task marked done leaves the Committed screen
# mark-done-counts-exclude-04: a done task is in no count any screen shows
# mark-done-no-completed-list-05: nothing lists completed work, and the control is not a reorder arrow
# mark-done-escapes-hostile-text-06: hostile text stays escaped in the fragment marking done returns
# mark-done-way-back-pool-07: completing a loose end offers a way back, and taking it returns the task
# mark-done-way-back-committed-08: completing a committed row offers a way back, and taking it returns the row
# mark-done-way-back-is-ephemeral-09: the way back lasts until the next action and does not survive a reload
# mark-done-second-undo-changes-nothing-10: taking a way back twice returns the task once
#
# THE WAY BACK IS A LINE, NOT A LIST -- one task, gone on your next action.
# `mark-done-no-completed-list-05` is the guard: if it ever accumulates it has
# become the archive `R-browsable-archive` refused.
#
# EPHEMERAL, SETTLED BY THE OWNER 2026-08-27. It rides the request the way
# `expanded=` does. `tasks.archived_at` would have made it survive a reload
# for free, no column bought; that was offered and declined, because a way
# back that reappears when you return to Pool for an unrelated reason offers
# to undo something you finished on purpose.
#
# UNDO IS PER TASK. A cleared trip offers no way back -- #125's debt, named
# rather than assumed, and why `trip-persistence`'s two "does not mention"
# assertions still hold.
#
# POSITION IS NOT RESTORED and that is correct: pool order is derived
# (`T-trips-are-derived-not-ranked`), so an undone task returns where the rule
# now puts it, which may re-form a trip (-07).
#
# ONE COLUMN, NO DONE-VERSUS-KILLED DISCRIMINATOR. The full argument lives in
# `mark_done/mod.rs`'s own header, which states it at length; `T-archived-at-only`
# is the row. Not restated here.
#
# THE CANVAS DRAWS NO DONE CONTROL -- a GAP under D-four-screens rather than a
# disagreement with it. The checkbox sits at each row's LEADING EDGE ON BOTH
# SCREENS, Committed included, before the 66px date cell: one rule and one
# place the thumb learns. The cost is named because nobody can see it -- a
# Committed row becomes four columns on a 430px canvas with 18px padding, so
# its text loses roughly 32px, the tightest thing on the phone.
#
# IT IS A CHECKBOX AND MUST NOT BE AN ARROW; 05 asserts that what this slice
# adds did not become the reorder control pool-screen-nothing-reorders-05
# forbids. A QUOTA TASK IS NEVER DONE -- it recurs, so completing it is logging
# a session (D-logging-is-retrospective-and-separate); leave that seam clean. A
# done task keeps its context tag, so "done" is a filter over one list rather
# than a second list.
#
# mark-done-trip-drops-below-three-02 IS REMOVED, NOT NARROWED: it asserted
# that marking 2 of 3 tagged tasks done dissolved the trip into loose ends --
# true when this slice landed, false since #122. Both cases it covered are now
# trip_progress.feature's -02 and -05, and a second contradicting copy here
# would be the next agent's stale-scenario bug.
Feature: A task you have done leaves the screen it lives on

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A pool task marked done leaves the Pool screen
    Given a pool task "buy screws" tagged "@homedepot"
    And a pool task "fix the door latch" with no context tag
    When "buy screws" is marked done
    And the pool screen is viewed
    Then the loose ends list shows "<loose>" items
    And the pool screen lists "fix the door latch" among the loose ends
    And the way back offers "buy screws"

    Examples:
      | loose |
      | 1     |

  # mark-done-leaves-committed-03: a committed task marked done leaves the Committed screen
  Scenario: A committed task marked done leaves the Committed screen
    Given a committed task "Book the dentist" with no context tag due "2026-08-25T08:30:00Z" as an "at"
    And a committed task "File the tax return" with no context tag due "2026-08-27T17:00:00Z" as a "by"
    When "Book the dentist" is marked done
    And the committed screen is viewed
    Then the committed screen lists "<remaining>"
    And the way back offers "Book the dentist"

    Examples:
      | remaining           |
      | File the tax return |

  # mark-done-counts-exclude-04: a done task is in no count any screen shows
  Scenario: A done task is in no count any screen shows
    Given a pool task "buy screws" tagged "@homedepot"
    And a pool task "fix the door latch" with no context tag
    And a committed task "Book the dentist" with no context tag due "2026-08-25T08:30:00Z" as an "at"
    When "buy screws" is marked done
    And "Book the dentist" is marked done
    And the pool screen is viewed
    Then the pool screen reports "<pool_meta>" beside its title
    When the committed screen is viewed
    Then the committed screen reports "<committed_meta>" beside its title

    Examples:
      | pool_meta | committed_meta |
      | 1 waiting | nothing dated  |

  # mark-done-no-completed-list-05: nothing lists completed work, and the control is not a reorder arrow
  Scenario: Nothing lists completed work, and the control is not a reorder arrow
    Given a pool task "buy screws" tagged "@homedepot"
    When "buy screws" is marked done
    And the pool screen is viewed
    Then the pool screen offers no reorder control
    And the pool screen offers no list of completed work

  # mark-done-escapes-hostile-text-06: hostile text stays escaped in the fragment marking done returns
  Scenario: Hostile text stays escaped in the fragment marking done returns
    Given a pool task "buy screws" tagged "@homedepot"
    And a pool task "<script>alert('boom')</script>" with no context tag
    When "buy screws" is marked done
    Then the response does not contain an unescaped "<script>" tag
    And the response contains the word "boom"

  # mark-done-way-back-pool-07: completing a loose end offers a way back, and taking it returns the task
  Scenario: Taking the way back returns a completed pool task, wherever the rule now puts it
    Given a pool task "buy screws" tagged "<tag>"
    And a pool task "buy nails" tagged "<tag>"
    And a pool task "buy a hinge" tagged "<tag>"
    When "buy screws" is marked done
    And the pool screen is viewed
    Then the pool screen offers the trips "<after_done>"
    When the way back to "buy screws" is taken
    And the pool screen is viewed
    Then the pool screen offers the trips "<after_undo>"
    And the trip "<tag>" shows "<open>" open items

    Examples:
      | tag        | after_done | after_undo | open |
      | @homedepot |            | @homedepot | 3    |

  # mark-done-way-back-committed-08: completing a committed row offers a way back, and taking it returns the row
  Scenario: Taking the way back returns a completed committed row
    Given a committed task "Book the dentist" with no context tag due "2026-08-25T08:30:00Z" as an "at"
    When "Book the dentist" is marked done
    And the committed screen is viewed
    Then the committed screen reports "<after_done>" beside its title
    When the way back to "Book the dentist" is taken
    And the committed screen is viewed
    Then the committed screen lists "<remaining>"
    And the committed screen reports "<after_undo>" beside its title

    Examples:
      | after_done    | remaining        | after_undo |
      | nothing dated | Book the dentist | 1 dated    |

  # mark-done-way-back-is-ephemeral-09: the way back lasts until the next action and does not survive a reload
  Scenario: The way back lasts until the next action and does not survive a reload
    Given a pool task "buy screws" tagged "@homedepot"
    And a pool task "fix the door latch" with no context tag
    When "buy screws" is marked done
    Then the way back offers "buy screws"
    When "fix the door latch" is marked done
    Then the way back offers "fix the door latch"
    When the pool screen is viewed
    Then the way back offers ""

  # mark-done-second-undo-changes-nothing-10: taking a way back twice returns the task once
  Scenario: Taking a way back twice returns the task once
    Given a pool task "buy screws" tagged "@homedepot"
    When "buy screws" is marked done
    And the way back to "buy screws" is taken
    And the way back to "buy screws" is taken
    And the pool screen is viewed
    Then the loose ends list shows "<loose>" items
    And the pool screen lists "buy screws" among the loose ends

    Examples:
      | loose |
      | 1     |
