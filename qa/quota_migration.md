# QA Procedure: Quotas triaged before the quota screen existed appear on it

Covers: `features/quota_migration.feature`.

## Interface used

The migration command, the four screens over HTTP, and `sqlite3` **read-only**
for corroboration. **Never `sqlite3` to write** — the whole point is what the
migration does on its own.

## This is the procedure that touches the owner's real data

**Read the real rows first.** From `~/.local/share/trellis/trellis.db` on
2026-08-26:

| `tasks.id` | capture text | `target_count` | `target_minutes_each` | `period` | tag |
|---|---|---|---|---|---|
| 4 | `workout` | 3 | 45 | `week` | — |
| 8 | `learning with lev` | 1 | 30 | `week` | — |

**Two rows. Both `week`. Neither tagged. Zero `month` rows anywhere.** 31
`pool`, 2 `quota`, 0 `committed`.

**⚠️ Take a copy of the owner's database before anything else, and work on the
copy.** A migration that goes wrong here loses the only data this product has
ever held. Say in the report that you did.

## The conversion is a multiplication, and it is a decision

**`weekly_target_minutes = target_count × target_minutes_each`.**

- 3 × 45 = **135** → reads **`2h 15m`**
- 1 × 30 = **30** → reads **`30m`**

It is the only reading that preserves the target the owner actually set, and
`D-quota-no-rollover`'s Monday reset makes *per week* the only period it could
land in. **Confirm the multiplication is stated in the migration's own comment
as a decision** — if the arithmetic is there with nobody's name on it, say so.

## `period` is retired, and the reason has to be in the log

`T-period-closed-set` closed `period` to `week | month` **deliberately**, so
retiring it is owed an argument. Two reasons carry it and a third does not:

1. **`D-quota-no-rollover` resets counters on Monday** and carries no
   shortfall forward. A monthly target measured by a weekly counter that
   resets **has no mechanism behind it.**
2. **The canvas draws only `hours a week`** on the one form that sets a target.
3. **Not an argument, and named so it is not mistaken for one:** the live
   database has no `month` rows. **A schema is not justified by today's
   contents.**

**Check that `T-period-closed-set` has a dated superseding entry carrying 1
and 2** before this merges. **The `decision citations resolve` gate went red
on `trip-persistence` for exactly this** and cost the owner a CI cycle on a
`strict` branch.

## Procedure — the owner's own upgrade path

**The live database has no `quotas` table at all.** It is still on a pre-`0014`
binary because nothing published between `bf8c7f9` and `f2a89a7`. **So on the
owner's machine `0014`, `0015` and `0016` run in one go**, and `quotas` is
empty when the conversion reaches it.

1. Copy the owner's database. Confirm with `sqlite3` that it has **no
   `quotas` table** and **two `kind='quota'` rows.**
2. Run the migration command against the copy.
3. Start the server against it. Read the Quota screen.

### Expected Observable Outcomes
- The migration **exits successfully** and the journal mode is still `wal`.
- The Quota screen offers **`workout`** and **`learning with lev`**, reading
  **`0m / 2h 15m`** and **`0m / 30m`**, meta **`2 quotas`.**
- **The 31 pool tasks are untouched** and the Pool screen is unchanged. Count
  them before and after.
- **Nothing is duplicated.** Exactly two quotas, not four. Check the meta *and*
  `sqlite3`.
- **Re-run the migration.** It is a no-op: still two quotas, still the same
  targets. **A conversion that runs twice and doubles something is the failure
  this step exists for.**

## Procedure — a name that is already taken

**This cannot arise on the owner's machine** — `quotas` is empty there — **but
it can on the preview's seeded database and on any machine that ran #147.**

1. Build a database that has been migrated as far as `0015`, holding a
   screen-defined quota **`Piano`** at 4 hours **and** a `kind='quota'` task
   named **`piano`** targeting 2 sessions of 30 minutes.
2. Run the migration. Read the Quota screen.
3. Repeat with **`PIANO`**.

### Expected Observable Outcomes
- **The migration exits successfully. It does not fail.** A migration that
  aborts at 7am on the owner's phone is the worst possible outcome, and
  `quotas.name` is `UNIQUE COLLATE NOCASE`, so a naive `INSERT` **will** hit
  the constraint. **If the migration errors, that is the finding.**
- **One quota, not two.** `Piano` survives, reading **`0m / 4h`** — the
  existing row's own target, **not the converted one's and not the two summed.
  Summing would invent a number nobody chose**, which is the same class of
  harm as the second counter this prevents.
- Meta reads **`1 quota`.**

## Procedure — the limit that is deliberately not asserted

**Read this before reporting it as a defect.**

`UNIQUE COLLATE NOCASE` lets a migration fold **case** for free. It **cannot**
reach `scheduler_core::quota::check_name`, which also folds spaces and
punctuation and measures edit distance — **that rule is Rust and a migration
is SQL.** So a `kind='quota'` task named **`Pi-ano`** beside a quota named
`Piano` **would convert to two rows.**

1. Build that database. Run the migration. Look at the screen.

### Expected Observable Outcomes
- **Expect two quotas, and do not report it as a regression.** No acceptance
  scenario asserts this case **on purpose**: asserting it would enshrine a
  wart, and a later migration that folded punctuation properly would then go
  red **for a good change.**
- **Do report what you actually saw**, and say whether it looks worth an
  issue. **It cannot arise on the owner's data** — there are no collisions of
  any kind there — which is why it was left as a known limit rather than
  solved speculatively.
- **If it converts to one row, that is also not a defect** — it means the
  implementation reached further than the spec required. Say which happened.

## Procedure — prove the checks can fail

**`T-a-check-must-be-seen-to-fail`.** Each breakage names **the assertion it
must trip.**

1. **Add the two targets instead of multiplying them** (3 + 45). → **`-01`
   fails on the `workout` row's readout**, which becomes `0m / 48m` rather
   than `0m / 2h 15m`. **The `learning with lev` row is the trap: 1 + 30 = 31
   and 1 × 30 = 30, so it fails by one minute** — a difference small enough to
   read past. **Confirm both rows fail and say what each said.**
2. **Convert only the first quota task found.** → **`-01` still passes on the
   `workout` row**, because each row runs as its own scenario. **The count is
   what catches this**: run the owner's real database, where the meta must
   read `2 quotas`. **This breakage is invisible to the acceptance suite and
   visible only in the procedure above** — which is the reason that step
   counts rather than merely looking.
3. **Let the migration `INSERT` without checking for an existing name.** →
   **`-02` fails on both rows, and it fails by the migration erroring**, not
   by showing two quotas — the constraint refuses it. **A different red, and
   worth telling apart** when reading the output.
4. **Keep the existing quota but overwrite its target with the converted
   one.** → **`-02` fails on the readout only**: one quota named `Piano`,
   reading `0m / 1h` instead of `0m / 4h`. The count is unmoved. **This is the
   failure mode that looks like it worked.**

### Expected Observable Outcomes
- **Four breakages, four distinct messages, then restore and a clean pass with
  a clean `git status`. Say which you ran.**
- **Breakage 2 is the one the acceptance suite cannot see.** Do that one.

## Independent of Implementation

This procedure depends only on what a database holds before the migration
runs, that the command succeeds, and what the Quota screen shows afterwards.
It does not depend on the migration's number, whether it is one statement or
several, or how the conversion is expressed in SQL.
