# trip-controls-expand-asks-for-nothing-01: the show-more control is a button that asks the server for nothing
# trip-controls-complete-names-its-count-02: a trip offers to complete what is still open, and says how many
# trip-controls-nothing-left-to-complete-03: a trip with nothing open left offers no complete-group control
# trip-controls-completes-the-hidden-too-04: completing the group completes what is hidden as well as what is shown
# trip-controls-stays-inside-the-group-05: completing one group leaves every other group and the loose ends alone
# trip-controls-unchecking-reverses-it-06: unchecking a struck item after a group completion puts it back
#
# WHAT IS ASSERTED HERE AND WHAT CANNOT BE. Expanding and collapsing are not
# in this file, and that is deliberate. THE TIER YOU ASSERT IN DECIDES WHAT
# THE IMPLEMENTATION MUST STORE, and #126 is the standing proof: it bought
# `captures.shown_kind`, a permanent append-only column for a display
# preference, and traced the cause afterwards -- the acceptance suite speaks
# only HTTP, so the state had to be server-rendered, so it had to be stored.
# A scenario here asserting "the trip is expanded" would buy the same column
# again. So expand state is asserted in `qa/trip_controls.md`, by the browser
# check that already drives real Chrome, and NOTHING IN THIS SLICE NEEDS A
# MIGRATION. #127's `cleared_at` was a durable consequence of a deliberate
# act; an expand state is a thing you did with your thumb.
#
# What HTTP genuinely sees is here: that the control is a button asking for
# nothing (01), and everything about the group completion, which is a real
# state change (02-06).
#
# --- THE CANVAS ANSWERS #120, READ RATHER THAN GREPPED --------------------
# `Trellis.dc.html:210` draws a borderless `<button>` with `g.onToggle`,
# `color: var(--color-primary-800)` and `letter-spacing: 0.02em`. Its
# behaviour is at 841-851 and it settles every open question in the brief:
#
#   const open    = !!s.expanded[g.key];
#   const visible = open ? g.list : g.list.slice(0, 3);
#   const more    = g.list.length - visible.length;
#   hasMore:   more > 0 || open,
#   moreLabel: more > 0 ? "Show " + more + " more" : "Show fewer",
#   onToggle:  () => setState(expanded[g.key] = !expanded[g.key])
#
# ONE LIST -- expanding replaces the visible slice, it does not append a
# second list. THE LABEL FLIPS, and the canvas's own collapsed wording is
# "Show fewer". THE CONTROL SURVIVES BEING EXPANDED (`more > 0 || open`), so
# it can take you back. AND THE STATE IS CLIENT STATE KEYED BY GROUP -- no
# request, no column, and each trip expands independently. `base.html` has
# loaded htmx and an inline script on every page since the shell shipped, so
# "a toggle needs client state this product does not have" was never true.
#
# THE REASONING THAT PRODUCED THE WRONG CONTROL IS RECORDED AND IS NOT SILLY.
# `pool/view.rs:22` says a native `<details>` "costs no request and no
# server-held state". Both true. The `<details>` was not too cheap; it was
# CHEAP AT THE WRONG THING -- a `<summary>` is static text, so the one thing
# the design needs, a label that changes, is the one thing it cannot do.
#
# ONE EXISTING SCENARIO CHANGES, AND IT IS THE STRUCTURE #120 EXISTS TO FIX.
# `pool-screen-truncation-06` asserted `the trip "@homedepot" lists "3"
# items`, and that step reads the FIRST `<ul class="trip-items">` in the
# panel -- an assertion that only means anything while there are two lists.
# With one list it counts five. It is revised in place to assert what HTTP
# can now honestly see (the trip holds all five, and the control offers the
# other two), and "only three are on screen" moves to the QA suite, where it
# is a fact about a rendered page rather than about a document. THE BRIEF
# ASKED FOR 21 UNTOUCHED FEATURES AND THIS IS ONE DEVIATION, NAMED: the
# scenario asserts the defect. Nothing else moves -- `trip_progress`'s counts
# already scan the whole panel rather than one list, and
# `pool-screen-nothing-reorders-05`'s trip has nothing hidden.
#
# AN EXPANDED TRIP STAYS EXPANDED WHEN YOU TICK SOMETHING IN IT. Neither
# issue names this and it is the hard part of the slice. Every checkbox in
# the panel swaps `#pool-body` `outerHTML`, so a client-only toggle is
# destroyed by the act of working the trip: expand a trip of eight, tick the
# sixth thing, and the panel collapses under your thumb -- taking the item
# you just struck off the screen with it. THAT IS EXACTLY THE FAILURE
# `D-a-trip-survives-being-worked` WAS WRITTEN AGAINST, arriving through a
# different door. How it survives is the coder's call; that it survives is
# not. Riding along with the request is fine and stores nothing; a column is
# not (see above), and if you reach for one, stop and say why first.
#
# --- THE COMPLETE-GROUP CONTROL (#125) ------------------------------------
# ONE TAP, AND THE LABEL CARRIES THE COUNT -- "Complete all 8". Settled by
# the owner 2026-08-24 on the following, which is a change of circumstance
# rather than a reading of the rule. `D-bulk-completion-is-explicit` was
# recorded 2026-08-23 calling completion "the least reversible thing the
# product does and the hardest to notice going wrong", and rejecting
# "relying on undo instead" because "a bulk action still needs the owner to
# notice it happened". THE NEXT DAY `D-a-trip-survives-being-worked` WAS
# EXTENDED so a completed trip item strikes through IN PLACE. In the pool a
# group completion now loses nothing and is the loudest thing the panel can
# do: eight strike-throughs at once and the label flipping to "8 of 8 done".
# The decision's own test -- that the owner notices -- is met by the panel,
# and it names "a dedicated group-completion checkbox" as the sanctioned
# shape. The destructive act is `✕ Clear done`, which is a separate control
# and already there.
#
# THE COUNT IS THE OPEN ITEMS, NOT THE GROUP'S SIZE, and it is why the
# control cannot be a glyph. You can be looking at three things and about to
# complete eight; a label that says which is the whole safeguard, and an icon
# cannot say a number. `trip-progress`'s header already reasoned this out for
# `✕`: THE DESIGN HAS NEVER ASKED A GLYPH TO CARRY AN ABSTRACT MEANING, and
# `✕` only works because the progress label beside it explains it.
#
# IT IS NOT ADJACENT TO `✕`. Two controls side by side on a 390px header,
# one of which strikes eight items and the other of which permanently clears
# them, is a slip away from the destructive one -- and the slip is likeliest
# immediately after using the first. Where it goes is the coder's call; that
# the two are separated is not.
#
# WHAT REVERSES IT: unchecking, item by item, exactly as
# `D-a-trip-survives-being-worked` already provides. THIS IS NOT #111 AND
# THIS SLICE DOES NOT BUILD ONE. Undoing six by unchecking six is tedious and
# is not undo -- the brief is right about that -- but nothing was lost, the
# panel shows precisely what happened, and the alternative is an undo surface
# this product has deliberately not built. Said plainly rather than implied.
#
# THE CANVAS DRAWS NO COMPLETE-GROUP CONTROL. Checked, not assumed
# (`T-canvas-is-authoritative-where-it-speaks`): `tripGroups` has a label, a
# count, a note, a list and a toggle, and nothing else. That is the EIGHTH
# gap it has left, after the done control, the date input, a settings
# surface, the app icon, the committed panel and the two #122 found. A gap is
# not a prohibition; it is originated here and said out loud.
#
# WHAT THE CANVAS DRAWS AND THIS PRODUCT STILL REFUSES: `g.note`, the
# per-group line reading "One stop clears all 3.", computed from a hardcoded
# list of which tags are places and which are sittings. That needs Trellis to
# know `@homedepot` is a shop, which is the managed taxonomy
# `D-context-tags-are-the-taxonomy` exists to refuse. It renders directly
# above the header this slice reworks -- so it is the thing not to pick up on
# the way past.
#
# NO REORDER CONTROL ON A TRIP, still, and `pool-screen-nothing-reorders-05`
# still asserts that absence. `T-trips-are-derived-not-ranked` records that
# two previous briefs got this wrong "having read the canvas by grep rather
# than reading it", so: reorder controls are approved FOR LOOSE ENDS (#95).
# A trip is a unit you clear in one stop, so the order of its items is noise;
# a loose end is a thing you decide about, so it earns a control.
#
# ONE WRITE PATH, ONE STATEMENT. A group completion goes through
# `mark_done`'s front door like every other completion
# (`T-cross-capability-invariants-need-an-owner` -- it is the only capability
# declaring `mod store;` privately, and this does not change that), and
# completing N tasks is one statement, not N round trips through a loop in a
# handler (`T-set-operations-execute-in-the-store`).
#
# #129 IS OPEN ON THIS SAME PANEL and changes when a group stops being a
# trip. No scope overlap: this slice touches the header's controls and the
# item list's structure, not grouping. THIS ONE LANDS FIRST -- it is
# specified and in the pipeline now -- and #129 inherits both controls.

