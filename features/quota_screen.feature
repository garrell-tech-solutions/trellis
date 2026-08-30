# mutation-stamp: sha256=3c50b374e258cb15325eb990367bcddfd29b5cb4bb6626e0d9cf6b85a549a921
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-27T04:26:46.417420996Z","feature_name":"The quota screen shows what you triaged as a quota","feature_path":"features/quota_screen.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:6a7bc25388ad53cb77b3b29b96c83ab6125365eabfbcd36fbee7ef3586011740","scenarios":[{"index":0,"name":"The fourth screen exists and names itself","scenario_hash":"37ae0666897ec22a83ade2ed8ba5b68c6a6d9f50929d36a9b50adb7882a9fe88","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-26T17:12:08.066627455Z"},{"index":1,"name":"An empty quota screen says what a quota is and points at Capture","scenario_hash":"97590824a00148e79a394a6225928241155bbd87b156826984106c56b11029eb","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-26T17:12:08.066627455Z"},{"index":2,"name":"A triaged quota reads its target with nothing logged yet","scenario_hash":"0c0cdd36fd656b1ad4da67629bde7c1a8abc9d20fd3ac1f67236b9b3c24b2866","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-26T17:12:08.066627455Z"},{"index":3,"name":"Quotas are listed in the order they were triaged","scenario_hash":"98a19d60e6f6ba4e135abe18fa86710938e7a56d616ebf3e8e4b0d18f2f6e17f","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-26T17:12:08.066627455Z"}]}
# acceptance-mutation-manifest-end

# quota-screen-fourth-screen-01: the fourth screen exists and names itself
# quota-screen-empty-explains-itself-02: an empty quota screen says what a quota is and points at Capture
# quota-screen-reads-its-target-03: a triaged quota reads its target with nothing logged yet
# quota-screen-triaged-order-04: quotas are listed in the order they were triaged
# quota-screen-retarget-05: a quota's weekly target can be changed, and the readout moves
# quota-screen-rename-06: a quota can be renamed, and the listing follows
# quota-screen-rename-refused-07: renaming onto a name already taken is refused
# quota-screen-remove-08: a removed quota leaves the screen and stops counting
# quota-screen-removed-name-returns-09: triaging a removed quota's name brings it back with its hours
#
# REMOVE MEANS ARCHIVE, settled by the owner 2026-08-29: the quota stops
# appearing and stops counting, its sessions are kept, and there is nowhere to
# browse them -- the bargain `D-kill-means-archive` already struck for tasks.
# Nothing is deleted, so the foreign key never fires.
#
# AND IT WOULD HAVE FIRED. The brief warned that a delete silently orphans
# sessions because no `PRAGMA foreign_keys` is set in the tree. THE OPPOSITE IS
# TRUE, proved by running it: sqlx sets it ON for every connection, so a delete
# is REFUSED with SQLite 787. A delete-based removal would have passed testing
# and 500'd on exactly the quotas the owner had been logging against.
#
# TRIAGING A REMOVED NAME REVIVES IT (-09) rather than being refused by a quota
# you cannot see. Freeing the name would mean rebuilding `quotas` -- `UNIQUE`
# is inline in 0014 -- against live data, for a case the Monday reset hides.
# SO A RENAME ONTO AN ARCHIVED NAME IS STILL REFUSED (-07): `NameStanding`
# weighs every quota, the only reading that agrees with the column
# (`T-collation-enforces-name-identity`). Triage alone resolves `Taken` by
# reviving.
#
# THE QUOTA'S `tasks` ROW IS LEFT ALONE -- `mark_done` is the one place
# `archived_at` is written on a task, and that row is invisible everywhere.
# -02's "offers no way to define a quota" holds, and -03 now says why: a
# control that CHANGES a quota is not one that CREATES it (#150).
#
# THIS SCREEN CREATES NOTHING (#150, owner 2026-08-26). `+ Define a new quota`
# and its form moved to `quota_triage_validation.feature`, where the one door
# is. That era's `-09`, asserting triaged quotas do NOT appear here, was
# inverted rather than weakened.
Feature: The quota screen shows what you triaged, and lets you change or remove it

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
    And the quota "Piano" offers to be changed and removed

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

  # quota-screen-retarget-05: a quota's weekly target can be changed, and the readout moves
  Scenario: A quota's weekly target can be changed
    Given a quota named "Piano" with a target of "4" hours a week
    And a session of "30" minutes on "Mon" is logged against "Piano"
    When the quota "Piano" is changed to "Piano" at "<hours>" hours a week
    And the "quota" screen is viewed
    Then the quota "Piano" reads "<readout>"
    And the quota "Piano" notes "<note>"

    Examples:
      | hours | readout   | note                          |
      | 2     | 30m / 2h  | 1h 30m left this week · 25% |

  # quota-screen-rename-06: a quota can be renamed, and the listing follows
  Scenario: A quota can be renamed, and what it logged comes with it
    Given a quota named "Piano" with a target of "4" hours a week
    And a session of "30" minutes on "Mon" is logged against "Piano"
    When the quota "Piano" is changed to "<new_name>" at "4" hours a week
    And the "quota" screen is viewed
    Then the quota screen offers the quotas "<new_name>"
    And the quota "<new_name>" reads "<readout>"

    Examples:
      | new_name     | readout  |
      | Piano theory | 30m / 4h |

  # quota-screen-rename-refused-07: renaming onto a name already taken is refused
  Scenario: Renaming onto a name already taken is refused
    Given a quota named "Piano" with a target of "4" hours a week
    And a quota named "Running" with a target of "3" hours a week
    When the quota "Running" is changed to "<name>" at "3" hours a week
    Then the change is refused with "“Piano” already exists at 4 h a week. Log your time against that one, or give this a different name."
    And the quota screen offers the quotas "<quotas>"

    Examples:
      | name   | quotas          |
      | piano  | Piano, Running  |
      | Pi-ano | Piano, Running  |

  # quota-screen-remove-08: a removed quota leaves the screen and stops counting
  Scenario: A removed quota leaves the screen and stops counting
    Given a quota named "Piano" with a target of "4" hours a week
    And a quota named "Running" with a target of "3" hours a week
    And a session of "30" minutes on "Mon" is logged against "Piano"
    When the quota "Piano" is removed
    And the "quota" screen is viewed
    Then the quota screen offers the quotas "<quotas>"
    And the quota screen reports "<meta>" beside its title

    Examples:
      | quotas  | meta    |
      | Running | 1 quota |

  # quota-screen-removed-name-returns-09: triaging a removed quota's name brings it back with its hours
  Scenario: Triaging a removed quota's name brings it back with what it logged
    Given a quota named "Piano" with a target of "4" hours a week
    And a session of "30" minutes on "Mon" is logged against "Piano"
    And the quota "Piano" is removed
    And a capture with raw text "Piano" is waiting in the untriaged queue
    When the capture is triaged as a quota named "Piano" with a target of "<hours>" hours a week
    And the "quota" screen is viewed
    Then the quota screen offers the quotas "Piano"
    And the quota screen reports "<meta>" beside its title
    And the quota "Piano" reads "<readout>"

    Examples:
      | hours | meta    | readout  |
      | 2     | 1 quota | 30m / 2h |
