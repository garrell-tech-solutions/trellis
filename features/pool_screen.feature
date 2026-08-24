# mutation-stamp: sha256=c189e5cb1fa3ce8e6c01060e055fb88710932c7835bfabb7acdf272ddb7e5bd4
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-21T21:20:12.529312396Z","feature_name":"The pool screen groups loose work by where it can be done","feature_path":"features/pool_screen.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:c4a6206113d624af68d486016b12551fe2fb489f346e0f956f5d199902112a29","scenarios":[{"index":0,"name":"Pool tasks with enough at one place become a trip; the rest fall to loose ends","scenario_hash":"dce621dbf11efcda2c31641e84cf2cc2a11da65bdcf9777d555bedfc96c19c2f","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-21T18:36:43.334687532Z"},{"index":1,"name":"Trips are ordered by how many things they clear, then alphabetically","scenario_hash":"26092b7208cb5ee38adb0edb9b6785537592a900330ba81aba760fe1f19d61ae","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-21T18:36:43.334687532Z"},{"index":2,"name":"Two spellings of one tag are one trip","scenario_hash":"c2d0430c5821b16786bfa100baeddb9e74e34dd5d751f1d0854292a013295410","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-21T18:36:43.334687532Z"},{"index":3,"name":"Committed and quota tasks do not appear","scenario_hash":"d16bdaad1df0b98a76bc46ed5d91ebc362ef0ae475b12d88e466189cec1051c5","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-21T18:36:43.334687532Z"},{"index":4,"name":"Everything is newest first and nothing on the screen can be reordered","scenario_hash":"2192791261be70d53fe91482c3457efddd4dda7f720711a2015779f79105275d","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-21T18:36:43.334687532Z"},{"index":5,"name":"A long trip shows three and offers the rest","scenario_hash":"92b668c98034b0f94fab4fedf926b576727316697c091d8f61adfdf7aaa1dc0d","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-21T18:36:43.334687532Z"},{"index":6,"name":"With nothing pooled the screen says so and points at Capture","scenario_hash":"03eb5e13152711637156de8088a3c47427bdb30692f1d71446c3a320420d790b","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-21T18:36:43.334687532Z"}]}
# acceptance-mutation-manifest-end

# pool-screen-trips-and-loose-01: pool tasks with enough at one place become a trip; the rest fall to loose ends
# pool-screen-trip-order-02: trips are ordered by how many things they clear, then alphabetically
# pool-screen-case-folded-grouping-03: two spellings of one tag are one trip
# pool-screen-only-pool-04: committed and quota tasks do not appear
# pool-screen-nothing-reorders-05: everything is newest first and nothing on the screen can be reordered
# pool-screen-truncation-06: a long trip holds every item and offers to show the rest
# pool-screen-empty-07: with nothing pooled the screen says so and points at Capture
# pool-screen-escapes-hostile-text-09: a hostile context tag stays escaped on this screen
#
# THE TAB-BAR SCENARIO IS GONE, not inverted. pool-screen-tabs-08 asserted
# the tab bar offered exactly Capture and Pool -- true only while Pool was
# the last tab. #94 adds Committed, and committed-screen-tabs-06 already
# names all three screens over all three tabs, so keeping a second,
# narrower assertion here would just be two features restating one fact
# until one of them drifts. The guarantee moved rather than lapsed, the
# same shape one_screen.feature's own no-header scenario took under #92.
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
# SCENARIO 06 CHANGED WITH #120, AND THE OLD ASSERTION WAS ABOUT STRUCTURE.
# It read "lists 3 items" for a trip of five, and `trip_visible_items` answers
# that by reading the FIRST `<ul class="trip-items">` in the panel -- which
# only distinguishes anything while the hidden items are a SECOND list inside
# a `<details>`. That two-list shape is precisely the defect #120 removes:
# the canvas draws one list whose visible slice grows, so over HTTP the trip
# now holds all five and the control offers the other two. THE GUARANTEE
# MOVED RATHER THAN LAPSED -- "only three are on screen" is now asserted in
# `qa/trip_controls.md`, against a rendered page, where it is a fact rather
# than an artefact of markup. See `features/trip_controls.feature`.
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

  # pool-screen-truncation-06: a long trip holds every item and offers to show the rest
  Scenario: A long trip holds every item and offers to show the rest
    Given "<total>" pool tasks tagged "@homedepot"
    When the pool screen is viewed
    Then the trip "@homedepot" lists "<held>" items
    And the trip "@homedepot" offers "<more>"

    Examples:
      | total | held | more        |
      | 5     | 5    | Show 2 more |

  # pool-screen-empty-07: with nothing pooled the screen says so and points at Capture
  Scenario: With nothing pooled the screen says so and points at Capture
    When the pool screen is viewed
    Then the pool screen shows an empty-state message
    And the pool screen offers a way back to Capture
    And the pool screen reports "<meta>" beside its title

    Examples:
      | meta  |
      | empty |

  # pool-screen-escapes-hostile-text-09: a hostile context tag stays escaped on this screen
  Scenario: A hostile context tag stays escaped on this screen
    Given a pool task "buy screws" tagged "<script>alert('boom')</script>"
    When the pool screen is viewed
    Then the pool screen does not contain an unescaped "<script>" tag
    And the pool screen contains the word "boom"
