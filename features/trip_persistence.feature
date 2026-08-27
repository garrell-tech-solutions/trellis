# mutation-stamp: sha256=ca4095f517afc44ebe07e81736504ff85ca07bd2b7c112ce486200f4e6f7e21b
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-27T04:28:57.564945143Z","feature_name":"A trip survives being tidied","feature_path":"features/trip_persistence.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:03b6b9924007468bb0ba4f40eeffb5bb54d176a57de8b5f835294d15a18c568f","scenarios":[{"index":0,"name":"A tidied trip stands until its last item is cleared","scenario_hash":"cca23246ec85c89b4192e8381bc06bbe31d9e124d9c9063b836bc09ba5ceb1aa","mutation_count":8,"result":{"Total":8,"Killed":8,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"},{"index":1,"name":"A tag that never reached three is not made a trip by being tidied","scenario_hash":"b88a1867cf6afbd83947731cefc89a16356a76ba6da6f95a4d35de96e4ff7cb7","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"},{"index":2,"name":"A tag whose run has ended must reach three again","scenario_hash":"fc99ecd71a3a664ca1895e42fd0e40398d0507adc319fa4a99524619aa03f3ee","mutation_count":5,"result":{"Total":5,"Killed":5,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"},{"index":3,"name":"Completing what a tidied trip has left does not dissolve it","scenario_hash":"7d2c8c0b4cdcb8389ed4cf9d88a04552c86e9dc21a6ea5082104a49f9ff4324a","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"},{"index":4,"name":"Unchecking inside a run puts the item back without re-earning the trip","scenario_hash":"a24c3202e73c69ee1b5c72b7eec8cd786c6d80a06ce309059c30de583e469d13","mutation_count":5,"result":{"Total":5,"Killed":5,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"}]}
# acceptance-mutation-manifest-end

# trip-persistence-stands-until-its-last-item-clears-01: a tidied trip stands until its last item is cleared
# trip-persistence-never-formed-stays-loose-02: a tag that never reached three is not made a trip by being tidied
# trip-persistence-re-earns-its-trip-03: a tag whose run has ended must reach three again
# trip-persistence-completing-what-is-left-holds-04: completing what a tidied trip has left does not dissolve it
# trip-persistence-unchecking-inside-a-run-05: unchecking inside a run puts the item back without re-earning the trip
#
# THE SAME DEFECT ONE TAP LATER. #122 stopped the panel dissolving when you
# CHECKED things off; it still dissolved when you CLEARED them -- five things
# at @homedepot, three got, tap clear to tidy, and the last two scatter into
# loose ends with the owner still standing in the shop.
# D-a-trip-survives-being-worked was EXTENDED by the owner 2026-08-24:
# CLEARING TIDIES THE PANEL, IT DOES NOT DISSOLVE IT.
#
# TWO RULES, WHERE #122 HAD ONE PREDICATE. Clearing is the first thing that
# removes an item from the uncleared set mid-run, so the single count splits:
#
#   A RUN at a tag begins when something lands there with nothing else
#   waiting, and ends when the last thing waiting there is cleared away.
#   FORMATION: a run of three or more things is a trip.
#   PERSISTENCE: it stays one for as long as the run lasts.
#
# THE RUN IS THE UNIT, NOT THE MOMENT, and the current run's own history is not
# in the data the rule can see -- `list_pool_tasks` never returns a cleared
# row. How the store knows a tag is inside a run is THE CODER'S CALL AND THE
# PULL REQUEST'S ARGUMENT. `cleared_at` is a millisecond timestamp rather than
# a flag, so a derivation is worth attempting;
# T-ephemeral-view-state-rides-the-request is the test if it needs a column,
# and since a trip that dissolves when you lock your phone in the car park is
# this same defect one gesture further out, A COLUMN MAY WELL BE THE HONEST
# ANSWER -- reaching for one without the argument is not.
#
# THE PANEL LEAVES ON A TAP, NEVER ON A TICK (01), settled by the owner
# 2026-08-25 against the brief's demo. Three reasons, and the third decides it:
# trip-progress-fully-done-06 already does exactly this at three items, so
# going at two would let the item count decide whether finishing a trip wipes
# it; a panel that vanishes on the last tick takes the undo with it (05); and
# NOTHING CLEARS ITSELF -- a panel sweeping its own leftovers is the automatic
# sweep the owner refused, while not sweeping them leaves them uncleared and
# invisible until the next capture resurrects the panel over month-old strikes,
# WHICH IS PRECISELY THE RUNAWAY #129 WAS FILED ABOUT.
#
# A TAG MUST RE-EARN ITS TRIP (03) -- the bound #129 asked for, and the reason
# the naive derivation (ever reached three, counting cleared ones) is wrong:
# over a year every frequently-used tag would become a permanent panel. The run
# ending is a real event rather than a time window, which the owner refused
# once already (#127).
#
# TWO SCENARIOS IN trip_progress.feature CHANGE, BOTH IN THE OPEN:
# trip-progress-clearing-can-drop-a-group-05 ASSERTED THE DEFECT and is
# reversed, not deleted and not weakened; trip-progress-clear-done-04 is the
# consequence, its `loose` count going to zero. A real reload and a restart are
# facts about a running server and live in qa/trip_persistence.md.

