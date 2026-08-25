# Handoff brief — `quota-screen`

**Date:** 2026-08-25 · **Issue:** #93 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **The fourth screen, and the last one that does not exist.** `platform/nav.rs:21` declares `ALL: [Page; 3]` with the comment *"a dead link is worse than no link"* — it has been waiting for this since #92.
>
> **This is the largest slice this pipeline has been handed, and the owner chose that knowingly** over a three-way cut. Read "If it does not fit" below **before** you start: there is a designated line to stop at, and stopping there is a success, not a failure.

---

## Demo

**On the phone.** Label the pull request `preview`.

1. **Tap the fourth tab.** It exists now.
2. **Define a quota** — `Piano`, `4` hours a week. Both fields are required and the form says so.
3. **Try to define `piano` again.** It warns you rather than quietly creating a second counter.
4. **Play piano for an hour. Tap `+1h`.** The bar moves; the readout says one hour against four.
5. **Tap `+30m`** after a short session. Then use `Other` to log the 20 minutes you did on Sunday.
6. **Expand the quota.** `This week` lists what you logged. **Fix the day you got wrong. Delete the one you did not actually do.** Both stick across a reload.
7. **Nothing carries into next week.** Monday it reads zero against four again.

**That loop — define, log, correct — is the slice.** If any step needs a `curl`, it is not done.

## A quota is not a task, and this is the load-bearing finding

Today `TaskKind::Quota { target_count, target_minutes_each, period }` (`scheduler-core/src/task/mod.rs:106`) makes a quota a **triaged capture**. The canvas and `D-quotas-are-selected-not-typed` describe something else: a quota is a **named container with a weekly hour target**, created **from this screen without a capture**, that sessions are logged against and items are later filed into.

**So this is a new first-class entity, not a reshape of three columns.** #93's *"that is a migration and a triage change"* undersells it, and the cut was decided on this reading.

**What that means for scope here:**

