# mutation-stamp: sha256=1af961528b9c7e581d6de0d96ef36dc6f4e4f1d74309f15806e2f56502f4a3f4
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-25T17:18:53.570356495Z","feature_name":"A trip survives being tidied","feature_path":"features/trip_persistence.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:03b6b9924007468bb0ba4f40eeffb5bb54d176a57de8b5f835294d15a18c568f","scenarios":[{"index":0,"name":"A tidied trip stands until its last item is cleared","scenario_hash":"cca23246ec85c89b4192e8381bc06bbe31d9e124d9c9063b836bc09ba5ceb1aa","mutation_count":8,"result":{"Total":8,"Killed":8,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"},{"index":1,"name":"A tag that never reached three is not made a trip by being tidied","scenario_hash":"b88a1867cf6afbd83947731cefc89a16356a76ba6da6f95a4d35de96e4ff7cb7","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"},{"index":2,"name":"A tag whose run has ended must reach three again","scenario_hash":"fc99ecd71a3a664ca1895e42fd0e40398d0507adc319fa4a99524619aa03f3ee","mutation_count":5,"result":{"Total":5,"Killed":5,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"},{"index":3,"name":"Completing what a tidied trip has left does not dissolve it","scenario_hash":"7d2c8c0b4cdcb8389ed4cf9d88a04552c86e9dc21a6ea5082104a49f9ff4324a","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"},{"index":4,"name":"Unchecking inside a run puts the item back without re-earning the trip","scenario_hash":"a24c3202e73c69ee1b5c72b7eec8cd786c6d80a06ce309059c30de583e469d13","mutation_count":5,"result":{"Total":5,"Killed":5,"Survived":0,"Errors":0},"tested_at":"2026-08-25T17:18:53.570356495Z"}]}
# acceptance-mutation-manifest-end

# trip-persistence-stands-until-its-last-item-clears-01: a tidied trip stands until its last item is cleared
# trip-persistence-never-formed-stays-loose-02: a tag that never reached three is not made a trip by being tidied
# trip-persistence-re-earns-its-trip-03: a tag whose run has ended must reach three again
# trip-persistence-completing-what-is-left-holds-04: completing what a tidied trip has left does not dissolve it
# trip-persistence-unchecking-inside-a-run-05: unchecking inside a run puts the item back without re-earning the trip
#
# THE SAME DEFECT ONE TAP LATER. #122 stopped the panel dissolving when you
# CHECKED things off; it still dissolved when you CLEARED them. Five things
# at @homedepot, three got, tap the clear control to tidy -- and the last two
# scattered into loose ends among everything else with the owner still
# standing in the shop. `D-a-trip-survives-being-worked` was EXTENDED by the
# owner 2026-08-24: CLEARING TIDIES THE PANEL, IT DOES NOT DISSOLVE IT.
#
# --- TWO RULES, AND THIS SLICE IS WHERE THEY STOP BEING ONE PREDICATE -----
# #122 could get away with one count because nothing left the uncleared set
# while a trip was being worked. Clearing is the first thing that removes an
# item from that set mid-run, so the single predicate now has to split:
#
#   A RUN at a tag begins when something lands there with nothing else
#   waiting, and ends when the last thing waiting there is cleared away.
#   FORMATION: a run of three or more things is a trip.
#   PERSISTENCE: it stays one for as long as the run lasts.
#
# THE RUN IS THE UNIT, NOT THE MOMENT. `pool.rs:121` re-derives trip-ness on
# every render from the uncleared set alone, and `list_pool_tasks` never
# returns a cleared row -- so the current run's own history is not in the
# data the rule can see. HOW THE STORE KNOWS A TAG IS INSIDE A RUN IS THE
# CODER'S CALL AND THE PULL REQUEST'S ARGUMENT, not something this file
# asserts. `cleared_at` is a millisecond timestamp rather than a flag, so the
# clears are ordered in time and a derivation is worth attempting;
# `T-ephemeral-view-state-rides-the-request` is the test if it needs a
# column, and a trip that dissolves when you lock your phone in the car park
# is this same defect one gesture further out -- SO A COLUMN MAY WELL BE THE
# HONEST ANSWER, and reaching for one without the argument is not.
#
# THE PANEL LEAVES ON A TAP, NEVER ON A TICK. Settled by the owner
# 2026-08-25, and it is the one thing here that was genuinely open. Tick the
# last open thing in a trip that clearing has taken down to two, and THE
# PANEL HOLDS, reading "2 of 2 done" with the clear control still on it (01).
# The brief's demo said it goes at that moment; it goes one tap later. THREE
# REASONS, and the third is the one that decides it:
#   - `trip-progress-fully-done-06` ALREADY DOES EXACTLY THIS at three items
#     -- a full trip with everything done keeps its panel reading "3 of 3
#     done" until you clear it. Going at two and staying at three would make
#     the item count decide whether finishing a trip wipes it, which is not
#     explicable to anyone holding a basket.
#   - YOU CAN UNCHECK WHAT YOU CAN SEE (#122). A panel that vanishes on the
#     last tick takes the undo with it (05).
#   - NOTHING CLEARS ITSELF. The owner chose an explicit control over any
#     automatic sweep, and a panel that swept its own leftovers away would be
#     that sweep wearing a trigger instead of a timer. The alternative that
#     does NOT sweep them is worse: the leftovers stay uncleared and
#     invisible, and the next thing captured at that tag resurrects the panel
#     reading "2 of 3 done" over month-old strikes -- WHICH IS PRECISELY THE
#     RUNAWAY #129 WAS FILED ABOUT, arriving through the fix for it.
#
# A TAG MUST RE-EARN ITS TRIP (03). This is the bound #129 asked for and the
# reason the naive derivation -- *ever reached three, counting cleared ones*
# -- is wrong: a tag cleared last month with one thing on it today is not a
# trip, and over a year every frequently-used tag would become a permanent
# panel. The run ending is the bound, and it is a real event rather than a
# time window: the owner has refused a time window once already (#127) and
# this does not ask for one.
#
# TWO SCENARIOS IN `trip_progress.feature` CHANGE WITH THIS SLICE, BOTH IN
# THE OPEN. `trip-progress-clearing-can-drop-a-group-05` ASSERTED THE
# DEFECT -- it is green today and had to go red -- and is reversed, not
# deleted and not weakened. `trip-progress-clear-done-04` also moves: its
# two survivors used to land in loose ends and now stay in the panel, so its
# `loose` count goes to zero. THE BRIEF SAID ONE SCENARIO REVERSES; IT IS
# ONE REVERSAL AND ONE CONSEQUENCE, and the second is named here rather than
# edited quietly. `trip-progress-fully-done-06` stays exactly as written, and
# so does every scenario in `trip_controls.feature`.
#
# WHAT IS NOT ASSERTED HERE. Reloading between every step is the demo's own
# instruction and every `the pool screen is viewed` above is a fresh request,
# so this file already proves the rule is not per-render state -- but a REAL
# reload, and a restart, are facts about a rendered page and a running
# server, and they live in `qa/trip_persistence.md`
# (`T-ephemeral-view-state-rides-the-request`: choose the tier from the
# nature of the state). Also not here: expanding and collapsing (#120, in the
# browser tier for the same reason), reordering loose ends (#95 --
# `T-trips-are-derived-not-ranked` still forbids a reorder control on a trip)
# and the query's in-memory grouping (#108, open on this exact query and not
# this slice's to fix).

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