Feature: The trip panel's own controls

  Background:
    Given the trellis server is running with an empty task list

  Scenario: The show-more control is a button that asks the server for nothing
    Given "<total>" pool tasks tagged "@homedepot"
    When the pool screen is viewed
    Then the trip "@homedepot" offers a show-more control named "<name>"
    And that control issues no request

    Examples:
      | total | name        |
      | 5     | Show 2 more |

  # trip-controls-complete-names-its-count-02: a trip offers to complete what is still open, and says how many
  Scenario: A trip offers to complete what is still open, and says how many
    Given "<total>" pool tasks tagged "@homedepot"
    And "<done>" of them are marked done
    When the pool screen is viewed
    Then the trip "@homedepot" offers a complete-group control named "<name>"

    Examples:
      | total | done | name           |
      | 8     | 0    | Complete all 8 |
      | 8     | 5    | Complete all 3 |

  # trip-controls-nothing-left-to-complete-03: a trip with nothing open left offers no complete-group control
  Scenario: A trip with nothing open left offers no complete-group control
    Given "<total>" pool tasks tagged "@homedepot"
    And "<done>" of them are marked done
    When the pool screen is viewed
    Then the trip "@homedepot" offers no complete-group control
    And the trip "@homedepot" reads "<label>"

    Examples:
      | total | done | label       |
      | 3     | 3    | 3 of 3 done |

  # trip-controls-completes-the-hidden-too-04: completing the group completes what is hidden as well as what is shown
  Scenario: Completing the group completes what is hidden as well as what is shown
    Given "<total>" pool tasks tagged "@homedepot"
    When the group "@homedepot" is completed
    And the pool screen is viewed
    Then the trip "@homedepot" shows "<struck>" struck items
    And the trip "@homedepot" shows "<open>" open items
    And the trip "@homedepot" reads "<label>"
    And the pool screen reports "<meta>" beside its title

    Examples:
      | total | struck | open | label       | meta      |
      | 8     | 8      | 0    | 8 of 8 done | 0 waiting |

  # trip-controls-stays-inside-the-group-05: completing one group leaves every other group and the loose ends alone
  Scenario: Completing one group leaves every other group and the loose ends alone
    Given "<total>" pool tasks tagged "@homedepot"
    And "<other>" pool tasks tagged "@supermarket"
    And a pool task "fix the door latch" with no context tag
    When the group "@homedepot" is completed
    And the pool screen is viewed
    Then the trip "@supermarket" shows "<other_open>" open items
    And the loose ends list shows "<loose>" items

    Examples:
      | total | other | other_open | loose |
      | 4     | 3     | 3          | 1     |

  # trip-controls-unchecking-reverses-it-06: unchecking a struck item after a group completion puts it back
  Scenario: Unchecking a struck item after a group completion puts it back
    Given "<total>" pool tasks tagged "@homedepot"
    And the group "@homedepot" is completed
    When one struck item is unchecked
    And the pool screen is viewed
    Then the trip "@homedepot" reads "<label>"
    And the trip "@homedepot" shows "<open>" open items

    Examples:
      | total | label       | open |
      | 4     | 3 of 4 done | 1    |