- **A new table.** Name, weekly hour target, and whatever identity the duplicate guard needs. Quotas are **not** rows in `tasks`.
- **`TaskKind::Quota` is not yours to remove.** It stays exactly as it is, `target_count`/`target_minutes_each`/`period` included, and the existing quota triage form keeps working untouched. **Two quota concepts coexist after this slice**, and `quota-filing` (slice 2, **#138**) closes that. Say nothing about `period` — its fate is slice 2's, with `T-period-closed-set` to amend or retire.
- **Existing `TaskKind::Quota` rows do not appear on this screen.** They are a different thing. Do not migrate them, do not display them, and do not let the query accidentally pick them up.

## Read the canvas, not #93's summary of it

`docs/design/Trellis.dc.html:238-352`. **The issue's bullet list flattens three things and this brief is the correction** — `T-canvas-is-authoritative-where-it-speaks` still has the specifier reading it directly, and this is the third slice where a PM summary and the drawing disagreed.

1. **The quick-log buttons are on every row and always visible** — `+30m`, `+1h` and `Other` at lines 264-268, **outside** `q.expanded`. Logging is one tap from the list; **you never have to expand a quota to log against it.** `Log a session` (day picker, minutes, `Add`) is a **per-row disclosure** behind the third button (`q.otherOpen`), not a section of the screen.
2. **`Filed here` and `This week` live in the same expanded row**, `Filed here` gated on `q.hasFiled` (line 287). **The canvas draws one row template with an optional section.** The log insists on **two first-class shapes**. Both are right: **model two shapes, draw one row.** This is `#94`'s situation exactly — canvas authoritative on layout, `docs/decisions.md` on behaviour, and the drawing of a behavioural distinction is ours. `Filed here` itself is **slice 2's**; what you owe here is a model that does not make a pure-hours quota into an item-bearing one with an empty list.
3. **The canvas draws reorder controls on quota rows** — `q.onUp` / `q.onDown`, `aria-label="Raise priority"`, lines 259-262. **They are deliberately out of scope** and filed as **#139**. Do not draw them, and do not assert their absence as a rule the way `pool-screen-nothing-reorders-05` does — that absence is undecided, not settled.

**What the canvas gives you concretely:** the 44px tap target on every control, `min="5" step="5"` on logged minutes, `min="0.5" step="0.5"` on the hours target, a 6px progress track with `q.fillStyle`, `font-variant-numeric: tabular-nums` on the readout and session summary, and the literal string *"hours a week — both are required"*.

## The duplicate-name guard is a product rule, not input polish

`D-quotas-are-selected-not-typed`: **"A mistyped name must not be able to create a quota."** The reasoning is the whole point — a typo *"creates a second quota that silently splits the week's hours across two counters and makes both wrong."*

The canvas draws the surface: `hasWarning` / `warningText` in a mint panel with a gold top border (lines 337-341), and a `createLabel`/`createStyle` that changes with it. **What triggers it is yours to specify** — at minimum a name that collides with an existing quota under the same identity rule.

**`T-collation-enforces-name-identity` is the precedent to read**, not to copy blindly: context tags settled name identity at the collation, in migration `0010`. **Say whether a quota's identity is the same rule**, and remember `D-context-tags-are-the-taxonomy`'s asymmetry — tags are free text *because* a typo is cheap there and expensive here.

## Monday, and the timezone that cannot be edited

`D-quota-no-rollover`: **counters reset Monday; shortfalls never carry forward. The editable target is the response to a persistent shortfall, not a carried debt.**

**"Monday" needs a timezone**, and this project already has one: `settings::current_timezone` + `scheduler_core::timezone::resolve`, exactly as `committed/body.rs:30-32` does it. Use that path — **do not** reach for UTC and do not add a second notion of the owner's zone (`T-timezone-is-a-setting`).

**Know the trap you are inheriting:** **#118** is open because `/timezone` is `POST`-only and reachable from no page. The live database reads `America/New_York`, so the owner is fine today, **but the schema default is `UTC`** (`0006_guardrails.sql:39`). This slice makes a second capability depend on a setting nobody can edit. **That is a note for the handoff, not a licence to fix #118 here.**

**"Editable target" is in scope** — it is the decision's own answer to a shortfall.

## Where the writes go

- **`T-set-operations-execute-in-the-store`.** *This week's* sessions for a quota, and the total against the target, are set operations. **They execute in the store** — no fetching every session and summing in a handler, and no `Vec` of everything ever logged.
- **`T-one-front-door-per-capability`.** Logging a session, editing one and deleting one are one capability with one front door. Three routes may call it; three write paths may not.
- **`T-templates-take-view-models`.** `http::view` holds what the page shows; the store holds what the query returned.
- **`T-422-is-product-wide`.** A rejected quota name or a bad minutes value returns `422` whose body is the re-rendered fragment it failed against. Nothing new to invent.
- **`T-forms-swap-one-fragment`.** A region rendered by more than one handler is one shared fragment with one id. **`#pool-body` is the pattern**; the log controls and `This week` will both want to swap the same row.

## If it does not fit — the line to stop at

The owner took this over a three-way cut with eyes open. **If it is going long, the clean line is: the entity, the screen, the tab and `+ Define a new quota` land; the session surface does not.**

**Stop there, open the pull request, and say so plainly in its body.** A quota you can define with a target reading `0h of 4h` is a coherent, demonstrable half. **What is not acceptable is a half-built session surface** — a `+30m` that writes nothing, a `This week` that cannot delete, or a Monday reset that is a `TODO`. Ship the smaller true thing.

**Do not re-cut it any other way without coming back**, and do not quietly drop the duplicate-name guard or the Monday reset to make the full slice fit — those are the two rules most likely to look like polish and neither is.

## Watch

1. **`T-a-check-must-be-seen-to-fail`.** Every gate, scenario and script this slice adds breaks first, visibly, and the observation is recorded. **`0416241` is the standard** — QA proved six breakages with distinct messages and found a real bug in its own check by doing it.
2. **`T-latency-is-a-qa-assertion`** — if you assert a budget, it is in the QA suite against a real server, never a unit or acceptance test.
3. **`T-qa-binds-tolerantly-to-markup`** — bind to ids and `data-` attributes, not to the shape of a `<div>`. **#137 just re-did the entire palette and typeface**; a check bound to a colour or a font will not survive the next one.
4. **`T-ephemeral-view-state-rides-the-request`** (`740d224`, and read it before you build the expand/`Other` disclosures). Which quota row is expanded, and whether its `Other` panel is open, are **exactly** the state that must not buy a column. `expanded=<tags>` on the pool is the worked example. **A logged session is the opposite** — a durable consequence of a deliberate act, and it earns its table.
   **And read `D-a-trip-survives-being-tidied` (`cb02b3a`) beside it**, settled in #129 the same day: a column there was *permitted* by that test and **derived anyway**, because deriving cost one slice's thought and a column is permanent (`T-migrations-append-only`). **The test says when storage is legitimate. It never says it is required.** This slice will face that question more than once — a quota's name, its target, its sessions, its ordering — so decide each one on its own merits rather than by precedent.
5. **DRY: real headroom now** — the gate measures product code only (`T-dry-measures-product-code`, #131) and the last two slices ran 1.86% and 1.81% against 3%. **A whole new screen with its own step module is still the most likely thing to cross it**; extract a shared family early rather than at the end.
6. **`scripts/qa/trip_controls.cjs` step 10 is the pattern** for anything client-side — real Chrome, CI-gated at `ci.yml:452`.
7. **#129 `trip-persistence` is in the pipeline ahead of this** and touches `pool/` only. No overlap expected — **say so if you find one.**
8. **All 23 acceptance features pass untouched.** Nothing here changes the Pool, the Committed screen, capture or triage.
9. Base branch is **`trunk`**; cut from `origin/trunk`, which now includes **#137**'s new palette and IBM Plex Sans. Scratch in `./tmp/`. **Open the pull request when QA is done and label it `preview`.**

## Out of scope

`Filed here` and the triage change (**#138**, slice 2), retiring `TaskKind::Quota` and deciding `period` (**#138**), reorder controls on quota rows (**#139**), fixing the unreachable timezone setting (**#118**), the pool query's in-memory grouping (**#108**), and **any visual design system** — #137 settled that today and it is not yours to revisit.

## Source

- Issue **#93** — the goal and the decisions table; **its "What the canvas draws" list is superseded by the section above**
- `docs/design/Trellis.dc.html:238-352` — the `isQuota` screen. **Read it, do not grep it**; the last two briefs that grepped it were wrong twice about what it draws
- `crates/scheduler-core/src/task/mod.rs:106` — `TaskKind::Quota`, which stays
- `crates/trellis-server/src/platform/nav.rs` — `Page`, `ALL`, and the comment naming the fourth tab
- `crates/trellis-server/src/committed/body.rs:30-32` — how a screen resolves the owner's timezone
- `crates/trellis-server/src/pool/` — the closest working model: `store` / `view` / `body` / `http`, and a fragment that swaps as one region
- `docs/decisions.md` — `D-quotas-are-selected-not-typed`, `D-logging-is-retrospective-and-separate`, `D-quota-no-rollover`, `D-four-screens`, `T-timezone-is-a-setting`, `T-set-operations-execute-in-the-store`, `T-ephemeral-view-state-rides-the-request`, `T-collation-enforces-name-identity`, `T-canvas-is-authoritative-where-it-speaks`
