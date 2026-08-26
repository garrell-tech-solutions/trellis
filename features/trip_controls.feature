# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-25T17:21:37.524078078Z","feature_name":"The trip panel's own controls","feature_path":"features/trip_controls.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:e41de56d021a4df0a2906182e5b5b3748db2828a3df26ee88982f7d4bdffee02","scenarios":[{"index":0,"name":"The show-more control is a button that asks the server for nothing","scenario_hash":"566c813c8b28137753e7556500e8ae21f88222cb4c27319da61c577370f41399","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-24T21:42:35.457706352Z"},{"index":1,"name":"A trip offers to complete what is still open, and says how many","scenario_hash":"65eba1324d00f4b84752fc27f49eb926adae07286e7468221a142b9ef7834dae","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-24T21:42:35.457706352Z"},{"index":2,"name":"A trip with nothing open left offers no complete-group control","scenario_hash":"ff24a4af3cfe0f556e9b4527b1d501a64c652ef11ff0ffb4bb4b42ec6adad373","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-24T21:42:35.457706352Z"},{"index":3,"name":"Completing the group completes what is hidden as well as what is shown","scenario_hash":"5431d46ccf84628b92600b342ddbe2d8d60ca483dc98510471bffbdc59c405f8","mutation_count":5,"result":{"Total":5,"Killed":5,"Survived":0,"Errors":0},"tested_at":"2026-08-24T21:42:35.457706352Z"},{"index":5,"name":"Unchecking a struck item after a group completion puts it back","scenario_hash":"a68fd67cb6e4c240513d6c39e1debbe9026796b15bcec7aa28279bd1f57bc150","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-24T21:42:35.457706352Z"}]}
# acceptance-mutation-manifest-end

# trip-controls-expand-asks-for-nothing-01: the show-more control is a button that asks the server for nothing
# trip-controls-complete-names-its-count-02: a trip offers to complete what is still open, and says how many
# trip-controls-nothing-left-to-complete-03: a trip with nothing open left offers no complete-group control
# trip-controls-completes-the-hidden-too-04: completing the group completes what is hidden as well as what is shown
# trip-controls-stays-inside-the-group-05: completing one group leaves every other group and the loose ends alone
# trip-controls-unchecking-reverses-it-06: unchecking a struck item after a group completion puts it back
#
# WHAT IS ASSERTED HERE AND WHAT CANNOT BE. Expanding and collapsing are not in
# this file, deliberately: THE TIER YOU ASSERT IN DECIDES WHAT THE
# IMPLEMENTATION MUST STORE. #126 is the standing proof -- the acceptance suite
# speaks only HTTP, so the state had to be server-rendered, so it bought a
# permanent append-only column for a display preference. A scenario here
# asserting "the trip is expanded" would buy the same column again, so expand
# state is asserted in qa/trip_controls.md by the browser check and NOTHING IN
# THIS SLICE NEEDS A MIGRATION. What HTTP genuinely sees is here: the control
# is a button asking for nothing (01), and the group completion is a real state
# change (02-06).
#
# THE CANVAS ANSWERS #120 (Trellis.dc.html:210 and 841-851, read rather than
# grepped): one list whose visible slice grows, a label that flips, a control
# that survives being expanded, and state keyed by group with no request and no
# column. base.html has loaded htmx and an inline script on every page since the
# shell shipped, so "a toggle needs client state this product does not have" was
# never true. The `<details>` it replaces was not too cheap; it was CHEAP AT THE
# WRONG THING -- a `<summary>` is static text, so a label that changes is the
# one thing it cannot do.
#
# AN EXPANDED TRIP STAYS EXPANDED WHEN YOU TICK SOMETHING IN IT. Neither issue
# names this and it is the hard part of the slice: every checkbox swaps
# `#pool-body` outerHTML, so a client-only toggle is destroyed by the act of
# working the trip. That is the failure D-a-trip-survives-being-worked was
# written against, arriving through a different door. Riding along with the
# request stores nothing; a column does.
#
# THE COMPLETE-GROUP LABEL CARRIES THE COUNT OF OPEN ITEMS, NOT THE GROUP'S
# SIZE, and that is why it cannot be a glyph: you can be looking at three
# things and about to complete eight, and an icon cannot say a number.
# D-bulk-completion-is-explicit is satisfied by the panel rather than by a
# confirmation -- a completed item strikes through in place, so the completion
# loses nothing and is the loudest thing the panel can do. IT IS NOT ADJACENT
# TO `X Clear done`, which permanently clears what it just struck; the slip is
# likeliest right after using the first. Unchecking reverses it, item by item;
# this slice does not build undo.
#
# ONE EXISTING SCENARIO CHANGES: pool-screen-truncation-06 asserted the very
# structure #120 exists to fix, and is revised in place. Still no reorder
# control on a trip (T-trips-are-derived-not-ranked; reorder is approved for
# loose ends, #95), and a group completion goes through mark_done's front door
# (T-cross-capability-invariants-need-an-owner) in one statement
# (T-set-operations-execute-in-the-store).

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
