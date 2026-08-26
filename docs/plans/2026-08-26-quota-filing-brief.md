# Handoff brief — `quota-filing`

**Date:** 2026-08-26 · **Issue:** #138 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **The owner walked into this an hour ago, on #147's preview.** *"It brings up the page but it doesn't show any of the things that I've triaged as a quota."*
>
> Base is `origin/trunk`, **green** at `f2a89a7`. Cut from it as normal.

---

## Why this slice, and why now rather than after #140

**Two quota concepts are live in the product at once, and the owner has data in the wrong one.**

- **Triage** writes to `tasks`: `kind='quota'` with `target_count`, `target_minutes_each`, `period` (`triage/store.rs:21`).
- **The Quota screen** reads `SELECT id, name, weekly_target_minutes FROM quotas ORDER BY id ASC` — **and nothing else** (`quota/store.rs:42`). `quotas` is migration `0014`'s own table, written only by `+ Define a new quota` on the screen itself.

`quota/store.rs`'s header says it outright: *"a different table from `tasks`, which `TaskKind::Quota` keeps using untouched."* **This was a knowingly accepted transitional state** (decisions log, 2026-08-25) and this slice is what closes it.

**The sequencing fact, checked rather than assumed.** Pool and Committed each filter `WHERE tasks.kind = '<their own>'`. A `kind='quota'` row therefore appears on **no** screen — its only window in the entire product is the `Tasks` list at the bottom of `/`, and `task_row.html` is one line: `[{{ kind }}] {{ text }} {{ tag }}`. **The sessions, minutes-each and period the owner typed are stored and rendered nowhere.**

**#140 deletes that list.** Land #140 first and quota triage becomes a silent write to a table nothing renders. **This slice goes first.**

## Demo

