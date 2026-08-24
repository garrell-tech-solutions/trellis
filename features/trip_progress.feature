# trip-progress-struck-in-place-01: a completed item stays in place, struck through, and the label reports progress
# trip-progress-panel-holds-02: checking items off never dissolves the panel
# trip-progress-uncheck-restores-03: unchecking a struck item puts it back
# trip-progress-clear-done-04: clear done removes the struck items and only those
# trip-progress-clearing-can-drop-a-group-05: clearing can take a group below the threshold, and it drops to loose ends
# trip-progress-fully-done-06: a group with nothing open left says so, and clearing empties it
# trip-progress-clear-control-appears-with-work-07: the clear control appears only once something is struck, and is named
# trip-progress-loose-ends-unchanged-08: a completed loose end still leaves the screen at once
#
# THE SCREEN DESTROYED THE LIST AT THE MOMENT IT WAS BEING USED FOR ITS ONLY
# PURPOSE. pool/store.rs:31 filters archived_at IS NULL, so a completed task
# left the query; pool.rs:101 then re-tested the threshold on every render, so
# the third check-off mid-shop dissolved the panel and scattered the rest into
# loose ends.
#
# THIS NARROWS #103, IT DOES NOT REVERSE IT. That slice rejected counting done
# items toward the threshold as the half-pass trap -- a trip panel of one that
# reads correctly until you look at the number. That was right about the
# NUMBER and wrong about the EXPERIENCE, and the fix is the half it did not
# consider: change the label, not the visibility. FORMATION still requires
# three OPEN items, so completed ones can never conjure a trip; PERSISTENCE
# counts everything displayed, so working a trip never dissolves it.
#
# NOTHING CLEARS ITSELF. The owner chose an explicit control over any time
# window: this screen changes state only when tapped, and a struck item is a
# record of what you did until you say otherwise.
#
# THE CONTROL IS A GLYPH, AND ITS POSITION CARRIES THE MEANING. `X` sits
# immediately after the "3 of 5 done" label, with aria-label="Clear done",
# and appears only once something is struck. The owner prefers icons to text
# buttons and the canvas agrees -- every glyph it draws is a bare button with
# an aria-label (up, down, enter, caret). But every one of those is
# DIRECTIONAL OR LITERAL: the design has never asked a glyph to carry an
# abstract meaning, and THERE IS NO CONVENTIONAL ICON FOR "CLEAR COMPLETED".
# X means close or delete, a bin means discard, an eye-slash means temporarily
# hidden -- all adjacent, all subtly wrong. So the glyph is placed where its
# neighbour explains it: directly after the count of done things. Somewhere
# else on the panel the same glyph would read as "close this group", which is
# the one misreading QA is told to watch for.
#
# THE CANVAS IS SILENT HERE, checked rather than assumed
# (T-canvas-is-authoritative-where-it-speaks). `poolRow` has no completed
# state at all -- no strike-through, no dimming, its only variation is a
# priority highlight this project does not build -- and `g.count` reads
# "3 things", never progress. That is the SIXTH and SEVENTH gap the canvas has
# left, after the done control, the date input, a settings surface, the app
# icon and the committed panel. Flagged, not filled silently.
#
# #103'S NO-UNDO ASSERTION IS NARROWED, NOT DELETED, and mark_done.feature
# changes with this slice. What survives: no browsable list of completed work
# (D-kill-means-archive -- "the moment an archive is browsable it becomes a
# place to hide from decisions") and no reorder arrow. WHAT CHANGES: a struck
# item still on the screen can be unchecked, because that is the direct
# inverse of the tap that struck it and a deliberate act in its own right.
# The rule is now: YOU CAN UNCHECK WHAT YOU CAN SEE, AND NOTHING BRINGS BACK
# WHAT HAS CLEARED.
#
# NO MIGRATION, AND THIS IS DELIBERATE. #126 bought a permanent column for a
# display preference on a premise that had already expired, and could not
# un-buy it. archived_at already carries everything this slice needs: struck
# means archived and still displayed, cleared means archived and no longer
# displayed, and "no longer displayed" is a consequence of the clear tap
# rather than a stored fact.
Feature: A trip survives being worked

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A completed item stays in place, struck through, and the label reports progress
    Given "<total>" pool tasks tagged "@homedepot"
    When "<done>" of them are marked done
    And the pool screen is viewed
    Then the trip "@homedepot" reads "<label>"
    And the trip "@homedepot" shows "<struck>" struck items
    And the trip "@homedepot" shows "<open>" open items

    Examples:
      | total | done | label       | struck | open |
      | 5     | 3    | 3 of 5 done | 3      | 2    |

  # trip-progress-panel-holds-02: checking items off never dissolves the panel
  Scenario: Checking items off never dissolves the panel
    Given "5" pool tasks tagged "@homedepot"
    When "<done>" of them are marked done
    And the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    And the loose ends list is empty

    Examples:
      | done | trips      |
      | 3    | @homedepot |
      | 5    | @homedepot |

  # trip-progress-uncheck-restores-03: unchecking a struck item puts it back
  Scenario: Unchecking a struck item puts it back
    Given "5" pool tasks tagged "@homedepot"
    And "3" of them are marked done
    When one struck item is unchecked
    And the pool screen is viewed
    Then the trip "@homedepot" reads "<label>"
    And the trip "@homedepot" shows "<open>" open items

    Examples:
      | label       | open |
      | 2 of 5 done | 3    |

  # trip-progress-clear-done-04: clear done removes the struck items and only those
  Scenario: Clear done removes the struck items and only those
    Given "5" pool tasks tagged "@homedepot"
    And "3" of them are marked done
    When the done items are cleared from "@homedepot"
    And the pool screen is viewed
    Then the trip "@homedepot" shows "<struck>" struck items
    And the loose ends list shows "<loose>" items

    Examples:
      | struck | loose |
      | 0      | 2     |

  # trip-progress-clearing-can-drop-a-group-05: clearing can take a group below the threshold, and it drops to loose ends
  Scenario: Clearing can take a group below the threshold, and it drops to loose ends
    Given "3" pool tasks tagged "@homedepot"
    And "1" of them are marked done
    When the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    When the done items are cleared from "@homedepot"
    And the pool screen is viewed
    Then the pool screen offers no trips
    And the loose ends list shows "<loose>" items

    Examples:
      | trips      | loose |
      | @homedepot | 2     |

  # trip-progress-fully-done-06: a group with nothing open left says so, and clearing empties it
  Scenario: A group with nothing open left says so, and clearing empties it
    Given "3" pool tasks tagged "@homedepot"
    And "3" of them are marked done
    When the pool screen is viewed
    Then the trip "@homedepot" reads "<label>"
    And the trip "@homedepot" shows "<open>" open items
    When the done items are cleared from "@homedepot"
    And the pool screen is viewed
    Then the pool screen offers no trips
    And the pool screen does not mention "@homedepot"

    Examples:
      | label       | open |
      | 3 of 3 done | 0    |

  # trip-progress-clear-control-appears-with-work-07: the clear control appears only once something is struck, and is named
  Scenario: The clear control appears only once something is struck, and is named
    Given "3" pool tasks tagged "@homedepot"
    When the pool screen is viewed
    Then the trip "@homedepot" offers no clear-done control
    When "1" of them are marked done
    And the pool screen is viewed
    Then the trip "@homedepot" offers a clear-done control named "<name>"
    And that control sits after the trip's progress label

    Examples:
      | name       |
      | Clear done |

  # trip-progress-loose-ends-unchanged-08: a completed loose end still leaves the screen at once
  Scenario: A completed loose end still leaves the screen at once
    Given a pool task "fix the door latch" with no context tag
    And a pool task "sharpen the mower" with no context tag
    When "fix the door latch" is marked done
    And the pool screen is viewed
    Then the pool screen does not mention "fix the door latch"
    And the loose ends list shows "<loose>" items

    Examples:
      | loose |
      | 1     |
