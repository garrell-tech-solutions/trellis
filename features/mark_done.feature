# mark-done-leaves-pool-01: a pool task marked done leaves the Pool screen
# mark-done-trip-drops-below-three-02: a trip that falls under three becomes loose ends, keeping its tag
# mark-done-leaves-committed-03: a committed task marked done leaves the Committed screen
# mark-done-counts-exclude-04: a done task is in no count any screen shows
# mark-done-no-undo-05: nothing offers to un-do it, and the control is not a reorder arrow
# mark-done-escapes-hostile-text-06: hostile text stays escaped in the fragment marking done returns
#
# ONE COLUMN, NOT TWO, AND NO DISCRIMINATOR YET. `tasks.archived_at` has
# existed since 0002 and nothing has ever written it; marking done writes it.
# T-archived-at-only stays literally true -- it remains the single archive
# signal.
#
# T-capture-leaves-inbox-once is the precedent the brief points at, and the
# interesting part is where it STOPS transferring. There, one column sufficed
# because the exit was DERIVABLE: a triaged capture has a `tasks` row
# referencing it and a dismissed one does not. Here there is no such row.
# Nothing downstream distinguishes a task you did from a task you killed, so
# the discriminator could not be derived -- it would have to be stored.
#
# It is not stored, because NOTHING CAN KILL A TASK TODAY. A column that can
# only ever hold one value is the speculative schema this project refuses --
# the same ground S4's `captures.life_area` was cut on, and the reason #48
# would not add a second timestamp for a state that could not occur. When a
# kill control arrives it adds the discriminator and backfills every existing
# row as `done`, which is PROVABLY correct rather than a guess: nothing else
# could have set the column.
#
# Recorded because M8's reckoning ("47 archived this quarter, 31 Learning")
# and D-quota-no-rollover's "you did 1 of 3 runs" both need the distinction
# eventually. They get it the day something can produce both values.
#
# THE CANVAS DRAWS NO DONE CONTROL. Its complete aria-label set is Raise
# priority, Lower priority, Minutes, Day, Save capture, Hours a week, Delete
# session. D-four-screens makes the canvas authoritative on layout, so this
# is a GAP IN THE DESIGN rather than a disagreement with it: the checkbox at
# each row's LEADING EDGE, on both screens, was chosen by the owner and is
# not drawn anywhere. Said plainly so the design can be corrected rather than
# quietly diverged from.
#
# LEADING EDGE MEANS LEADING EDGE ON COMMITTED TOO, before the 66px date
# cell -- one rule and one place the thumb learns, ticking down a column
# rather than reaching across a row. The cost is named because nobody can see
# it: a Committed row becomes four columns on a 430px canvas with 18px
# padding, so its text loses roughly 32px, and a long item beside
# "BY THU 17:00" and a context tag is the tightest thing on the phone.
#
# THE CONTROL IS A CHECKBOX AND MUST NOT BE AN ARROW.
# pool-screen-nothing-reorders-05 asserts the absence of every reorder
# control and its QA document calls finding one a defect; #95 owns those. 05
# below asserts that what this slice adds did not become one.
#
# A QUOTA TASK IS NEVER DONE -- it recurs, so completing it is logging a
# session, which is #93's (D-logging-is-retrospective-and-separate:
# completing may OFFER to log time and never does it silently). No done
# control belongs on quota work, and this slice must leave that seam clean.
#
# A DONE TASK KEEPS ITS CONTEXT TAG. The tag lives on the capture and nothing
# removes it, so "done" is a filter over one list rather than a second list --
# which is what lets M8 later ask what was done at @homedepot.
Feature: A task you have done leaves the screen it lives on

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A pool task marked done leaves the Pool screen
    Given a pool task "buy screws" tagged "@homedepot"
    And a pool task "fix the door latch" with no context tag
    When "buy screws" is marked done
    And the pool screen is viewed
    Then the pool screen does not mention "buy screws"
    And the pool screen lists "fix the door latch" among the loose ends

  # mark-done-trip-drops-below-three-02: a trip that falls under three becomes loose ends, keeping its tag
  Scenario: A trip that falls under three becomes loose ends, keeping its tag
    Given a pool task "buy screws" tagged "@homedepot"
    And a pool task "return the drill" tagged "@homedepot"
    And a pool task "pick up trim" tagged "@homedepot"
    When the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    When "buy screws" is marked done
    And "return the drill" is marked done
    And the pool screen is viewed
    Then the pool screen offers no trips
    And the loose ends list shows "pick up trim" tagged "<tag>"

    Examples:
      | trips      | tag        |
      | @homedepot | @homedepot |

  # mark-done-leaves-committed-03: a committed task marked done leaves the Committed screen
  Scenario: A committed task marked done leaves the Committed screen
    Given a committed task "Book the dentist" with no context tag due "2026-08-25T08:30:00Z" as an "at"
    And a committed task "File the tax return" with no context tag due "2026-08-27T17:00:00Z" as a "by"
    When "Book the dentist" is marked done
    And the committed screen is viewed
    Then the committed screen does not mention "Book the dentist"
    And the committed screen lists "<remaining>"

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

  # mark-done-no-undo-05: nothing offers to un-do it, and the control is not a reorder arrow
  Scenario: Nothing offers to un-do it, and the control is not a reorder arrow
    Given a pool task "buy screws" tagged "@homedepot"
    When "buy screws" is marked done
    And the pool screen is viewed
    Then the pool screen offers no way to un-do a completed task
    And the pool screen offers no reorder control
    And the pool screen offers no list of completed work

  # mark-done-escapes-hostile-text-06: hostile text stays escaped in the fragment marking done returns
  Scenario: Hostile text stays escaped in the fragment marking done returns
    Given a pool task "buy screws" tagged "@homedepot"
    And a pool task "<script>alert('boom')</script>" with no context tag
    When "buy screws" is marked done
    Then the response does not contain an unescaped "<script>" tag
    And the response contains the word "boom"