**On the phone.** Label the pull request `preview`; the poller picks it up once the suite is green (**#146** — a red branch is invisible rather than merely untidy).

1. **Tap the fourth tab.** `workout` and `learning with lev` are there as real quotas, **beside** any you defined on the screen — with bars, readouts and the quick-log controls. **They came across from triage; you did not retype them.**
2. **Tap `+1h` on `workout`.** The bar moves. Everything #147 shipped still works on a quota that arrived this way.
3. **Capture something** — *"finish chapter 3"*. Triage it. **Tap `Quota`.**
4. **You are offered chips, not a form.** `workout`, `learning with lev`, `Piano`. **No sessions field, no minutes-each field, no period dropdown.** Pick `learning with lev`.
5. **Expand `learning with lev`.** `Filed here` lists *finish chapter 3*.
6. **Expand `workout`.** No `Filed here` section at all — nothing has been filed there.

**What you can see that you could not before:** the two quotas you triaged days ago, on the screen that exists for them.

## Scope

**In:**
- **Migrate the two existing `kind='quota'` rows into `quotas`.** Migration `0016` (`T-migrations-append-only`; `0015` is applied and immutable).
- **Filing:** choosing `Quota` at triage files the capture **into an existing quota, chosen from chips** (`D-quotas-are-selected-not-typed`).
- **`Filed here`** on the expanded row, gated on the quota having filed items.
- **Retire `TaskKind::Quota`** and the triage form's `target_count` / `target_minutes_each` / `period` fields.
- **Decide `period`.** See below — it is owed a reason, not a deletion.

**Out:** editing, renaming, retargeting or removing a quota (**#148**); reorder controls (**#139**), **whose absence must not be asserted — on a quota it is undecided, not settled**; **#118**; **#145**; **#146**; **#108**; **#149**; **#140**; and any visual design system.

## The live data, so the migration is designed against facts rather than possibilities

Read from the owner's database (`~/.local/share/trellis/trellis.db`) on 2026-08-26:

| `tasks.id` | capture text | `target_count` | `target_minutes_each` | `period` | tag |
|---|---|---|---|---|---|
| 4 | `workout` | 3 | 45 | `week` | — |
| 8 | `learning with lev` | 1 | 30 | `week` | — |

**Two rows. Both `week`. Neither tagged. Zero `month` rows anywhere.** Task counts overall: 31 `pool`, 2 `quota`, 0 `committed`.

**And the live database has no `quotas` table at all** — it is still on a pre-`0014` binary, because no release published between `bf8c7f9` and `f2a89a7`. **So on the owner's machine this migration runs `0014`, `0015` and `0016` in one go**, and the `quotas` table will be empty when `0016` reaches it. **Do not write a migration that assumes a name collision cannot happen anyway** — it can on any other database, including the preview's seeded one.

**The conversion is a multiplication:** `weekly_target_minutes = target_count × target_minutes_each`. 3 × 45 = 135. 1 × 30 = 30. **State it in the migration's comment as a decision** — it is the only reading that preserves the target, and `D-quota-no-rollover`'s Monday reset makes *per week* the only period it can land in.

## `period`: owed a reason, and here is the evidence for it

`T-period-closed-set` closed `period` to `week | month` **deliberately**, so retiring `month` needs an argument, not a deletion. **Three facts, and the specifier should weigh them rather than inherit a conclusion:**

1. **`D-quota-no-rollover` resets counters on Monday and carries no shortfall forward.** A monthly target measured by a weekly counter that resets has no mechanism behind it.
2. **The canvas draws only `hours a week`** (`Trellis.dc.html`, the define form's *"hours a week — both are required"*).
3. **The live database has zero `month` rows**, so nothing is lost in practice — but **that is the weakest of the three** and must not be the argument. A schema is not justified by today's contents.

**Write the row.** If `month` goes, `T-period-closed-set` gets a dated superseding entry with this reasoning; the PM will place it. **Say so in your *first* handoff, not your last** — the `decision citations resolve` gate went red on `trip-persistence` for exactly this and cost the owner a CI cycle on a `strict` branch.

## `T-three-task-kinds` and `T-unknown-kind-rejected` both move, and one of them is a trap

**`T-three-task-kinds` says `kind` is a three-variant sum type.** Retiring `TaskKind::Quota` makes it two. That is a settled decision changing, and it needs the same treatment as `period`.

**The trap is `T-unknown-kind-rejected`:** *"`kind` is validated against the three variants at the boundary. A submission naming anything else is rejected with `422 {"unknown_kind": <submitted>}`."* **So does `kind=quota` posted directly now become an *unknown kind*, or does it mean *file into a quota*?** `features/unknown_kind_rejection.feature`'s own title is *"Triage rejects a kind outside pool, committed and quota"*. **Both answers are defensible and they are not the same product.** Decide it, say which, and make the feature file say it too.

**Eleven feature files mention quota**, and `features/quota_triage_validation.feature` (17 lines) is written entirely against the target fields. **`T-quota-targets-required` is the decision under it** and it does not survive this slice intact.

**#90's trap applies to every one of them:** *"if a scenario's point is that nothing happens, its parameters are not under test."* Nine mutants survived on exactly that once. **A validation scenario whose rejected fields stop existing does not become a passing scenario — it becomes a scenario asserting nothing.**

## The canvas draws a filed item as a bare line of text, and that is a decision to make deliberately

`Trellis.dc.html:291-293`: `<sc-for list="{{ q.filed }}" as="f"><div>{{ f.text }}</div></sc-for>`. **No checkbox. No delete. No controls of any kind.**

**Read it against what sits directly beneath it:** each session in `This week` carries a day `<select>`, a minutes `<input>` and a delete button. **The mock is perfectly capable of drawing controls on a row and chose not to here.** That makes the absence look like a statement rather than an oversight — but `T-canvas-is-authoritative-where-it-speaks` is the rule that says **check rather than assume**, and this is the **fourth** time that distinction has had to be drawn on this screen. **Make the call, record it, and do not let it be settled by whichever template was easiest.**

**The question underneath it:** can a filed item be completed? If yes, from where — and does `mark_done` gain a third caller (`T-one-front-door-per-capability`, `T-cross-capability-invariants-need-an-owner`)? If no, say what a filed item *is*, given it began as a capture.

## Where the writes go

- **`T-one-front-door-per-capability`.** Filing is one capability with one front door, however many routes call it.
- **`T-set-operations-execute-in-the-store`.** A quota's filed items, and whether it has any, are set operations — one statement, not a fetch-and-filter in a handler.
- **`T-422-is-product-wide`.** Filing into a quota that does not exist is a `422` whose body is the re-rendered fragment it failed against.
- **`T-forms-swap-one-fragment`.** `Filed here` appearing and the row it lives in change together: one fragment, one id.
- **`T-ephemeral-view-state-rides-the-request`.** `#quota-body`'s `expanded=` hook exists now (#147). **Filing from triage swaps `#lists`, not `#quota-body`** — check whether anything needs to survive that swap before assuming it does not.

## Watch

1. **`quota::check_name` runs in memory** — `store::existing_names` has no `WHERE`, a tracked Gaps entry deferred to *"the quota identity ruling that belongs with #138."* **This is #138.** Either rule on it and fix it, or say in the handoff why it still waits. Do not leave it deferred to itself.
2. **`T-collation-enforces-name-identity`.** `quotas.name` is `UNIQUE COLLATE NOCASE`. The migration turns `captures.raw_text` into a quota name — **two captures whose text differs only by case or trailing space collide**, and a migration that fails on the owner's machine at 7am is the worst place to discover it.
3. **`T-a-check-must-be-seen-to-fail`.** #147's own gap is filed as **#149** — three QA procedures that ran green and were never seen red, because no acceptance mutation can reach them. **Do not repeat the shape.** Pair each breakage with the assertion it must trip and report which you ran.
4. **#145 — the manifest cannot record a survivor.** Report what the tool printed, not what the file says. Do not hand-edit it.
5. **DRY was 2.51% at #147**, up from 2.23%, against a 3% product-code threshold. **Two slices in a row have raised it.** Extract a shared family early rather than at the end.
6. **`phone_layout.cjs` seeds `/` with `expectOverflow: true`.** This slice shortens the triage form. If `main` stops overflowing at 390px that gate goes red **for the right reason** — re-seed, do not relax the assertion.
7. **`/quota` is inside `colour.cjs` and `phone_layout.cjs` now** (#147). `Filed here` is new markup on a gated screen: expect the palette and contrast gates to have an opinion, and treat what they find as a finding rather than something to style around.
8. **A skipped check is not a blocked check.** `migrations are append-only` is `if: github.event_name == 'pull_request'` (`ci.yml:73`) and skips on every `trunk` push **by design**. The previous brief read that as damage and was wrong; the correction is on #93. **Read the `if:` before reporting a skip.**
9. Scratch in `./tmp/`. **Open a new pull request and label it `preview`.**

## Open questions for the specifier

1. **Does `kind=quota` posted directly become an unknown kind, or the filing verb?** (`T-unknown-kind-rejected`.)
2. **Can a filed item be completed, and from where?** (The canvas draws no control.)
3. **Does a filed item keep its context tag, and does it still appear on the Pool screen if it has one?** Both of the owner's quota rows are untagged, so the live data will not tell you.
4. **What is the migration's answer to a name collision** between a converted capture and an existing screen-defined quota?

## Source

- **#138** — the issue, and the PM comment recording the owner hitting this in real use
- `crates/trellis-server/src/quota/store.rs:1-3,42` — the two tables, and the header that says so
- `crates/trellis-server/src/triage/store.rs:21` — what triage actually writes
- `crates/trellis-server/src/inbox/store.rs:107` — `list_tasks`, the only window a quota-kind row has
- `crates/trellis-server/templates/task_row.html` — one line, and no target on it
- `docs/design/Trellis.dc.html:285-320` — `Filed here` beside `This week`, and the controls one has that the other does not
- `features/quota_triage_validation.feature`, `features/task_kinds.feature`, `features/unknown_kind_rejection.feature` — the three that do not survive intact
- `docs/decisions.md` — `D-quotas-are-selected-not-typed`, `D-quota-no-rollover`, `T-period-closed-set`, `T-three-task-kinds`, `T-unknown-kind-rejected`, `T-quota-targets-required`, `T-collation-enforces-name-identity`, `T-one-front-door-per-capability`, `T-set-operations-execute-in-the-store`, `T-canvas-is-authoritative-where-it-speaks`, `T-migrations-append-only`
- **#148** (a quota cannot be edited or removed — the rename half waits on this slice's identity ruling) · **#149** · **#140** (why this goes first)
- PR **#147** / merge `f2a89a7` — the half that shipped, and the green base