Feature: A trip survives being tidied

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A tidied trip stands until its last item is cleared
    Given "<total>" pool tasks tagged "@homedepot"
    And "<done>" of them are marked done
    And the done items are cleared from "@homedepot"
    When "<rest>" of them are marked done
    And the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    And the trip "@homedepot" reads "<label>"
    And the trip "@homedepot" shows "<open>" open items
    And the trip "@homedepot" shows "<struck>" struck items
    When the done items are cleared from "@homedepot"
    And the pool screen is viewed
    Then the pool screen offers no trips
    And the pool screen does not mention "@homedepot"
    And the loose ends list shows "<loose>" items

    Examples:
      | total | done | rest | trips      | label       | open | struck | loose |
      | 5     | 3    | 2    | @homedepot | 2 of 2 done | 0    | 2      | 0     |

  # trip-persistence-never-formed-stays-loose-02: a tag that never reached three is not made a trip by being tidied
  Scenario: A tag that never reached three is not made a trip by being tidied
    Given "<total>" pool tasks tagged "@homedepot"
    And a pool task "milk" tagged "@supermarket"
    And a pool task "coffee" tagged "@supermarket"
    When "milk" is marked done
    And "@homedepot errand 0" is marked done
    And the done items are cleared from "@supermarket"
    And the done items are cleared from "@homedepot"
    And the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    And the trip "@homedepot" reads "<label>"
    And the loose ends list shows "coffee" tagged "@supermarket"
    And the loose ends list shows "<loose>" items
    And the pool screen does not mention "milk"

    Examples:
      | total | trips      | label    | loose |
      | 3     | @homedepot | 2 things | 1     |

  # trip-persistence-re-earns-its-trip-03: a tag whose run has ended must reach three again
  Scenario: A tag whose run has ended must reach three again
    Given "<total>" pool tasks tagged "@homedepot"
    And "<done>" of them are marked done
    And the done items are cleared from "@homedepot"
    When a pool task "buy a hinge" tagged "@homedepot"
    And a pool task "pick up trim" tagged "@homedepot"
    And the pool screen is viewed
    Then the pool screen offers no trips
    And the loose ends list shows "<loose>" items
    When a pool task "return the drill" tagged "@homedepot"
    And the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    And the trip "@homedepot" reads "<label>"

    Examples:
      | total | done | loose | trips      | label    |
      | 3     | 3    | 2     | @homedepot | 3 things |

  # trip-persistence-completing-what-is-left-holds-04: completing what a tidied trip has left does not dissolve it
  Scenario: Completing what a tidied trip has left does not dissolve it
    Given "<total>" pool tasks tagged "@homedepot"
    And "<done>" of them are marked done
    And the done items are cleared from "@homedepot"
    When the pool screen is viewed
    Then the trip "@homedepot" offers a complete-group control named "<name>"
    When the group "@homedepot" is completed
    And the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    And the trip "@homedepot" reads "<label>"
    And the trip "@homedepot" offers no complete-group control
    And the loose ends list shows "<loose>" items

    Examples:
      | total | done | name           | trips      | label       | loose |
      | 5     | 3    | Complete all 2 | @homedepot | 2 of 2 done | 0     |

  # trip-persistence-unchecking-inside-a-run-05: unchecking inside a run puts the item back without re-earning the trip
  Scenario: Unchecking inside a run puts the item back without re-earning the trip
    Given "<total>" pool tasks tagged "@homedepot"
    And "<done>" of them are marked done
    And the done items are cleared from "@homedepot"
    And the group "@homedepot" is completed
    When one struck item is unchecked
    And the pool screen is viewed
    Then the pool screen offers the trips "<trips>"
    And the trip "@homedepot" reads "<label>"
    And the trip "@homedepot" shows "<open>" open items

    Examples:
      | total | done | trips      | label       | open |
      | 5     | 3    | @homedepot | 1 of 2 done | 1    |
