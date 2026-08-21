# pool-screen-trips-and-loose-01: pool tasks with enough at one place become a trip; the rest fall to loose ends
# pool-screen-trip-order-02: trips are ordered by how many things they clear, then alphabetically
# pool-screen-case-folded-grouping-03: two spellings of one tag are one trip
# pool-screen-only-pool-04: committed and quota tasks do not appear
# pool-screen-nothing-reorders-05: everything is newest first and nothing on the screen can be reordered
# pool-screen-truncation-06: a long trip shows three and offers the rest
# pool-screen-empty-07: with nothing pooled the screen says so and points at Capture
# pool-screen-tabs-08: the tab bar offers Capture and Pool and marks the current one
# pool-screen-escapes-hostile-text-09: a hostile context tag stays escaped on this screen
#
# THE TRIP THRESHOLD IS THREE, and it is the idea the whole screen turns on.
# A context tag becomes a trip only once three things are waiting there;
# fewer, and those items drop into loose ends STILL SHOWING THEIR TAG. Two
# errands at the supermarket are not a trip, they are two strays that happen
# to share a place. The canvas says this (`tripThreshold || 3`) and the
# brief's demo contradicts it by drawing a two-item group as a trip; the
# canvas wins on layout, and the owner confirmed the number.
#
# REORDERING IS NOT IN THIS SLICE. The canvas draws up/down arrows on trip
# items, on loose ends and on quota rows; none of them are built here, and 05
# asserts their absence rather than leaving it to be inferred -- a coder
# reading the canvas would otherwise add them in good faith.
#
# WHEN IT COMES, IT BELONGS TO LOOSE ENDS ALONE, and the threshold is why.
# A TRIP IS A UNIT YOU CLEAR IN ONE STOP, so the order of its items is noise
# and should never get a control. A LOOSE END IS A THING YOU DECIDE ABOUT, so
# it should. Recorded here because it is the reasoning the reordering slice
# inherits, and it was reached by reading the canvas against the threshold
# rather than from the brief, which excluded reordering on the mistaken
# ground that the canvas drew none.
#
# TRIPS AND GROUPS GET NO PRIORITY, IN THIS SLICE OR ANY LATER ONE. They are
# ranked by how many things they clear -- derived from the data, never
# maintained, and exactly what "worth a trip" means.
#
# ITEM ORDER INSIDE A TRIP IS NEWEST FIRST, inherited from the inbox rather
# than invented, because the order is noise and a second convention would be
# one more thing to remember. It is also what the canvas's own sort degrades
# to once priority is removed from it: its tiebreak is `b.seq - a.seq`.
#
# DELIBERATELY NOT BUILT, though the canvas draws it: the per-group note
# ("One stop clears all 3."). The canvas computes it from a hardcoded list of
# which tags are places and which are sittings, which needs Trellis to know
# that @homedepot is a shop -- the managed taxonomy
# D-context-tags-are-the-taxonomy exists to refuse. Raised rather than
# resolved quietly, per D-four-screens.
#
# Scenario 05 names the same step three times -- <before>, <after>, <top> --
# because it asserts one order at three points as it changes, which is the
# whole scenario. Reported as placeholder drift and it is not; splitting it
# would lose the progression that is the point.
#
# `tasks.priority` already means P1-P4 for committed work. A loose end's
# manual order is a different thing and must not be spelled with that column.
Feature: The pool screen groups loose work by where it can be done

  Background:
    Given the trellis server is running with an empty task list

  Scenario: Pool tasks with enough at one place become a trip; the rest fall to loose ends
    Given a pool task "buy screws" tagged "@homedepot"
    And a pool task "return the drill" tagged "@homedepot"
    And a pool task "pick up trim" tagged "@homedepot"
    And a pool task "milk" tagged "@supermarket"
    And a pool task "coffee" tagged "@supermarket"
    And a pool task "fix the door latch" with no context tag
    When the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    And the trip "@homedepot" reads "<count>"
    And the loose ends list shows "milk" tagged "@supermarket"
    And the loose ends list shows "fix the door latch" with no context tag
    And the pool screen reports "<meta>" beside its title

    Examples:
      | trips      | count    | meta      |
      | @homedepot | 3 things | 6 waiting |

  # pool-screen-trip-order-02: trips are ordered by how many things they clear, then alphabetically
  Scenario: Trips are ordered by how many things they clear, then alphabetically
    Given "<bakery>" pool tasks tagged "@bakery"
    And "<attic>" pool tasks tagged "@attic"
    And "<cellar>" pool tasks tagged "@cellar"
    When the pool screen is viewed
    Then the pool screen offers the trips "<trips>"

    Examples:
      | bakery | attic | cellar | trips                     |
      | 4      | 3     | 3      | @bakery, @attic, @cellar  |

  # pool-screen-case-folded-grouping-03: two spellings of one tag are one trip
  Scenario: Two spellings of one tag are one trip
    Given a pool task "buy screws" tagged "@HomeDepot"
    And a pool task "return the drill" tagged "@homedepot"
    And a pool task "pick up trim" tagged "@HOMEDEPOT"
    When the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    And the trip "@HomeDepot" reads "<count>"

    Examples:
      | trips      | count    |
      | @HomeDepot | 3 things |

  # pool-screen-only-pool-04: committed and quota tasks do not appear
  Scenario: Committed and quota tasks do not appear
    Given a pool task "buy screws" tagged "@homedepot"
    And a committed task "file the return" tagged "@homedepot"
    And a quota task "practise piano" tagged "@homedepot"
    When the pool screen is viewed
    Then the pool screen does not mention "file the return"
    And the pool screen does not mention "practise piano"
    And the pool screen reports "<meta>" beside its title

    Examples:
      | meta      |
      | 1 waiting |

  # pool-screen-nothing-reorders-05: everything is newest first and nothing on the screen can be reordered
  Scenario: Everything is newest first and nothing on the screen can be reordered
    Given a pool task "buy screws" tagged "@homedepot"
    And a pool task "return the drill" tagged "@homedepot"
    And a pool task "pick up trim" tagged "@homedepot"
    And a pool task "fix the door latch" with no context tag
    And a pool task "sharpen the mower" with no context tag
    When the pool screen is viewed
    Then the trip "@homedepot" lists "<trip_order>"
    And the loose ends are in the order "<loose_order>"
    And the pool screen offers no reorder control

    Examples:
      | trip_order                                 | loose_order                            |
      | pick up trim, return the drill, buy screws | sharpen the mower, fix the door latch  |

  # pool-screen-truncation-06: a long trip shows three and offers the rest
  Scenario: A long trip shows three and offers the rest
    Given "<total>" pool tasks tagged "@homedepot"
    When the pool screen is viewed
    Then the trip "@homedepot" lists "<shown>" items
    And the trip "@homedepot" offers "<more>"

    Examples:
      | total | shown | more        |
      | 5     | 3     | Show 2 more |

  # pool-screen-empty-07: with nothing pooled the screen says so and points at Capture
  Scenario: With nothing pooled the screen says so and points at Capture
    When the pool screen is viewed
    Then the pool screen shows an empty-state message
    And the pool screen offers a way back to Capture
    And the pool screen reports "<meta>" beside its title

    Examples:
      | meta  |
      | empty |

  # pool-screen-tabs-08: the tab bar offers Capture and Pool and marks the current one
  Scenario: The tab bar offers Capture and Pool and marks the current one
    When the "<screen>" screen is viewed
    Then the tab bar offers exactly "<tabs>"
    And the tab bar marks "<current>" as the current tab

    Examples:
      | screen  | tabs           | current |
      | capture | Capture, Pool  | Capture |
      | pool    | Capture, Pool  | Pool    |

  # pool-screen-escapes-hostile-text-09: a hostile context tag stays escaped on this screen
  Scenario: A hostile context tag stays escaped on this screen
    Given a pool task "buy screws" tagged "<script>alert('boom')</script>"
    When the pool screen is viewed
    Then the pool screen does not contain an unescaped "<script>" tag
    And the pool screen contains the word "boom"
