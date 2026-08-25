# Trellis — Decisions Log

Append-only. Newest at the bottom of each section.

This file exists to record **why**, and especially **what was rejected and on
what grounds**. The project board tracks what we are doing; it cannot tell a
future contributor that a plausible-sounding idea was already considered and
refused. If you are about to propose something listed under "Rejected", read
the reasoning first — and if you still disagree, add a dated entry arguing
against it rather than quietly reversing it.

- **State** (what is in flight, what is done): the project board —
  https://github.com/orgs/garrell-tech-solutions/projects/4
- **Phases and acceptance criteria**: GitHub Milestones and their epic issues.
- **Architecture and vocabulary**: `docs/design/architecture.md` — type shapes,
  layer names, the `schedule()` signature, the closed enums, and an index of
  what is still undefined. Epics link to `docs/design/brief.md`, which has
  never existed; those links should point at the architecture reference.
- **Rationale and rejected options**: this file.

## How to read the IDs

Decisions are keyed by a **stable slug**, never a number. `T-jiff-epoch-millis`,
not `T3`.

| Prefix | Where | Means |
|---|---|---|
| `D-*` | this file | Settled **product** decision |
| `T-*` | this file | Settled **technical** decision |
| `R-*` | this file | **Rejected** — considered and refused, with why |
| `C`_n_ | issues #2–#6 | **Correction** — a place the original brief contradicts itself |
| `U`_n_ | issue #7 | **Underspecified** — the brief is silent and an agent will guess |

**Open questions are issues, not rows here.** A question with no answer is
state, and state lives on the board. Settled decisions live in this file because
every agent reads it at startup, from a worktree, offline — and because the
point of the "Rejected" section is that someone about to re-propose an idea
trips over the reasoning without having to know it exists.

### Why slugs

Sequential numbers were allocated on branches, and branch order is not merge
order. That failed twice in two days. `T17` was claimed independently by two
branches; then a third branch renumbered its source comments to a scheme that
never existed, leaving every citation off by one — and each wrong number
resolved to a *real* decision, so nothing looked broken.

A slug is derived from content, so two branches cannot silently collide, and a
citation that is wrong does not quietly resolve to somebody else's decision.
Slugs are never renumbered, so a reference in a source comment stays true for
the life of the file. **CI enforces that every slug cited under `crates/`
exists here.**

### Former numbering

Commits, pull requests and issues written before 2026-08-13 cite the old
numbers. This table is the only reason to keep them.

| Slug | Was |
|---|---|
| `D-gaps-offered-not-filled` | D11 |
| `D-guardrails-never-yield` | D3 |
| `D-inaction-archives` | D7 |
| `D-kill-means-archive` | D5 |
| `D-menu-of-three` | D10 |
| `D-no-pool-on-calendar` | D9 |
| `D-pool-is-default` | D2 |
| `D-quota-no-rollover` | D13 |
| `D-silence-means-done` | D4 |
| `D-single-user` | D1 |
| `D-skipped-review-ages` | D6 |
| `D-staleness-unset` | D12 |
| `D-three-strike` | D8 |
| `D-visible-slices` | D14 |
| `R-auto-promote-on-age` | R4 |
| `R-browsable-archive` | R2 |
| `R-collaboration` | R6 |
| `R-gcal-webhooks` | R9 |
| `R-guardrail-override` | R1 |
| `R-incremental-patching` | R10 |
| `R-multi-tenancy` | R5 |
| `R-plugin-surface` | R7 |
| `R-pool-on-calendar` | R3 |
| `R-supabase` | R8 |
| `T-archived-at-only` | T14 |
| `T-auto-close-event` | T13 |
| `T-capture-surfaces` | T8 |
| `T-classifier-covers-domain` | T22 |
| `T-complexity-8` | T9 |
| `T-core-no-tokio` | T4 |
| `T-empty-equals-absent` | T18 |
| `T-fact-plan-line` | T20 |
| `T-gcal-is-render-target` | T7 |
| `T-greedy-with-repair` | T6 |
| `T-handrolled-gcal-client` | T5 |
| `T-jiff-epoch-millis` | T3 |
| `T-migrations-append-only` | T21 |
| `T-module-boundary` | T15 |
| `T-mutation-parallelism` | T10 |
| `T-period-closed-set` | T19 |
| `T-pins-in-constraints` | T12 |
| `T-quota-targets-required` | T17 |
| `T-rust` | T1 |
| `T-sqlite-sqlx` | T2 |
| `T-templates-take-view-models` | T23 |
| `T-three-task-kinds` | T11 |
| `T-unknown-kind-rejected` | T16 |

Open questions moved to issues: O1 → #38 · O2 → #1 (settled) · O3 → #39 ·
O4 → #40 · O5 → #41 · O6 → #36. Issue #1 is titled `OQ2`; same question.

---

## Settled — product

