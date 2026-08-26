# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-26T17:13:56.866499632Z","feature_name":"Quotas triaged before the quota screen existed appear on it","feature_path":"features/quota_migration.feature","background_hash":"f421fc66d06a0859aaedbc32143033d9bcb61eda4df569c1c77d214253346da0","implementation_hash":"sha256:75684c8c6afc0562225d276c770d990e818c22c4a9ed1dd4d676ef5ee87ad012","scenarios":[]}
# acceptance-mutation-manifest-end

# quota-migration-converts-what-was-triaged-01: quotas triaged before the quota screen existed appear on it, with their targets multiplied out
# quota-migration-no-second-counter-02: a converted quota does not become a second counter beside a quota of the same name
#
# THIS IS THE SCENARIO THE OWNER WALKED INTO. #138 opens with "it brings up
# the page but it doesn't show any of the things that I've triaged as a
# quota", and the two rows below are the owner's real data, read from
# `~/.local/share/trellis/trellis.db` on 2026-08-26: `workout` at 3 sessions
# of 45 minutes, `learning with lev` at 1 of 30. Both weekly, neither
# tagged, and ZERO `month` ROWS ANYWHERE.
#
# --- THE CONVERSION IS A MULTIPLICATION, AND THAT IS A DECISION ----------
# `weekly_target_minutes = target_count × target_minutes_each`. 3 × 45 =
# 135, which reads `2h 15m`; 1 × 30 = 30, which reads `30m`. IT IS THE ONLY
# READING THAT PRESERVES THE TARGET the owner actually set, and
# `D-quota-no-rollover`'s Monday reset makes "per week" the only period it
# could land in anyway. The multiplication belongs in the migration's own
# comment as a decision, not as arithmetic nobody signed.
#
# --- `period` IS RETIRED, AND HERE IS THE ARGUMENT ------------------------
# `T-period-closed-set` closed `period` to `week | month` deliberately, so
# retiring it is owed a reason rather than a deletion. TWO REASONS CARRY IT
# AND A THIRD DOES NOT:
#   1. `D-quota-no-rollover` RESETS COUNTERS ON MONDAY and carries no
#      shortfall forward. A MONTHLY TARGET MEASURED BY A WEEKLY COUNTER THAT
#      RESETS HAS NO MECHANISM BEHIND IT -- it is not a stricter product, it
#      is a number that can never be reckoned.
#   2. THE CANVAS DRAWS ONLY `hours a week`, on the one form that sets a
#      target (`Trellis.dc.html`, "hours a week -- both are required").
#      `T-canvas-is-authoritative-where-it-speaks` and it speaks here.
#   3. NOT AN ARGUMENT, AND NAMED SO IT IS NOT MISTAKEN FOR ONE: the live
#      database holds no `month` rows. A SCHEMA IS NOT JUSTIFIED BY TODAY'S
#      CONTENTS, and if the first two reasons did not hold, this one would
#      not rescue it.
# `T-period-closed-set` needs a dated superseding entry carrying 1 and 2;
# the PM places it. FLAGGED IN THE FIRST HANDOFF, NOT THE LAST -- the
# `decision citations resolve` gate went red on `trip-persistence` for
# exactly this and cost the owner a CI cycle on a `strict` branch.
#
# --- WHAT THE MIGRATION MEETS ON THE OWNER'S OWN MACHINE ------------------
# The live database HAS NO `quotas` TABLE AT ALL -- it is still on a
# pre-`0014` binary, because nothing published between `bf8c7f9` and
# `f2a89a7`. So `0014`, `0015` and `0016` RUN IN ONE GO and `quotas` is
# empty when the conversion reaches it. -02 EXISTS ANYWAY: the preview's
# seeded database and any machine that ran #147 can hold a screen-defined
# quota, and a migration that fails at 7am is the worst possible place to
# find that out.
#
# -02 KEEPS THE EXISTING QUOTA'S OWN TARGET rather than the converted one's,
# and rather than adding them. The `quotas` row was the later deliberate
# act, and SUMMING TWO TARGETS WOULD INVENT A NUMBER NOBODY CHOSE -- which
# is the same class of harm as the two counters this scenario prevents.
#
# --- WHAT -02 DELIBERATELY DOES NOT ASSERT --------------------------------
# `T-collation-enforces-name-identity` puts `UNIQUE COLLATE NOCASE` on
# `quotas.name`, so A MIGRATION CAN FOLD CASE FOR FREE. It CANNOT reach
# `scheduler_core::quota::check_name`, which also folds spaces and
# punctuation and measures edit distance -- that rule is Rust and a
# migration is SQL. So "Pi-ano" beside "Piano" WOULD convert to two rows.
#
# THAT CASE IS LEFT UNASSERTED ON PURPOSE, and the distinction matters:
# asserting it would enshrine a wart, and a later migration that folded
# punctuation properly would then go red FOR A GOOD CHANGE. Unasserted is
# not the same as unnoticed -- it is recorded here, it cannot arise on the
# owner's data (no collisions of any kind), and it is the QA document's job
# to look at it rather than the acceptance suite's job to freeze it.
Feature: Quotas triaged before the quota screen existed appear on it

  Background:
    Given a trellis database that still keeps quota tasks apart from quotas

  Scenario: An upgraded database shows what was triaged as a quota
    Given it holds a quota task "<text>" targeting "<sessions>" sessions of "<minutes_each>" minutes per week
    When the migration command is run
    And the "quota" screen is viewed
    Then the quota screen offers the quotas "<text>"
    And the quota "<text>" reads "<readout>"
    And the quota screen reports "1 quota" beside its title

    Examples:
      | text              | sessions | minutes_each | readout     |
      | workout           | 3        | 45           | 0m / 2h 15m |
      | learning with lev | 1        | 30           | 0m / 30m    |

  # quota-migration-no-second-counter-02: a converted quota does not become a second counter beside a quota of the same name
  Scenario: A converted quota does not become a second counter beside a quota of the same name
    Given it holds a quota named "Piano" with a target of "4" hours a week
    And it holds a quota task "<text>" targeting "2" sessions of "30" minutes per week
    When the migration command is run
    And the "quota" screen is viewed
    Then the quota screen offers the quotas "<quotas>"
    And the quota "Piano" reads "<readout>"
    And the quota screen reports "1 quota" beside its title

    Examples:
      | text  | quotas | readout |
      | piano | Piano  | 0m / 4h |
      | PIANO | Piano  | 0m / 4h |