| # | Decision | Reasoning |
|---|---|---|
| D-single-user | Single user, single tenant. | "Family" is a domain of the user's own time, not a shared household schedule. |
| D-pool-is-default | Pool is the default task kind; committed is the exception. | Motion's schedule-everything model is exactly why it churns. A calendar that is ~40% committed is one the user trusts, because a block exists only because it had to. |
| D-guardrails-never-yield | **SUPERSEDED 2026-08-23 by `D-context-tags-are-the-taxonomy`** — guardrails were removed with the life-area model; there is nothing left to yield or hold. **Kept because the argument is about priority, not about guardrails:** it is the reason a P1 hard deadline does not get to breach a boundary the owner set, and any future availability model inherits it. Guardrails never yield to deadlines. | A P1 hard deadline with Work hours full raises a conflict; it does not breach the wall. An override that exists will get used, and then the walls are decorative. |
| D-silence-means-done | Silence on a check-in means **done**. | Assuming "not done" is intuitively safer but is the exact mechanism by which every abandoned task app decayed: unanswered work accumulates and the backlog inflates with phantom obligations. |
| D-kill-means-archive | Kill means archive. Keep the row; build no browsable archive UI. | The row feeds the reckoning ("47 archived this quarter, 31 Learning" is real signal). The moment an archive is browsable it becomes a place to hide from decisions. |
| D-bulk-completion-is-explicit | **RESOLVED 2026-08-23: the permitted control is a "complete group" button at the top of the group.** It completes the group's remaining items and nothing else; the struck-through items `D-a-trip-survives-being-worked` keeps in place stay as they are. **It exists only to complete that group**, which is precisely what this row allows, and it is the only bulk-completion control in the product. **No action completes more than one task unless the owner either checked each one individually, or used a control whose only purpose is completing that group.** A trip, a tag group, or any other derived collection **never** gets a single control that finishes everything in it as a side effect of doing something else. **A dedicated group-completion checkbox is allowed and is the sanctioned way to do it** — what is forbidden is bulk completion arriving as the consequence of another gesture. | **Settled by the owner 2026-08-23, before anything can do it.** Nothing in Trellis completes more than one task today: #103 shipped one checkbox per row, and `pool_body.html` carries three of them and no group control. **The rule is recorded now precisely because it is cheap now** — the expensive version is discovering it after a trip panel grew a convenient *"done"* on its header. **The argument is `D-inaction-archives` and `D-manual-triage-until-llm` applied to the other end of a task's life.** Both say the same thing about defaults: *a wrong pre-fill is accepted silently, a blank field is filled deliberately.* Completion is the least reversible thing the product does and the hardest to notice going wrong — **a trip is a group the system derived, not one the owner assembled** (`T-trips-are-derived-not-ranked`: group membership is a tag match and order is computed), so a control that finishes *the group* finishes a set the owner never chose. **The failure it prevents is specific:** tapping something on a trip's header, at the leading edge of a row, on a phone, and losing six items whose only shared property is that they mention `@homedepot`. #111 exists because one such mistap is already possible; this keeps the blast radius at one. **Bounds, so it does not over-apply:** it governs **completion and dismissal**, not display, filtering, retagging or reordering — a control that changes what is *shown* may act on a group freely. **Rejected: relying on undo instead.** Undo (#111) shortens the recovery, and a bulk action still needs the owner to notice it happened, which is exactly what a mistap does not produce. |
| D-skipped-review-ages | A skipped weekly review ages everything one more week. No deeper sweep. | The week the user skips is the week their life was chaotic — the worst possible moment to punish them. Aging is self-correcting. |
| D-inaction-archives | Inaction archives. Survival requires a deliberate act. | Inverts the property shared by every task app the user has abandoned, where inaction preserved tasks. |
| D-three-strike | Three-strike: on the third deferral, "keep" is removed. Commit or kill. | A task protected weekly but never touched is a lie. This is the one place in the system where friction is a feature; everywhere else, optimize it away. |
| D-no-pool-on-calendar | Pool tasks never appear on the calendar — not even as all-day chips. | The calendar's entire value is that everything on it is true. Ambient chips are the first crack. |
| D-menu-of-three | **SUPERSEDED 2026-08-20 by `D-menu-is-a-worklist`** — superseded, not refuted; it may return when there is a scheduler behind it. ~~The menu returns exactly three options plus a rest option.~~ | A long ranked list is a blank page with extra steps — the same decision fatigue the product exists to eliminate. |
| D-gaps-offered-not-filled | Emergent gaps are offered, not filled. | Dragging Thursday's deep work into a random Tuesday hole is behaving badly. Recompute committed work at domain boundaries; surface the menu mid-stream. |
| D-staleness-unset | **PARTLY SUPERSEDED 2026-08-23.** Staleness threshold is deliberately unset; instrument first, tune at the first monthly reckoning. ~~Per-domain config.~~ — there are no domains; a context tag is not a place to hang config. | The pool's turnover rate is unknown until the system runs. ~~Seeded at 21 days / ≥3 offers.~~ **The offer half is dead:** `times_offered` was a `D-menu-of-three` concept and exists nowhere in the repo, so the seed cites a metric nothing measures. **The instruction was also never carried out** — nothing measures task age either, so there is nothing to tune from. Tracked at #38. |
| D-quota-no-rollover | An unmet quota does **not** roll over. A missed week is missed. | Carrying it forward re-creates exactly the accumulating phantom obligation that D-silence-means-done (silence-means-done) exists to prevent. "You did 1 of 3 runs last week" is a reckoning fact, not a debt. Paired with T-three-task-kinds. |
| D-visible-slices | **Every pipeline slice ends in something the user can run and see.** A slice is not done when its tests pass; it is done when the owner can start the app and watch the new behaviour happen. A slice with no visible surface carries the thinnest surface that exposes it. Size does not matter — smaller is better. | This is the owner's professional standard with their own clients, not a preference about this project: continuous visible delivery is the customer's best case, and building for fifteen slices before the customer can try anything is the failure mode it exists to prevent. Three things it buys, all of which the plan was otherwise deferring. **Feedback:** #20's risk register (its **R1/R2/R3** — *not* the identically-numbered rejected options `R-guardrail-override`/`R-browsable-archive`/`R-pool-on-calendar`, which this row cited by mistake; see the *Former numbering* table) names three assumptions that are all behavioural — silence-means-done, the ~40% committed ratio, whether the menu beats a list. None can be tested by building a correct scheduler; they need the owner living with the thing, so calendar time is the scarce resource, not engineering time. #9's AC-6 already conceded this by requiring `/stats` live "weeks before it is read". **Clear thinking:** a working system is a better argument about what to build next than a roadmap is. **Value:** the product is useful as an inbox long before it is useful as a scheduler. Cost, accepted knowingly: some slices grow a surface they would not otherwise need, and early surfaces are plain and will be reworked. Rejected alternative: keep the horizontal milestone cut and add UI at the end — that is precisely the fifteen-slice wait, and it defers every behavioural risk to the point where acting on what is learned is most expensive. |
| T-commitment-is-chosen-not-derived | **Whether a committed item is an *at* or a *by* is chosen explicitly at triage, never inferred from whether a time was typed.** `deadline_type` is renamed **`commitment`**, taken off the triage form and left in the schema unread. On screen a *by* carries a `BY` prefix in the same cell an *at* fills with its time. **A past item still shows, marked, and first.** | Settled inside `committed-screen` (#94), resolving the disagreement `D-four-screens` predicted: `D-committed-is-at-or-by` says the two *"display differently and behave differently"*, and **the canvas draws no distinction at all** — no badge, no second column, and the string `"at"` appears nowhere as a concept. Canvas authoritative on layout, log on behaviour, so the distinction stood and its drawing was ours. **Deriving it from the presence of a time was the obvious shortcut and is wrong**, for one reason worth keeping: it **cannot express a hard *by*** — *"the tax return, by 5pm on Jan 31, and that one cannot slip"* — a real commitment that would have been **silently unrepresentable**, which is the failure mode `T-unknown-kind-rejected` and `T-period-closed-set` were each written against. A *by* with a time is legal and must stay a *by*. **`deadline_type` came off the form because it had gone back to changing nothing**: its only behaviour was `T-hard-refuses-soft-slips`', which lives in the scheduler `D-dogfood-first` paused — the exact state U2 complained of before that decision fixed it. It stays in the schema on #88's ground, and the distinction from `scheduler_core::ratio` is the useful part: **a derivable number is recoverable from rows that stayed; a hard/soft judgement the owner typed is not.** When the scheduler returns, an *at* is hard and a *by* is soft — derived, not typed twice. **Past items showing is not a display choice but a trust one**: a commitments screen that silently drops what you missed is the one failure it cannot have, in a product whose whole value is that the owner trusts what it shows — and it falls out of chronological order with no special casing. **`ord` is not built.** The canvas sorts by it, but its only derivation anywhere gives an untimed item `ord: 99` to sort it last, and Trellis requires a deadline at committed triage, so chronological is total. The fixture's `ord` agreeing with date order was **coincidence**, which is precisely what made it look like a separate concept. |
| D-committed-is-at-or-by | **A committed item is either an *at* or a *by*.** **At** is a fixed block — *2pm dentist* — placed where it says. **By** is a deadline with slack before it. They display differently and behave differently. | Settled by the owner 2026-08-20. The current model gives every committed task a `deadline` plus a `deadline_type` of `hard \| soft`, which asks *may this slip* and never asks *is this a time or a limit*. **Those are different questions and only the second one matters without a scheduler**: a dentist appointment is not a deadline that happens to be tight — nothing may be scheduled around it because it is not schedulable at all. **A *by* is precisely what a scheduler would later have slack to place into**, which is why this distinction is the one that survives the solver being deferred: it marks exactly which items automatic placement would have something to do with. That makes it evidence for `D-dogfood-first`'s bar rather than a workaround for the missing scheduler. |
| D-quotas-are-selected-not-typed | **A quota is chosen from a small set of chips, never typed.** Creating one is a deliberate control requiring a **name and a weekly hour target**, and can be done **directly from the Menu** without a capture. **A mistyped name must not be able to create a quota.** Two first-class shapes: **item-bearing** (specific things to get through) and **pure-hours** (piano practice — no items, ever). Expanding an item-bearing quota shows its items; expanding a pure-hours quota shows **this week's individual sessions**, each editable and deletable. | Settled by the owner 2026-08-20. **The asymmetry with `D-context-tags-are-the-taxonomy` is the point and is not an inconsistency.** Context tags are free text because a typo costs one badly-grouped item; a quota carries a **weekly hour target and a running total**, so a typo does not mis-file an item — it **creates a second quota** that silently splits the week's hours across two counters and makes both wrong. A set of three to five is small enough to pick from and large enough to matter. **Pure-hours quotas are first-class rather than item-bearing quotas with no items**, because *"practice piano four hours a week"* has no completable objects and never will — modelling it as an empty list makes the natural view (this week's sessions) the exception rather than the shape. |
| D-logging-is-retrospective-and-separate | **Logging hours is independent of completing items.** Quick taps for **+30m** and **+1h**, plus arbitrary duration entry. Completing an item **may offer** to log time and **never does it silently**. **No start/stop timer — all logging is retrospective.** **Counters reset Monday; shortfalls never carry forward**, and **the editable target is the intended response to a persistent shortfall.** | Settled by the owner 2026-08-20, extending `D-quota-no-rollover` from *"an unmet quota does not roll over"* to the mechanism that makes it true weekly. **Separating logging from completion is the load-bearing half:** an hour spent on a quota is a fact about the week; finishing an item is a fact about the item; and inferring either from the other produces a number the owner stops trusting — which is the same failure `T-capacity-never-under-reports-demand` guarded against on the demand side. **No timer, because a timer makes the tool something you must remember to start**, and an unstarted timer silently reports zero — inaction producing a false number rather than a missing one, which inverts `D-inaction-archives`. Retrospective entry can be wrong but is never wrong *by omission*. **A persistent shortfall is answered by editing the target, not by carrying debt**: *"you did 1 of 3 last week"* is a reckoning fact (`D-quota-no-rollover`), and if it is true every week the target was wrong, not the week. |
| D-dogfood-first | **Trellis is re-planned around daily use. Target: the owner using it every day by 2026-09-03.** The Menu (M3.5) ships **before** the Scheduler Core (M3). M3's remaining slices (#76–#79) are **paused pending evidence**: two weeks of real use, plus the owner able to **name the friction automatic placement would remove**. *"If I can't name it, we don't build it."* | Settled by the owner 2026-08-20, after 187 commits and no daily use. *"Capture works; nothing consumes what I capture. Every open milestone builds machinery for a plan I can't yet see."* **`D-visible-slices` was satisfied and insufficient.** Every slice ended in something runnable and each was honestly demonstrable — and the product accumulated **six pages, four of them read-only instruments feeding a solver that does not exist**. A per-slice test cannot see a property that only degrades across slices; that is the third time this project has met that shape, after the navigation gap (#58) and triage growing to six fields. **The inversion is the fix**: build the consuming surface first, then let real use say what machinery it needs. **M3 is not cancelled** — it is held to an evidence bar, which is a stronger position than the one it had. Its five slices were fully unblocked on 2026-08-19 by six decisions made in a day, and every one of those decisions remains valid whenever the evidence arrives. |
| D-dogfooding-drives-the-roadmap | **Daily use is the standard driver for every milestone, and the guiding principle for the project's direction.** A milestone is justified by friction the owner can *name* from using Trellis, not by a plan's internal logic. **`#20`'s R2 is retired**: the committed:pool ratio experiment is closed, not paused, and the risk register stops claiming an experiment is running. | **Settled by the owner 2026-08-23**, generalising `D-dogfood-first` from a one-time re-plan into the standing rule. **The evidence is the argument.** In the two days after Trellis became reachable on a phone, real use produced **four** defects that every gate and every review had passed: `#101` scroll dead on touch (present in three shipped slices, flagged in all three pull-request bodies, fixed in none), `#110` committing a task meant typing a UTC timestamp by hand, `#119` every triage disclosure opening at once, and `#122` working a trip destroying the trip. **Measurement produced none of them.** **Why R2 is retired rather than rebuilt.** `#88` deleted `/stats` and `scheduler_core::ratio`, and `T-dead-core-code-earns-its-keep` justified that on rebuild cost — a sound argument about **code** that never addressed the **measurement**. R2's design was to run *"for weeks of real capture data before M3 ships"*, and `D-dogfood-first`'s window is exactly those weeks, so the choice was genuinely live: rebuild before 2026-09-03, or close it. **The owner closed it, and the reasoning generalises:** a committed:pool ratio is a metric from the era when the plan was the product. **Over-committing is something you feel in a week of use, not something you read off a number** — and the four defects above are what noticing-by-using actually yields. **What this costs, named:** R1 (*silence means done*) and R3 (`state_hash` fidelity) remain open and are **not** retired by this — they are untested assumptions about behaviour, and R1 in particular still depends on `#4`'s undecided three-bucket reckoning. **`D-manual-triage-until-llm` also accepted a cost on the explicit basis that `/stats` still counted**; that sentence was already marked false and this closes it properly rather than leaving it dangling. **Rejected: rebuild the counter for the dogfooding window.** It would have measured the fortnight, but nothing was waiting on the number and the fortnight's job is to produce nameable friction, which it is already doing at two defects a day. |
| T-cross-capability-invariants-need-an-owner | **When one capability's correctness depends on an invariant another capability establishes, the dependency needs a test that crosses the boundary — a doc comment is not a tie.** `scheduler_core::pool::group` buckets by plain string equality, which is correct **only because** `capture::resolve_tag` canonicalises a tag's spelling on the way in. | Found by the architect in #92, and it is the third sighting of a shape this log has already named twice: **a doc comment defending a rule with a claim about state that nothing enforces.** `time_at_minutes` asserted *"guardrail minutes are always within a single day"* (#74); `life_areas::guardrails` claimed *"a pool-only life area's `bands` is always empty"* while reporting 32h for one marked never scheduled (#70); now `group` relies on tags arriving already canonical. **The tell is unchanged: a comment explaining why a check is unnecessary.** **What is new is how it hid, and that is the part worth carrying.** Every fixture on the pool path seeded tags through `capture::store::insert` **with the spelling already final** — skipping the exact step the screen depends on. **A fixture that constructs state directly skips the invariant the production path establishes**, which is #70's *right end state by the wrong path* arriving a fourth time, now in unit fixtures rather than in a scenario. **The failure it hid is not subtle-but-harmless.** Three items under one tag is exactly `T-trips-are-derived-not-ranked`'s threshold; split across two spellings they are groups of **2 and 1**, both below it, so **both fall to loose ends and the trip disappears from the screen entirely** — a visible, user-facing hole with no test under it, and precisely the failure #82's case-folding was decided to prevent, arriving from the side nobody was watching. **The rule generalises past tags**: Quota and Committed will both consume this key, and each will inherit the same dependency. |
| T-trips-are-derived-not-ranked | **A context tag becomes a *trip* at three items.** Smaller groups fall into **loose ends**, still showing their tag. **Group order is derived, not chosen** — most items first, alphabetical tiebreak — so **trips and groups never carry a priority control, in this slice or any later one.** When manual reordering arrives it belongs to **loose ends alone**. | Read out of `docs/design/Trellis.dc.html` by the specifier for #92 — `tripThreshold || 3` and `b.list.length - a.list.length \|\| a.key.localeCompare(b.key)` — and confirmed by the owner. **The threshold is what makes `ctxLabel` on a loose item explicable**: a two-item `@homedepot` is not worth a trip, but it is still worth knowing where it is, so it keeps its tag in the loose list. **The derivation is the load-bearing half.** A trip is *a unit you clear in one stop*, so the order of its items is noise; a loose end is *a thing you decide about*, so it earns a control. `D-menu-is-a-worklist` names manual priority and the canvas draws six *"Raise priority"* controls, so both agreed all along — **only the PM's brief dissented, twice, having read the canvas by grep rather than reading it.** The split between them is reachable only by reading the ordering rule *against* the threshold, which is why the reasoning is kept in the feature header rather than deleted with the deferred scenario. **Scenario 05 asserts the absence of every reorder control** rather than leaving it inferred, because a coder reading the canvas would add them in good faith. **One thing the canvas draws and this product refuses:** a per-group note — *"One stop clears all 3."* — computed from a hardcoded list of which tags are **places** and which are **sittings**. That needs Trellis to know `@homedepot` is a shop, which is precisely the managed taxonomy `D-context-tags-are-the-taxonomy` exists to refuse. **Raised rather than resolved quietly**, per `D-four-screens`: the canvas is authoritative on layout, the decisions log on behaviour, and a layout that implies a behaviour the log forbids is a disagreement, not an instruction. |
| D-a-trip-survives-being-worked | **EXTENDED 2026-08-24 by the owner, from use: once a tag has formed a trip it stays one until it has no open items left.** Clearing tidies the panel; it does not dissolve it. **The threshold answers "is this worth a special trip?" once, when the group forms** — re-asking it mid-shop is the same category error this row was written to fix, arriving one tap later. **The word that failed was "displayed":** the original text said persistence counts everything displayed, and clearing removes items from displayed, so five-with-three-cleared counted as two and fell to loose ends **with the owner still standing in the shop.** **A trip holds together while you work it, and a completed item stays in place, struck through, rather than vanishing.** The group's label reports progress — *"3 of 5 done"* — not a bare count. **Formation still requires three *open* items**, so three completed ones never conjure a trip; **persistence counts everything displayed**, so checking things off never dissolves the panel you are working from. **Unchecking a struck-through item puts it back.** | **Settled by the owner 2026-08-23, from real use — the fourth thing `D-dogfood-first` has produced that no review caught.** **The use case is the argument:** you are standing in Home Depot with five things tagged `@homedepot`. Today, `pool/store.rs:31` filters `AND tasks.archived_at IS NULL` and `pool::group` re-tests `list.len() >= TRIP_THRESHOLD` **on every render** — so on the third check the panel dissolves and the last two scatter into loose ends among everything else, **mid-shop, with no record of what you already got.** The screen destroys the list at exactly the moment it is being used for its only purpose. **This narrows #103's decision rather than reversing it.** That slice rejected counting done items toward the threshold as the *"half-pass trap"* — a trip panel of one that reads correctly until you look at the number. **That was right about the number and wrong about the experience**, and the fix is the half it did not consider: **change the label instead of hiding the items.** A progress label cannot lie the way a bare count can. **Rejected: keep the group but let completed items vanish.** It stops the scattering and still erases every trace of what you did, so there is no progress and no undo — and being able to see what you have already got is half of why the list exists when you are holding a basket. **Consequence for `T-archived-at-only`:** *recently completed* is expressible with the timestamp already stored — no new column, no discriminator. **When a struck-through item finally clears is the open question**, and it belongs to the slice. **Consequence for #111:** in the pool, **unchecking is the undo**, so that slice narrows to Committed rather than duplicating this. |
| D-four-screens | **AUTHORITY BOUNDED 2026-08-24 — the canvas leads a slice, then the slice leads.** The canvas is authoritative on layout **for a slice being specified against it, at that moment**. **Once a slice ships and the owner has used it, the implementation is the record.** The four screens themselves are unchanged. **Trellis has four screens — `Capture`, `Pool`, `Quota`, `Committed` — in a bottom tab bar.** `docs/design/Trellis.dc.html` is the layout source; the tabs are its `isCapture` / `isPool` / `isQuota` / `isCommitted` guards. **The canvas is authoritative on layout; `docs/decisions.md` remains authoritative on behaviour.** | Settled by the owner 2026-08-21 on seeing the drawn design — *"I liked the 4 screen design and any decision previous to that is outdated."* **What it supersedes is the container, not the content.** `D-menu-is-a-worklist` described *one view* answering *what is worth doing now*, on *two daily screens*; every behaviour it named survives unchanged — pool grouped by context, quotas with hours against a weekly target, committed in date order, manual priority, **no solver, no backward pass, no splitting, no pins**, and **no settings page with everything controlled inline**. Four tabs is not a settings page; it is the same worklist with its three parts given their own room. **The practical consequence is that #85 is three slices rather than one**, and that was the question the cut had been held for. Pool needs no migration — context tags landed in #82 and a pool task reads its tag through its capture — while Quota needs the reshape from `target_count × target_minutes_each × period` to a weekly hour target, and Committed needs `D-committed-is-at-or-by`. **The design also settles what the header is**: PR #87 refused to draw four tabs because *"`app_shell.feature` pins those six labels, and `T-nav-is-the-site-map` makes the header the route table — four tabs would be a header that lies about what the product does."* That objection died with #88, which removed the six pages, deleted `app_shell.feature` **and deleted `platform::nav`** — so the tab bar now has a design and a stylesheet and **no renderer**, and the first Menu tab rebuilds it. `T-nav-is-the-site-map` needs no amendment: with four routes the site map has four entries. |
| T-canvas-is-authoritative-where-it-speaks | **AMENDED 2026-08-24 by the owner: a gap is no longer a debt.** **The two practices that survive:** the specifier **reads the canvas before implementing** — that is what caught the disclosures (#126) and the show-more (#120), and it costs nothing even when the canvas is stale — and **the pipeline still never edits the canvas.** **What ends:** logging gaps as though the canvas were owed an update. A gap only means something while the canvas is *ahead*; once the implementation leads, the canvas is simply behind, which is expected. **The eight open gaps are closed as a category, not as work.** **`D-four-screens` makes the canvas authoritative on layout. It is authoritative where it *speaks*; where it is silent, the silence is a gap, not a prohibition.** A slice that needs a control the canvas does not draw **builds it, says so in the pull request, and it is logged here** — it does not invent one quietly, and it does not stall waiting for a drawing. **The canvas is never edited by the pipeline.** | **Recorded 2026-08-23 after the fourth instance, all within eight days.** The canvas draws **zero** checkboxes, **zero** `type="date"`/`"time"`/`"datetime-local"` inputs, no settings surface, and no app icon — so the pipeline has had to originate: **the done control** (#97/PR #103, and it named the cost: a Committed row became four columns on 430px); **the committed date input** (#110/PR #117, where the specifier checked the canvas rather than trusting my brief's reading of it and found the gap real); **a surface for the timezone setting** (#118, still open — and the hard part is that `D-menu-is-a-worklist` says *"no settings page"* out loud); and **an app icon** (#112). **Why a rule rather than a fifth conversation:** each gap cost a round trip, and the reasoning was identical every time. **The failure this prevents is subtler than a missing control** — it is that every future reader treats the canvas as the source of truth, so each undrawn control quietly weakens the authority `D-four-screens` grants it. That is `T-collation-enforces-name-identity`'s failure from the other side: a description and the thing it describes drifting apart while both read as current. **What it does not do:** it does not decide that the canvas should stay silent. **Bringing the four back into the canvas is the owner's call and remains open** — this row makes the drift visible and bounded rather than accumulating unrecorded, and it makes the practice explicit so the next slice does not have to ask. **Rejected: let the pipeline edit the canvas.** The canvas is synced from a Claude Design project and the local copy is byte-identical to the remote; a pipeline edit forks the two and destroys the authority it was trying to serve. |
| D-menu-is-a-worklist | **Container superseded 2026-08-21 by `D-four-screens`** — the behaviours below all stand; they live on three tabs rather than in one view, and "two daily screens" is now four. **The Menu is ~~one view~~ what answers *what is worth doing now*: pool grouped by context, quotas with hours against their weekly target, committed items in date order, and manual priority. No solver, no backward pass, no splitting, no pins.** **Trellis has two daily screens — Capture and Menu — and no settings page; everything is controlled inline.** | **Supersedes `D-menu-of-three`**, which specified *"exactly three options plus a rest option"* and rejected a ranked list as *"a blank page with extra steps — the same decision fatigue the product exists to eliminate."* That argument was about a **prompt**; what the owner needs is a **worklist**. The distinction is what the item is for: three-plus-rest answers *choose one now* and is right when the system knows enough to rank; a grouped worklist answers *show me everything worth doing, arranged so I can pick* and is right when the ranking lives in the owner's head. **With no solver, the system does not know enough to offer three**, and offering three anyway would be inventing a ranking rather than reflecting one. `D-menu-of-three` may return when there is a scheduler behind it; it is superseded, not refuted. **Two screens is the constraint that keeps it honest** — a settings page is where a product puts the controls it could not fit into the thing you actually use, and this product has just spent a milestone proving how easily surfaces accumulate. |
| D-context-tags-are-the-taxonomy | **A capture carries a free-text context tag, autocompleting on prior values. Context tags are the product's only taxonomy — numerous, cheap, disposable. Life areas are replaced.** The triage requirement drops immediately; the life-area, guardrail, free-time, capacity and exception modules are removed **after** the owner is dogfooding, not before. | Settled by the owner 2026-08-20, superseding `T-life-areas-are-data` and `T-life-area-required-at-triage`. **The win the owner named is batching by location, not finding a time slot**: *"much of my pool is errands attached to places."* A context tag — `@homedepot`, `@supermarket` — groups work by where it can be done, which is what makes a pool item actionable when you are already there. A life area answers *what part of my life is this*, which is a reporting question, and the reporting that would have consumed it (M8's reckoning) is further away than ever. **Free text is deliberate and is the opposite of the life-area rule**: `T-life-areas-are-data` made a managed set with a well-formedness rule because a life area had to have a guardrail. A context tag has no such obligation, so the cost of a typo is one badly-grouped item rather than an unschedulable one — and autocomplete on prior values is enough structure. **It cannot be backfilled**, which is why it ships first: every day of capture without it is a day of data that can never be grouped. **The demolition was deferred and then un-deferred the same day (#88), and the reversal is worth reading.** The PM argued that removing the modules first *"would destroy the evidence"* of whether guardrails were worth having. The owner's answer — *"we can mark the unused for removal, it's still in git"* — defeats it: **the evidence is two weeks of real use, not the code**, and git keeps the code, so restoring it is a revert rather than a rebuild. The argument was weaker than it sounded and should not be reached for again. **What survived the reversal is the sizing**, and it shrank: roughly 3,650 lines across five server and five core modules, eight acceptance features and their QA scripts — but **no migration.** `tasks.life_area_id` carries a foreign key into `life_areas`, so dropping the table means rebuilding `tasks` under `T-migrations-append-only`; leaving every table in place costs SQLite nothing, keeps the owner's data as a second safety net beside git, and removes the only *construction* in the job. **The demolition is pure deletion.** |
| D-life-area-owns-its-time | **SUPERSEDED 2026-08-23 by `D-context-tags-are-the-taxonomy`** — life areas are replaced by context tags, which are free text and carry **no hours at all**. **This is the row that has no successor: Trellis today knows what you want to do (quota targets) and not when you are free.** Reintroducing availability means reintroducing this decision, not translating it. **Each life area carries its own weekly guardrail — the hours in which its work may be scheduled. A task goes in its own life area's hours by default and may not go outside them; borrowing another life area's hours is an explicit per-task permission (`allowed_windows`). Guardrails may overlap in clock time, and where they do their life areas compete.** A life area with no guardrail must be marked **pool-only**: never placed, only offered by the menu. | Settled by the owner 2026-08-17, resolving the half of #6 that #6 never asked. **Reservation is the default and sharing is opt-in — that one line is what makes a guardrail do both jobs the settled decisions demand of it.** `D-guardrails-never-yield` needs **containment**: a P1 hard deadline against full Work hours raises a conflict and does not breach the wall, or the walls are decorative (`R-guardrail-override`). `T-three-task-kinds` needs **reservation**: *"a Fitness guardrail nothing is scheduled into is a wall protecting an empty room"* — if Fitness, Learning and Home share one "evenings" pool, a busy Learning week silently eats the gym. A mask owned by one life area gives containment directly; making that ownership the default gives reservation **without a protection rule**, because Fitness's 06:00 slot is safe precisely when nothing else has asked for it. Overlap stays legal because Saturday evening genuinely being either Learning or Home is a real preference, not an error — the scheduler picks by deadline and priority. **Rejected: a small shared set of named windows** (Work hours, Personal hours) that life areas point into. It makes adding a life area one step instead of two, and it fails situation two outright — inside a shared Personal window, side-project work eats the evening and Fitness gets nothing, with no place to express that Fitness should have been protected. It also reintroduces the two-lists-plus-a-mapping shape `T-life-areas-are-data` rejected for capture tags versus scheduler domains, where *"the mapping between the two lists would become the real list, kept in a third place nobody names."* **Rejected: a strict partition**, every hour owned by exactly one life area. Stronger guarantees and the menu would always know which life area a gap belongs to, but it makes overlap an error rather than a choice and forces the owner to account for every schedulable hour up front. **Cost, accepted:** adding a life area is two acts — name it, then say when it happens. `T-life-areas-are-data` already required exactly that, so this makes an existing rule concrete rather than adding a burden, and *pool-only* is the one-step escape for a life area you have not thought about yet. **Note on #6's own wording:** it places a Learning task in `{Work, Personal}`, but "Personal" is not a life area — the seed is Work · Fitness · Learning · Family · Home. #6 was written before `T-life-areas-are-data` and was using the superseded vocabulary; under this decision that task reads *Learning, permitted into Work's hours*. |
| D-recurrence-is-re-commitment | **A recurring commitment is not a rule the system runs; it is a decision the owner makes again each period.** Cadence comes from quota's `period` (`week \| month`), placement from a **pin**, and survival from **re-committing at the weekly review** — prompted at each task's own period boundary, not on the review's weekly rhythm. **A skipped review changes nothing**: the commitment stays on the schedule. It ends the first time the owner *attends* a review and does not re-commit. | Resolves #72, raised by the owner on 2026-08-18 after using the product — *"I want to learn with a friend every Tuesday at 20:00–20:30"* — and finding no way to say it. Nothing fitted: a guardrail is life-area level, **committed** means a deadline the scheduler places around, **quota** is cadence without a clock, and a **pin** is one interval, so a weekly commitment would be 52 of them. **The owner's answer beat all four options this role offered**, and it is `D-inaction-archives` one domain over: a recurrence *rule* is exactly the thing where inaction preserves — set once, generating forever, outliving the friendship — which is the phantom-obligation failure this product exists to invert. Re-commitment makes the recurrence survive only by a deliberate act. **It also closes `T-pins-in-constraints`' "pins have no defined death"** for this class: a recurring pin dies at the period boundary unless renewed. **The skipped-review case was the one real objection and it dissolved on reading `D-skipped-review-ages`**, which already says a skipped review *"ages everything one more week. No deeper sweep."* Nothing dies from a skipped review today, so a recurring commitment dying would be the anomaly. The two archive rules coexist cleanly once stated: **skipping the review kills nothing; attending it and staying silent kills the item** (U7). The deliberate act is attendance. **Composition, not new primitives:** `T-period-closed-set` already closed `period` to `week \| month`, so *"the beginning of each month"* needs no new vocabulary — the review preceding a month boundary asks, and the owner places the pin. A recurrence rule would have had to invent a second period vocabulary beside the one quota has. **Rejected: the calendar owns it** — put the standing event in Google Calendar and let `busy` keep the scheduler off it (`T-gcal-is-render-target`). Free time and capacity *supply* stay correct, but the half-hour counts as neither Learning demand nor Learning done, so M8's reckoning cannot see it and `D-quota-no-rollover`'s *"1 of 3 sessions"* can never include it — `T-capacity-two-axes`' concern one layer over: not wrong about the clock, wrong about the work. **Rejected also:** a recurrence rule on `pin` (makes a Constraints-layer interval into a generator), a fourth task kind (reopens `T-three-task-kinds` for something that composes from three), and recurring task instances (most machinery, and it is what every calendar app does rather than what this product is for). **Accepted cost:** skip reviews for two months and eight Tuesdays stay blocked by a commitment that may have lapsed in life. It is on the calendar every week and one review clears it, so it is not the *invisible* constraint `T-pins-in-constraints` warns about. **What M3 must build is unchanged** — `pin { task_id, start, end, source }`, one interval. The recurrence lives in the review (M8). |
| D-placed-whole-or-not-at-all | **A task is scheduled entirely or not at all. There is no partial placement.** A 6h task with 4h available before its deadline is not scheduled; it appears in the infeasibility report with a reason, and the owner moves the deadline, cuts scope, or frees the time. | Settled by the owner 2026-08-18, as the first of the three decisions the M3 invariants turned out to contain. **The alternative — book the 4h and report 2h unplaced — is genuinely more useful in the moment and was rejected on two grounds.** First, this product's instinct is to surface a conflict rather than cope quietly with it: `D-guardrails-never-yield` says a P1 against full hours *"raises a conflict; it does not breach the wall"*, and the infeasibility report exists precisely to say *this did not fit, and here is why*. A half-booked task converts a decision the owner must make into four hours of work that feels like progress. Second, the cost is structural: **"partly placed" is a third state that the calendar, the menu (M3.5), the reality-flex loop (M6), the weekly review and the capacity view would each have to represent**, and a partly-scheduled task on the calendar looks like a plan when it is not — which is `D-no-pool-on-calendar`'s *"the calendar's entire value is that everything on it is true"* under a different disguise. **This is what makes invariant 3 well-defined**: conservation under splitting can say a placed task's chunks sum *exactly* to its estimate only because there is no case where they sum to less. |
| D-manual-triage-until-llm | **Nothing classifies a capture until the LLM lands at M9. Triage is fully manual until then, and the life-area picker offers no preselection — the human chooses, every time.** The keyword implementation `T-classifier-covers-domain` scheduled for M1 is **cancelled, not deferred**: M9 builds the trait once, against a real implementation. | Settled by the owner 2026-08-17, reversing the M1 half of `T-classifier-covers-domain`. **The argument is `D-inaction-archives` relocated to triage: a wrong pre-fill is accepted silently, a blank field is filled deliberately.** A keyword classifier guessing Home for "call dentist" produces a task filed in Home, because the default is what happens — in a product whose entire value is that the user trusts what it shows. That cost is paid on every capture, while the benefit, at keyword accuracy over natural language, is not reliably positive. **It dissolves rather than contradicts `T-classifier-covers-domain`'s central argument**, which was that growing the trait's output shape at M1 beats retrofitting it after M9 depends on it: with no trait at M1 there is nothing to retrofit *into*, and designing an interface against a strawman implementation risks shaping it around what keyword matching happens to afford. M9 designs it holding the real implementation, which is strictly better information. **What survives from `T-classifier-covers-domain` and moves wholesale to M9:** one trait, output covering `kind`/`deadline`/`priority`/`domain`/`title` with per-field confidence, invoked from a background worker between capture and triage, OpenRouter behind a hand-rolled client. **What changes at M9:** its criterion "API failure or timeout falls back to the keyword implementation" has nothing to fall back to, and becomes *falls back to empty fields and manual triage* — a degraded mode that is exactly the shipped product rather than a second classifier nobody validated. That deletes M9's "classifier trait + keyword impl refactor" story outright, so this makes **M9 smaller as well as M1**. **Cost, accepted and named:** `#20`'s R2 calls the committed:pool ratio "the highest-leverage counter in the product" and names the remedy for a bad one as *"triage defaults and classifier bias"*. Until M9 only the first half of that remedy exists. ~~`/stats` still counts and the >50% alarm still fires (`#45`), so the experiment narrows rather than stops.~~ **False as of 2026-08-23:** #88 deleted `/stats` and `scheduler_core::ratio` with no replacement counter, so the experiment **stopped** rather than narrowed. Whether R2's counter returns before the dogfooding window opens is open and belongs to #20. **Consequence for the picker, which is the whole point:** a default that is merely first-by-id is the same silent-wrong-default one layer down, so the life-area select carries no preselection and a submission naming none is rejected like any other missing required field. |
| D-a-trip-survives-being-tidied | **A *run* at a tag begins when something lands there with nothing else waiting, and ends when the last thing waiting there is cleared away. A run of three or more is a trip, and it stays one for as long as the run lasts.** **The panel leaves on a tap, never on a tick** (owner, 2026-08-25): tick the last open item in a trip and it holds, reading *2 of 2 done*, until `✕ Clear done` is tapped. **A tag must re-earn its trip** — three cleared last month plus one captured today is not a trip. The bound is **an event, not a time window**. Completes `D-a-trip-survives-being-worked`, whose word *displayed* failed: clearing is precisely what stops something being displayed. | Settled inside `trip-persistence` (#129, PR #141), the third and last slice on this panel. **The threshold asks *"is this worth a special trip?"* once, when the group forms**; re-asking it mid-shop is the category error #122 was written to fix, and clearing was the one remaining door it came through. **The tap-not-tick rule is a deviation from the brief's own demo, made deliberately, and the third of its three reasons is the one that decides it.** `trip-progress-fully-done-06` already holds a fully-struck trip at *three* items, so going at two and staying at three would let **the item count decide whether finishing a trip wipes it**. **You can uncheck what you can see** — a panel that vanishes on the last tick takes its own undo with it, which is the group-completion undo gap #135 left, reappearing one slice later. And **nothing clears itself**: the alternative that ends a run on the last tick without sweeping leaves the strikes uncleared and invisible, so the next capture at that tag **resurrects the old panel over month-old strikes** — precisely the runaway #129 was filed against. **No column, and the attempt was the brief's instruction.** The run is derived from `cleared_at` and `tasks.id`, both already stored; the boundary is the last clear that left nothing behind it. `T-ephemeral-view-state-rides-the-request` would have **permitted** a column here — a trip that dissolves while the phone is locked in the car park is this defect one gesture further out, so it fails the *should-a-reload-forget-it* test — which makes this the useful precedent: **the test says when storage is legitimate, not when it is mandatory.** Deriving first cost one slice's thought and saved a permanent migration (`T-migrations-append-only`). **The rule moved into the store, and the engineering standard's answer is in the pull request rather than after it:** which clear ended a run runs in a correlated subquery where `cargo-mutants` cannot reach, and what still proves it is `store.rs`'s run tests going through `clear_done` rather than writing `cleared_at` by hand, plus six properties over `pool::group` — including **monotonicity, that a larger run never costs a tag its panel**. `RunSizes` carries the answer across the boundary as a specification value object keyed by tag; an earlier draft hung the count on `PoolTask` and let `list.first()` speak for a whole tag, which the architect replaced so that **disagreement is not representable**. | 

## Settled — technical

| # | Decision | Reasoning |
|---|---|---|
| T-rust | Rust. | The scheduler core is interval arithmetic over sum types, the invariants are property-testable, and the artifact is a static musl binary plus one SQLite file. |
| T-sqlite-sqlx | SQLite + `sqlx`, WAL mode. | Self-hosted Supabase is ~8 containers solving problems this product does not have. All queries stay inside `scheduler-db` so a Postgres swap stays mechanical. |
| T-jiff-epoch-millis | `jiff`, not `chrono`. | Guardrails are civil wall-clock. `jiff` models zoned vs civil time as distinct types and forces an explicit decision at a DST gap. Store UTC epoch millis, convert at the boundary. |
| T-core-no-tokio | `scheduler-core` must not depend on tokio. | If it compiles without an async runtime, the algorithm has been kept honest. Enforced in CI from M0. |
| T-handrolled-gcal-client | Hand-rolled `reqwest` + `serde` Google Calendar client. | Only six operations are needed; ~300 lines fully controlled beats fighting a generated crate. |
| T-greedy-with-repair | Greedy-with-repair, not a constraint solver. | Explainable and fast enough for one user. Keep the interface clean so an optimizer can be swapped in behind it later. |
| T-gcal-is-render-target | Google Calendar is a render target, never a source of truth for task blocks. | |
| T-capture-surfaces | Capture surfaces for v1: web quick-box and Telegram. CLI and iOS Shortcut deferred. | |
| T-complexity-8 | Cyclomatic complexity threshold is **8** per function, not the template's 4. | Derived from the domain model rather than borrowed from convention. The largest enums (`Domain`, `BlockState`) have 5 variants, so an exhaustive `match` over one scores 6 — meaning a threshold of 4 would fail on almost every match in a codebase that is deliberately sum-type-heavy. Enforced at 4, the gate would push agents to split clear matches into indirection that is strictly worse to read, and the analyzer would be shaping the architecture instead of guarding it. 8 leaves headroom for one guard above the largest match while still catching genuine branching thickets. **A function over 8 is carrying logic that is not the match — extract that, do not flatten the match.** Revisit if a legitimate enum grows past 6 variants. |
| T-dry-measures-product-code | **The DRY gate measures product code only; the acceptance harness is excluded.** The threshold stays at 3%. **Inline `#[cfg(test)]` tests stay measured** — they live in product files and a directory exclusion cannot reach them, nor should it. | **Settled by the owner 2026-08-24**, after the gate reached zero headroom and six queued slices were all screen slices. **The diagnosis is what made it a scope question rather than a discipline question.** The architect found that **568 of the duplicated lines — 62% — sit inside `crates/acceptance-tests/src/steps/`**, spread across *pairs* of screen modules rather than concentrated in one extractable shape: `committed_screen`↔`pool_screen` 83, `committed_date`↔`disclosures` 54, `context_tags`↔`pool_screen` 48. **The harness is 8,100 lines and grows 400–800 per screen**, because a sixth screen is written the way the fifth was. **Deduplication had been real and worth doing three times and still could not change that.** **The gate exists to catch copy-pasted product logic**, and it was spending most of its signal on test scaffolding whose repetition is structural — and arguably correct, since a step module that shares helpers with another screen's is harder to read, not easier. **Measured before deciding, so the threshold did not have to move:** product-only Rust is **2.09%** (223 duplicated lines of 10,692) against **3.54%** including the harness. **0.9 points of headroom restored and the gate still bites** — the failure mode this avoided is changing what is measured and quietly turning the gate off, which is `#52`'s complaint in a new place. **Rejected: raise the threshold.** Cheapest, and it makes the number mean nothing. **Rejected: restructure the step modules.** It is the only option that actually reduces duplication and it is real work with no product value; **it stays available and unfiled rather than pretended away.** **Open, and adjacent:** `#52` — the threshold says *tokens* and the gate reads jscpd's `percentage`, which is duplicated **lines**. **This decision does not settle that**, and the two should not be confused: one is what is measured *over*, the other is what is *counted*. |
| T-mutation-parallelism | Mutation parallelism is 8 for `scheduler-core` and **1** everywhere else. | `scheduler-core` is pure — no I/O, no shared mutable state — so its mutants are safely parallel. Every other crate's mutants open the same SQLite file and truncate each other's database between test setup and assertion. WAL mode does not save you. The symptom is timeouts on mutants that cannot possibly hang, so it reads as flakiness and gets "fixed" by raising the timeout. **Do not raise the timeout.** Use `--jobs 1`, or give each worker a `TMPDIR`-scoped database and record how here. `scheduler-gcal` mutants run against the mock transport, never the live API. |
| T-three-task-kinds | Task `kind` is a **three-variant** sum type — `Committed \| Pool \| Quota` — committed in the schema at M1. Quota fields (`target_count`, `target_minutes_each`, `period`) are nullable. Quota *scheduling* waits until M8. | Resolves #1. The weekly review is itself a recurring commitment, so without a quota kind M8 needs a bespoke recurring mechanism for exactly one task — which is how a special case becomes permanent. Fitness is one of five domains and inherently quota-shaped: no deadline, so it cannot be committed; but as pool it is never placed, and a Fitness guardrail nothing is scheduled into is a wall protecting an empty room. Capacity math is false in the same way C5 makes the Work number false if quota demand is uncounted. And in Rust, adding a variant later turns every `match` into a compile error — that is the *good* case; the bad case is `if committed { .. } else { /* pool */ }`, which silently treats quotas as pool. Behaviour waits because quota semantics are unsettled and M3 is already the riskiest milestone. |
| T-pins-in-constraints | Pins are a first-class entity in the Constraints layer: `pin { task_id, start, end, source }`. `pinned` is **not** a column on `Block`. Lands at **M3**, with the `schedule()` signature that consumes it — not at M1. | Resolves C1. Blocks are Plan-layer: disposable, engine-written, deleted wholesale on every recompute (R-incremental-patching). A pin is user-authored intent, so a pin living on a block cannot survive the recompute that deletes its row — which makes M3's regeneration property ("delete all future blocks, re-run, get byte-identical placements") and M6's "a drag creates a pin that survives the next recompute" mutually unsatisfiable. `schedule(tasks, hard_events, guardrails, **pins**, now)` had already made the call implicitly by taking pins as an *input*. Rejected alternative: keep `pinned` on `Block` and exempt pinned rows from deletion — that makes the Plan layer partly durable, the exact fact/plan confusion C2 is separately untangling, and weakens M3's strongest property to "delete all *non-pinned* blocks". Deferred to M3 because pins have no M1 behaviour: no `Block` to drop the column from, no scheduler to consume them, no drag to create one. |
| T-auto-close-event | Auto-close is recorded as an **event**, not a column: `auto_close_event { task_id, closed_at, remaining_minutes_before, undone_at }`. Lands at **M6** with auto-close itself. | Resolves C4, and goes further than the issue proposed. The AC bundled two requirements into one column: *undo restores exactly*, and *R-guardrail-override can measure an undo rate*. A column on `task` serves the first and cannot serve the second — it is overwritten on the second close, so the event denominator is wrong and a task that auto-closes repeatedly (the strongest possible evidence that silence does **not** mean done for that work) collapses to a single row. Undo restores from the latest event with `undone_at IS NULL`; undo rate is `count(undone_at IS NOT NULL) / count(*)`, which is what R-guardrail-override's 15%/30% thresholds actually need. Falling back to `estimated_minutes` was never viable: it is correct only for never-started tasks, where undo matters least, and silently inflates every partially-completed one. Additive, so it does not block M1. |
| T-archived-at-only | `archived_at` is the single archive signal. `status: dropped` is removed. | Resolves N1. Two fields for one state, in a system where D-inaction-archives makes archiving the *default* outcome reached implicitly from several paths — decay pass, three-strike, and simply closing the review (U7) — means every path gets two chances to set one and forget the other. The resulting half-archived task is alive on whichever surface filters the field that was missed: a task returning from the dead, in a product whose entire value is that the user trusts what it shows. M8's all-surfaces proptest is the test designed to catch this, and it can only assert a clean invariant against one field. `archived_at` also carries strictly more information — the reckoning's "47 archived this quarter, 31 Learning" needs a timestamp, which `status: dropped` cannot supply. |
| T-module-boundary | Three modules, dependencies pointing inward: `scheduler-core` holds the rules; `trellis-server::http` translates requests into core inputs; `trellis-server::store` translates core types into rows. Adapters name core types; the core names neither. | The rules had been living inside the axum handlers, expressed as `serde_json::Value` probes and `StatusCode` returns — the function that wrote a task row took a *transport* type as a parameter and returned an *HTTP* type as its error. That leaves no seam to test a rule at: answering "is this triage valid?" required a running server and a SQLite pool. It also made T-sqlite-sqlx's "a Postgres swap stays mechanical" untrue, since the queries were spread across handler modules instead of confined to one layer. The split is what makes `scheduler-core` non-empty for the first time, and gives T-core-no-tokio's no-tokio rule something to protect. `store/mod.rs` carries a unit test asserting that no store module names `axum` or `StatusCode`: a layering rule nothing checks is a comment. |
| T-unknown-kind-rejected | `kind` is validated against the three variants at the boundary. A submission naming anything else is rejected with `422 {"unknown_kind": <submitted>}`. | T-three-task-kinds committed to a three-variant sum type, but the M1 implementation read `kind` as `payload.get("kind").and_then(as_str).unwrap_or("")` and stored whatever string arrived, so `{"kind":"banana"}` wrote `banana` into a column whose domain is three values. That is the failure mode T-three-task-kinds named — not the `if committed {} else {}` shape it predicted, but a weaker one, with no discrimination at all. Since `tasks.kind` carries no `CHECK` constraint, the only thing standing between a typo and durable out-of-domain data was the caller. Rejecting is a **behaviour change on input nothing specifies**: no feature, QA procedure or unit test covered an unrecognised kind, and the old permissiveness was an artifact of `unwrap_or("")` rather than a decision. Rejected alternative: keep a fourth catch-all variant to preserve the old behaviour exactly — that reintroduces the stringly-typed hole inside the very type introduced to close it, and makes every future `match` carry an arm that means "we do not know what this is". |
| T-quota-targets-required | Quota triage requires `target_count`, `target_minutes_each` and `period`, rejected the same way `committed`'s three fields are. | T-three-task-kinds made the *columns* nullable for a schema reason — one `tasks` table shared by three kinds, most columns unused per row. That is a storage fact, not a triage-time permission. A quota row with no target can never be scheduled at M8 (nothing to place) and can never appear in D-quota-no-rollover's reckoning (`count(done)/target_count` has no denominator) — the same "wall protecting an empty room" failure T-three-task-kinds named for Fitness-as-pool, relocated to quota-with-no-target instead of pool. Requiring the fields at triage costs nothing today (no M1 surface depends on omitting them) and closes the hole before a real quota row can be created. |
| T-empty-equals-absent | An empty string and an absent key report identically — both use the existing `{"missing_field": <name>}` shape. | `require()` treated `Some("")` as present, which is the empty-string half of the hole this slice closes; the other half is deciding what the closed case reports. Distinguishing "you sent nothing" from "you sent an empty string" is a distinction a client rarely intends — an unfilled HTML form field and an absent field are the same submitter mistake. One shape, one code path in `require()`, rather than a second rejection variant carrying no information the caller can act on differently. |
| T-period-closed-set | `period` is a closed set: `week \| month`. | `deadline_type` and `priority` were closed in this same slice (both became validated sum types in `scheduler-core`) for the reason D-guardrails-never-yield states — the fields the M3 scheduler branches on cannot carry undefined values. `period` is exactly that kind of field for quota scheduling at M8: "3 sessions per `fortnight`" is not a case M8's cadence math is written to handle, and typos (`"weekk"`) currently store the same way a legitimate value would. Only `week` is exercised by any M1 example; `month` is added now because closing the set later, after a real quota row exists, is the same free-now/expensive-later trade T-jiff-epoch-millis already made for `deadline`. |
| T-fact-plan-line | **The fact/plan line runs inside the `Block` table, by block state.** `proposed` and `published` future blocks are the **Plan layer** — disposable, engine-written, deleted wholesale and regenerated on every recompute. `in_progress`, `completed` and `missed` blocks, plus pins, are the **Constraints layer** — immutable facts, read by the engine and never written by it. The determinism property is therefore: *delete every `proposed`/`published` future block, re-run with the same facts, get byte-identical placements.* The signature is `schedule(tasks, busy, guardrails, pins, facts, prior_plan, now) -> placements`. | Resolves C2 (#3), ratified by the owner 2026-08-12. As originally written — "delete every block and regenerate" — the property was not merely wrong but **untestable**, because the move penalty makes the objective depend on previous placements and past blocks are immutable inputs. Wiping the table would destroy history and pins alongside the plan. Drawing the line inside the table rather than splitting it keeps one query surface while making the disposable set precisely definable. Two naming rules come with it, because the vocabulary was in use before it was defined: (1) **"Plan" and "Constraints" are the layer names**; "Facts" is informal shorthand for the immutable block subset, not a third layer. (2) **"Layer" is reserved for this domain split** — T-module-boundary's code organisation is the **module boundary**, not layers, because a `Block` row is otherwise Plan-layer and store-layer at once and the word stops carrying information. Note T-module-boundary's inline five-argument rendering of `schedule()` predates this and is an abbreviation, not a competing decision; the seven-argument form above is the contract, and #11's AC-1 already says "corrected signature per C2". |
| T-migrations-append-only | **Never edit a migration that has been applied. Add a new numbered one.** Enforced in CI: migration files present on `trunk` may be added to, never modified or deleted (#32). Because SQLite has no `ALTER COLUMN`, a type change means the table-rebuild pattern — create the corrected table, `INSERT INTO new SELECT … FROM old`, drop the old, rename — inside a *new* migration. | `sqlx` records each applied migration by checksum, so editing one makes every existing database refuse to boot: `migration N was previously applied but has been modified`, with no fallback and no remediation path. The `triage-validation` slice did exactly this, changing `deadline TEXT` to `INTEGER` inside `0002_tasks.sql`; the damage was zero only because no database happened to exist at that moment. D-visible-slices removes that luck — the owner now runs the app every slice and will always have a live database. A convention is not enough here, because SQLite's missing `ALTER COLUMN` makes editing the old file the path of least resistance every single time: the correct route is roughly fifteen lines of rebuild in a new file, the wrong route is a one-word edit in an old one. The brief that authorised it reasoned "`tasks` has no production data, so this is free today" — which conflates *no production data* with *no existing database*, and is the specific mistake the CI gate exists to make unmakeable. Note `features/migrations.feature` cannot catch this: its idempotency scenario re-runs the *same* migration set, never a changed one. |
| T-releases-are-identified-by-build | **Every push to `trunk` publishes a GitHub Release tagged `v<commit date>.<run number>`, and no `Cargo.toml` version is ever bumped.** The binary reports `trellis <tag> (<commit>)` via `option_env!`. `ops/update.sh` installs the newest release by default, a named one on request — **which is what rollback is** — and refuses four ways before installing: commit not on `trunk`, hash mismatch, not statically linked, and **a binary whose own `--version` does not name the release it came from.** | **Built 2026-08-24 at the owner's direction (#133, PR #134), first releases `v2026.08.25.174` and `.176`.** Before it, the only copy of any build was a workflow artifact with `retention-days: 90`: **nothing installed was recoverable next quarter, there were zero tags, and `trellis --version` printed usage** — you could install a binary and be unable to identify it, which made every other guarantee hollow. **Semver was rejected**: it advertises a compatibility contract with no consumers. **The date is read from the commit, not `date -u`**, so a re-run days later cannot give an old commit a newer-looking tag. **`Cargo.toml` stays at `0.1.0`** because driving it from the tag would mean CI committing a generated number back to `trunk` on every merge, to sync a field no reader consults. **Notes carry the `compare/<tag>...<tag>` link as well as the commits**, because *"what changed between the one that worked and this one"* is a question about a range **no single release knows it is in**. **The release job does not rebuild.** It takes the artifact `gate` proved static, re-checks it, checks the binary already names the tag it is about to be published under, records its SHA-256 in the notes, publishes, then **downloads the asset back out and verifies it against that hash.** **`contents: write` is scoped to the release job alone.** Job-level `permissions:` *replaces* workflow-level, so that token carries nothing else — and `gate`, which runs three unpinned third-party actions and builds the shipped binary (**#109**), keeps its read-only token. **Attestation is proposed, not adopted:** `actions/attest-build-provenance` would bind the artifact to a reviewed commit, and is a fourth unpinned action in exactly the job #109 is about. **Take it after #109 pins by digest, and pin all four in the same change.** **Two bugs found by watching the failure paths, both of which would have shipped green:** `gh api` prints its 404 body to **stdout**, so a captured `|| true` swallowed `{"message":"Not Found"}` and **every first-ever release would have been refused**; and `grep` under `set -o pipefail` killed `update.sh` one line before its own error message, so a release with no recorded hash **exited 1 saying nothing** — the silent failure the check exists to prevent, inside the check. **Previews are unchanged and stay separate:** `ops/preview.sh` uses pull-request run artifacts and pins to `headSha` from this repo. Per-branch and pre-merge versus trunk-only and post-merge — **different mechanisms, deliberately.** |
| T-classifier-covers-domain | **Domain and title categorization folds into the existing classifier trait (M1 keyword impl, M9 LLM impl) — not a second pipeline.** The trait's output grows `domain` and `title` alongside `kind`/`deadline`/`priority`, carrying per-field confidence the same way. The keyword implementation guesses `domain` by keyword match and passes `title` through unchanged; M9's LLM implementation improves both behind the same trait. **Classification is invoked from a background worker after capture, not inside `POST /captures` and not synchronously at triage.** | Two asks arrived separately — classify a capture's task *kind*, and tag it with a *domain* and a cleaned-up *title* — and they are the same mechanism: classify raw text, cheap rules first, LLM later, fall back safely, measure against labelled data. M9 (#19) already commits to exactly that shape, so a standalone categorization worker would duplicate the trait, the fallback and the evaluation harness for a second field set. Growing the trait's output at M1 rather than at M9 is the same argument T-three-task-kinds made for the three-variant `kind`: retrofitting an output shape after M9 depends on it is the expensive order. **On invocation timing**, the apparent conflict between "a trait implies a synchronous call" and "a background poller" dissolves — a trait describes *swappability*, not *when it is called*, and a worker can call a synchronous `classify()` perfectly well. What is genuinely settled is *where*: `POST /captures` has a 50ms budget asserted by `capture_endpoint.feature`, which no LLM round trip fits inside; and blocking triage on a network call puts the latency in front of the user at the one moment they are waiting. So the worker fills the fields between capture and triage. Provider is **OpenRouter** behind a hand-rolled `reqwest`+`serde` client, model pinned by `OPENROUTER_MODEL` with a cheap default — swappable without a code change, consistent with T-handrolled-gcal-client's precedent against generated SDKs. |
| T-templates-take-view-models | **Templates render view models, never store row types.** `http::view` holds what a page shows; `store` holds what a query returned. Handlers map between them. | The inbox slice had `inbox.html` and `capture_row.html` rendering `store::capture::UntriagedCapture` directly — a `sqlx::FromRow` struct, documented as "a capture as the inbox view needs it", which is persistence described in terms of a page. The tell was in `create_capture`: to return the new row's markup it **hand-built an `UntriagedCapture`** for a capture it had just written and never read back, because the template demanded that type. A struct being fabricated to satisfy a renderer is no longer a row. Left alone, every later view inherits the pattern and the templates end up bound to the schema — and the coupling bites in both directions, since `store` would grow a view-shaped type per page. The two structs carry the same single field today and the mapping is one line; that is precisely why this is the cheap moment to draw the line, before the triage screen needs a row id to aim an action at and the calendar needs formatted times that are not columns. This applies T-module-boundary's module boundary to the delivery side — it is not a new boundary, and "layer" stays reserved for the Plan/Constraints domain split (T-fact-plan-line). |
| T-toolchain-pinned | **`rust-toolchain.toml` pins an exact rustc version, not a channel**, and declares the `clippy`/`rustfmt` components and the `x86_64-unknown-linux-musl` target alongside it. Bumping the version is a deliberate act in its own commit. CI asserts the active `rustc` matches the pin. | The file said `channel = "stable"`, which pins nothing — it resolves to whatever stable each machine happens to hold. Two places in the repo nevertheless described it as a pin and reasoned from that: `README.md`'s prerequisites, and the CI comment block whose step is named "Install the toolchain pinned by rust-toolchain.toml". The gap was real and already open — the owner's checkout was on 1.91.1 while `ubuntu-latest` resolved `stable` to 1.97.1, six releases apart. What that costs is not the version lag but the *asymmetry*: `cargo clippy` in CI runs six releases of new lints against code that is clean locally, so a branch that changed nothing goes red on a schedule nobody controls, and the failure looks arbitrary at the moment it appears. The components and target moved into the same file because they are the same class of fact — CI was adding them by hand (`rustup component add`, `rustup target add`), which is a fresh clone's build breaking in a way the workflow already knew how to prevent. Rejected alternative: keep the floating channel and correct the two descriptions to say "selects stable" — honest, and it keeps CI permanently ahead of every developer machine by an unbounded margin, which is the property that produces the arbitrary red build. Rejected also: pin in CI only, via a toolchain action — that makes the workflow the source of truth for something every local `cargo` invocation also needs, and `rust-toolchain.toml` exists precisely so both read the same line. Cost, accepted: the runner's preinstalled stable is now usually the wrong version and gets downloaded (~1 min/run), and adopting a new toolchain becomes a chore someone has to do rather than something that happens. That chore is the point — it lands the new lints on a commit whose subject is the bump. |
| T-package-by-business-domain | **The code is organised by business domain — the capabilities Trellis provides — not by technical role.** A reader learns what Trellis *does* from `ls`, not what it is built with. `T-module-boundary`'s dependency rule survives unchanged; its `http/` + `store/` directory shape does not. **Vocabulary, fixed here: bare "domain" always means a life area** (`T-life-areas-are-data`); **"business domain", always both words, means this packaging axis.** | The owner's standing architectural preference — Uncle Bob's screaming architecture — which had never been written down anywhere, so `T-module-boundary` was settled without it and landed the opposite: `ls src/` said *this speaks HTTP and has a database*. Recorded now because an unrecorded preference is one an agent cannot honour, and three agents had already built against its inverse. **The rule reaches crates too.** The brief's six — `scheduler-db`, `scheduler-gcal`, `scheduler-bot`, `scheduler-web`, `scheduler-bin` — are role names (db, web, bot, bin) and are rejected as a plan; a crate earns existence when a capability needs a real boundary (a purity gate, an independent dependency set), not because a layer has a name. That closes #24, whose three options were all layer-shaped and none of which considered capability packaging. **Renaming existing crates is excluded from the first pass**, because `scheduler-core` is named twice in `swarmforge/constitution/articles/stack.prompt` — the mutation-parallelism table and the core purity gate — and no agent may edit the constitution without explicit owner direction. So the decision covers crates and the execution cannot; that split is deliberate, not an oversight. **The one real trap, and it is this decision's own shape turned on itself:** `store/mod.rs` carries two tests that glob `src/store/*.rs`, and when that directory ceases to exist they do not fail — they stop covering anything. `T-module-boundary` wrote its own warning — *"a layering rule nothing checks is a comment"* — and this is how that rule dies quietly. A replacement gate ships inside the restructure (#44), not after it. |
| T-life-areas-are-data | **SUPERSEDED 2026-08-20 by `D-context-tags-are-the-taxonomy`.** The modules remain in the tree until after dogfooding. **Life areas — what this log elsewhere calls domains: Work, Fitness, Learning, Family, Home — are user-managed rows, editable from the running app.** Not a Rust enum, not a config file. Adding one is never a development task. A fresh database seeds five; the set belongs to the user from then on. **A life area is well-formed only once it has a guardrail, or is explicitly marked pool-only** — enforced at M2, when guardrails exist. | Resolves #36, which framed this as closed-enum versus extensible-plain-data and read the enum as obligatory by analogy with `T-three-task-kinds`, `T-unknown-kind-rejected` and `T-period-closed-set`. **The analogy does not hold, and that is the load-bearing part.** Those are fields the scheduler *branches on* — a `match` with a different body per variant, which is why `T-three-task-kinds` warned that `if committed {} else {}` silently mistreats a third variant. A life area is a **lookup key, not a discriminant**: there is no per-life-area code path, since the scheduler looks up its guardrail, counts its capacity and groups the reckoning by it identically for every one. A set you `match` on must be closed; a set you index by need not be. **And the enum was never protecting against the real hazard.** The stated risk was a life area no guardrail governs and no capacity number counts (`D-guardrails-never-yield`: a life area outside the walls is covered by no wall) — but a non-exhaustive `match` reports missing arms, never a missing wall. The guardrail-completeness rule above is the check that actually catches it, and it needs no compiler. **`T-complexity-8` is unaffected:** its threshold of 8 is derived from *"the largest enums (`Domain`, `BlockState`) have 5 variants"*, and `BlockState` still has exactly five (`T-fact-plan-line`), so the derivation stands on the other enum alone. #36's claim that the justification stops describing the codebase does not survive contact with it. **This is not `R-plugin-surface`**, which refuses a general-purpose extensibility surface; this is one entity of the product's own model being editable by its single user (`D-single-user`), the same class of thing as the guardrails have always been. Rejected alternative: a config file read at startup — it satisfies "no rebuild" and still fails this project's own bar, that *a schema element with no observable behaviour has nothing to specify against*; the management surface is precisely what makes this specifiable at M1 rather than deferred. Rejected also: two vocabularies, an extensible capture tag plus a closed scheduler `Domain`, as the capture-categorization proposal assumed — every consumer of a life area (guardrails, capacity, the reckoning, menu diversity at #41) is a scheduler concern, so the mapping between the two lists would become the real list, kept in a third place nobody names. The seed keeps every life area cited in a settled decision's reasoning — Fitness (`T-three-task-kinds`), Learning (`D-kill-means-archive`), Family (`D-single-user`), Work (`D-pool-is-default`) — and adds Home for the errand and admin traffic (`buy milk`, `renew passport`) that neither candidate set housed. It is a seed, not a ratification: it is editable the moment the app runs, which is exactly why getting it wrong is now cheap. |
| T-forms-swap-one-fragment | **A page's forms live inline in the row they act on; a page region rendered by more than one handler is one shared fragment with one id; and a rejection re-renders that same fragment carrying the error, returned as `422`.** htmx is configured to swap on 422 as well as 2xx. | Settled inside the `triage-from-page` pipeline run and recorded here afterwards, because the brief asked for each as a precedent — *"whichever you pick becomes the pattern for every form in this product"*, *"it is the first error-display pattern in the product"* — and the answers ended up living only in code and a commit message. The three are one design, not three preferences: forms inline in the row is what makes the row the unit of action; one shared `#lists` fragment is what lets a triage submitted from the inbox update both the inbox and the task list in a single swap; and re-rendering that same fragment on rejection is what keeps the error attached to the row that caused it, rather than inventing a second error surface. `http::lists` exists because the fragment gained a second renderer — the same "when a second caller appears, the shared thing gets its own home" move as `view`, `payloads` and `app_client`. **The cost, named because it is global and easy to miss:** the 422 swap is configured once for the page (`htmx.config.responseHandling.unshift` in `inbox.html`), so 422 is a swappable status for *every* htmx request on it, present and future — including the quick-add box, which never asked for it. Any endpoint that returns 422 with a body that is not the re-rendered fragment will have that body swapped into the DOM. Accepted deliberately, in exchange for one error convention instead of per-form handling; the guard is that **422 means exactly "validation rejection, body is the re-rendered fragment" everywhere in this product**, and an endpoint that cannot honour that must not use 422. |
| T-capability-owns-its-queries | **A business domain owns the SQL it issues, not the table it touches.** Three capabilities write to `captures`: `capture` inserts the row, `triage` stamps `triaged_at`, `inbox` lists the untriaged. Each query lives in that capability's own `store.rs`. No module owns a table. | The rule that decides whether `T-package-by-business-domain` actually happened, or whether the technical layer merely survived under a capability's name. The tempting alternative is "`capture` owns the `captures` table, so every query against it lives there" — which sounds like ownership and is in fact the old `store/` directory with a new label: `triage` and `inbox` would both reach into `capture` for persistence, the dependency arrows would point sideways between capabilities rather than inward, and `ls src/` would go back to describing storage. Table-shaped ownership also gets the coupling exactly backwards. `inbox`'s listing query is coupled to *what the inbox shows* — it changed when the inbox gained a task list, and it will change again when life areas land (#47) — not to what the `captures` table is. Two queries against one table for two different reasons are two facts, and putting them in one file because the table is one table is the same category error `T-templates-take-view-models` corrected on the delivery side, where a `sqlx::FromRow` struct was documented as "a capture as the inbox view needs it". Accepted cost, and it is real: the same table is now written from three files, so a schema change touches all three rather than one, and a nine-line test preamble is duplicated between two `store.rs` files. Both are the price of the arrows pointing inward, and both are visible — the compiler finds the schema change, and the duplication was reported by the DRY gate rather than hidden. Enforced by `platform/boundary.rs`, which asserts no module outside a `store.rs` or `platform/db.rs` writes production SQL, so a capability cannot quietly start querying from its handler instead. |
| T-set-operations-execute-in-the-store | **Ordering, filtering, grouping and limiting are business rules owned by `scheduler-core`, and they execute in the store.** The core says *what* the order is; the query does it. **A list query that returns a collection carries its own `ORDER BY`, `WHERE`, `GROUP BY` and `LIMIT`** — the store never hands back an unbounded, unordered set for a pure function to sort. Where a rule is too rich for a bare clause, the core passes a **specification value object** describing it in domain terms, and the store translates that into SQL. `scheduler_core_purity.sh` still forbids `sqlx` in the core, so a specification must be expressible without it. | **Adopted 2026-08-23 from the company standard at `https://www.garrellts.com/docs/agent-architecture`, which binds every Garrell Tech Solutions repository and instructs agents to reject a plan that violates it.** Its invariant 2: *"the application layer must never load an entire collection into memory to perform slicing or reordering that the underlying persistence mechanism can handle natively."* **Trellis already passed the standard's other two invariants before ever reading it** — no storage syntax in the core (enforced by `scheduler_core_purity.sh` and `platform/boundary.rs`) and opaque retrieval (`T-one-front-door-per-capability`) — and it **already followed this one in four places**: `inbox/store.rs:22` and `:88` order in SQL, `capture/store.rs:82` uses `ORDER BY id ASC LIMIT 1`, `:97` uses `GROUP BY` with `MIN(id)`. **The two newest screens broke the pattern**: `pool/store.rs:29` and `committed/store.rs:34` fetch everything unordered, and `scheduler_core::pool` then sorts, buckets, orders groups by count and truncates to `VISIBLE_TRIP_ITEMS` — the last of which is pagination in memory. **The older code got it right and nobody wrote the rule down**, which is the ordinary way this drifts. **The cost, named:** these rules are the most-tested code in the crate — proptested, mutation-tested, `T-trips-are-derived-not-ranked` made executable — and SQL is where `cargo-mutants` cannot reach. This project has shipped a proptest that could not fail and nine mutants surviving a scenario that could not fail; **doing it deliberately a third time would be worse than the violation.** That is precisely why the rule keeps the *rules* in the core and moves only the *execution*, and why any slice moving one must say what still proves it. **Rejected: reading invariant 2 as scale-gated.** The standard's principle says *large collections* and Trellis is one user with hundreds of rows, but the invariant carries no qualifier and the cost of complying rises with the data rather than falling. Tracked as #107 (committed, the cheap case) and #108 (pool, which needs the specification object). |
| T-capacity-never-under-reports-demand | **TRANSLATED 2026-08-23: the capacity view is gone; the rule now governs the Quota screen.** Read *capacity number* as *a quota's hours against its weekly target*. **The rule is unchanged and still load-bearing — a number about what you owe may overstate, never understate** — so an item with no estimate is excluded from the sum and surfaced as a count rather than counted as zero. **Every choice in the capacity number resolves the same way: it may overstate what you owe, never understate it.** An unestimated committed task is **excluded from the sum and surfaced as a count**, never counted as zero. Quota proration is horizon ÷ period, rounding **up** to the whole minute, and **a month is 30 days flat** rather than the calendar month. "Over" is **strictly above 100%**, with utilisation shown on every row rather than only on the ones that breach. | Settled inside the `capacity` slice, 2026-08-18, closing M2. The four answers look like four decisions and are one: **a capacity number wrong in the direction of *"you have more time than you do"* is the failure this milestone exists to prevent**, and it is the failure #6 was settled to fix — the Work number reporting 22h free while the hours are physically full, discovered Thursday at 4pm. Every rounding therefore breaks toward *more demand*. **Counting an unestimated task as zero is the subtle one**: it is arithmetically tempting and it silently shrinks demand, so the task is excluded and its existence reported instead — the number stays honest and the owner can see exactly how much of it is unknown. `None` means *"written before the estimate was required"*, never *"takes no time"*, the reading `T-life-area-required-at-triage` already gave `life_area_id`. **The 30-day month is not laziness**: a fourteen-day horizon can straddle two calendar months, and *"which month's length"* has no answer then — a flat 30 is wrong by at most a day and is wrong the same way every time, which a calendar month is not. **On the threshold**, `D-staleness-unset` is applied literally — *instrument first, tune at the first reckoning* — because a margin picked today is picked with no fortnight of real numbers behind it. The page reports 98% and lets the owner judge it; the number to warn at is chosen once there is data to choose from. |
| T-qa-binds-tolerantly-to-markup | **A QA script finds things in HTML by regex, so it must match the stable part and tolerate the rest.** Bind to an id, a `data-` attribute or a class the script owns; **never to attribute order, adjacency, incidental attributes, or copy quoted verbatim.** A restyle is not a behaviour change and must not break QA. | Requested by the specifier for #82, after the same failure caught a **third** set of scripts. **CI cannot see this class of breakage, by construction.** PR #87 landed a 636-line stylesheet and restyled three templates **without touching a single feature, QA script or step module** — so nothing asserted against what it changed and every gate stayed green, while `capture_row.html`'s `<li>` gained `class="row"` and broke `qa_capture_row_block`'s exact match. #90 found that one by re-running QA; #82 then found `qa_quick_add_endpoint`, which assumed `hx-post` immediately followed `<form`, and `qa_capture_id_from_response`, which assumed the capture row's `<li>` carried no trailing attributes. **Three sightings, one cause: the scripts asserted structure they did not own.** This is not a fail-open — the gates were honest; there was simply **no gate**, because a change that touches only templates asserts against nothing. **The related repair, and the better half:** `qa/dismiss_capture.md` quoted the empty-state copy verbatim and broke when #87 reworded it; it now asserts **sameness with the ordinary empty state** instead. *Pinning copy has broken once and told nobody anything* — an assertion should name the property, not the pixels. **The general form is the same one this project keeps meeting**: an assertion that binds to something incidental is an assertion about the wrong thing, and it fails at a moment that teaches you nothing about the code. |
| T-a-check-must-be-seen-to-fail | **A check earns its name only once it has been observed to fail.** A slice that adds a gate, a scenario, a property or a script **breaks the thing it guards, watches the check fail, restores it, and watches it pass** — and says so in the pull request. **A check that has only ever been seen green is an untested claim**, whether it is untested because nothing runs it or because nothing could make it fail. | **Recorded 2026-08-23 after the fourth instance in two days**, proposed by the specifier in PR #113. The pattern has two shapes that produce the same artefact — a green report guaranteeing nothing. **A check nothing runs:** the phone-layout browser check shipped ungated (#101, then #106); **the property tests had never run in CI once** — `origin/trunk`'s `ci.yml` contained zero occurrences of `include-ignored` against 41 `#[ignore]` attributes, so #11's strongest acceptance criterion (*"Invariants 1–5 hold under proptest, ≥ 1000 cases"*) was unenforced from the day it was written, **and the property added under `T-cross-capability-invariants-need-an-owner` to stop #92's defect recurring was itself never run.** **A check that cannot fail:** `task_kinds.feature`'s regexes match `"<(\w+)>"`, so one Examples cell feeds both the request and the assertion; nine mutants survived a scenario that could not fail; and a scenario asserting invariance under exactly the transformation the mutator applies is untestable by that mutator **and looks fully covered**. **The cost of compliance is small and already demonstrated.** #105 reverted the CSS to the bug, confirmed two named assertion failures and exit 1, restored, confirmed a clean pass, and separately proved the check **fails closed** rather than skipping when its browser is missing. That is the whole practice. **Why it needs to be a rule rather than a habit:** every one of these was *reported accurately* — the reports said the tests passed and they had. **The gap is between a true statement and a guarantee**, and it is invisible to any reader who does not go looking, which is why it recurred four times while being flagged in prose each time. **Bounds:** it governs checks a slice *introduces or changes*, not a demand to re-break every existing gate on every slice. **Rejected: trusting a green run as evidence the check works.** That is the exact inference that produced all four. |
| T-dead-core-code-earns-its-keep | **Code in `scheduler-core` with no caller is kept when rebuilding it would be expensive *and* its return is scheduled; deleted when rebuilding it is cheap.** `scheduler_core::schedule` and `::interval` **stay** — M3 is paused, not cancelled. `scheduler_core::ratio` **goes**. | Requested by the specifier in its handoff note for #88, because **two modules failing the identical "no caller" test got opposite verdicts** and without a rule that reads as arbitrary. **The test is the cost of rebuilding, not the cost of keeping.** `schedule` is 775 subtle lines carrying 1000-case properties, the placement order that `T-plan-is-stored-and-explicitly-regenerated` makes a contract, and four of the five ratified invariants; reconstructing it means re-deriving decisions that took six settlements in a day (2026-08-19), and `D-dogfood-first` **schedules its return** against a named evidence bar rather than leaving it open-ended. `ratio` is 223 lines whose hard parts — the settled denominator of all three kinds, the fourteen-day window, the ten-task floor — **are recorded in this file and in #45's pull request, not only in the code**. Rebuilding it is transcription. **That asymmetry is the whole rule**: dead code is a liability the gates keep measuring, so it must buy something, and what it buys is the re-derivation it saves. **Both halves are needed.** Cheap-to-rebuild code kept "just in case" is the liability with no purchase; expensive code kept with no scheduled return is a bet nobody has to justify — and `D-dogfood-first` is exactly the sort of statement that makes the second half checkable. **The corollary is that a decision log lowers the cost of deletion**: `ratio` is safe to delete *because* its reasoning was written down, which is a return on this file that had not been noticed before. |
| T-latency-is-a-qa-assertion | **A wall-clock latency budget is asserted in the QA suite against a real server on a quiet machine — never in a unit or acceptance test.** Capture's **50 ms** budget stays as the stated design constraint and moves out of `capture_endpoint.feature` and `platform::app::tests`. | Approved by the owner 2026-08-20, closing #66. **The obvious harm was the flake**: `capture-endpoint-persists-quickly-01` fired spuriously in four of six slices — 612 ms at load average 15.5 during #70 — costing a re-run and a paragraph of adjudication every time, and CI has gated the acceptance suite since #53, so a busy runner produced a red build about somebody else's mutation job. `T-toolchain-pinned` named that failure precisely: *"the failure looks arbitrary at the moment it appears"*, and the standard response to an arbitrary red build is to stop reading it. **The harm that actually decides it is worse and was silent.** The architect found in `e49b9a8` that under mutation-run contention the request took **1.885 s**, which failed `cargo-mutants`' unmutated **baseline** — so it refused to test a single mutant and **the entire `trellis-server` crate had zero mutation coverage, with no error message connecting the cause to the effect.** A gate reporting nothing while appearing to pass is the exact shape this project has now met four times: `dry.sh`'s fail-open (#68), the `src/store/*.rs` glob that would have stopped covering anything (`T-package-by-business-domain`), a proptest whose generator could not produce a failing input (#73, #81), and this. **The general rule worth carrying: a wall-clock assertion is not merely unreliable, it is load-bearing for tooling that fails closed in the wrong direction.** It is a property of the deployed system, and measuring it on a contended machine yields a number that is **false rather than merely noisy**. Keeping 50 ms as a design constraint costs nothing — `capture`'s own module header cites it, and `T-classifier-covers-domain` reasons from it to put classification in a background worker. **What changes is only where it is checked**, and by something that can be run when the machine is quiet and re-run when it is not. |
| T-required-fields-are-specified-per-transport | **Requiring a new field is not specified until every transport that must supply it is specified.** The boundary rejecting its absence is half; the form **offering** it is the other half, and they are separate scenarios. | Recorded because it cost a shipped-broken page. `capacity` made `estimated_minutes` required for committed triage and specified it at the boundary — `committed_triage_validation` gained the field in both Examples tables — while `capture_row.html`'s committed form was never given an input for it. **Every committed triage submitted through the actual page returned `422 missing_field=estimated_minutes`**, and the entire acceptance suite stayed green, because it triages its committed fixtures over JSON. QA found it reproducing the brief's demo against a live server. **The gap is invisible to a suite that exercises one transport**, and this product has run two transports over one triage path since #33 — with a third at M7 (`T-capture-surfaces`). **Note what did not save it:** #33's property asserts the page and the endpoint are one code path by submitting one arbitrary submission through both and requiring identical results. That is equivalence *given a submission* — **it cannot catch a transport that is unable to produce the submission at all.** `triage_from_page` already carries `triage-from-page-committed-closed-choices-04`, asserting what the committed form offers, which is exactly where the missing scenario belonged. |
| T-availability-only-subtracts | **SUPERSEDED 2026-08-23 by `D-context-tags-are-the-taxonomy`** — exceptions and guardrails are both gone, so nothing narrows anything. **The asymmetry it names is the durable part:** *"working this Saturday"* is a different availability, never an exception to the usual one, and any model that lets an exception *add* hours has two ways to say the same thing. **Nothing narrows a guardrail except by removing hours from it. A dated exception removes; it never adds.** *"Working this Saturday"* is not an exception — it is a different guardrail. **The same shape serves M3's pins and M4's calendar busy**, which are also subtractive; `free_intervals` takes them as further inputs of one kind, not as a second mechanism. | Settled inside the `exceptions` slice, 2026-08-18, answering #61's second open question — the one the brief flagged as *the one place this slice could quietly breach `D-guardrails-never-yield`*. An additive exception is an override with a friendly name: a wall that can only shrink for a day cannot be argued into yielding, and *"an override that exists will get used, and then the walls are decorative"* (`R-guardrail-override`). **The second reason outlives this slice and is why the row is here rather than in the issue:** dated exceptions are the **first subtrahend** this product has ever had, and pins (`T-pins-in-constraints`, M3) and calendar busy (M4) are both subtractive too. One shape serves all three. An additive exception would have made this a per-day override of the mask, which neither inheritor wants and which would have had to be unpicked twice. `Interval` in `scheduler_core::free_time` is deliberately not named after a producer for the same reason. **Exceptions are whole dates**, not time ranges: M4's calendar busy is the mechanism for intra-day unavailability, and building half-day exceptions now means two mechanisms for one need. **On work already placed into removed hours** — nothing today, since nothing is placed until M3, but recorded for M6's reality-flex loop to inherit: it becomes **a conflict the owner is shown**, never silently kept (which breaches the wall) and never silently dropped (which loses a commitment). |
| T-configuration-is-removed-work-is-archived | **TRANSLATED 2026-08-23: guardrail bands and dated exceptions no longer exist; the live configuration is the timezone setting and a quota's target.** The rule is unchanged — **the test is whether anything downstream ever counts it.** Work items are archived and kept; configuration is removed outright, because nobody will ask how many quota targets were edited last quarter. **`D-kill-means-archive` governs work items, not configuration.** A task, a capture, a life area — anything whose row feeds a later reckoning — is archived and kept. A guardrail band or a dated exception is **removed outright**. | Requested by the specifier in its handoff note, which is now the second time a slice has named the row it needed rather than leaving it to archaeology. `D-kill-means-archive`'s reasoning is entirely about **evidence**: *"the row feeds the reckoning ('47 archived this quarter, 31 Learning' is real signal)"*, and `T-archived-at-only` wants a timestamp for the same purpose. **Configuration produces no such signal.** Nobody will ever ask how many guardrail bands were deleted last quarter, and M8's reckoning has no denominator that a removed exception belongs in — so archiving it accumulates rows that no surface reads and no count includes, which is the *"place to hide from decisions"* `D-kill-means-archive` refuses, arriving from the opposite direction. The precedent already existed and was unstated: **#59 removes guardrail bands outright** and nobody argued. The distinction is worth having written down because *keep the row* reads as a universal in this log and is not one — the test is whether anything downstream ever counts it. |
| T-hard-refuses-soft-slips | **`deadline_type` decides what happens when a deadline cannot be met. A **hard** deadline is a feasibility constraint: a task that cannot finish by it is not scheduled, and is reported `deadline_unreachable`. A **soft** deadline may be overrun: the task is scheduled late and carries a **projected finish**.** | Ratifies U2 (#7), open since 2026-08-12, and settled by the owner 2026-08-18. U2's complaint was that *"the field is declared and never used again; as written it is dead"* — `deadline_type` has been in the schema with a `CHECK` since `0003`, has been required at triage since #29, and has never changed a single behaviour. This is the behaviour. **The distinction has to exist somewhere or the field should be deleted**, and deletion was the honest alternative considered: this project has removed `scheduler_core::life_area::same_name` and reduced `TriageRejection::UnknownKind` to a unit variant on exactly that reasoning. It survives because *"I would like this by Friday and the world will not end"* is a real and common shape, and **a projected finish is more useful to its owner than a refusal** — where for a tax return the refusal is the whole point. Note the asymmetry with `D-placed-whole-or-not-at-all`: a soft-deadline task still schedules **whole**, just later. Slipping is about *when* it finishes, never about *how much* of it gets placed. |
| T-plan-is-stored-and-explicitly-regenerated | **The plan is stored and regenerated only when asked — never recomputed on page load.** Triage a task after generating and the plan is unchanged until you generate again. **Placement order is a contract**: least slack (`deadline − now − estimate`), then priority, then **task id ascending**. | Settled inside `schedule-forward-pass`, 2026-08-19. `R-incremental-patching` settled that the plan is recomputed from scratch and never patched; it never said **when**. Recompute-on-load satisfies the demo and fails the scenario that matters: **a block moving under the owner between one glance and the next is the opposite of something to work from**, and `D-no-pool-on-calendar`'s *"the calendar's entire value is that everything on it is true"* is worth nothing if true means *true at the instant you looked*. **The third tiebreak is not pedantry**, and this slice proved it twice. S5's regeneration property demands **byte-identical** placements, and without a total order two tasks equal on slack and priority are ordered by whatever the query returned. It was also the thing the architect's order-independence property **failed to test**: the property passed with the tiebreak deleted, because `any_task`'s wide estimate and deadline ranges make an exact tie vanishingly rare, so the generator almost never produced the input the tiebreak exists for. **A total order is what makes a stored plan a plan rather than a snapshot.** *"Slack with no deadline"* gets no rule because a committed task has always had one (required since #29), which is the same obsolete-premise shape U3 had. |
| T-unplaceable-reason-precedence | **The four unplaceable reasons are tried in a fixed order — `no_window` → `deadline_unreachable` → `capacity_exceeded` → `chunk_policy_unsatisfiable` — because more than one can be true at once.** A task against 4h of free time in 2h windows is **`chunk_policy_unsatisfiable`, not `capacity_exceeded`**: time remains, *contiguous* time does not. | Settled inside `schedule-forward-pass`, answering a question the brief did not ask and could not be specified without. #11 fixed the enum's **membership** and said nothing about **precedence** — and a closed set of reasons where several apply simultaneously is not a closed answer until the order is contract. Invariant 5 requires every unplaceable task to carry *a* reason; it never said **which**, so two implementations could both satisfy it and disagree on every interesting case. **The fourth is the one worth understanding.** Reporting `capacity_exceeded` for a task that fails only on contiguity would be a number-shaped lie of exactly the kind `T-capacity-two-axes` exists to prevent — the owner would widen a guardrail that is not full. And the reason **stays honest about the capability rather than the task**: at S1 the chunk policy is *"one whole chunk"* (`D-placed-whole-or-not-at-all`), so when S3 adds splitting the same task becomes placeable and the same reason correctly stops applying. |
| T-interval-is-its-own-module | **`Interval` and its algebra live in `scheduler_core::interval` — their own module, not one named after a producer.** M4's calendar busy intervals are not guardrail-derived and M3 consumes both. | **Settled at #60 on 2026-08-19 and recorded here retroactively, because it was never written down and the next two slices built past it.** The PM decided at the time that a type shape belonged in `docs/design/architecture.md` rather than in this file, and **then did not write it there either** — so the decision existed only in PR #70's body, which nothing re-reads. It shipped inside `free_time`, stayed through #61 and #62, and the predicted cost arrived exactly as stated: `schedule` had to import `free_time` while having nothing to do with projecting a guardrail, and **interval subtraction got built as two private helpers inside `schedule`**, where M4's second producer could not reach them and would have written them a third time. **The process lesson is the durable part.** A brief's open questions get triaged by the PM into *worth a decision row* and *architecture.md's job* — and only the first bucket has a forcing function, because rows are written in the same pass that reads the pull request. **"The architect will put it in the architecture reference" is not a plan.** An answer worth asking for is recorded in the same pass it is read, wherever it goes. |
| T-splitting-is-opt-in | **Trellis never breaks a task up unless the owner said it may.** A committed task carries a **splittable** flag, **unticked by default**. Unticked, it is one block or it does not fit. Ticked, the scheduler may chunk it under `T-minimum-session-per-life-area`, **which is thereby narrowed to opted-in tasks only.** A task that will not fit whole is told so — *"needs 6 contiguous hours; Work never has more than 4"* — rather than quietly chopped. | Settled by the owner 2026-08-19, and it reverses the brief's default rather than clarifying it. The question that produced it: *"why is Trellis splitting it up? … it seems to me that the **USER** will be breaking tasks down."* **The brief assumed homogeneous work** — review 200 CVs, write 5,000 words — where any 2h is as good as any other 2h. **Most committed work is not that.** *"Write the Q3 deck"* is outline → draft → polish, and **the seams belong to the owner, not the scheduler**; chopping it 2+2+2 cuts across them and produces three sittings that each begin by rediscovering where the last one stopped. **Why a flag rather than removing splitting outright**, which was the owner's first instinct and nearly right: homogeneous work genuinely exists, and forcing its decomposition into *"CVs 1–50, CVs 51–100"* is manual work the scheduler can do correctly. The flag lets each task say which kind it is, which is knowledge only the owner has. **Why the default is unticked**, and why this is not the silent-wrong-default `D-manual-triage-until-llm` refused: **"don't chop my work" is the conservative reading of silence**, whereas no life area is conservative — Work is not safer than Fitness, so an unanswered picker had to stay unanswered. A default that means *do nothing I was not asked to do* is a different object from one that guesses. **The cost the owner named and accepted:** nothing orders the pieces. *"Deck: outline"*, *"draft"* and *"polish"*, all due Friday and equal on slack, may be placed in any order — **ordering between tasks is a dependency and Trellis has no such concept.** For independent sittings this is fine; for genuinely sequential work the owner sequences by giving the pieces different deadlines, or lives with it. A dependency graph is not v1. |
| T-pin-binds-a-task-not-a-chunk | **A pin binds a task to an interval, never a chunk.** For a non-splittable task that is the whole block. For a splittable one, a piece of exactly that interval is placed there and the remainder is scheduled around it. **Two consequences.** Invariant 1 reads: no block overlaps a pin **other than the pin binding its own task** — `free_intervals` subtracts pins from general free time, and a pinned task is placed into its pin directly. And **a pin is exempt from the minimum session**: pin thirty minutes in a life area whose minimum is ninety and it holds. | Answers the first of the two questions C1 raised on 2026-08-12 and the last thing blocking M3 (#78): *"for a 6h task split across three days, which chunk does the pin bind — and chunks have no identity to refer to."* **The question had no answer and could not have one**: `R-incremental-patching` regenerates the plan from scratch, so the next run produces fresh chunks with nothing carried over to match a drag against. **But the type never mentioned chunks** — `pin { task_id, start, end, source }` binds a *task* to an *interval*, and reading it literally is both correct and the only reading that survives regeneration, precisely because it references nothing that gets regenerated. `T-splitting-is-opt-in` is what makes it unambiguous rather than merely workable: a non-splittable task has exactly one block, so there was never a chunk to disambiguate. **The invariant-1 clause is a correction to `T-invariants-one-to-five`, made one day after ratifying it**, and it was found by working through this case rather than by re-reading: as written, a pinned task's own block *overlaps its own pin*, which the invariant forbade. **The minimum-session exemption follows from `T-fact-plan-line`**: pins are Constraints — owner-authored input — and the minimum exists to stop the **engine** manufacturing useless fragments. A rule that governs engine output has no business overruling an explicit instruction from the person it serves. |
| T-backward-pass-with-margin | **The backward pass takes hard-deadline tasks only, and places them at latest-feasible-start *minus a safety margin*.** The margin is **best-effort, never a constraint** — it pulls placement earlier, and where there is no room earlier, placement falls back to latest-feasible-start. **A margin must never turn a placeable task into `deadline_unreachable`.** Seeded at **20% of the task's estimate**. A P1 with a **soft** deadline is not in the backward pass; it enters the forward pass at the front of the priority tiebreak. | Resolves U3 (#7), settled by the owner 2026-08-19. **U3's premise was obsolete**: it asks about *"a P1 with no deadline"*, and `T-three-task-kinds` put `deadline` and `priority` **both inside** `Committed` and made both required on the same day — so priority exists only on committed tasks and committed tasks always have a deadline. The representable question was P1-with-a-*soft*-deadline, and it answers itself: the backward pass exists to **guarantee** a deadline is met while preserving packing, and a soft deadline has nothing to guarantee because `T-hard-refuses-soft-slips` lets it slip. U3's recommendation was right; its reason was not available to it. **The margin is the owner's amendment and it is the substantive part.** The brief specified latest-feasible-start, which is **zero slack**: any disruption to the last planned session makes the deadline miss, and M6's recompute is a thin answer because replanning Thursday evening on a Friday deadline leaves Friday. Front-loading instead was rejected on the packing that the backward pass exists for — a 20h task due Friday, placed early, eats the Monday slot a 2h task due Wednesday needed, and reports `deadline_unreachable` for the small one when both could have fitted. **Late-minus-a-margin keeps the packing and buys back slack**, which is what a person actually does. **The best-effort rule is what makes it safe**: a margin that could cause an infeasibility would be a scheduling preference overriding a real constraint, which is the shape `D-guardrails-never-yield` refuses everywhere else. **The seed is a proportion of the estimate, not of the time to the deadline**, deliberately: 20% of the span would drag a task due in six months five weeks forward and erode the very packing the backward pass was chosen for, while a proportion of the estimate scales with the thing most likely to go wrong — a bigger task has more sessions to disrupt. `D-staleness-unset`'s *instrument first, tune at the first reckoning* applies to the number, not to the shape. |
| T-minimum-session-per-life-area | **RESTS ON A REMOVED MODEL, 2026-08-23.** Life areas no longer exist, so *"each life area carries a minimum"* has no subject; splitting is explicitly out of scope per `D-menu-is-a-worklist`. **Parked with M3 (`D-dogfood-first`), not superseded — the question is real and unanswered, but it cannot be restated until it is known what carries the minimum.** A context tag is the obvious candidate and is probably wrong: tags are numerous, cheap and disposable by design, and a minimum session is a commitment. Also still open: `buffers` in its `free_intervals` reference is undefined (#80). **Each life area carries a minimum session length — the smallest piece of a split worth scheduling.** **Narrowed 2026-08-19 by `T-splitting-is-opt-in`: it governs only tasks the owner marked splittable.** It governs **committed** tasks only: pool is never placed, and a quota task's `target_minutes_each` already defines its session. **Two rules follow.** A task shorter than the minimum is placed **whole** — the minimum governs how a bigger task is chopped, not whether a small task may exist. And **every chunk of a split must clear the minimum**, or the task is `chunk_policy_unsatisfiable`. Seeded **Work 90 · Learning 45 · Family 30 · Fitness 20 · Home 15**, new life areas defaulting to 30. **No maximum.** | Settled by the owner 2026-08-19, filling a hole several decisions had already leaned on: `chunk_policy_unsatisfiable` has sat in the closed reason enum since #11 with **nothing able to make it fire**, and `T-blocks-do-not-cross-guardrail-seams`' surviving cost — *"a task whose minimum chunk forbids the smaller piece"* — could not be sized because no minimum existed. **The trade is real in both directions**: without a minimum, a 6h deck takes a 30-minute Monday gap, which is time enough to open the file; with one, that gap is skipped and the task may not fit at all — and `D-placed-whole-or-not-at-all` makes that consequence sharp, since there is no partial placement to soften it. **Per life area rather than one global number, because a 30-minute run is a run and thirty minutes of deep work is nothing.** A single figure protecting deep work forbids short Fitness sessions, and one permitting them books 20-minute deck slots. `D-staleness-unset`'s *instrument first, tune at the first reckoning* argued for a single seeded number and lost on that asymmetry — but its instinct survives in the **defaults**: the field sits beside the guardrail on a form the owner already visits, pre-filled, so setup costs nothing unless they care. **`T-life-areas-are-data`'s well-formedness rule does not grow** — a life area needs a guardrail or a never-scheduled mark, and a minimum session always has a default, so it can never be the thing that is missing. **No maximum for v1**: if six contiguous hours exist, use them. A maximum would force a split where none is needed, and *"nobody should work six hours straight"* requires a **break** between the pieces, which is a different concept — and one that may be what `free_intervals`' undefined `buffers` term was for. |
| T-blocks-do-not-cross-guardrail-seams | **RESTS ON A REMOVED MODEL, 2026-08-23.** There are no guardrails and no life areas, so there is no seam to cross. **Parked with M3 (`D-dogfood-first`), not superseded.** The underlying question — *may one piece of work be placed as a single block across two differently-governed stretches of time, or must it become adjacent blocks* — returns the moment any availability model does, and the answer here was reached carefully; do not re-derive it, restate it. **A block never spans two life areas' guardrails, even where they are adjacent.** Work that runs across the seam is placed as **adjacent blocks at the same wall-clock times** — 16:00–17:00 in Work's hours and 17:00–19:00 in Learning's, worked straight through. This confirms invariant 2 (`T-invariants-one-to-five`) and closes **U4** (#7), open since 2026-08-12. | Settled by the owner 2026-08-19. **The decisive argument is not the one U4 offered**, which was that invariant 2 is *"simple and property-testable"*. It is `T-capacity-two-axes`: capacity is **consumed from the guardrail occupied**, so a block spanning Work's hours and Learning's has consumed one hour of the first and two of the second and **must be divided for accounting whatever the calendar shows**. Letting it cross does not avoid the split — it moves the split out of the thing the owner looks at and into arithmetic they cannot see. A crossing block would also **belong to two life areas at once**, which every surface that groups by life area — the calendar, the menu (M3.5), the reckoning (M8) — would then have to represent. **The recorded cost was overstated and is corrected here.** The 2026-08-12 note said a 2h task across a 17:00 seam *"cannot use those two contiguous free hours"*. That is only true of a task that cannot be **split**: M3 has chunk policy and a `chunk_policy_unsatisfiable` reason, so the ordinary case places the work at exactly the hours it wanted, as two blocks. **The cost that survives is narrow:** a task whose minimum chunk forbids the smaller piece — a 2h task with a 2h minimum straddling the seam — is unplaceable where a crossing block would have fitted. Whether that case ever arises depends on a chunk policy **nobody has set**, which is why deferring U4 to M3 was a legitimate option and was declined: the answer does not change with the policy, only the size of its cost does. **Vocabulary, because conflating three things cost a round trip:** *guardrails* **overlap** (two life areas may claim the same hours and compete for them, `D-life-area-owns-its-time`); *blocks* **never overlap** (invariant 1); and **adjacency** — two guardrails meeting at a seam — is a third thing, which is what U4 was actually about. |
| T-invariants-one-to-five | **The five invariants `#11` asserts by number, written down for the first time.** **1** No two blocks overlap — with each other, or with facts (`in_progress`, `completed`, `missed` blocks) or pins. **2** A block lies entirely within one allowed window. **3** Conservation under splitting: a placed task's chunks sum exactly to its estimate. **4** Every placed task with a hard deadline finishes at or before it; one that cannot is unplaceable, never placed late (`T-hard-refuses-soft-slips`). **5** The placed/unplaceable partition is total, every unplaceable task carrying a reason from the closed enum. | **Invariants 1, 3 and 4 had never existed anywhere in this project**, while #11's strongest acceptance criterion — *"Invariants 1–5 hold under `proptest`, ≥1000 cases"* — asserted all five by number. Recovered: 2 from U4 (#7), 5 from #11's own text. Reconstructed and offered on 2026-08-12 with the log saying *"do not build against these"*; ratified by the owner 2026-08-18 after being decomposed into the three product decisions they actually contained. **They cover where a block may be (1, 2), how much work survives (3), when it must finish (4), and that nothing is silently dropped (5).** Two corrections were made to the 2026-08-12 reconstruction while ratifying. **Invariant 1 was silent about `facts`:** `free_intervals` computes `mask − busy − pins − buffers`, which does not include them, yet `schedule()` takes `facts` as a separate input — so without naming them, the engine could place work on top of the block the owner is currently doing. **Invariant 4 was stated as "hard deadlines hold"**, which is true and is read as "deadlines hold", which is false — `T-hard-refuses-soft-slips` makes soft deadlines overrunnable by design. **The reason enum does not grow.** A committed task with no estimate cannot be scheduled, and there is no reason code for it — because the state cannot occur: estimates are required at triage from `0008`, and the owner's live database held **zero** committed tasks when this was measured, so no unestimated row exists or can be created. Every kind-conditional field on `tasks` is nullable in storage and enforced at the triage boundary; the estimate is not special, and guarding one of eight in the schema was rejected as inconsistent. **What is deliberately not an invariant:** *guardrails never yield*, *determinism* and *idempotence* are already separate acceptance criteria on #11 — an invariant here is a property a proptest can falsify from a `schedule()` output alone, and those three are not that shape. |
| T-fold-counts-both-passes | **SUPERSEDED 2026-08-23 by `D-context-tags-are-the-taxonomy`** — there is no guardrail band for a DST fold to repeat. **`T-timezone-is-a-setting` is still live and the hazard is not gone:** the first feature that projects a wall-clock rule across dates meets this again, and should be sent here rather than rediscovering it. **A repeated wall-clock hour inside a guardrail is free time twice.** At a fall-back fold, a band's start resolves to the **earlier** of its two real instants and its end to the **later**, so the interval spans the full repeated hour rather than picking one occurrence. Its mirror needs no separate rule: at a spring-forward gap the hour that does not exist is simply not counted. | Settled inside the `free-time` slice, 2026-08-18, answering #60's first open question — and recorded here because `T-jiff-epoch-millis` says a fold *"forces an explicit decision"*, and because **once made it is invisible in the numbers the function returns.** A 7h total on 7 November and 5h on 14 March against a 6h band look like arithmetic bugs to anyone who does not know this row exists. The argument is that the owner's own clock is the authority: at both instants of the repeated hour their clock reads a time inside the band, and the day really does have twenty-five hours, so counting it once would silently discard an hour they can actually work. **The gap needs no matching decision, which is the part worth understanding:** `Disambiguation::Earlier` and `::Later` resolve a gap identically — the civil reading that never existed slides to the nearest valid instant either way — so start and end both land on real instants, and because intervals are compared as **instants** rather than as civil minutes, the missing hour disappears on its own. That is `T-jiff-epoch-millis`'s *"store UTC epoch millis, convert at the boundary"* paying for itself: a naive civil subtraction would have reported 6h on every one of these days and hidden both transitions. **In UTC all three days are 6h**, which is the first observable consequence of `T-timezone-is-a-setting` and the reason its UTC default is loud rather than quiet. |
| T-free-time-horizon-fourteen-days | **SUPERSEDED 2026-08-23 by `D-context-tags-are-the-taxonomy`** — free time was removed with the guardrails it subtracted from. **The reasoning is still the interesting part:** fourteen days holds exactly two of every weekday whatever day it starts on, which is why the constant is not tunable. **The free-time horizon is fourteen days, a fixed constant** — exactly two weeks, so the window holds exactly two of every weekday whatever day it starts on. **`/stats`' fourteen is a coincidence** (a backward-looking measurement window for R2 — different direction, different purpose). **#62's capacity horizon is the same number by rule**, because its supply is the sum of these intervals. | Answers #60's second open question, and recorded because the project now has *three* fourteens and no way to tell which are load-bearing. Two weeks rather than "about a fortnight" is what makes every total in `free_time.feature` a fixed number instead of one that drifts with the day the suite happens to run — a horizon of thirteen or fifteen days holds three of some weekdays and two of others, so an identical guardrail would report a different total on a Tuesday than on a Friday, and every example would have to be computed rather than stated. **The distinction between the two kinds of fourteen is the point of the row:** changing `/stats`' window is an R2 decision about how long a ratio should average over, and changing this one silently changes #62's capacity answer. Anyone tempted to extract a shared `FOURTEEN_DAYS` constant should read this first — the numbers are equal and the concepts are not, which is the same trap `T-fact-plan-line` refereed for the word *layer*. |
| T-timezone-is-a-setting | **The owner's timezone is one stored value for the whole product, shown and changed on the life areas page — not a flag, not an env var, not a page of its own. It defaults to `UTC`, never to the host's zone.** | Settled inside the `guardrails` slice, 2026-08-18, resolving #59's longest-reach open question. `D-single-user` fixes it at one zone, not one per guardrail. **It is data the owner changes when they move, not deployment configuration** — a `--timezone` flag is cheapest and, as the brief put it, least honest, because it makes a fact about the person into a fact about how the process was started. A `/settings` page was rejected against `T-nav-is-the-site-map`: every route-table page earns a header link, and one field does not earn a permanent entry in a four-item nav. **The UTC default is the load-bearing half.** Reading the host's zone would be the friendlier-looking answer and is the wrong one: a guardrail authored as `09:00` would silently mean a different instant depending on where the server happened to boot, while the page went on displaying `09:00`. That is exactly the silent-wrong-default class `D-manual-triage-until-llm` refused for the life-area picker — *a default the user never chose is silently wrong* — and it is worse here, because the picker's wrong answer is visible on the row and this one is not visible anywhere. UTC is obviously wrong for most owners, which is the point: it is wrong **loudly**, and the fix is one field on a page they already use. **`settings` earns its own directory** rather than living in `platform` or on `life_areas`: `platform` is machinery, and this is owner-authored data of exactly the kind a life-area name is. It exposes `settings::current_timezone` as its front door (`T-one-front-door-per-capability`). |
| T-guardrail-well-formedness | **SUPERSEDED 2026-08-23 by `D-context-tags-are-the-taxonomy`** — there is no guardrail to be well-formed and no life area to save one against. **What survives is the shape of the rule:** overlap refused rather than merged, and *never scheduled* stated in a column rather than inferred from absence. Both are the same argument `T-archived-at-only` makes about one signal, and both would be re-decided the same way. **A life area's own bands may touch but may not overlap, and an overlap is refused rather than merged. "Never scheduled" is an explicit column, not the absence of bands. Both are decided when the owner saves that life area's guardrail, and the message lands on that row.** | Three answers from #59 that are one subject: what makes a life area's schedule configuration valid. **Overlap is refused, not merged**, because merging silently rewrites what the owner typed into something they did not type, in a product whose entire value is that they trust what it shows — and a rejection can name the conflict, where a merge leaves them to notice that `Mon 09:00–12:00` plus `Mon 11:00–17:00` quietly became one band. Touching is fine; the check lives in `scheduler-core` because it survives changing HTTP. Overlap *between* life areas stays legal and expected (`D-life-area-owns-its-time`). **"Never scheduled" is a column** because `T-life-areas-are-data`'s rule — a life area is well-formed only with a guardrail or an explicit mark — is unstateable if no-bands means both *deliberately has no hours* and *has not been set up yet*. `T-archived-at-only` argued against two fields for one state; these are genuinely two states, and the same call went the other way in `T-capture-leaves-inbox-once` a day earlier for the same reason: count the states first, then the columns. **The third answer was a question the brief never asked**, and it is the one that made the other two implementable: all five seeded life areas begin with neither a guardrail nor a mark, so "a life area that is neither is refused" cannot attach to the *state* — nothing would ever be creatable. It attaches to the **act**: each life area's guardrail editor has its own Save, and saving with neither is what is refused. A rule about a state that every row starts in is a rule that forbids the product from having rows. |
| T-422-is-product-wide | **`422` means exactly one thing everywhere in Trellis: a validation rejection whose body is the re-rendered fragment it failed against.** The htmx override that makes it swappable lives once, in `base.html`, so every page present and future carries it. **An endpoint that cannot honour that contract must not return 422.** | Widens `T-forms-swap-one-fragment`, which scoped the override to *"every htmx request on [the inbox page], present and future"* and named that as its accepted cost. The scoping did not survive contact: `life_areas.html` copied the line — **without the comment explaining it** — the moment a second page grew a form, so the "one page" boundary was already fiction and the rule was living in two places with one explanation. A rule kept in two copies has two chances to drift, and the copy without the reasoning is the one that drifts first. **This is a widening, not a move, and the honest part is what it costs:** `stats.html` had no forms and did not load htmx at all, and now loads both. The obligation `T-forms-swap-one-fragment` described as page-local is now global — every endpoint this product will ever have owes the 422 contract, including ones with no fragment to re-render. Accepted because the alternative is per-page opt-in, which is a convention nothing checks and which had already failed once by omission of a comment. Asserted on all three pages rather than left incidental. **The interaction to remember:** this is why navigation uses plain `<a href>` rather than `hx-boost` (`T-nav-is-the-site-map`) — a boosted navigation to an endpoint answering 422 would have that body swapped into the DOM, so keeping navigation and fragment-swapping as separate mechanisms is what keeps this override's blast radius knowable. |
| T-nav-is-the-site-map | **Every page in the route table gets a header link. The nav is the site map, not a curated shortlist.** Navigation is plain `<a href>` full page loads, never `hx-boost`. The current page is marked with `aria-current="page"` — markup, not styling. | Settled with the owner during the `app-shell` slice, 2026-08-17, and recorded as a **rule rather than an instance** because the question recurs on a schedule: `/stats` raised it, #59's guardrails page and #62's capacity page each raise it again, and a per-page judgment call is a decision re-litigated four times with no principle accumulating. The tempting alternative — *the nav lists places you work, instruments live elsewhere* — requires deciding what an instrument is, and `/stats` and a capacity view are both read-only numbers that the owner is nonetheless expected to act on. **A page reachable only by typing its URL is a page that does not exist**, which is the finding this slice came from: three pages, no links, and the product got less navigable with every one added because every slice was judged alone. **On the duplication, kept deliberately:** `platform::nav::Page::path` and `platform::app::build_app` name the same paths independently, so renaming a route leaves the header offering a link that 404s. Unifying them was rejected — `.route(Page::Inbox.path(), …)` costs the route table its readability as the shortest statement of what this server does — so they stay two statements and a test walks `nav::ALL` through the real router to make them agree. **The test walks the list rather than restating it**, which is the difference between covering the pages that exist and covering the pages someone remembered to add; the acceptance feature's `Examples` table names today's three by hand and would miss page five. |
| T-capture-leaves-inbox-once | **A capture leaves the inbox exactly once, and one column records it.** `captures.triaged_at` is renamed `left_inbox_at` (migration `0005`) because it now means "left the inbox", full stop. There is no `dismissed_at`. **Which exit it took is derivable** — a `tasks` row references it, or it does not. Both exits ask whether the capture is still open first, so a second triage is refused exactly as a triage-then-dismiss is. | Resolves #48's first open question, settled inside the pipeline 2026-08-17. Two nullable timestamps would permit a row both triaged and dismissed — the precise shape `T-archived-at-only` was written against: *"two fields for one state … every path gets two chances to set one and forget the other,"* leaving a row alive on whichever surface filters the field that was missed. **One column forbids the impossible state outright rather than policing it with a `CHECK`**, and a discriminator column would only restate a fact the `tasks` foreign key already carries. The specification had settled this the other way — two columns plus a `CHECK` — having costed the single-column shape as a full `captures` table rebuild; `ALTER TABLE … RENAME COLUMN` made it one line, and the argument was reopened on the better facts rather than the shape being kept because it was already written. **The exactly-once rule is wider than the brief asked** and is the part worth keeping: the brief asked only that a capture never be both triaged *and* dismissed, but both handlers must ask "is this capture still open?" regardless, so the general rule is *less* code than two asymmetric guards. It also closed a real defect nothing had noticed — `POST /captures/{id}/triage` twice previously wrote **two task rows for one capture**, because triage had no already-triaged check at all. A rule stated generally caught a case a rule stated narrowly would have left open. |
| T-inbox-owns-membership | **The inbox owns whether a capture is still in it, and taking one out** — `inbox::capture_is_open`, `inbox::close_capture`, and the single message both exits report. `inbox::lists` is private and the membership queries are `pub(super)`, so reaching past the door does not compile. `platform/boundary.rs` gains a fourth check: **no capability names another capability's `store` in production.** | `T-one-front-door-per-capability` was recorded on 2026-08-16 from the life-areas slice and was, until now, a rule nothing checked — which is the failure `T-module-boundary` named against itself (*"a layering rule nothing checks is a comment"*). The failure it catches here was not hypothetical: `dismiss` arrived with its own `store` holding a **byte-identical copy** of `triage`'s is-this-capture-open query and its `left_inbox_at` write, each defensible alone under `T-capability-owns-its-queries`. That is the seam where "a capability owns the SQL it issues" degenerates into every capability owning a copy of the same SQL — and the answer is not to weaken query ownership but to notice that **membership is the inbox's question**, not the asker's. `dismiss/store.rs` was deleted outright: dismissal issues no SQL of its own. **The check is production-only, deliberately.** A test that sets up "a triaged capture" by calling the capability that writes one is the *right* fixture — retyping its `INSERT` would be a second copy of the schema, the worse failure — so the gate strips `#[cfg(test)]` before scanning, as it already did for the SQL rule. **What it does not do:** the other capabilities still declare `pub mod store`, so the compiler permits what the gate forbids. The gate is a substring scan and architecture.md is explicit that it is *"a lint, not a proof"*; closing the compiler half is tracked as #63. |
| T-capacity-two-axes | **SUPERSEDED 2026-08-23 by `D-context-tags-are-the-taxonomy`** — capacity was removed and context tags cannot be consumed from. **Quota is the nearest live concept and it is deliberately one axis, not two:** hours against a weekly target, with no notion of whose hours were spent. **Capacity is consumed from the guardrail occupied and attributed to the task's life area. Two numbers, both reported.** *"Work: 5h of 8h used — 2h of that is Learning you allowed in."* | Resolves #6 (C5), settled 2026-08-17. The contradiction was real and the spec picked neither horn: charge a borrowed hour to the **task's** life area and the availability number lies — Work reports 22h free while its hours are physically full, and the owner over-commits and discovers it Thursday at 4pm, which is the exact failure the capacity pass exists to prevent. Charge it to the **guardrail** and the life-area number lies — you did two hours of Learning and the weekly review reports none. They are different questions: *how much of this wall is left* is about the clock, *how much Learning did I do* is about the work. One number cannot answer both, and both have a consumer — the capacity view (M2) needs the first, `D-quota-no-rollover`'s reckoning and M8's per-life-area report need the second. The second clause is also the useful one in practice: it is what tells the owner **why** Work hours are full when the Work tasks alone do not explain it. Two things fall out of existing decisions and are not separately settled here: quota demand **counts** toward capacity (`T-three-task-kinds` — *"capacity math is false in the same way C5 makes the Work number false if quota demand is uncounted"*), and pool consumes **nothing**, because pool is never placed (`D-no-pool-on-calendar`). M2's acceptance criterion 4 was already written against this proposal; it is now ratified rather than assumed. |
| T-life-area-required-at-triage | **SUPERSEDED 2026-08-20 by `D-context-tags-are-the-taxonomy`** — the requirement is dropped. ~~Every task names a life area at triage — all three kinds, no exceptions.** `tasks.life_area_id` is nullable in storage only because SQLite cannot add a `NOT NULL` column without a default to a table that may hold rows. `None` means "written before migration `0004`", never "has no life area". | Direct application of `T-quota-targets-required`: a column's nullability is a storage fact, not a triage-time permission. A task in no life area is one no guardrail governs and no capacity number counts — the failure `T-life-areas-are-data` names as *a life area outside the walls is covered by no wall*, arriving from the task side instead of the life-area side. Requiring it costs nothing before M2 and closes the hole before a real orphan row can exist. The cost landed immediately and visibly: five already-green QA fixtures broke on the new required field, each confirmed as fixture drift rather than regression. **Kind is still checked first** (`T-core-owns-validation-order`), so a submission wrong about both reports the kind, leaving every existing rejection scenario's outcome unchanged. Paired with `D-manual-triage-until-llm`, which is what makes the requirement honest: required *and* silently pre-filled would be a field the user never actually chooses. |
| T-collation-enforces-name-identity | **TRANSLATED 2026-08-23 to context tags, and the mechanism changed with it.** `migrations/0010_context_tags.sql` carries the collation forward by name — *"the same argument `life_areas.name` made before #88, applied here to something typed rather than picked, where the slip is likelier"* — but **drops the `UNIQUE`**, because a tag is not a managed row a second capture already has; many captures share one tag by design. **The consequential half reversed:** with no unique row to be the canonical spelling, choosing which spelling wins moved out of the schema and into `capture::resolve_tag`, which is exactly the *"function in `scheduler-core`"* shape this decision originally rejected — and `T-cross-capability-invariants-need-an-owner` is the bill for it, because `pool::group`'s plain string equality is correct only while that function runs on every write path. **The original reasoning was right and the new model could not keep it**, which is worth more than either half alone. **Two life-area names are the same name once trimmed and case-folded, and `life_areas.name UNIQUE COLLATE NOCASE` is where that is enforced** — not a function in `scheduler-core`. Lookups match through the same collation. | The slice shipped `scheduler_core::life_area::same_name` stating the rule, with seven unit tests and **no caller anywhere in the tree**; production went through the column. A second statement of a rule that nothing calls is one nothing keeps honest — its tests would have passed forever while the real behaviour lived elsewhere, which is `T-module-boundary`'s *"a layering rule nothing checks is a comment"* with the polarity reversed: here the check is real and the *description* is the dead thing. Deleted rather than wired in, because a constraint the database enforces cannot be bypassed by a write path that forgot to call something, and "Work" and "work" as two indistinguishable picker entries is exactly what the rule exists to prevent. Two consequences, recorded because neither is visible from the Rust: `T-sqlite-sqlx`'s "mechanical" Postgres move must carry the collation across (`CITEXT`, or a functional unique index), and if the rule ever outgrows a collation — Unicode folding — it moves into the core and the constraint becomes the backstop. `parse_name` stays in the core, because trimming and rejecting blank is decidable from the string alone with no database. |
| T-one-front-door-per-capability | **Example translated 2026-08-23:** `life_areas::active_options` no longer exists. The live front doors are `capture::resolve_tag`, `capture::distinct_tags`, `inbox::render_lists` and `mark_done::mark_task_done` — each one function in a `mod.rs`, each hiding a query composed with a mapping. **The rule is unchanged and is now enforced**, per `platform/boundary.rs` and `T-inbox-owns-membership`. **A capability that other capabilities read exposes one function for it, in its `mod.rs`.** Three callers needed the pickable life areas; they call `life_areas::active_options`, never `store::list_active` composed with `LifeAreaOption::from`. Reaching across for a *type* stays fine; reaching across for the *recipe* does not. | The management page, the inbox fragment whose triage forms carry the picker, and a quick-added capture rendering that same row had each composed the query with the mapping for itself. A caller that must know which query *and* which mapping to combine is holding a copy of another capability's internals, and three copies drift. Same move as `http::view`, `steps/payloads.rs`, `steps/app_client.rs` and `http::lists` — when a second caller appears, the shared thing gets its own home — applied one level up, to a capability's public surface rather than to a helper. **This is the complement to `T-capability-owns-its-queries`**, and the two are only safe together: that decision says a capability owns the SQL it issues, and without this one "own your own queries" degenerates into every capability hand-assembling every other capability's internals, which is the sideways dependency it was written to prevent. |
| T-core-owns-validation-order | **RESTS ON A REMOVED EXAMPLE, 2026-08-23.** `WellFormedTriage` and `unknown_life_area` are both gone — a triage submission no longer names a life area at all (`T-life-area-required-at-triage`, superseded). **The rule stands and is why it is worth keeping:** an ordering rule living in each adapter is one the second adapter gets wrong, and this product still runs more than one transport over one triage path. `T-required-fields-are-specified-per-transport` is the live neighbour to read alongside it. **When validation has a required order, the core composes it and returns one result; the adapter keeps only the part that genuinely needs something the core cannot have.** `WellFormedTriage::from_fields` decides kind first and then that *some* life area was named; `unknown_life_area` stays the adapter's rejection, because resolving a name against the table needs a database. | Triage validation arrived as two core functions an adapter had to call in a fixed order — kind first, so a submission naming neither reports `unknown_kind` rather than a life-area complaint — with `require_life_area` public so each adapter could get the sequence right on its own. **A rule every delivery mechanism has to remember for itself is one the second delivery mechanism gets wrong.** This product already runs two transports over one triage path (JSON and the page), and `T-capture-surfaces` adds a third at M7. The invariant #33 spent a property test pinning — *the page and the endpoint are one code path, not two* — is precisely what an ordering rule living in each adapter quietly breaks, and it breaks it in the way that still compiles and still passes every example test that does not exercise both failures at once. `T-module-boundary`'s inward-pointing rule at the granularity of a call sequence: the order is a rule, so it lives where the rules live. |
| T-ephemeral-view-state-rides-the-request | **A view choice the owner made with a thumb rides along with the request that needs it and is then forgotten; only a durable consequence of a deliberate act earns a column.** Which trips are expanded is read live off the DOM by an `htmx:configRequest` hook, appended to every request `#pool-body` issues as `expanded=<tags>`, and echoed straight back into the rendered fragment. Nothing is stored, and a fresh page load starts collapsed. **The general rule underneath it:** *the tier you assert in decides what the implementation must store*, so **choose the tier from the nature of the state, not the state from the tier you happen to be asserting in.* Ephemeral view state is asserted in the browser tier; a durable one may be asserted over HTTP. | Settled inside `trip-controls` (#120, #125, PR #135), and it is the **corrected** answer to the question `disclosures` (#119, PR #126) got wrong eight days earlier. That slice reasoned: *the acceptance suite speaks only HTTP; to make the assertion true over HTTP the state had to be server-rendered; to be server-rendered across a request it had to be stored* — and bought `captures.shown_kind`, **a permanent, append-only column for a display preference**, migration `0012`. The reasoning is valid and the conclusion is still wrong, because **the premise was a choice**: two tiers that could have held the assertion already existed and neither was used — `scripts/qa/phone_layout.cjs` drives real Chrome under a CI gate, and `base.html` has loaded htmx and an inline script on every page since #58. **`cleared_at` (#122, migration `0013`) is the contrast that makes the line real** — clearing a trip is a deliberate act with a consequence that must survive a reload, so it earned its column on the same test this one fails. The cost of getting it wrong is asymmetric and permanent: `T-migrations-append-only` means a column bought for a display preference can never be taken back, only added to. | 

**Renumbered on merge, then superseded.** This branch allocated numeric IDs that `trunk` had already given to other decisions, and its source comments were left citing the stale numbers. Both problems are gone: decisions are keyed by slug now, and the citations were migrated with a CI gate behind them. Kept as the record of why.

## Rejected

| # | Rejected | Why |
|---|---|---|
| R-guardrail-override | Any guardrail override, however well-guarded. | See D-guardrails-never-yield. |
| R-browsable-archive | A browsable archive UI. | See D-kill-means-archive. |
| R-pool-on-calendar | Pool tasks on the calendar in any form. | See D-no-pool-on-calendar. |
| R-auto-promote-on-age | Automatic promotion of pool → committed on aging. | A task the system unilaterally puts on the calendar because it is old is a task the user ignores, and ignored blocks destroy calendar credibility. Age raises menu ranking and flags in review; the decision stays with the user. |
| R-multi-tenancy | **DEFERRED, NOT REFUSED — clarified by the owner 2026-08-23.** Multi-tenancy, accounts, auth, RLS. **"Not a never; right now I want single tenancy because I'm the only tenant."** | Single user. **The one-line form of this row read as permanent and was taken that way**, including by this log's own summaries — hence the clarification. **What it costs to reverse, measured 2026-08-23 so the decision is priced rather than guessed:** 36 queries across 8 `store.rs` files and 3 live tables (`captures`, `tasks`, `settings`). Migrations are append-only, so it is additive columns plus a backfill, and **`platform/boundary.rs` already forbids SQL outside a `store.rs`, so the set of places needing a tenant predicate is enumerable** — that gate could be extended to fail any query lacking one. **The code is the small half.** SQLite has no row-level security, so every query becomes wrong-by-default and one missed predicate leaks another person's data. **And the security boundary inverts.** This row rejects in-app auth *because* the tailnet is the entire boundary; other tenants mean a public surface — TLS, abuse, rate limiting, other people's backups, uptime obligations, a privacy posture. Cloudflare cannot host Trellis (Workers cannot run axum, Containers have ephemeral disk, D1 is a rewrite), so it would need a real host with real operational duty. **One-time-password email sign-in on its own is rejected for a different reason:** it is the cheap part — a token table, a session cookie, an email sender, no password hashing or reset flow — but **while there is one user the tailnet already authenticates them**, since only the owner's devices can reach the service. It earns its keep only alongside multi-tenancy, never before it. |
| R-collaboration | Collaboration, shared calendars, delegation, colleague-visible busy time. | Task blocks are advisory to the user, authoritative to the system. |
| R-plugin-surface | A plugin or general-purpose extensibility surface. | |
| R-supabase | Supabase (self-hosted or otherwise). | See T-sqlite-sqlx. If Postgres is ever genuinely needed, run plain Postgres. |
| R-gcal-webhooks | Google Calendar push webhooks for v1. | `watch` channels need a public HTTPS endpoint, which fights self-hosting. Poll every 60–120s; a tunnel can be added later if instant reaction is wanted. |
| R-incremental-patching | Incremental patching of the plan / mutation in place. | The scheduler is recomputed from scratch on every trigger. Same inputs, same output. |

---

## Version notes

### 2026-08-12 — Initial log

Extracted from the settled product brief. Board created, milestones M0–M9 plus
spike S3 opened.

**Source-of-truth split adopted:** the board holds state, milestones and epic
issues hold acceptance criteria, `docs/design/brief.md` holds architecture, and
this file holds rationale. No overlap, so nothing needs syncing. A
`docs/roadmap/` document was drafted and removed as duplicative of the board —
note this departs from the `swarmforge/roles/PM.prompt` convention, which
expects a roadmap document under `docs/roadmap/`; the departure is deliberate.

**Spec corrections raised against the brief, awaiting decision** (C1–C5, tracked
as issues): pins belong in the Constraints layer rather than as a bool on Block;
the "delete every block and regenerate" property is false as written and needs
the fact/plan line drawn inside the Block table; `Block::missed` is unreachable
under D-silence-means-done; auto-close undo is unimplementable without storing pre-close
`remaining_minutes`; and per-domain capacity accounting is incoherent with
`allowed_windows`.

### 2026-08-12 — M1 schema decisions settled

#1, C1, C4 and N1 settled in session, unblocking M1. Recorded as `T-three-task-kinds`, `T-pins-in-constraints`, `T-auto-close-event`,
`T-archived-at-only` and `D-quota-no-rollover`. C2, C3, C5 and the remaining U-series points stay open.

**M1's blocker list is now empty**, but not because all four were answered in
M1's favour — two of them turned out to belong downstream:

- **C1 → M3.** Pins have no M1 behaviour. There is no `Block` to drop `pinned`
  from until M3, no scheduler to consume a pin, and no calendar drag to create
  one until M6. The contradiction C1 identifies is entirely between M3's
  signature and M6's drag handling.
- **C4 → M6.** An event table is additive, so it lands with auto-close rather
  than with the task schema.

Both were deferred on the same principle: **a schema element with no observable
behaviour has nothing to specify against.** The specifier's Gherkin describes
externally visible behaviour, so a table nothing reads or writes yet cannot be
covered by an acceptance test or by an end-to-end QA suite — QA has no user
interface affordance to drive it. Adding such a table early buys nothing and
commits to a shape before the behaviour that would validate it exists.

**M1 epic (#9) needs rewording accordingly.** Its acceptance criterion 7 —
"Schema includes `remaining_minutes_at_auto_close` (C4), `pin` as its own table
(C1), `archived_at` as the single archive signal (N1)" — should retain only the
`archived_at` clause. M6 and M3 pick up the other two.

**One gap left open deliberately:** `archived_at` also has no M1 write path.
Nothing at M1 archives a task — capture *dismissal* stamps the capture row, not
a task, and every genuine archive route (decay pass, three-strike, closing the
review) arrives at M8. The column is specified at M1 because triage creates the
row it lives on, but its *behaviour* — an archived task appearing in none of
menu, pool or review — is M8's proptest to own.

**Two questions C1 raises that its issue does not answer**, needed before M3:

1. A pin binds `task_id` to one interval, but M3 splits tasks into chunks. For a
   6h task split across three days, which chunk does the pin bind — and chunks
   have no identity to refer to. Or does pinning suppress splitting entirely?
2. Pins have no defined death. Does a pin outlive its deadline? Can the user
   clear one? A forgotten pin is an invisible constraint that makes the
   scheduler look broken, and the infeasibility report's closed reason enum
   (`no_window`, `capacity_exceeded`, `deadline_unreachable`,
   `chunk_policy_unsatisfiable`) has no code for "a stale pin is in the way".

### 2026-08-12 — M1 layering

Recorded as T-module-boundary and T-unknown-kind-rejected. The M1 triage slice worked, but all of it lived in the
axum handlers: `scheduler-core` was three lines of doc comment, and the crate
purity rule T-core-no-tokio guards was guarding an empty room.

**One externally visible behaviour change**, called out because it is not a
refactor: triaging with a `kind` outside `pool | committed | quota` — including
omitting `kind` entirely — now returns `422 {"unknown_kind": <submitted>}`
instead of `201` with the arbitrary string written to `tasks.kind`. See T-unknown-kind-rejected for
why this was treated as a schema-integrity hole rather than behaviour worth
preserving. Everything the acceptance criteria *do* specify is unchanged,
including the `{"missing_field": ...}` rejection body and the order the three
committed fields are reported in.

**What the sum type bought immediately.** "A pool task has no deadline" and "a
quota task has no deadline" were previously assertions about the payload a test
happened to send; the handler would have stored a deadline on a pool task had
one been supplied. They are now properties of the type, and
`crates/scheduler-core/tests/task_properties.rs` asserts them against arbitrary
inputs rather than the two example rows.

**Known-red gate, untouched and pre-existing:** `scripts/analyzers/complexity.sh`
reports three violations, all in `crates/acceptance-tests/src/steps/`
(`capture::dispatch` 14, `triage::dispatch` 17, `when_triaged` 9) against T-complexity-8's
threshold of 8. Verified identical before and after this change. These are step
dispatchers — regex chains where every arm is a one-line delegation — so T-complexity-8's
"extract the logic that is not the match" does not straightforwardly apply, and
the fix is more likely to be splitting the step modules by Gherkin phase than
flattening anything. Left alone deliberately rather than absorbed into a
layering change.

### 2026-08-12 — Delivery shape, and the vocabulary gap

**D-visible-slices changes what a slice is.** Every pipeline slice now ends in something the
owner can run and see. This is a resequencing, not new scope: M1's remaining
work is unchanged, but it is cut so that each merge is observable rather than
grouped by component. M1's story 2 — "Untriaged queue UI" — was already in
scope and simply had not been sliced yet.

The constraint that forced the point: at the time of writing the application
serves **two routes, both POST, both JSON**, no `askama` dependency and no
templates. There is no GET route. Trellis cannot be opened in a browser at all;
the only way to observe it is `curl`. Under the previous milestone cut that
remained true until roughly M6.

**T-fact-plan-line settles the layer vocabulary** (C2, #3). Worth recording why it sat open
so long: T-pins-in-constraints was *settled* while standing on C2's *unratified* proposal, and
M3's acceptance criteria (#11) were written in vocabulary that nothing in the
repo defined. Every agent reading "Plan layer" was inferring it.

**Still missing, and now the largest documented gap:** `docs/design/brief.md`.
Nineteen issues link to it; it has never existed. The specific casualty is
**invariants 1–5**, asserted by number in #11's strongest acceptance criterion
("Invariants 1-5 hold under `proptest` … >= 1000 cases"). Only two are
described anywhere in the repo or the tracker:

- **Invariant 2** — a block lies entirely within **one** allowed window
  (recovered from U4).
- **Invariant 5** — the placed/unplaceable partition is **total**, every
  unplaceable task carrying a reason from the closed enum (recovered from #11
  AC-6).

1, 3 and 4 exist nowhere. Reconstruction offered and awaiting ratification:
non-overlap; conservation under splitting; hard deadlines hold. Until they are
written down M3 cannot be specified, because the specifier cannot write Gherkin
for a criterion whose terms are undefined.

### 2026-08-12 — triage-validation open questions settled

Issue #29's three open questions settled with the user before specification.
Recorded as `T-quota-targets-required`, `T-empty-equals-absent` and
`T-period-closed-set`. (They carried numeric IDs at the time, and were
renumbered once on merge to dodge a collision — see "Former numbering".)

- **Quota target required at triage (`T-quota-targets-required`).** T-three-task-kinds's nullable columns were a
  schema-sharing fact, not a triage-time permission; leaving target fields
  optional at triage would have let a quota row exist that M8 can never
  schedule and D-quota-no-rollover's reckoning can never report on.
- **Empty and absent report identically (`T-empty-equals-absent`).** Both remain
  `{"missing_field": <name>}`. This also settles the empty-string half of the
  defect issue #29 raised against `require()` — the fix is one path, not two.
- **`period` closed to `week | month` (`T-period-closed-set`).** Matches how `deadline_type` and
  `priority` are closed in the same slice, for the same D-guardrails-never-yield reason: M8's cadence
  math cannot branch on an unvalidated string.

### 2026-08-12 — triage-validation harness seams and the complexity gate

The triage-validation slice landed the closed domains in `scheduler-core`,
which is where T-module-boundary says they belong — the layering held with no correction
needed. The architectural work this round was in the acceptance harness, which
had grown two missing seams.

**The definition of a valid submission had no home.** Closing four field
domains meant every "…is rejected because X is wrong" scenario needed a
payload valid in every respect *except* X. Written inline, the canonical
committed payload became a literal repeated twelve times across five step
modules, so closing one more field's domain would mean finding and editing
every copy. `steps/payloads.rs` now holds the canonical payload per kind plus
`with_field`/`without_field`, and a scenario states only what it varies.

**Nothing knew how a scenario reaches the application.** `capture` and
`triage` had each grown a copy of "build the router from the world's pool,
POST, record the outcome", and the copies had already drifted: one timed the
round trip and discarded the body, the other kept the body and discarded the
timing — so a step could assert only what its own module happened to record.
`steps/app_client.rs` returns status, body and elapsed together.

DRY: 3.26% → 2.66%, back under the 3% threshold it had crossed. *(Corrected
2026-08-13: this line read 2.62%, a transcription slip; the architect's own
commit message has 2.66%, which is what re-measurement confirms.)*

**On the complexity gate, deliberately not "fixed".** Three violations stand,
all pure regex dispatch chains: `steps/capture.rs::dispatch` (14),
`steps/triage.rs::dispatch` (17), `steps/mod.rs::dispatch` (9). *(Corrected
2026-08-13: this read "three violations **remain**", which implies all three
predate the slice. Two do. `steps/mod.rs::dispatch` is **introduced here** —
5 on the base, 9 after adding four step modules to the top-level dispatcher.
The total held at three only because `when_triaged` dropped off in the same
change. The argument below is unaffected; the framing hid an added violation,
which is exactly how a red gate gets waived on the next read.)* Every arm is a
one-line delegation, so by T-complexity-8's own rule — *a function over 8 is carrying
logic that is not the match; extract that, do not flatten the match* — there
is nothing to extract. The only way to move the number is a
`(Regex, handler)` table, and because the handlers are `async` with differing
arities, a uniform table in Rust needs a boxed-future wrapper function per
step: roughly forty wrappers to replace forty one-line branches, which is
exactly the "indirection that is strictly worse to read" T-complexity-8 refuses. **Do not
flatten these into a registry to make the number go down.** If they are ever
worth changing it should be for a cohesion reason — splitting a step module by
Gherkin phase — not for the metric.

The distinction is visible in this slice: `steps/triage.rs::when_triaged` also
scored 9 and *did* come off the list, because it was genuinely doing two
things (driving HTTP, and recording onto `World`) and separating them was
worth doing on its own merits. The number moved as a side effect of an
improvement, which is the only reason it should ever move.

**Property coverage doubled, 6 → 12**, all in
`crates/scheduler-core/tests/task_properties.rs`: every value inside a closed
domain round-trips; anything outside one is rejected as invalid naming that
field; a deadline naming no real instant is rejected (T-jiff-epoch-millis); empty and absent
produce the *identical* rejection (T-empty-equals-absent, asserted as an equality between the
two outcomes rather than against a fixed expectation, so it survives a change
to either); a missing field is reported before an invalid one; and equivalent
textual spellings of one instant store one deadline. The last three were
checked against deliberate breakages of `require()`, of the check ordering in
`committed_from`, and confirmed to fail — a property that cannot fail is not
coverage.

### 2026-08-13 — Decision-ID collision on merge

`triage-validation` and `trunk` independently allocated **T-quota-targets-required**. The slice
branched from `a0889b4` before trunk's entry existed, so both took what was
correctly the next free number at the time; neither side erred.

Resolved by renumbering **trunk's** decision — the fact/plan layer model, C2/#3,
now `T-fact-plan-line` — and leaving the slice's three (now
`T-quota-targets-required`, `T-empty-equals-absent`, `T-period-closed-set`)
untouched. That direction was chosen purely
on blast radius: the slice's numbers are cited in nine places in `crates/`
(`scheduler-core/src/task.rs`, `task_properties.rs`, two step modules), while
trunk's had three references, all in documentation. Renumbering the cheaper side
kept a merge fix out of product code.

**The underlying problem is unfixed:** the log has no ID allocation mechanism,
so any two concurrent branches will collide again the moment both add a
decision. Options if it recurs — allocate IDs only at merge time, prefix them
per branch, or drop sequential numbering for dated slugs. Not worth solving
until it costs more than this did.

*It recurred within hours, and cost more. Sequential numbering was dropped the
same day — see "Decisions are keyed by slug" below.*

### 2026-08-13 — Migration discipline, and two corrections

**T-migrations-append-only** makes editing an applied migration a CI failure (#32). Recorded because
the near-miss was invisible: `triage-validation` shipped an in-place edit to
`0002_tasks.sql`, and it cost nothing only because no database existed at that
moment. Under D-visible-slices that luck is gone.

Two corrections to the `triage-validation` entries above, both found by
re-measuring the claims rather than reading them:

- The DRY figure read **2.62%**; it is **2.66%**. Transcription slip — the
  architect's commit message had it right.
- "Three violations **remain**" implied all three predate the slice. Two do;
  `steps/mod.rs::dispatch` went 5 → 9 *in* that slice. Corrected to "stand".

Neither changes an argument, and both are small. They are recorded rather than
quietly patched because the previous slice's log entry contained a claim that
was simply false — "verified identical before and after" when the count had
gone 1 → 3 — and the habit worth building is that measurable assertions in this
file get measured.

### 2026-08-13 — Capture categorization folded into the classifier trait

A brainstorming session designed LLM auto-categorization for captures, then
found mid-design that it overlapped M9. Reconciled as **T-classifier-covers-domain**: one trait, two
implementations, output grown to carry `domain` and `title`. Source:
`docs/plans/2026-08-13-capture-categorization-handoff.md`.

Two corrections to that proposal, applied here rather than inherited:

- **The invocation-timing question it left open is partly a false conflict.**
  It framed "trait implies synchronous" against "background poller" as a
  design fork. A trait describes swappability, not call timing; a worker can
  call a synchronous `classify()`. The real constraint is *where*, and that is
  already settled by evidence — `capture_endpoint.feature` asserts a 50ms
  budget on `POST /captures`, which no LLM round trip fits inside, and
  blocking triage puts the wait in front of the user at the moment they are
  waiting. Recorded in T-classifier-covers-domain as the worker filling fields between capture and
  triage.
- **The provisional domain list contradicts this log**, and that is *not*
  settled — raised as **#36**. The proposal lists Work, Health, Home, Learning,
  Social. This file already names Fitness (T-three-task-kinds), Learning (D-kill-means-archive), Family (D-single-user)
  and Work. Two different sets of five, and nobody noticed because the domain
  set has never been written down in one place. Which is the same root cause
  as the missing invariants: `docs/design/brief.md` does not exist, so the
  vocabulary lives in whichever entry happened to mention it.

#36 also surfaces a harder question the proposal states as a settled
constraint: capture domains as extensible plain data versus `Domain` as a
5-variant enum that T-complexity-8's complexity threshold is derived from. One concept or
two is a real decision, and it needs making before the M1 classifier story is
specified.

### 2026-08-13 — inbox-view: the first page, and where its data comes from

The slice that finally made Trellis openable in a browser also introduced the
first template, and with it the first chance to get the delivery side of T-module-boundary's
module boundary wrong. It very nearly did. Recorded as **T-templates-take-view-models** — this branch's
own history gave it a number `trunk` had already spent, and its source comments
cited the stale numbering; see "Decision-ID collision on merge" above. Both are
moot now that decisions are keyed by slug and a CI gate checks the citations.

**What the templates were rendering.** `InboxTemplate` held
`Vec<store::capture::UntriagedCapture>` and `capture_row.html` read
`capture.raw_text` — so the HTML was bound to a `sqlx::FromRow` struct and, by
extension, to the columns `list_untriaged` happens to select. The store type's
own doc comment gave the direction away: "a capture as the inbox view needs
it". Persistence was being described in terms of a page.

The decisive evidence was in `create_capture`, not the inbox. To hand the
quick-add box back the new row's markup, it constructed an `UntriagedCapture`
by hand from the submitted text — a database row shape, for a capture it had
just written and deliberately not read back, existing only because the
template's type demanded it. A struct being fabricated to satisfy a renderer
has stopped being a row.

`http::view` now holds the view models. It depends on nothing, so a page's
data can be built and rendered without a database; handlers do the mapping,
one line each. `UntriagedCapture` keeps its name and goes back to describing
the query.

This is small on purpose — one field, one line of mapping. The point is not
the size of today's duplication but the direction of tomorrow's: this is the
first of many pages, and the alternative precedent is templates bound to the
schema by M6.

**Property coverage** gained the inbox's projection invariant
(`store::capture`, `#[ignore]`d per convention): for any queue and any triaged
subset of it, `list_untriaged` returns *exactly* the untriaged captures in
*exactly* newest-first order. The three example tests sample two captures and
one triaged one; the property pins both halves of the query. Checked by
breaking `ORDER BY id DESC` to `ASC` and by dropping the `WHERE` — each fails
the property, neither fails every example test.

**Complexity is unchanged at 4 violations**, all still the regex dispatch
chains covered by the 2026-08-12 entry; `steps/inbox_view.rs::dispatch` is the
new one and is the same shape. The reasoning there stands: nothing to extract,
and a table would cost ~40 boxed-future wrappers. DRY sits at 2.0%.

### 2026-08-13 — Decisions are keyed by slug; open questions are issues

Two failures in two days made the numbering untenable. `T17` was claimed
independently by `trunk` and by the `triage-validation` branch, which forked
before the other existed. Then `inbox-view` renumbered its source comments to a
scheme that had never been settled, leaving every citation in
`scheduler-core/src/task.rs` off by one — and because each wrong number
resolved to a *real* decision, the comments read as correct. A dangling
reference announces itself; a reference to the wrong true thing does not.

The cause is structural: an integer allocated on a branch is a distributed
counter with no coordinator, and branch order is not merge order. Renaming is
not a fix, it is the next collision deferred.

**Two changes.** Settled decisions are keyed by content-derived slug, so two
branches cannot silently pick the same key and nothing is ever renumbered.
Open questions leave this file for the tracker — they were duplicated between
the Open table and issues #4, #6, #7 and #36 already, which is the same
duplicated state the project rejected when it deleted `docs/roadmap/`.

Rejected alternative: move settled decisions to GitHub issues too, which would
also make collisions impossible since issue numbers are server-allocated. It
loses more than it buys. Every agent reads this file at startup, from a
worktree, often offline; the rationale should ship with the clone. And the
"Rejected" section works precisely because someone about to re-propose a
refused idea trips over the reasoning while reading — a closed issue has to be
searched for by someone who already suspects it exists.

Enforcement matters more than the convention: CI fails if a slug cited under
`crates/` is absent from this file. The off-by-one above was undetectable;
under the gate it is a failed build. Same shape as
`T-migrations-append-only` — the rule is only real once something checks it.

### 2026-08-13 — An architecture reference, assembled rather than written

`docs/design/architecture.md` now exists. It is a vocabulary reference, not the
product brief: type shapes, the two unrelated things called "layers", the
`schedule()` signature, the closed enums, and an index of what is still
undefined.

Nothing in it is new. Every entry was recovered from code, migrations, this log
or an epic's acceptance criteria, and each is marked **built**, **specified** or
**GAP** so a reader knows whether they are looking at something verifiable
today, something fixed by decision but unwritten, or a hole. The "built" rows
were checked against the tree rather than transcribed.

It exists because two failures this week had the same cause. Nineteen issues
link to `docs/design/brief.md`, which has never existed, so the vocabulary lived
in whichever decision happened to need an example — which is how the project
came to hold two different sets of five domains (#36) without anyone noticing,
and how "Invariants 1-5" came to be asserted by number in M3's strongest
acceptance criterion while three of them are described nowhere.

The brief itself is still absent and still the owner's to supply. This does not
replace it. It replaces the *dangling reference* to it.

### 2026-08-13 — triage-from-page: a payload nobody could use, and a shared fragment

The slice extended the delivery module correctly — `view::CaptureRow` grew the
`id` and `error` that `T-templates-take-view-models` predicted it would need,
and `store::TaskWithCaptureText` stayed a row rather than becoming a second
view model. Two things wanted straightening.

**`TriageRejection::UnknownKind` carried a payload no adapter could use.**
Both consumers destructured it as `_` and reported `kind_submitted` from the
request instead, with a comment explaining that the core's copy is not the one
to trust: JSON `{"kind": 7}` reaches the core as `None`, because reading it as
a string is what turned it into one. So the core was carrying a value that is
never better than the adapter's and sometimes strictly worse, and the only
documentation it had was a warning not to use it. It is now a unit variant.
The core says *this kind is not one of the three*; what arrived is the
adapter's to report, because the adapter is the only place it still exists.

A sum-type payload every consumer ignores is not extra information, it is a
second source of truth that loses. Behaviour is unchanged — both responses
were already built from the request.

**The `#lists` fragment now lives beside its two renderers, not inside one.**
`ListsTemplate` and `build_lists` sat in `http::inbox`, so the triage endpoint
depended on the inbox *page* to render the fragment they share. `http::lists`
holds both; `inbox` and `triage` are now siblings over it. Same move as
`view`, `payloads` and `app_client`: when a second caller appears, the shared
thing gets its own home rather than one caller reaching into the other.

**Property coverage** gained the invariant this slice's own brief asserts —
"the page and `POST /captures/{id}/triage` are one code path, not two". That
is now two hand-written, field-by-field translations into the same
`TriageFields`, which is exactly where the claim can quietly stop being true:
add a field to the core, wire it into one transport, and it still compiles and
still passes every example test that does not use that field. The property
submits one arbitrary submission through both and requires the results to be
identical. Checked by dropping `period` from the form mapping alone — it
fails, shrinking to the single field.

**Wording repair, called out rather than made quietly:** `T-module-boundary`
opened with "Three layers", written before `T-fact-plan-line` reserved *layer*
for the Plan/Constraints domain split. Changed to "Three modules". The
substance is untouched; the row was contradicting the naming rule that
supersedes it, in the one document agents consult to learn the vocabulary.

**Complexity is 5**, all still regex dispatch chains in the acceptance step
modules — `triage_from_page::dispatch` is the new one, same shape. The
2026-08-12 entry's reasoning stands. DRY 1.96%, CRAP 0.

### 2026-08-13 — The toolchain pin that was not a pin

`rust-toolchain.toml` said `channel = "stable"` and had said so since the M0
skeleton commit, untouched since. That is a floating channel: it pins nothing.
`README.md` and the CI workflow both described it as a pin — the CI step is
literally named "Install the toolchain pinned by rust-toolchain.toml" — and the
comment block above that step reasons carefully, and correctly, about rustup's
short-circuit behaviour when installing a pin that was never there.

T-toolchain-pinned closes it: an exact version, with the components and musl
target that CI had been adding by hand moved into the same file.

This is the third instance this week of the same failure, and worth naming as a
shape: **a description that outruns the thing it describes**. Nineteen issues
linked to a `docs/design/brief.md` that never existed; "Invariants 1-5" were
cited by number while three were written down nowhere; and a file called a pin
floated. In each case the prose was confident, load-bearing, and read by people
who then reasoned from it — which is what makes it worse than silence. Nobody
plans around a gap they can see.

The gate added with it follows `T-migrations-append-only` and the decision-slug
checker: CI compares `rustc --version` against the channel in the file and
fails if they differ. The pin is now the kind of claim that cannot quietly stop
being true.

### 2026-08-14 — Two decisions the repo had been reasoning without

Both recorded as `T-package-by-business-domain` and `T-life-areas-are-data`.
Neither is new information about the product; both are things the owner already
believed and the repo had never been told.

**The word collision is the story of the session.** The PM opened by asking the
owner to settle #36 — which five *domains*, meaning life areas: Work, Fitness,
Learning, Family. The owner answered about *packaging the codebase by domain*,
meaning business capabilities. One word, two meanings, and a full round trip
spent before either party noticed they were discussing different subjects.

That is the third time this project has paid for an overloaded word in a week.
`T-fact-plan-line` had to referee "layer", reserving it for the Plan/Constraints
split and demoting `T-module-boundary`'s code split to "the module boundary",
explicitly because *"a `Block` row is otherwise Plan-layer and store-layer at
once and the word stops carrying information."* The same fix is applied one word
over: **bare "domain" is a life area; "business domain", both words, is the
packaging axis.** Written into `docs/design/architecture.md` next to the "layer"
entry, since that file exists to hold exactly this.

**On the packaging rule.** Worth being honest that this reopens a settled
decision. `T-module-boundary` was argued well and its dependency rule was right;
what it lacked was any knowledge that the owner wanted the tree to scream its
intent, because nobody had written that down. The rule survives, the directory
shape does not, and #24 closes with a fourth option its own list never
contained. The trap is recorded in the decision itself: the boundary tests glob
a directory that is about to stop existing, and they will pass while covering
nothing.

**On life areas, a correction to the argument this log made.** #36 argued the
enum was the safe answer by analogy with the closed sets — and the PM repeated
that argument twice before the owner's constraint ("I don't want to compile and
run the app just to create a new life area") exposed it as a bad analogy. The
closed sets are all fields the scheduler *branches* on. A life area is indexed,
never matched. The distinction was available the whole time and nobody drew it,
which is how a good precedent gets over-applied: `T-three-task-kinds`,
`T-unknown-kind-rejected` and `T-period-closed-set` were each right, and their
shared conclusion was mistaken for a rule about sets in general.

Two claims made against the enum's removal turned out to be false and are
retracted here rather than left standing in #36. `T-complexity-8` does **not**
lose its derivation — `BlockState` still has five variants and carries it alone.
And the enum was never the thing guarding against a life area with no guardrail;
a non-exhaustive `match` cannot express that. The guardrail-completeness rule in
`T-life-areas-are-data` is the check that was actually wanted, and it wants no
compiler.

Making the list editable from the running app — rather than from a config file,
which was the PM's recommendation — is also what makes it *specifiable at M1*.
A config file would have left the entity with no observable behaviour, and this
log has already ruled that such a thing has nothing to specify against. The
management surface is the behaviour. Tracked as #47; #36 closes against it.

### 2026-08-14 — Three delivery patterns that were settled in code and nowhere else

Recorded as `T-forms-swap-one-fragment`, retroactively, covering decisions the
`triage-from-page` slice made and did not write down: forms inline in the row,
one shared fragment per page region, and rejection as a 422 re-rendering that
same fragment.

The brief had asked for exactly these, by name, and said why they mattered —
*"whichever you pick becomes the pattern for every form in this product"* and
*"say what you chose; it is the first error-display pattern in the product."*
The slice chose well. It answered in a commit message and a template, and the
decisions log — the file every agent reads at startup, from a worktree — got the
architect's module-placement note and nothing about the three patterns every
future page will inherit.

This is the inverse of the failure named on 2026-08-13. That entry described **a
description that outruns the thing it describes**: prose confidently asserting a
brief, a set of invariants, a pin, none of which existed. This is the same gap
from the other side — the thing exists, is correct, is load-bearing, and has no
description at all. Both produce an agent reasoning from something that is not
there. The second is easier to miss, because nothing dangles: a reader who opens
`capture_row.html` sees the pattern and assumes it was decided somewhere.

The specific item worth having written down is the 422 configuration. It is a
one-line global override of htmx's response handling, it applies to every
request the page will ever make, and it is invisible from any of the handlers
that depend on it. That is not a detail a future slice should have to
rediscover by breaking it.

**Process note for the PM role:** these were caught by comparing what the brief
asked to be decided against what the log received, after the pull request had
already merged. Doing that comparison at the pipeline-available notification —
before the merge, while the slice's authors are still reachable — is cheaper and
is what the role's own step 5 asks for.

### 2026-08-14 — The tree now names the product (#44, PR #49)

`ls crates/trellis-server/src/` reads `capture inbox platform triage`. It read
`http store` this morning.

**`T-capability-owns-its-queries` is the decision that made it real**, and it is
recorded above as its own slug because it is the one a future contributor will
argue with. Splitting `store/capture.rs` three ways looks like scattering
persistence; the argument for it is that the alternative is the old `store/`
directory wearing a capability's name. Worth restating the tell: under
table-shaped ownership, `triage` and `inbox` would both depend on `capture` for
persistence, so the arrows would run sideways between capabilities instead of
inward — which is `T-module-boundary`'s rule broken by a change made in its
name.

**`platform/` is named to say "not a capability" out loud.** Three subjects and
one honest bucket beats promoting `db` or `web` to a subject. It holds the
composition root, the connection, the clock, the static assets, the shared
response mapping, and the boundary gate.

**The gate replaced itself properly, which was the risk this change carried.**
`T-module-boundary` shipped a check that globbed `src/store/*.rs`, and this
restructure dissolved that directory — so the two tests would not have failed,
they would have stopped covering anything. That is the same rule dying quietly
that `T-module-boundary` wrote its own warning about. `platform/boundary.rs`
walks `src/` rather than naming a directory, puts a floor under every assertion
so a walk that returns nothing fails instead of passing, and covers its own
source file — the SQL needle is assembled from two halves at runtime rather than
kept on an exception list, because an exception list is the one place a real
violation can hide. It was verified by breaking it four ways and watching each
fail, including recreating `src/store/` and renaming every `store.rs` away while
still compiling: the vacuous-coverage case the old tests died of.

It also gained the half the old check never had — nothing outside a `store.rs`
or `platform/db.rs` writes production SQL — and it gates the packaging rule
itself, so an `http/` or `store/` reappearing at the crate root is a failing
test rather than a review comment.

**#24 closes with this.** Its three options were all layer-shaped: split
`scheduler-db` out, accept the collapse, or enforce a module rule. None of them
considered organising by capability, which is the answer. The brief's remaining
role-named crates (`scheduler-web`, `scheduler-bot`, `scheduler-bin`) are
rejected as a plan by the same reasoning; a crate earns existence when a
capability needs a real boundary, not because a layer has a name. Renaming the
crates that do exist is deliberately excluded: `scheduler-core` is named twice
in `swarmforge/constitution/articles/stack.prompt`, which no agent may edit
without the owner's direction.

**Measured, unchanged:** complexity 5 violations (the same `acceptance-tests`
dispatch chains covered by the 2026-08-12 entry, identical scores), CRAP 0,
lint 0, coverage 96.41%, acceptance 11/11, `cargo test --workspace` 204 passing.
No `.feature` file, no migration, no template, no crate name and nothing under
`swarmforge/` was touched, and the route table is byte-identical — this changed
where code lives and nothing about what the product does.

**DRY moved 1.61–1.74% → 2.15%** (threshold 3), and most of that is not code.
Six of seven new clone pairs are jscpd matching *prose* between `README.md` and
`docs/design/architecture.md`; one is real, a nine-line test preamble now shared
by two `store.rs` files, which is `T-capability-owns-its-queries`' accepted cost
showing up exactly where it should. The gate scans markdown because
`scripts/analyzers/dry.sh` restricts no formats — raised as **#50**. Every DRY
figure recorded in this file (3.26%, 2.66%, 2.0%, 1.96%) was measured the same
way, so none of them is comparable to what a Rust-only gate will report; the
correcting line goes in when #50 establishes the new baseline. Noted here rather
than quietly, per the habit this log adopted on 2026-08-13: measurable
assertions in this file get measured.

### 2026-08-14 — The complexity gate's first catch, and the merge that caused it (#55)

`trunk` went red 24 seconds after PR #53 added
`scripts/ci/complexity_baseline.sh`, when PR #54 landed the `stats_ratio` step
module on a green check that had been measured against a tree without the gate
in it. Both pull requests were correct alone and both were green alone.

**A gate added by one pull request is not in force for a pull request branched
before it.** #54's green check was an honest statement about the tree #54 was
built on and said nothing about the tree it was merging into. Whichever of two
such branches merges second has to rebase, re-measure and regenerate — the
interaction was predicted on both PRs before either merged, and nothing in the
merge process asked for it.

**The gate caught two different things, and only one of them was a baseline
edit.**

`steps/mod.rs::dispatch` 11 → 12 is one more step module contributing one more
one-line arm: the *"a dispatch chain gaining branches is a new step"* case the
gate's own guidance names. The number is updated and no code changed.

`steps/stats_ratio.rs::dispatch` at 17 was not that. Its ten regex arms are the
legitimate shape the other five baselined entries share, but three of them
unwrapped an `example_value` result before delegating, and each of those
two-armed `match`es costs 2 — so six of the seventeen points were conditionals
that are not the dispatch. That is `T-complexity-8` verbatim: *a function over 8
is carrying logic that is not the match — extract that, do not flatten the
match*. The resolution moved into two handlers (`then_page_shows_example`,
shared by the two steps that assert the resolved example value verbatim, and
`then_reports_in_window`, which builds its phrase the way `then_reports_counts`
already did). Every arm is now a one-line delegation and `dispatch` measures
**17 → 11**, which is the figure recorded in the baseline. Neither new handler
is over the threshold (2 each), and no other score moved.

Recording 17 would have made the gate's first use the precedent that a number
nobody wants to move is a number you write down instead — on a file whose whole
value is that its rows were defended. The distinction is the one the 2026-08-12
entry drew with `when_triaged`: *the number moved as a side effect of an
improvement, which is the only reason it should ever move.*

**Measured:** complexity 6 over the threshold (the five dispatch chains of the
2026-08-12 entry plus `stats_ratio`, all pinned and matching the baseline
exactly), CRAP 0, lint 0, coverage 96.71%, DRY 2.03% → 1.96% (one clone pair
fewer, 29 → 28), acceptance 12/12, `cargo test --workspace` 250 passing. No
product code, no `.feature` file, no migration, no crate name and nothing under
`swarmforge/` was touched — this is an extraction inside a test-support module
plus a baseline file.

The gate itself was re-verified rather than assumed, the way #53 was accepted: a
new over-threshold function, one unwrap re-inlined so a pinned score moved
11 → 13, and a baseline row renamed so it covered nothing. Each failed, each
named the right function, and each was reverted.

### 2026-08-16 — life-areas: four decisions the slice made, and did not record

`#47` merged as **PR #57**. `T-life-areas-are-data` settled *that* life areas are
user-managed rows; the slice had to settle four things about *how*, and it
settled all four well. It recorded them in commit messages, in
`docs/design/architecture.md`, and in code. **This file got nothing.**

Recorded now as `T-life-area-required-at-triage`,
`T-collation-enforces-name-identity`, `T-one-front-door-per-capability` and
`T-core-owns-validation-order`.

**This is the second consecutive slice to do it, and the failure was already
named.** The 2026-08-14 entry recorded three `triage-from-page` patterns
retroactively and closed with a process note addressed to this role: *"these
were caught by comparing what the brief asked to be decided against what the
log received, after the pull request had already merged. Doing that comparison
at the pipeline-available notification — before the merge, while the slice's
authors are still reachable — is cheaper."* That comparison was not done at the
notification this time either. The brief for `life-areas` asked four numbered
open questions; all four were answered; none reached this file. **The gap is
not that agents fail to decide — they decide well. It is that the brief poses
the questions and nothing checks that the answers come home.**

Worth noting what the four have in common, because it is not the feature: three
of them are about *where a rule lives* — in the column, behind one function, in
the core rather than in each adapter. That is the same subject
`T-module-boundary`, `T-capability-owns-its-queries` and
`T-templates-take-view-models` keep circling, and it is what this codebase is
apparently still deciding.

**The one that will be re-argued** is `T-collation-enforces-name-identity`,
because deleting a core function with seven passing unit tests reads as removing
coverage. It is the opposite: the tests covered a function production never
called, so they would have gone green forever while the real rule lived in a
column nobody had written down. The cleaner spotted it and correctly refused to
delete it as cleanup, flagging it for an architectural call instead — which is
the handoff working.

**A testing-practice finding worth not rediscovering.** The hardener found that
`life-areas-duplicate-03` could never be killed by Gherkin acceptance mutation:
its Examples table varied `"Work"/"work"/"WORK"`, and the mutator's only string
mutation is a single-character case flip — so **the mutation and the property
under test shared one axis**, and no choice of example values could have made it
observable. Split into three literal scenarios, one per casing. The general
shape: *a scenario asserting invariance under exactly the transformation the
mutator applies is untestable by that mutator, and looks fully covered.*

**Measured:** 14 acceptance features (was 12), 100% mutation kill on every
touched file, CRAP 0, DRY 2.71% (threshold 3), complexity baseline matching
exactly, `scheduler-core` still free of tokio and sqlx.

### 2026-08-17 — No classifier until the LLM; triage is manual

Recorded as `D-manual-triage-until-llm`. The owner cancelled M1's keyword
classifier outright — *"hand classification or nothing"* — and with it the
silent default on the life-area picker.

**This reverses the M1 half of a settled decision, so it is argued here rather
than done quietly**, per this file's own header. The reasoning is in the row;
the short form is that a keyword classifier's wrong guess is *accepted*, while a
blank field is *filled* — `D-inaction-archives` one domain over.

Three consequences, in the order they bite:

- **M1's acceptance criterion 5 leaves the milestone.** With it goes story S4,
  which never had an issue. **M1's remaining work is `#48` alone.**
- **`captures` gains no classification columns.** S4 was to have added
  `captures.life_area`; a schema element with no observable behaviour has
  nothing to specify against, which this log settled on 2026-08-12.
- **M9 (`#19`) shrinks too**, which was not obvious. Its criterion *"falls back
  to the keyword implementation with no capture lost"* assumed a keyword
  implementation exists. It becomes *falls back to empty fields and manual
  triage* — degrading to the shipped product rather than to an unvalidated
  second classifier — and M9's "classifier trait + keyword impl refactor" story
  is deleted.

**`#9`'s AC-2 is amended a second time**, on the same grounds as 2026-08-14. It
read *"a single action from the inbox, with no intermediate form"*; a manual
life-area pick makes it two interactions. The literal count goes, the property
stays: **pool must not require expanding a form, and must cost strictly fewer
inputs than committed or quota.** Today that is 2 against 6. The count was a
proxy for friction on the default path the first time it was amended, and it is
a proxy again — `D-pool-is-default` requires pool to be cheapest, not to be one
click.

**The cost is R2's, and it is real.** `#20` calls the committed:pool ratio the
highest-leverage counter in the product and names the remedy for a bad one as
*"triage defaults and classifier bias"*. Only triage defaults exist until M9.
`/stats` has been counting since `#45` and the >50% alarm still fires, so the
experiment narrows rather than stops — but if the ratio comes back wrong before
M9, half the planned response will not be available.

### 2026-08-17 — #6 settled, and the question it never asked

Recorded as `T-capacity-two-axes` (the question #6 asks) and
`D-life-area-owns-its-time` (the one it depends on and never states). **M2 is
unblocked.**

**#6's own question turned out to be the easy half**, and its acceptance
criterion had already been written against the answer — M2 AC-4 says *"reports
minutes per window and separately attributes them per domain (C5)"*. So the
board said "blocked pending decision" while the epic had quietly committed to
one. That is worth noticing as a pattern rather than a one-off: **an epic's
acceptance criteria can encode a decision the log has not made**, and then the
blocker looks open while the work is already shaped. It was ratified on its
merits, not inherited.

**The half nobody had asked was what a guardrail is attached to.** #6 places a
Learning task in windows `{Work, Personal}` — but "Personal" is not a life area,
and never was; the seed is Work · Fitness · Learning · Family · Home. #6 predates
`T-life-areas-are-data` and was using the superseded vocabulary, which made it
read as though two separate lists had been settled somewhere. They had not.
Once life areas became unbounded user data, *"a life area is well-formed only
once it has a guardrail"* stopped being a note and became a constraint on what
M2 builds — with no answer to "so when may side-project work be scheduled?"

**The resolution's whole weight is on one line:** reservation is the default,
sharing is opt-in per task. Two settled decisions demand opposite things of a
guardrail — `D-guardrails-never-yield` wants **containment** (work must not
breach the wall), `T-three-task-kinds` wants **reservation** (*"a wall
protecting an empty room"*). A mask owned by one life area gives containment
outright; making that ownership the default gives reservation **with no
protection rule at all**, because Fitness's 06:00 is safe exactly when nothing
else has claimed it. That is the same move `T-life-areas-are-data` made when it
replaced an enum with an invariant — the structure does the work a rule would
otherwise have to.

**Vocabulary, before M2 writes it into code.** "Window" is the fourth overloaded
word in this project, after "layer" (refereed by `T-fact-plan-line`) and
"domain" (refereed by `T-package-by-business-domain`, after a full round trip in
which the PM and the owner discussed different subjects without noticing). It is
**already** ambiguous inside M2's own acceptance criteria, which use it for a
guardrail mask in `free_intervals(window, range)` and for a rolling time span in
"14-day capacity view" — and `/stats` uses it for a time span too. Fixed in
`docs/design/architecture.md` next to the other two: **guardrail** is a life
area's mask, **range** is a span of time, and bare "window" is not used. The
signature becomes `free_intervals(guardrail, range)`.

**What this does not settle**, and both still block M3: **U4** — whether a block
may cross a guardrail boundary — which #7 recommends keeping as invariant 2
states it, and which now has a sharper cost, since overlapping guardrails make
adjacent-boundary cases common rather than rare. And **invariants 1, 3 and 4**,
still described nowhere, still asserted by number in #11's strongest criterion.

### 2026-08-17 — dismiss-capture (#48, PR #64), recorded before the merge

Recorded as `T-capture-leaves-inbox-once` and `T-inbox-owns-membership`.
**M1 closes when PR #64 merges.**

**The process note from 2026-08-14 was finally honoured.** That entry asked this
role to compare what a brief asked to be decided against what the log received
*at the pipeline-available notification, before the merge, while the slice's
authors are still reachable* — and then the next two slices did it after the
fact anyway. This one was done at the notification. The slice made it easy: its
pull request answers all four of the brief's open questions in a table, with
reasoning, which is exactly what the brief asked for and what `life-areas` and
`triage-from-page` each left in commit messages.

**The question it got right by changing its mind.** The specification settled
the impossible-state question on two columns plus a `CHECK`, having costed the
single-column shape as a full `captures` rebuild — SQLite has no `ALTER COLUMN`,
so that estimate was reasonable. `ALTER TABLE … RENAME COLUMN` made it one line.
The shape was reopened on better facts rather than kept because it was already
specified, and the argument that replaced it is stronger: *which* exit a capture
took is derivable from whether a `tasks` row references it, so a second column
would restate a fact the foreign key already carries.

**A defect nobody had reported, found by stating a rule generally.** The brief
asked only that a capture never be both triaged *and* dismissed. The slice
shipped *a capture leaves the inbox exactly once*, because both handlers have to
ask the same question anyway — and that caught `POST /captures/{id}/triage`
twice writing **two task rows for one capture**. Triage had never had an
already-triaged check. The narrow rule would have left it open.

**`T-one-front-door-per-capability` stopped being a comment.** It was recorded
on 2026-08-16 with nothing checking it, and the very next slice tried to break
it in the most defensible way available: `dismiss` arrived with its own `store`
carrying a byte-identical copy of `triage`'s membership query, which
`T-capability-owns-its-queries` appears to license. The resolution — membership
is the *inbox's* question, and `dismiss/store.rs` deletes entirely — is the one
that keeps both rules true at once. `platform/boundary.rs`'s fourth check is
what stops the third exit making a third copy.

**Two things to watch, neither blocking.**

- `capture-endpoint-persists-quickly-01` asserts a **50 ms** wall-clock budget
  and failed at 624 ms under concurrent load, then 120–144 ms, then passed at
  0.22 s three runs running. A budget that tight is load-sensitive by
  construction and this is not the slice that made it so. Raised as **#66**.
- The DRY figure is 2.82% against a threshold of 3, its highest since the gate
  started measuring code rather than prose (#50). Not a violation; worth
  noticing before the next slice adds to it.

**Measured:** 15 acceptance features (14 baseline + `dismiss_capture`), QA 15/15,
`cargo test --workspace` green, 100% language-mutation kill on every touched
file, Gherkin mutation 10/10 · 6/6 · 2/2, CRAP 0, coverage 97.04%, complexity
baseline recorded with reasons, core purity holds.

### 2026-08-17 — app-shell (#58, PR #67): the product gets a frame

Recorded as `T-422-is-product-wide` and `T-nav-is-the-site-map`. **M1's three
pages are reachable from each other for the first time**; M2's four arrive
reachable rather than adding to a debt.

**The finding this slice came from is about how slices are judged, not about
navigation.** `D-visible-slices` asks whether the owner can run a slice and
watch the new behaviour happen, and all three page slices passed *individually*
while the product got **less** navigable with each one — because no slice was
responsible for the whole. The third page made it worse than the second, and
nothing was positioned to notice. A per-slice test cannot see a property that
only degrades across slices.

**`T-forms-swap-one-fragment`'s scoping did not survive its first test, and the
way it failed is the useful part.** It recorded the 422 override as page-local
and named that as its accepted cost. When `life_areas` grew a form, the line was
copied — **without the comment explaining it**. So the boundary was already
fiction, the rule already lived in two places, and the copy that would drift
first was the one carrying no reasoning. Widened deliberately rather than
patched: the obligation is now product-wide, `stats.html` loads htmx it never
needed, and an endpoint that cannot honour *422 means "validation rejection,
body is the re-rendered fragment"* must not return 422.

**A duplication kept on purpose, which is rarer here than removing one.**
`nav::Page::path` and `app::build_app` name the same three paths
independently. Unifying them would cost the route table its readability as the
shortest statement of what the server does, so they stay two statements and a
test walks `nav::ALL` through the real router. **The test walks rather than
restates** — the acceptance feature's `Examples` table names today's three
pages by hand and would miss page five, which is precisely the failure the
brief's open question 2 predicted.

**Where the answers landed, and the pattern that is now working.** PR #64
established answering a brief's open questions in the pull-request body, in a
table, with reasoning. #67 did the same and the architect additionally moved
them into `docs/design/architecture.md`. Three consecutive slices had answered
only in commit messages; two consecutive have now answered in a form this file
could take directly. **The remaining gap is that neither slice wrote here** —
the rationale and the rejected alternatives still arrive via this role at the
notification. That is working, and it is one person deep.

**A coverage gap the slice stated rather than hid:** three facts in its QA
walkthrough are browser-only — that navigation is a full page load, that the
back button walks the pages, and that the header does not flicker or duplicate
when a fragment swaps. The first two follow by construction from a static check
that the links carry no `hx-*` attribute; the third is covered by nothing and
needs eyes on a browser. This project has no browser automation and has said so
since `inbox_view`; the honest position is that visual regressions are
uncovered, not that they are unlikely.

**Measured:** 16 acceptance features (15 baseline + `app_shell`), QA 16/16,
`cargo test --workspace` green, 100% language-mutation kill on every touched
file, Gherkin mutation 12/12, CRAP 0, **DRY 2.82% → 2.78%** — down, because the
hoisted 422 line removed a real duplicate. No migration; this slice writes
nothing.

### 2026-08-18 — guardrails (#59, PR #69): Trellis learns what "when" means

Recorded as `T-timezone-is-a-setting` and `T-guardrail-well-formedness`. **M2
slice 1 of 4.** `T-life-areas-are-data`'s promise from 2026-08-14 — *"a life
area is well-formed only once it has a guardrail, or is explicitly marked
pool-only — enforced at M2, when guardrails exist"* — is now enforced.

**A fifth question, which the brief did not ask and which the other answers
depended on.** All five seeded life areas start with neither a guardrail nor a
mark, so *"a life area that is neither is refused"* cannot attach to the state:
nothing would ever be creatable. It attaches to the **act** — each life area's
guardrail editor has its own Save, and saving with neither is what is refused.
Worth noticing as a shape: **a well-formedness rule that every row starts in
violation of is a rule that forbids the product from having rows.** The brief
carried this rule for four days across two issues and an epic without anyone
spotting that it had no subject.

**A label was wrong in a way that was nobody's error.** The owner read
"pool-only" as *this life area accepts only pool tasks*, which is a perfectly
available reading and false — pool is the default kind and every life area takes
all three. It means *this life area has no hours, so its work is never placed
and only ever surfaces in the menu*. The page now reads **"never scheduled —
menu only"**; `pool_only` stays the column and `pool-only` the slug here, with
both the feature header and `qa/guardrails.md` recording that they are one
concept. The QA doc carries an explicit instruction **not** to test that such a
life area rejects committed tasks — **the misreading written down so that it
does not get implemented by someone who arrives at it independently.** That is
worth copying: a plausible-but-wrong reading, once found, is cheaper to record
than to re-refute.

**The testing finding, and it generalises well beyond this slice.** Gherkin
acceptance mutation surfaced twelve survivors, all in scenarios the specifier
wrote, in two shapes:

- Two scenarios asserted only *"rejected, and still shows no guardrail"*. The
  implementation collapses an unparseable time, an `end <= start`, and a missing
  weekday into that one outcome — so a mutated Examples value was still
  rejected, for a different reason, with identical observable results. **No
  value in either table was ever under test.** The general rule: *when an
  implementation collapses several causes into one outcome, a scenario that
  asserts only the outcome puts none of its Examples values under test.* The
  stronger fix is to assert the **reason**; the hardener took the other valid
  route and wrote literal scenarios.
- Two more echoed the same `<zone>` placeholder in both the input and the
  assertion — the `task_kinds.feature` anti-pattern the brief explicitly warned
  against — and compounded it, since `jiff` resolves zone names
  case-insensitively, so the mutator's case flip was a no-op by construction.
  Same axis collision as `life-areas-duplicate-03` on 2026-08-16.

The specifier reported all of this against its own work in the pull request,
including the stronger fix it had missed. That is the third consecutive slice to
answer its brief's questions in the pull-request body, and the first to volunteer
a weakness a reader would not otherwise have found.

**`settings` earns a directory.** A seventh top-level name is a real cost under
`T-package-by-business-domain`, and the argument for it holds: `platform` is
machinery, and a timezone the owner edits is owner-authored data of exactly the
kind a life-area name is. One front door, `settings::current_timezone`.

**Watch this, and it got sharper an hour after it was written.** **DRY is 2.97%
against a threshold of 3** — 0.03 of headroom, the tightest this project has
run, and #60 adds a page, an algebra and a step module. The next slice should
expect to cross it and be told so rather than discovering it at the gate.

**And the gate that measured it was fail-open until PR #68 landed.**
`scripts/analyzers/dry.sh` discarded jscpd's exit status with `|| true`, so a
missing report read as *no duplication found* and exited 0 — every way jscpd
could fail to run was indistinguishable from a clean tree, and a misspelled
format reported **0.00%**, which reads as an improvement rather than an absent
measurement. The recorded percentages in this file are real, because they came
from runs that produced numbers; what was never true is that a **green** DRY
check meant the subject had been measured.

**PR #69 branched before #68**, so its DRY green was measured by the old gate.
That is the 2026-08-14 pattern verbatim — *a gate added by one pull request is
not in force for a pull request branched before it* — and it is the second time
this project has hit it. With 0.03 of headroom and a newly strict gate, #69
should rebase and re-measure before merging rather than after.

**Measured:** 18 acceptance features (16 baseline + `guardrails` +
`timezone_setting`), QA 18/18, `cargo test --workspace` green, 100%
language-mutation kill after four fixes, CRAP 0, fmt/clippy/core purity pass,
#66 did not fire. Migration `0006`. Two acceptance criteria are deliberately
absent from the Gherkin with the reason in the feature header — restart
persistence (acceptance runs in-process) and an archived life area keeping its
guardrail (it renders nowhere, so only QA's read-only `sqlite3` can witness it).

### 2026-08-18 — free-time (#60, PR #70): the milestone's hard part, and a loop closing

Recorded as `T-fold-counts-both-passes` and `T-free-time-horizon-fourteen-days`.
**M2 slice 2 of 4.** `free_intervals(guardrail, range)` exists, with a 1000-case
proptest behind it.

**The specifier asked for the `T-` row rather than leaving it to be found.** Its
handoff note read *"fold decision still needs a T- row"*, and its pull request
said so at length: the decision had landed in a doc comment stating, in its own
words, that *"the fold decision lives here, and only here"* — accurate, and the
problem. **That comment also said "Recorded in `docs/decisions.md`", which was a
forward reference to something that did not exist**, which is the 2026-08-13
failure — *a description that outruns the thing it describes* — in miniature and
caught by its own author.

This is the loop that has been closing for five slices. The 2026-08-14 process
note asked this role to compare what a brief asked to be decided against what
the log received. #64 started answering in the pull-request body. #59's
architect moved answers into the architecture reference. **#60 went further and
told the PM which row was missing.** The gap is no longer discovery; it is
allocation, which is a much cheaper thing to owe.

**The architect found a defect the specifier's scenarios could not have caught,
and the specifier said so.** `life_areas::guardrails` — the front door this
slice added — handed out a life area's bands **regardless of its `pool_only`
column**, defended in its own doc comment by the claim that *"a pool-only life
area's `bands` is always empty"*. Nothing has forbidden that combination since
#59, and `life_area_row.html` checks `pool_only` first, so the management page
hid the bands and the claim looked true. Measured against the running server:
mark Work never-scheduled, give it `Mon+Tue 09:00–17:00`, and `/free-time`
reported **32h for a life area the owner had marked never scheduled.**

The lesson is about test *ordering*, not coverage:
`free-time-empty-is-an-answer-03` marks a life area never-scheduled from a clean
state, so it reports 0h whether the rule is honoured or not. **Catching this
needed a life area that has bands *and then* is marked** — a sequence the
scenario did not have. A scenario that reaches the right end state by the wrong
path can be green against an implementation that does not enforce the rule at
all. Carried into #61 as an acceptance scenario rather than added here, so the
hardener's freshly-stamped mutation manifest does not go stale.

**A doc comment defending a rule with a claim about state is the shape to
watch.** *"`bands` is always empty for a pool-only life area"* was a claim
nothing enforced, in the one place a reader would look for enforcement.

**The DRY gate bit, exactly as #52 predicted and #68 made possible.** QA's own
additions took the aggregate from 2.98% to **3.03%**, tripping the now
fail-closed threshold. Before #68 that would have been a red build only if
jscpd happened to run; the fail-open would have passed it on any hiccup.
Resolved by generalising two row-block finders into one helper and factoring the
DST scenarios' shared setup — landing at **2.93%**, tighter than the slice
started. **The gate produced a real improvement rather than a waiver**, which is
the first time this project can say that about DRY.

**`app_shell` changed, and the inverted diagnostic is the good part.**
`T-nav-is-the-site-map` puts every route-table page in the header, so adding one
necessarily changes what `app_shell`'s per-page scenarios assert — 17 features
untouched and 1 legitimately changed, not 18 untouched. The slice wrote the
inversion into the feature and the QA doc: **if `app_shell` had *not* needed
changing, the page is missing from `nav::ALL`** — which compiles and ships with
no link. A test whose *failure to change* is the alarm.

**Gherkin acceptance mutation: 0 survivors**, against #59's twelve two days
earlier, on a specification written knowing what those twelve were.

**#66 fired for real, with numbers.** `capture-endpoint-persists-quickly-01`
failed at **612 ms against its 50 ms budget at load average 15.5**, from other
agents' mutation runs; 19/19 on a quiet machine, reproduced independently by the
hardener. That is no longer a suspicion about a load-sensitive assertion, it is
a measurement, and it is on the issue.

**Measured:** 19 acceptance features, QA 19/19, `cargo test --workspace` green,
property suite green including the 1000-case `free_intervals` proptest, 100%
language-mutation kill after one real gap closed, CRAP 0, DRY 2.93%, core purity
holds. No migration — this slice computes and renders, and stores nothing.

### 2026-08-18 — exceptions (#61, PR #71): the first subtrahend, and a brief that was wrong

Recorded as `T-availability-only-subtracts` and
`T-configuration-is-removed-work-is-archived`. **M2 slice 3 of 4.** Migration
`0007`; 20 acceptance features.

**The brief's demo did not work, and the slice fixed it rather than shipping
it.** It said: mark 20–24 August away, watch Work's fortnight drop from 80h to
40h. **20 August 2026 is a Thursday**, so that range is Thu–Fri–Sat–Sun–Mon and
removes **three** weekdays, not five — 56h, not 40h. The slice pinned today to
Monday 17 August so the fourteen-day horizon holds exactly ten weekdays, and
moved the exception to 24–28 August. Same headline, dates that produce it.

Recorded because this file's own habit is that **measurable assertions get
measured**, and the PM's are not exempt. It is also the second unforced error
from this role in two days, after `Raised, not fixed: #66` auto-closed an issue:
both are cases of writing a specific claim without checking it, in a document
other agents then build against. **A demo with dates in it is a computation, and
it should be run before it is handed out.**

**Four Gherkin mutants survived, and the variant is new.** Two `Examples` rows
in `exceptions-removes-days-01` exist to assert an exception **changes nothing**
— one covering a weekend the guardrail does not claim, one outside the horizon
entirely. Both assertions are correct and valuable. But **any date substitution
that preserves "this range is empty" leaves the result identical**, so their
values were unobservable by construction.

The specifier had written this exact rule into two previous features' headers
and applied it to weekday enumerations, then missed it for **dates chosen to be
inert**. The general form, now three sightings deep
(`life-areas-duplicate-03`, `timezone_setting`, here): **if a scenario's point
is that nothing happens, its parameters cannot be under test.** Split into
literal scenarios, the established remedy.

**`T-capability-owns-its-queries` needed a distinction it did not have.** The
architect deleted a byte-identical copy of `find_active_life_area_id` that
`exceptions::store` had shipped — **each copy citing that decision**. It
licenses separate queries for **two facts** that happen to share a table; this
was **one fact asked twice for the same reason** — *does this name resolve to a
life area work may be filed under?* Now `life_areas::active_id_for_name`, behind
the front door. The cost was one slice from stopping being theoretical:
"active" means `archived_at IS NULL` today, and `pool_only` is a second
dimension of the same question. **The test is whether the two callers would
want the same answer if the definition changed** — if yes, it is one fact.

**A repeat shape worth naming before it recurs a third time.**
`remove_exception`'s handler survived being replaced with a no-op, exactly as
`remove_guardrail_band`'s did in `dismiss-capture`: **a removal test whose
assertions are equally true of an empty response that touched nothing** — 200,
and the body lacks the removed item. Any removal control wants a test that the
thing was there first.

**DRY arrived red at 3.28%**, the second time in three slices, fixed by real
deduplication to 2.89%. That is now twice the fail-closed gate (#68) has
produced an improvement rather than a waiver, and twice #52's warning that the
headline understates the risk has held.

**Two things confirmed against precedent rather than re-decided**, which is what
the log is for: no new page, checked against #70's inversion — *if `app_shell`
had needed changing here, that would have been the bug* — and the removal
question, checked against #59 having already removed guardrail bands with nobody
arguing.

**Scenario 08 is the one PR #70 owed**: a life area given bands and *then*
marked never-scheduled. Its QA procedure says explicitly **not** to simplify it
by marking a bare life area, because that is the version that passed while the
defect was live.

**Measured:** 20 features, QA 20/20, `cargo test --workspace` and the property
suite green, **the proptest extended rather than duplicated** so disjoint,
sorted and positive-duration now hold over projection *and* subtraction in one
property, 100% language-mutation kill after two real gaps closed, CRAP 0, DRY
2.89%, core purity holds. Measured on the merged result against `trunk` at
`c69ff42`. #66 fired for both QA and the hardener under contention and passed
quiet; not this slice's.

### 2026-08-18 — capacity (#62, PR #73): M2 closes, and three bugs worth more than the feature

Recorded as `T-capacity-never-under-reports-demand` and
`T-required-fields-are-specified-per-transport`. **M2 is complete**: guardrails,
free time, exceptions, capacity. Trellis can say what each life area has, what
it has promised, and when the promise does not fit.

**Three real defects, and the two that matter are gaps in specifications, not in
code.** The specifier reported both against its own work.

**1. Committed triage was broken from the page, and the whole suite stayed
green.** `estimated_minutes` became required for committed triage; the committed
form was never given an input for it; **every committed triage through the
actual page returned `422 missing_field=estimated_minutes`.** The acceptance
suite triages its committed fixtures over **JSON**, so a page whose committed
control could never succeed passed twenty-one features. QA found it reproducing
the brief's own demo against a live server.

**What did not save it is the instructive part.** #33 spent a property test on
exactly this risk — *the page and the endpoint are one code path, not two* — by
submitting one arbitrary submission through both transports and requiring
identical results. That asserts equivalence **given a submission**. It cannot
assert that a transport is *able to produce* one. A form missing a field is
invisible to it. Recorded as `T-required-fields-are-specified-per-transport`.

**2. `free_intervals`' sortedness property could not fail** — #60's headline
acceptance criterion. Deleting `sort_by_key` outright left all four properties
green, because the generator gave every band a distinct weekday, so no date
could carry two intervals and the output was sorted by construction. **A
property test is only as strong as the inputs it can produce**, and *"asserted
by proptest, ≥1000 cases"* does not specify that the failure is reachable. The
generator now places bands in slots of a day so several may share a weekday; the
mutation fails. Two bands on one weekday is not contrived — `overlaps` is
half-open precisely so touching bands are both kept.

**3. Strengthening that generator immediately found a latent DST bug**, and a
deterministic one rather than the contention flake this project keeps meeting:
`band_interval` resolved a band's end with `Disambiguation::Later`, correct for
a fold and wrong for a gap. **The second defect was hiding the third**, which is
the argument for fixing weak tests before trusting strong-sounding claims about
them.

**A GAP recorded rather than fixed, at the architect's request — issue #74.**
Three layers disagree about `end_minutes`: the schema's `CHECK` permits `1440`,
the form cannot produce more than `1439`, and the core panics on `1440`. Nothing
creates one because the form is the only writer. **It is the second sighting of
a shape worth naming: a doc comment defending a rule with a claim about state
that nothing enforces.** `time_at_minutes` asserts *"guardrail minutes are
always within a single day"*; PR #70 had `life_areas::guardrails` claiming *"a
pool-only life area's `bands` is always empty"* while reporting 32h for one
marked never scheduled. **The tell is a comment that explains why a check is
unnecessary.**

Worth noticing that #74's option 3 — teach the core that 1440 means end-of-day —
is **a product question wearing a bug's clothes**: may a guardrail run to
midnight? The form's 23:59 cap looks like an artifact of parsing `HH:MM` rather
than a decision anyone made, and options 1 and 2 both foreclose it.

**Two existing features legitimately changed, both rules firing rather than
drift:** `app_shell` (a fifth page means a `nav::ALL` entry and a fifth per-page
scenario — and had it *not* needed changing, the page would ship with no link)
and `committed_triage_validation` (it enumerates committed's required fields by
name, and there is now a fourth). Nineteen untouched.

**Measured:** 21 acceptance features, QA passing, `cargo test --workspace` and
the property suite green, 100% language-mutation kill after one real gap closed,
CRAP 0, fmt/clippy/core purity pass, measured on the merged result against
`trunk` at `7f12450` at load < 4. Migration `0008`. **`T-capacity-two-axes` is
untouched and unmet here by design** — attribution needs placement, and there is
no scheduler until M3.

### 2026-08-18 — The invariants, written down at last

Recorded as `D-placed-whole-or-not-at-all`, `T-hard-refuses-soft-slips` and
`T-invariants-one-to-five`. **#11's strongest acceptance criterion can be
specified for the first time.** `docs/design/architecture.md`'s longest-standing
GAP is closed, and **U2 (#7) closes with it.**

**What made this tractable was decomposition, not analysis.** The invariants had
sat unratified since 2026-08-12 — six days — presented each time as one
five-part ratification, which is five judgements bundled into one question. Two
of the five needed no decision at all (nobody argues for scheduling the owner in
two places at once, and 2 and 5 were already settled). **The remaining three
were ordinary product questions wearing formal clothes**, each answerable from a
situation:

- *A 6h task, 4h free before its deadline — book what fits, or refuse?*
- *A deadline that cannot be met — is the calendar allowed to be late?*
- *A committed task with no estimate — where do you find out?*

Asked that way they took one round each. **The lesson is about how this role
frames decisions, not about scheduling**: a request to ratify a set is a request
to make every decision in it simultaneously, and the owner correctly refused it
twice before it was broken up.

**The third question dissolved rather than being answered**, and only because it
was measured. The concern was that an unestimated committed task cannot be
scheduled while invariant 5 requires every unplaceable task to carry a reason
from a closed enum containing no such code. **The owner's live database holds
zero committed tasks** — 5 pool, 1 quota, and it has not yet reached migration
`0008`. Estimates are required at triage from `0008` onward, so the state has no
instances and no way to acquire one. **The enum does not grow.** A theoretical
hole that costs a permanent variant in a closed enum is worth ten minutes of
`sqlite3` before it costs a design.

It also surfaced that **the estimate is not special**: `deadline`,
`deadline_type`, `priority`, `target_count`, `target_minutes_each`, `period` and
`life_area_id` are all nullable, kind-conditional, and enforced only at the
triage boundary. Guarding one of eight in the schema was rejected as
inconsistent — the boundary is the guard, as `T-unknown-kind-rejected` already
had it.

**Two corrections to the 2026-08-12 reconstruction**, both found while writing
the statements out:

- **Invariant 1 was silent about `facts`.** `free_intervals` computes
  `mask − busy − pins − buffers` — no facts — while `schedule()` takes them as a
  separate input. Unstated, the engine could place work over the block being
  worked right now. This is what writing an invariant down does that leaving it
  implied does not.
- **"Hard deadlines hold" is read as "deadlines hold"**, which
  `T-hard-refuses-soft-slips` makes false by design.

**`deadline_type` finally does something.** It has been in the schema with a
`CHECK` since `0003`, required at triage since #29, and has never changed a
behaviour — U2's *"as written it is dead"*. Deletion was the honest alternative
and was considered on this project's own precedent (`same_name` deleted,
`TriageRejection::UnknownKind` reduced to a unit variant). It survives because a
projected finish is more useful than a refusal for the class of thing where
Friday is a preference, and the refusal is the whole point for a tax return.

**M3 still has two blockers**, both product decisions and neither an agent's:
**#72** (a task pinned to a recurring time has no home in the model) and **U4**
(may a block cross a guardrail boundary — #7 recommends keeping invariant 2 as
written, and overlapping guardrails have since made adjacent-boundary cases
common rather than rare).

### 2026-08-19 — U4 confirmed, and a fourth overloaded word

Recorded as `T-blocks-do-not-cross-guardrail-seams`. **U4 (#7) closes**, and
with `T-hard-refuses-soft-slips` closing U2 yesterday, the U-series is down to
five: U1, U3, U5, U6, U7. **M3's only remaining blocker is #72.**

**The right answer for a different reason than the one on offer.** U4 argued for
invariant 2 because it is *"simple and property-testable"* — a real argument and
not the load-bearing one. `T-capacity-two-axes` decides it: capacity is
**consumed from the guardrail occupied**, so a block spanning Work's hours and
Learning's has consumed one hour of the first and two of the second and **must
be divided for accounting whatever the calendar shows**. Crossing does not avoid
the split; it moves the split out of the thing the owner looks at and into
arithmetic they cannot see. A decision made three days earlier settled a
question left open for seven.

**The recorded cost was wrong, and had been quoted twice.** The 2026-08-12 note
said a 2h task across a 17:00 seam *"cannot use those two contiguous free
hours"*. **Splitting rescues the ordinary case entirely** — the work lands at
exactly the hours it wanted, as two adjacent blocks worked straight through. The
surviving cost is narrow: a task whose minimum chunk forbids the smaller piece.
That is the third time a claim in this file has been corrected by re-measuring
it rather than re-reading it, and it is the habit paying off again.

**Whether that cost ever arises depends on a chunk policy nobody has set**,
which made *defer U4 to M3* a legitimate option. Declined because the answer
does not change with the policy — only the size of its cost does.

**"Overlap" is the fourth word this project has had to pin**, after *layer*
(`T-fact-plan-line`), *domain* (`T-package-by-business-domain`) and *window*
(2026-08-17). Three distinct things were being called one:

- **Guardrails overlap** — two life areas claiming the same hours, legal and
  expected, and they compete for them (`D-life-area-owns-its-time`).
- **Blocks never overlap** — invariant 1, and never with a fact or a pin either.
- **Adjacency** — two guardrails meeting at a seam. Not overlap at all, and the
  thing U4 was actually about.

The conflation was this role's: the phrase *"overlapping guardrails make
adjacent-boundary cases common"* appeared in a PM review and in a brief, and it
is wrong twice over — overlap does not create the adjacency case, and what makes
adjacency common is per-life-area guardrails, which is
`D-life-area-owns-its-time` and not overlap. **Four words in nine days is a rate
worth noticing**: this project's vocabulary is dense, and the cheapest moment to
pin a word is the first time two people use it differently.

### 2026-08-19 — Recurrence, answered by the owner and composed from what existed

Recorded as `D-recurrence-is-re-commitment`, closing #72. **A recurring
commitment is not a rule the system runs; it is a decision the owner makes again
each period.**

**The question came from using the product**, which is what `D-visible-slices`
exists to produce: *"I want to learn with a friend every Tuesday at
20:00–20:30"*, and nothing in the model could say it. This role offered four
options — a recurrence rule on `pin`, a fourth task kind, recurring task
instances, or let Google Calendar own it. **The owner's answer beat all four**,
and it was already the project's own spine: a recurrence rule is exactly the
thing where **inaction preserves**, which `D-inaction-archives` exists to
invert.

**The objection I raised dissolved on reading a decision already on file.** I
argued that a skipped review would silently cancel a commitment another person
is holding. `D-skipped-review-ages` already says a skipped review *"ages
everything one more week. No deeper sweep."* **Nothing dies from a skipped
review today**, so a recurring commitment dying would have been the anomaly, not
the rule. The two archive decisions coexist cleanly once said out loud —
**skipping the review kills nothing; attending it and staying silent kills the
item** (U7) — and the deliberate act `D-inaction-archives` requires is
*attendance*.

That is the second time in two days an objection of mine was answered by a
decision that had been sitting in this file the whole time (the other:
`T-capacity-two-axes` deciding U4). **The file is doing its job; the failure is
reading it too late.**

**The owner's follow-up did real work too.** *"Things will be pinned by weeks,
whereas I'd like to do something at the beginning of each month"* is a genuine
hole in re-commitment-as-stated: a monthly thing is dormant for three of four
reviews and nothing prompts it. The fix is that **what a review asks about is
driven by each task's own `period`, not by the review's weekly rhythm** — and
`T-period-closed-set` had already closed `period` to `week | month`, so no new
vocabulary was needed. A recurrence rule would have had to invent a second
period vocabulary beside the one quota already has.

**Composition over primitives, which is the shape worth remembering:** cadence
from quota's `period`, placement from a pin, survival from a deliberate act at
the period boundary. **Three existing things, no fourth.**

**M3 is unaffected** — it builds `pin { task_id, start, end, source }`, one
interval, exactly as specified; the recurrence is M8's. **So #72 never blocked
M3 after all**, which was not obvious when it was raised.

**What still blocks M3**, and it is now one thing: **which chunk a pin binds
when a task splits across days**, given chunks have no identity to refer to — or
whether pinning suppresses splitting entirely. Raised alongside C1 on 2026-08-12
and named on #11's own pin story as needing an answer first. Pin *lifetime*, the
other question from that pair, is answered for recurring pins and still open for
one-off ones.

### 2026-08-19 — The chunk policy, and a term nobody defined

Recorded as `T-minimum-session-per-life-area`, settled while M3 S1 (#75) runs.
**#77 (S3) is unblocked on the count that blocked it.**

**It filled a hole three decisions had already leaned on.**
`chunk_policy_unsatisfiable` has sat in #11's closed reason enum since the
milestone was written, with **nothing able to make it fire**;
`T-blocks-do-not-cross-guardrail-seams` recorded its surviving cost as *"a task
whose minimum chunk forbids the smaller piece"* and could not size it; and #77
could not be specified at all. **A closed enum with a variant nothing can
produce is the same shape as an invariant nobody wrote down** — a name being
reasoned from rather than a thing that exists.

**The trade is real in both directions**, which is why it needed the owner
rather than a default. Without a minimum, a 6h deck takes a 30-minute Monday gap
— time enough to open the file. With one, that gap is skipped and the task may
not fit at all, and `D-placed-whole-or-not-at-all` makes that consequence sharp
because there is no partial placement to soften it.

**Per life area rather than one number, on an asymmetry a single figure cannot
straddle: a 30-minute run is a run; thirty minutes of deep work is nothing.**
`D-staleness-unset`'s *instrument first, tune at the first reckoning* argued for
one seeded global number and lost on that — but its instinct survives in the
**defaults**, which sit beside the guardrail on a form the owner already visits.
Setup cost is zero unless they care.

**Scope narrowed usefully while framing it:** the policy governs **committed**
tasks only. Pool is never placed (`D-no-pool-on-calendar`) and a quota task's
`target_minutes_each` already defines its session — so *"3 runs a week, 30
minutes each"* was never a chunking question. That was not obvious until the
question was written out.

**Two rules stated rather than left to be discovered**, because the second is
the one an implementation gets backwards. A task **shorter** than the minimum is
placed **whole** — the minimum governs how a bigger task is chopped, not whether
a small task may exist, and the inverse makes a 20-minute email unschedulable in
a life area with a 90-minute minimum. And every chunk must clear the minimum or
the task is `chunk_policy_unsatisfiable`, which constrains **the result, not the
algorithm**: 4.5h + 0.5h is invalid, 2.5h + 2.5h is fine, the scheduler picks.

**No maximum**, and refusing one turned up **#80**. *"Nobody should work six
hours straight"* is a real opinion, but a maximum without a **break** between
the pieces is a split for no reason. Reaching for where a break would live
surfaced that **`free_intervals` subtracts `mask − busy − pins − buffers` and
`buffers` is defined nowhere** — not in this file, not in an acceptance
criterion, not in `crates/`. Three of the four subtrahends have owners; the
fourth has never had a definition. #60 implemented `free_intervals` and listed
what it does not yet subtract without mentioning buffers, because there was
nothing to mention.

**That is the 2026-08-13 failure again** — *a description that outruns the thing
it describes* — and it was found the same way the invariants were: by trying to
reason from the term and discovering there was nothing behind it. Marked GAP in
`architecture.md` with an instruction not to build against it. **Fourth such
term in this project**, after the product brief, the invariants, and the
now-corrected invariant-2 cost.

### 2026-08-19 — U3, and the owner amending the brief's scheduler

Recorded as `T-backward-pass-with-margin`. **U3 (#7) closes; #76 (S2) is
unblocked.** The U-series is down to four — U1, U5, U6, U7 — and **none of them
blocks M3.**

**U3's premise was obsolete and nobody had noticed for seven days.** It asks
what the backward pass does with *"a P1 with no deadline"*. `T-three-task-kinds`
put `deadline` and `priority` **both inside** `Committed` and made both
required — **on the same day U3 was written**, against the original brief. Since
then, priority has existed only on committed tasks and committed tasks have
always had a deadline, so the case U3 worries about has never been
representable. The live question was P1-with-a-*soft*-deadline, which answers
itself: the backward pass exists to **guarantee** a deadline is met, and a soft
deadline has nothing to guarantee (`T-hard-refuses-soft-slips`). **U3's
recommendation was right and its reason was not available to it.**

**The substantive change came from the owner, and it is an amendment to the
brief's scheduler rather than a clarification of it.** *"Why wouldn't we
frontload hard work?"* is the question that produced it. The brief specified
latest-feasible-start, which is **zero slack**: any disruption to the last
planned session misses the deadline, and M6's recompute is thin comfort when
replanning on Thursday evening against a Friday deadline leaves Friday.

**Front-loading instead was rejected on the packing the backward pass exists
for**, and the example is worth keeping because it is not obvious: a 20h task
due Friday, placed early, consumes the Monday slot that a 2h task due Wednesday
needed — and reports `deadline_unreachable` for the small one **when both could
have fitted.** A big far-deadline task placed early eats slots that near-
deadline work needs. That is the whole argument for a backward pass, and it had
never been written down either.

**Latest-feasible-start minus a margin keeps the packing and buys back the
slack** — which is what a person actually does with a deadline. **The
best-effort rule is what makes it safe:** the margin pulls placement earlier and
falls back where there is no room, so it can never cause an infeasibility. A
scheduling *preference* overriding a real *constraint* is precisely the shape
`D-guardrails-never-yield` refuses everywhere else in this product.

**Third time this week a settled decision answered an open question**, after
`T-capacity-two-axes` deciding U4 and `D-skipped-review-ages` dissolving the
recurrence objection — and the second time the *reason on file* turned out to be
weaker than a reason available elsewhere in the same file.

**M3's remaining blocker is one:** which chunk a pin binds when a task splits
(#78). Sharper now than yesterday, because `T-minimum-session-per-life-area`
finally defines what a chunk is.

### 2026-08-19 — Splitting becomes opt-in, and M3's last blocker falls

Recorded as `T-splitting-is-opt-in` and `T-pin-binds-a-task-not-a-chunk`.
**Every M3 blocker is now closed** — the invariants, U2, U4, U3, the chunk
policy, and the pin question that had been open since 2026-08-12.

**The pin question was answered by refusing its premise.** *"Which chunk does
the pin bind?"* has no answer and cannot have one: `R-incremental-patching`
regenerates the plan from scratch, so there is nothing to match a drag against.
**But `pin { task_id, start, end, source }` never mentioned chunks.** Reading
the type literally is both correct and the only reading that survives
regeneration, because it references nothing that gets regenerated. **A question
that stood for seven days was a misreading of a type that had been written down
the whole time.**

**It became unambiguous rather than merely workable because of a decision made
in the same conversation**, and that one came from the owner rejecting the
brief's premise: *"why is Trellis splitting it up? … it seems to me that the
**USER** will be breaking tasks down."*

**The brief assumed homogeneous work** — review 200 CVs, write 5,000 words —
where any 2h is as good as any other 2h. **Most committed work is not that.**
*"Write the Q3 deck"* is outline → draft → polish; the seams belong to the
owner, and chopping across them produces three sittings that each begin by
rediscovering where the last one stopped. Splitting is now **opt-in per task,
unticked by default**, and a task that will not fit whole is *told* so rather
than quietly chopped.

**Removing splitting entirely was the owner's first instinct and was nearly
right.** It was declined because homogeneous work does exist, and forcing its
decomposition into *"CVs 1–50, CVs 51–100"* is manual work the scheduler can do
correctly. The flag lets each task say which kind it is — **knowledge only the
owner has**, which is the same argument `T-life-areas-are-data` made for a set
only the owner can maintain.

**On the default being unticked**, because it looks like the silent-wrong-default
this project has twice refused and is not: **"don't chop my work" is the
conservative reading of silence**, whereas no life area is conservative — Work
is not safer than Fitness, so `D-manual-triage-until-llm` had to leave the
picker unanswered. **A default meaning *do nothing I was not asked to do* is a
different object from one that guesses.**

**A correction to `T-invariants-one-to-five`, one day after ratifying it.**
Invariant 1 read *"no block overlaps a pin"* — and a pinned task's own block
sits **exactly on** its pin, which the invariant forbade. It now excepts the pin
binding a block's own task. **Found by working a case through, not by
re-reading**, which is the third correction this week that re-measurement
produced and reading did not.

**Accepted cost, named by the owner when choosing it:** nothing orders the
pieces of a task decomposed by hand. Three 2h tasks all due Friday are equal on
slack and may be placed in any order. **Ordering between tasks is a dependency
and Trellis has no such concept**; a dependency graph is not v1.

**`T-minimum-session-per-life-area` survives, narrowed** to opted-in tasks —
settled four hours earlier, and this is the second time in a day a decision has
been scoped by the next one. That is what a decision log is for; the cost is
that a reader of the earlier row alone would over-apply it, which is why the
narrowing is written into that row rather than only here.

### 2026-08-19 — schedule-forward-pass (#75, PR #81): the first plan, and a decision that went missing

Recorded as `T-plan-is-stored-and-explicitly-regenerated`,
`T-unplaceable-reason-precedence` and — retroactively —
`T-interval-is-its-own-module`. **Trellis puts work on a schedule for the first
time.**

**The brief's generator warning was used within hours of being written, and it
worked.** #73 had found that `free_intervals`' sortedness property passed 1000
cases while being structurally incapable of failing, so this brief did not just
name the invariants — it specified **what each generator must be able to
produce**. The architect's order-independence property then **passed with the
task-id tiebreak deleted**, because `any_task`'s wide ranges make an exact
slack-and-priority tie vanishingly rare and the generator almost never produced
the input the tiebreak exists for. Same failure mode as #73, **caught before
merge rather than two slices later.** A lesson written into a brief is worth
more than the same lesson written into a log entry.

**And a decision went missing, which is this role's failure and cost two
slices.** At #60 the interval type was settled into its own `scheduler-core`
module rather than one named after a producer, *because M4's busy intervals are
not guardrail-derived*. **The PM judged that a type shape belonged in
`docs/design/architecture.md` rather than in this file, and then did not write
it there either.** It existed only in PR #70's body. It shipped inside
`free_time`, survived #61 and #62 unnoticed, and the predicted cost arrived
verbatim: interval subtraction was built as **two private helpers inside
`schedule`**, unreachable by M4's second producer, which would have written them
a third time.

**The process defect is specific and fixable.** A brief's open questions get
triaged into *worth a decision row* and *architecture.md's job*, and **only the
first bucket has a forcing function** — rows are written in the same pass that
reads the pull request, while the second bucket depends on someone remembering.
*"The architect will put it in the architecture reference"* is not a plan. **An
answer worth asking for gets recorded in the pass that reads it, wherever it
goes.**

**A question the brief did not ask turned out to be unspecifiable without.**
#11 fixed the reason enum's **membership** and said nothing about
**precedence** — and since more than one reason can be true at once, invariant 5
was satisfiable by two implementations that disagree on every interesting case.
The fourth reason carries the point: a task against 4h of free time in 2h
windows is **not** `capacity_exceeded`. Time remains; *contiguous* time does
not, and reporting capacity would send the owner to widen a guardrail that is
not full.

**Two smaller things worth keeping.** `UnplaceableReason` shipped with its four
words written out in `trellis-server`, so a closed **core** vocabulary was a fact
about one delivery mechanism — the enum said there were four, the server said
what they were called, `0009`'s CHECK said it again, and nothing tied the three;
a property now walks `ALL` and stores each. And `replace_plan` took tuples where
two of three fields shared a type, so **transposing start and end still
compiled** — typed slices now.

**Measured:** 22 acceptance features, `cargo test --workspace` and the property
suite green, **18 mutation survivors killed**, Gherkin mutation 45/46 (the one
survivor lowercases the `Z` in an RFC 3339 deadline, which `jiff` parses either
way — a no-op, not a gap), all three crates at 100% differential kill.
**Measured on the merged result against `trunk` at `7047b8a`**, which landed
mid-slice — merged before measuring, per the brief. DRY arrived **red at 3.22%**
and was deduplicated to **2.96%**: third time in five slices, and the standard
held every time.

### 2026-08-21 — remove-unused (#88, PR #90): 19,000 lines out

Recorded as `T-dead-core-code-earns-its-keep`, **requested by name in the
specifier's handoff note** — the third slice running to say which row it needed
rather than leaving it to archaeology.

**The largest deletion this project has done**: five server modules, six core
modules, ten features and ten QA documents, **−19,011 lines**. `scheduler-core`
is down to `interval`, `schedule`, `task`, `timezone`; the server to `capture`,
`dismiss`, `inbox`, `platform`, `settings`, `triage`. One screen. **DRY fell
2.86% → 1.9%** because the denominator lost 3,650 lines of product code.

**The brief's GONE list was one short and the slice caught it.**
`stats_ratio.feature` drives `GET /stats` and would have been left specifying a
route that 404s. That is the PM's error: the brief enumerated modules carefully
and enumerated features from memory. **An enumeration written from the tree and
an enumeration written from recall are different artifacts**, and only one of
them was checked.

**`app_shell.feature` was deleted rather than shrunk, against the brief**, and
the argument is better than the instruction: eight of its ten scenarios assert
per-page shell behaviour for pages that no longer exist, and what remains is the
negation of the feature's own title — *"every page carries the same navigation
header"* becoming *"no header exists"*. **A feature cannot shrink into its own
opposite.** Its surviving assertion is rehomed in `one_screen.feature`, whose
subject is true. `T-nav-is-the-site-map` needs no amendment: with one route the
site map is empty, and an empty site map renders nothing.

**`T-latency-is-a-qa-assertion` paid out inside the pull request that
implemented it.** The acceptance suite passed **at load average 9.2** — a run
that would have failed and needed re-running under the old assertion, as it did
in four of six slices.

**PR #87's drift was invisible to CI, and only re-running QA found it.** That
pull request landed a 636-line stylesheet and restyled three templates
**without touching a single feature, QA script or step module** — so CI stayed
green while `capture_row.html`'s `<li>` gained `class="row"`, breaking the
exact-match regex every QA script uses to find a capture row, and while the
empty-state copy changed under a QA document quoting it verbatim. **CI can only
see what something asserts against.** The repair is the interesting part: the
document now asserts **sameness with the ordinary empty state** rather than a
literal string, because pinning the copy has broken once and told nobody
anything.

**The #71 trap was walked into again, by the person who had written it down
twice.** `one_screen`'s scenario 01 asserts removed routes return 404 — and
mutating `/stats` to `/statX` still 404s, so nine Gherkin mutants survived and
the `path` column was never under test. **The honest fix is not literal
scenarios**: it is a column where the path *distinguishes* outcomes, which
arrives when #85 adds `/menu` and the scenario gains a `200` row. Third sighting
of this shape, and the first where the remedy is *wait for the input that makes
the assertion falsifiable* rather than restructure the scenario.

**One thing to know before the next deletion.**
`platform/boundary.rs`'s `no_capability_names_another_capabilitys_store`
asserts `capabilities.len() >= 5` and now finds **six directories — one of which
is `platform`**, which this codebase documents as *"named to say 'not a
capability' out loud"*. **It passes by counting a non-capability.** The next
deletion trips its own *"covers almost nothing"* warning, which is the floor
doing its job late rather than failing to.

### 2026-08-21 — context-tags (#82, PR #91): a slice that came back smaller

Recorded as `T-qa-binds-tolerantly-to-markup`, **requested by name in the
handoff note** — the fourth slice running to say which row it needed.

**Its second trip, and the demolition improved it.** PR #89 was built against
modules #88 removed and was closed; #88 then took the *"life area stops being
required"* half with it, leaving a slice about one thing. **A slice re-briefed
against the tree as it is beat the same slice briefed against the tree as it
was.**

**`T-latency-is-a-qa-assertion` did its job one slice after moving**, and found
a correctness bug rather than a slow one. The 50 ms capture budget now lives in
QA; this slice added a field to that very endpoint. The specification forbade
computing suggestions in the capture path — **and the architect found the
implementation had spent the budget elsewhere anyway**: `insert, look the tag
up, update` is three round trips where two will do, and **between the second and
third the row sat on disk untagged while `create` had already handed its caller
the tag it was supposed to carry.** A window with no reader today that a
re-render or a retry would eventually find. **A performance constraint surfaced
a durability bug**, which is the argument for keeping the budget rather than
deleting it as flaky. QA then *measured* it — ten capture requests before and
after populating 22 tags, all under 50 ms — instead of asserting it on a
contended machine, which is the whole point of the move.

**The deliberate asymmetry is now written where it will be read.** Context tags
are free text (`D-context-tags-are-the-taxonomy`); quotas are picked and never
typed (`D-quotas-are-selected-not-typed`). The slice put that contrast in the
feature header and the QA document, **where the next person tempted to add a
managed tag set will meet it** — rather than only in this file, which is read at
startup and not at the moment of temptation.

**Case-folding was settled on a failure that does not exist yet**, and the
reasoning is worth keeping: `T-collation-enforces-name-identity` died with #88,
but it was *evidence* for the argument rather than its source. `@HomeDepot` and
`@homedepot` are one tag because **typing slips more readily than picking**, and
the harm is invisible until #85 — a Menu showing two Home Depot lists and
sending the owner to the shop twice. Settled now because the data is being
written now and cannot be re-spelled later.

**A correction to PR #90, self-reported by the specifier who signed it off.**
#90's implementation also deleted `features/context_tags.feature` and
`qa/context_tags.md` — neither on #88's GONE list, both specifying a slice that
was **parked, not cancelled**. Its body named ten deletions; the diff carried
twelve. Harmless in effect, since this slice rewrites both files. **The lesson
generalises: a count cannot say which files went.** The PM reviewed that pull
request and did not catch it either, having checked the *set* of modules against
the tree and the *number* of features against the claim.

**Measured:** 14 acceptance features at load average 9.6, `cargo test
--workspace` and the property suite green, 14 QA scripts, mutation-clean, all
four CI gates — one of which caught a citation this slice line-wrapped across
two lines, so only the fragment before the break reached `decision_slugs.sh`.
Migration `0010` adds `context_tag TEXT COLLATE NOCASE` with a `CHECK` mirroring
the trim-and-non-empty rule, and **no `UNIQUE`**: many captures share a tag by
design.

### 2026-08-21 — The Pool spec read the canvas; the brief had only grepped it

Recorded as `T-trips-are-derived-not-ranked`, and **#95 files the reordering
slice** the owner deferred out of #92.

**The brief was wrong about the canvas in three places, and the specification
caught all three.** It is worth listing them because they share one cause:

- **`tripThreshold || 3`.** A context tag becomes a *trip* only at three items;
  smaller groups fall into loose ends **still showing their tag** — which is
  what the otherwise unexplained `ctxLabel` on a loose item is for. **The
  brief's demo drew a two-item group as a trip.**
- **Group order was already decided** — `b.list.length - a.list.length ||
  a.key.localeCompare(b.key)`. The brief asked *"how are groups ordered?"* as
  an open question and offered three candidates. **The canvas had answered it.**
- **`poolMeta` is `pool.length + " waiting"`**, not the brief's *"6 loose"* —
  which would have made *loose* mean two different things on one screen.

**And the brief said twice that the canvas draws no reorder control. It draws
six**, `aria-label="Raise priority"`, on trip items, loose ends and quota rows.
`D-menu-is-a-worklist` names manual priority too, **so the canvas and this file
agreed all along and only the brief dissented.**

**The cause is one thing: the PM read the design by `grep` and wrote a brief as
if it had read the design.** Structure was extracted with regular expressions —
section headings, `sc-if` guards, uppercase titles — and then asserted about.
Every error above lives in the canvas's **script**, which no such extraction
touches. **A brief that says "the canvas wins on layout" and then paraphrases
the canvas is a second source pretending to be a citation.** The remedy is
narrow and real: **cite the canvas by construct, or say you have not read it.**

**The deferral nearly cost the reasoning, which is why it is recorded here.**
Reordering belongs to **loose ends alone** — a trip is a unit you clear in one
stop, so the order of its items is noise, while a loose end is a thing you
decide about. **Trips and groups get no priority control, ever.** That split is
reachable only by reading the ordering rule *against* the threshold, and the
brief had excluded reordering on the mistaken ground that no control was
drawn — which would have deleted the reasoning along with the scenario. It
survives in the feature header and now in #95.

**One thing the canvas draws that this product refuses**, raised rather than
resolved quietly per `D-four-screens`: the per-group note *"One stop clears all
3."* is computed from a hardcoded list of which tags are **places** and which
are **sittings**. That would need Trellis to know `@homedepot` is a shop — the
managed taxonomy `D-context-tags-are-the-taxonomy` exists to refuse. **A layout
that implies a behaviour the log forbids is a disagreement, not an
instruction**, and the specification said so instead of building it.

**Scenario 05 asserts the *absence* of every reorder control**, and its QA
document says finding one is a defect. That is the right shape for a deferral:
**a coder reading the canvas would add them in good faith**, so the guarantee is
written down rather than left to inference. #95 narrows that assertion when it
lands; it does not delete it.

**#90's deferred fix landed too.** `one_screen`'s `path` column is falsifiable
at last — every row expected 404, so mutating `/stats` to `/statX` still 404'd
and nine mutants survived. `/` and `/pool` at **200** fix it, which is exactly
the remedy predicted: *wait for the input that makes the assertion falsifiable.*

### 2026-08-21 — pool-screen (#92, PR #96): the first Menu tab, and a hole with no test under it

Recorded as `T-cross-capability-invariants-need-an-owner`. **Trellis has a
second screen.**

**The architect found the gap that hides.** `scheduler_core::pool::group`
buckets by plain string equality, correct **only because**
`capture::resolve_tag` canonicalises on the way in — **two capabilities holding
one invariant between them, with a doc comment as the only tie.** Third sighting
of a shape this file has named twice (#70, #74), and the tell is unchanged: *a
comment explaining why a check is unnecessary.*

**How it hid is the new part.** Every fixture on the pool path seeded tags
through `capture::store::insert` **with the spelling already final**, skipping
the exact step the screen depends on. That is #70's *right end state by the
wrong path* for the **fourth** time — and the first in **unit fixtures** rather
than in a scenario, which is a place this project had not thought to look.

**And the failure was user-visible, not theoretical.** Three items under one tag
is exactly `T-trips-are-derived-not-ranked`'s threshold; split across two
spellings they are groups of **2 and 1**, both below it — so **both fall to
loose ends and the trip disappears from the screen.** Exactly the failure #82's
case-folding was decided to prevent, arriving from the side nobody was watching.

**One thing left unverified, and it matters more than it reads.** *"No browser
automation in this stack, and this is a phone-first screen whose tab bar is
sticky-bottom — the layout itself is unchecked."* **That is not only a tooling
gap; it is the deployment gap.** The owner cannot check it on a phone either,
because **nothing in the roadmap ever put Trellis on the network.** The server
has been a debug binary launched by hand on `127.0.0.1`, the only release build
on disk is from 2026-08-13 — **179 commits ago** — and nothing runs it, keeps it
running, or survives a reboot. `#83` closed on viewport and manifest, and the
PM let *"Capture works from a phone, **anywhere**"* close on a stylesheet.
**Two weeks of dogfooding needs something to dogfood on**, and that is now the
only thing on the critical path that no slice owns.

**The specification read the canvas and the brief did not**, which is recorded
in the previous entry and is visible in the result: the trip threshold, the
derived group order, `poolMeta`, and six reorder controls the brief twice said
were not drawn. **The threshold is the load-bearing discovery** — it is what
makes the screen mean something, because two errands sharing a place are strays
and three are a trip worth making.

**Measured:** 15 acceptance features at load average 10.0, `cargo test
--workspace` and the property suite green, mutation-clean, **zero reorder
controls in `pool.html`** — the cut held. `one_screen` changed twice, both rules
working: its `path` column is falsifiable at last (#90's deferred fix), and its
no-header scenario is **gone rather than inverted**, the guarantee moving to
`pool-screen-tabs-08`, which asserts what the header holds rather than that it
is absent.

### 2026-08-21 — committed-screen (#94, PR #100): three of four screens

Recorded as `T-commitment-is-chosen-not-derived`. **Capture, Pool and Committed
exist. Only Quota (#93) remains of `D-four-screens`.**

**The disagreement `D-four-screens` was written to handle actually arrived, and
the protocol worked.** `D-committed-is-at-or-by` says an *at* and a *by*
*"display differently and behave differently"*; the canvas draws no distinction
whatsoever. Canvas authoritative on **layout**, this file on **behaviour** — so
the distinction stood, the drawing of it was the slice's to invent, and it was
**raised rather than resolved quietly**. That is the second time the rule has
paid (the first was #92 refusing the places-vs-sittings taxonomy).

**Deriving at/by from whether a time was typed was the obvious shortcut and is
wrong**, for a reason worth keeping: it **cannot express a hard *by*** — *"by
5pm on Jan 31, and that one cannot slip"* — which would have been **silently
unrepresentable**. Same failure `T-unknown-kind-rejected` and
`T-period-closed-set` were each written against, arriving as a *missing
combination* rather than an unvalidated string.

**Two process findings, and one is about this role.**

**A slice arrived with a failing CI gate, and the claim outran it.** The
implementation commit said five gates passed where four did —
`complexity-baseline.json` was never updated, so `complexity_baseline` failed on
a clean checkout of the handed-off commit. The architect verified by stashing
and re-running, then **fixed it by review rather than by bumping**: all four
movements are new one-line dispatch arms, not conditionals sneaking into a
match, which is the distinction `T-complexity-8` is actually about. That is the
gate working and the *verification claim* failing — a shape this file has met
before, on 2026-08-13, where a version note claimed *"verified identical before
and after"* while a count had gone 1 → 3.

**`git fetch` updates `origin/trunk`, not the local branch named `trunk` — and
the local branch only advances when the PM fast-forwards it in the main
checkout, never when a pull request merges on GitHub.** The specifier nearly cut
#94's pull request from a base two merges stale, and caught it. **This is
partly the PM's doing**: every brief's first gotcha reads *"merge `trunk` before
your final measurement"*, and the trap is that **the branch named `trunk`
locally is not the thing that word means in that sentence.** Briefs should say
`origin/trunk` explicitly. Confirmed live while writing this entry — local
`trunk` was at `c4102a8` while `origin/trunk` was two merges ahead.

**Raised, not fixed, and already filed:** past deadlines stay on the Committed
screen forever, because nothing in Trellis can mark a task done (**#97**). The
Committed tab is where that becomes visible daily, and it will fill with history
across the fortnight of dogfooding.

**Still unverified, for the third phone-first screen running:** the layout.
`BY THU 17:00` is the longest string the 66px `tabular-nums` cell must hold and
the likeliest to clip, and **nobody has seen it.** The owner has since chosen a
live per-branch preview instance as the fix.

**Measured:** 16 acceptance features, `cargo test --workspace` and the property
suite green, mutation-clean, **no reorder control in any template** — the canvas
draws arrows on this screen too, and `pool-screen-nothing-reorders-05` is the
only thing standing between the design and a feature nobody approved.

### 2026-08-23 — Translating the log through the pivot: 16 rows marked, none deleted

**The question the owner asked was what to toss. The answer the evidence gives is: nothing.**

`scripts/ci/decision_slugs.sh` checks citations from `crates scripts qa features
.github` — **not `docs/`** — so **42 of 96 rows are uncited today and could be
deleted without CI noticing.** That is exactly what makes deletion a trap: most
of those 42 are decisions for work that is **parked, not dead** — M3's backward
pass, pins, splitting and regeneration; the GCal client; the weekly review.
Deleting them does not remove clutter; it guarantees those calls get
re-litigated when M3 unparks, which is the one thing this log exists to prevent.

**The rot was never volume. It was that only 3 of 96 rows were marked, while
`D-context-tags-are-the-taxonomy`, `D-four-screens` and #88 had moved the ground
under roughly forty.** A row that reads as live and is not is worse than a long
file.

**Three dispositions, and the distinction between them is the point:**

- **SUPERSEDED** — the subject is gone and the reasoning is kept for whoever
  reintroduces it: `D-guardrails-never-yield`, `D-life-area-owns-its-time`,
  `T-guardrail-well-formedness`, `T-free-time-horizon-fourteen-days`,
  `T-availability-only-subtracts`, `T-capacity-two-axes`, `T-fold-counts-both-passes`.
- **TRANSLATED** — the rule is alive and only its nouns died:
  `T-capacity-never-under-reports-demand` (now the Quota screen),
  `T-configuration-is-removed-work-is-archived` (now the timezone setting and a
  quota's target), `T-one-front-door-per-capability` (new examples),
  `T-collation-enforces-name-identity` (see below).
- **RESTS ON A REMOVED MODEL** — parked with M3, real and unanswered, and **not
  restatable until it is known what carries the concept**:
  `T-minimum-session-per-life-area`, `T-blocks-do-not-cross-guardrail-seams`,
  `T-core-owns-validation-order`.

**`D-life-area-owns-its-time` is the row with no successor, and it is the one to
remember.** Context tags are free text and carry **no hours at all**, so
**Trellis today knows what you want to do — quota targets — and nothing about
when you are free.** Reintroducing availability means reintroducing that
decision, not translating it.

**The best find was a reversal nobody recorded.**
`T-collation-enforces-name-identity` chose `life_areas.name UNIQUE COLLATE
NOCASE` **specifically to avoid** identity living in a function that a write
path could forget to call. `0010_context_tags.sql` carries the collation forward
by name — *"the same argument `life_areas.name` made before #88"* — but **drops
the `UNIQUE`**, because many captures share one tag by design. With no unique
row to be the canonical spelling, choosing the winning spelling moved into
`capture::resolve_tag` — **the exact shape the original decision rejected** —
and `T-cross-capability-invariants-need-an-owner` is the bill, since
`pool::group`'s string equality is correct only while that function runs on
every write path. The original reasoning was right and the new model could not
keep it. **That is worth more than either half alone, and it was invisible
because both rows read as live and neither pointed at the other.**

**Three sentences were not stale but false, which is worse — they were being
reasoned from:**

- `D-manual-triage-until-llm` claimed *"`/stats` still counts and the >50% alarm
  still fires"*. #88 deleted `/stats` and `scheduler_core::ratio` with **no
  replacement counter**, so R2's experiment **stopped** rather than narrowed —
  and `D-dogfood-first` opens a two-week real-use window the counter was built
  to feed. Raised on #20; the owner's call.
- `D-visible-slices` cited `R-guardrail-override`/`R-browsable-archive`/`R-pool-on-calendar`
  where it meant #20's **R1/R2/R3**. The rejected-options series shares
  R-numbering with the risk register and the slugs were substituted
  mechanically. The prose was right; the citations pointed at unrelated rows.
- `D-staleness-unset` seeds *"21 days / ≥3 offers"*. `times_offered` was a
  `D-menu-of-three` concept and **exists nowhere in the repo**, and nothing
  measures task age either, so the instruction to *instrument first* was never
  carried out. Tracked at #38.

**What this changes about how the log is maintained:** a decision that supersedes
another must say which rows it strands. `D-context-tags-are-the-taxonomy`
stranded eleven and named one. **The supersession is cheap to write and the
sweep is not**, which is why it went six days undone and why the next one should
list its casualties in the same commit that records it.

### 2026-08-24 — The canvas leads a slice, then the slice leads

**The owner named the contradiction and it was real.** `D-four-screens` made
`docs/design/Trellis.dc.html` authoritative on layout **perpetually**;
`D-dogfooding-drives-the-roadmap` says real use decides. **Both cannot hold** —
a design made before use cannot outrank use. **Eight gaps in eight days is what
that contradiction looked like from the outside**, and every one of them cost a
round trip to raise, log and explain.

The owner's framing is the one to keep: *"The design is not canonical. It was a
best first effort."*

**What the canvas was genuinely good at, because the answer was never to discard
it.** It settled #101 by **construct** — line 45's phone frame is
`height:844px; overflow:hidden`, which decided between two directions the brief
could not choose between. It supplied the **44px** touch target and the **66px**
date cell. It caught the triage disclosures being an invention (#126) and the
"show more" being the wrong control (#120). **Those are layout facts no text
description carries.**

**But look at how it won those two arguments.** In both, the implementation had
drifted **without anyone noticing** — the canvas's value was as **a second
opinion that could not be rationalised away**, not as an authority with better
taste. **You can keep the second opinion without keeping the chain of command**,
and that distinction is what this amendment turns on.

**So: authority is per-slice and time-bounded.** The canvas leads a slice being
specified against it. Once that slice ships and the owner has used it, **the
implementation is the record**. New features get their own small canvas in
Claude Design, authoritative for that slice and then retired; the master canvas
becomes historical rather than something to retrofit into.

**Two practices survive unchanged**, and they are the load-bearing ones:

- **The specifier reads the canvas before implementing.** That is what caught
  both divergences, and it costs nothing when the canvas is stale — **a second
  opinion is worth having out of date, as long as nobody has to obey it.**
- **The pipeline never edits the canvas.** The local copy is byte-identical to
  the Claude Design remote; a pipeline edit forks the two.

**What ends is gap-logging as debt.** A gap only means something while the canvas
is *ahead*. **The eight are closed as a category, not as work** — and the
owner's standing instruction to draw four of them in is withdrawn by this,
rather than left as a task nobody will get to.

**The cost, accepted knowingly:** *"the design decides"* is no longer available
to settle a disagreement. It settled two. From here those resolve **by the owner,
from use** — which is what `D-dogfooding-drives-the-roadmap` already says, so the
change removes a contradiction rather than adding a risk.

**A caution for whoever reads this next.** The pull toward a stale canvas is
real: it is concrete, it is in the repository, and it answers questions the
decisions log leaves open. **Being concrete is not the same as being current.**
`T-collation-enforces-name-identity` is the same failure from the other side — a
description and the thing described drifting apart while both read as current.

### 2026-08-25 — trip-controls (#120, #125, PR #135): the column that was not bought

**The brief asked one question and told the pipeline not to answer it the way
`#126` had.** *Where does toggle state live?* — and: *"If you find yourself
reaching for a column, stop and say why in the handoff before writing it."*
**No migration was written.** `T-ephemeral-view-state-rides-the-request` records
the answer, and it is the first time this project has reversed a design mistake
**before** it cost anything rather than after.

**Why it is worth a row rather than a commit message.** The two slices are eight
days apart and reached opposite conclusions from the *same* correct premise.
`#126`'s reasoning is not sloppy — it is a valid chain from *"the acceptance
suite speaks HTTP"* to *"the state must be stored"*. **What it never examined is
the first link.** The suite speaking HTTP is a fact about which tier the slice
chose to assert in, and this project already had two others: a CI-gated browser
check driving real Chrome, and an inline script on every page since `#58`.
**A premise that is really a choice is the hardest kind to notice**, which is why
it gets a slug instead of a caution.

**The line between the two, stated so a future slice can apply it:** `#122` bought
`cleared_at` in migration `0013` **and was right to**. Clearing a trip is a
deliberate act whose consequence must survive a reload. Expanding a panel is
something you did with your thumb ten seconds ago. **Same shape, opposite
answers, and the test that separates them is whether a reload should forget it.**

**And the cost is not symmetric.** `T-migrations-append-only` means `shown_kind`
can never be removed, only added to. **A column bought wrongly is permanent; a
query parameter chosen wrongly is one slice's rework.** When the two designs are
otherwise close, that asymmetry decides it.

**What the pipeline did well, recorded because the roles that did it will not
read their own commit messages again.** The coder wrote both listeners as
`document.body.addEventListener` first — which throws, because the block runs in
`<head>` before `<body>` exists — **and said so plainly instead of quietly
fixing it.** That is the only reason the architect could name client behaviour as
an uncovered tier (`docs/design/architecture.md`), and the only reason QA knew
which assertion to write. **A defect reported is worth more than a defect
fixed**, and this is the second slice running where that has been true.

**One correction to `#136`, filed by the PM against a note that was already
stale.** The architect recorded *"nothing executes the `configRequest` hook's
contract"* at `8151323`; QA's `0416241` — **the very next commit** — added
`scripts/qa/trip_controls.cjs` step 10, which ticks a real checkbox in real
Chrome and asserts the panel is still expanded with all eight items painted
after the swap. **That is the hook end to end**, gated at `ci.yml:452`. The
architecture note and `#136` both describe a gap that closed while they were
being written.

**What remains of `#136` is narrower and still real.** Of the six breakages QA
proved, **none breaks the hook** — so the `survives-being-worked` assertion has
never been seen red. `T-a-check-must-be-seen-to-fail` is unsatisfied for exactly
one assertion, and the historically real breakage (`document.body`) is the one to
revert and observe. **The fix is a revert-and-watch, not a new check.**

### 2026-08-25 — A quota is an entity, and #93 becomes two slices

**The finding that reshaped the slice:** `TaskKind::Quota { target_count,
target_minutes_each, period }` makes a quota **a triaged capture**. Everything
settled about quotas since describes something else. `D-quotas-are-selected-not-typed`
has a quota **created from the Menu without a capture**, **chosen from chips** at
triage, carrying **a name and a weekly hour target**, and **holding items filed
into it**. **None of those are properties a task kind can have.**

**So a quota is a first-class entity, and #93's own framing —** *"that is a
migration and a triage change"* — **understates it by a whole table.** The
decision was settled on 2026-08-20; the schema it contradicts is migration
`0002`, from M1. **Nobody had read the two against each other for five days**,
which is the same failure mode as `T-collation-enforces-name-identity`: a
description and the thing described both reading as current.

**The cut, chosen by the owner.** Slice 1 (**#93**) is the entity, the fourth
tab, `+ Define a new quota` and **the whole logging loop**. Slice 2 (**#138**) is
filing, `Filed here`, and retiring the task kind — **which is where `period`'s
fate gets decided**, since the canvas draws only *hours a week* and
`D-quota-no-rollover`'s Monday reset leaves `month` no evident meaning.

**A three-way cut was offered and rejected** — entity, then logging, then filing.
The owner took the larger first slice knowingly. **The recorded reason to prefer
it:** a quota screen whose bar always reads zero is a demo nobody can judge, and
*define → log → correct* is the loop the screen exists for. **The recorded risk:**
it is the largest slice this pipeline has been handed. The brief names a
designated stopping line — entity and define land, session surface does not — so
that overrunning produces a coherent half rather than a half-built one.

**Two quota concepts coexist between the slices, deliberately.** Slice 1 leaves
`TaskKind::Quota` and the existing triage form untouched. **That is a knowingly
accepted transitional state**, not an oversight, and #138 closes it.

**Read the canvas, do not grep it — the third instance.** `T-trips-are-derived-not-ranked`
records the PM dissenting twice on the Pool from a grepped reading. This brief
found three more: the quick-log controls are **outside** the expanded row, so
logging never requires expanding; `Filed here` and `This week` share **one** row
template with the former gated; and **the canvas draws reorder controls on quota
rows** (`q.onUp` / `q.onDown`, lines 259-262) that **no decision covers**.

**The reorder question is deferred to #139 rather than answered here**, and the
reason is worth keeping: `T-trips-are-derived-not-ranked` forbids ranking a trip
because *"a trip is a unit you clear in one stop"*, while #95 grants loose ends a
control because *"a loose end is a thing you decide about."* **A quota looks like
the second — but `D-quota-no-rollover` gives it a readout and a bar that may
already say everything a ranking would.** It is durable state, so it would earn a
column under `T-ephemeral-view-state-rides-the-request` — **which is exactly why
it should be wanted from use before it is built.**

### 2026-08-25 — trip-persistence (#129, PR #141): a run, and the test that permits rather than compels

**`D-a-trip-survives-being-tidied` closes the panel this project spent three
slices on.** #122 made a trip survive being *worked*; #135 gave it controls;
this makes it survive being *tidied*. **All three were the same defect wearing a
different gesture**, and each was found by the owner within an hour of using the
one before.

**The bound #129 asked for turned out to be an event, not a window.** *"When does
trip-ness expire?"* looked open-ended — the issue feared *"over a year every
frequently-used tag becomes a permanent panel"* — and the answer was already half
written down. `trip-progress-fully-done-06` asserts a tag with nothing open left
leaves the screen; a run therefore **ends on its own**, and a tag must reach three
again to earn another. **The brief found that pair by reading the feature file
rather than the issue**, and it turned "design the expiry rule" into "name the
one already implemented on the other side."

**The precedent worth keeping is about `T-ephemeral-view-state-rides-the-request`,
not about trips.** That row was written eight days after `shown_kind` bought a
column for a display preference, and the obvious misreading of it is *"do not
store things."* **This slice is the counter-example: a column here would have
been legitimate** — a trip that dissolves while the phone is locked in the car
park fails the *should-a-reload-forget-it* test outright. **The test tells you
when storage is permitted. It never tells you it is required.** The brief asked
for a derivation to be attempted first; it worked; and the difference between
"permitted" and "taken" is one slice's thought against a permanent migration.

**A deviation from the brief, made in the open, and the pipeline was right.**
The demo said the panel goes when the last open item is ticked. It goes one tap
later, on `✕`. **Three reasons, and the third is the one that matters**: ending a
run on a tick without sweeping leaves the strikes uncleared and invisible, so the
next capture at that tag resurrects a panel over month-old strikes — **the exact
runaway #129 was filed to prevent, re-entering through the fix for it.** A brief
is not a specification, and a pipeline that had implemented the demo literally
would have shipped the bug the slice was for.

**The second reason is a debt from #135 surfacing on schedule.** *You can uncheck
what you can see* — a panel that vanishes on the last tick takes its own undo
with it. #135 left group completion with no single-gesture undo and this brief
flagged the edge; it came back one slice later as an argument about a different
control. **Debts in this product do not sit still; they change shape.**

**What still proves a rule that moved into the store, stated before the query was
written** — the company standard's requirement, met in the pull-request body
rather than after the fact. `store.rs`'s run tests go through `clear_done` rather
than writing `cleared_at` by hand, and six properties pin the half of the rule
that stayed in the core, **monotonicity among them: a larger run never costs a
tag its panel.** The architect's replacement of a per-task count with `RunSizes`
is the same instinct one level up — **a run size is a fact about a tag, and the
type now says so, so disagreement is not representable.**

**One process note, recorded because it cost the owner a CI cycle.** The slice
cited `D-a-trip-survives-being-tidied` in three source comments before the row
existed, so `decision citations resolve` went red on the branch and the row had
to land on `trunk` afterwards. **`trunk` is `strict`**, so the branch then needs
updating before it can merge. **The citation gate and the PM's ownership of this
file pull against each other**, and the cheap fix is for a slice that knows it
will need a row to say so in its *first* handoff rather than its last.
