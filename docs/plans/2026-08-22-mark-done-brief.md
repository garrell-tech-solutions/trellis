# Handoff brief — `mark-done`

**Date:** 2026-08-22 · **Issue:** #97 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **Found by the specifier while building #94, and verified: nothing in Trellis can mark a task done — not the implementation, and not the canvas.** The only removal control the design draws is `Delete session`, for a logged quota session.

---

## Goal

**A task you have done leaves the screen it lives on, and stays gone.**

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

1. Capture and triage three errands tagged `@homedepot`. **Pool** shows them as a trip — three things, one stop.
2. Go to Home Depot. Come back. **Mark two done.**
3. Reload. **`@homedepot` is no longer a trip** — one item left, below the threshold, so it drops to loose ends still carrying its tag.
4. Restart the server. The two you did have not come back.

**Step 3 is the point.** Without it the screen keeps sending you on a trip you already made.

## Why this is urgent and not merely missing

`D-dogfood-first` targets daily use by **2026-09-03**. A Pool screen that only grows is worse after a week than no Pool screen — and worse than it looks, because of `T-trips-are-derived-not-ranked`: **completed items keep counting toward the trip threshold**, so the grouping that makes Pool worth opening is the first thing to rot.

**The capture side has an exit and the task side does not.** #48 gave the inbox a "no" and `T-capture-leaves-inbox-once` made it exactly-once. Once a capture becomes a task, it is permanent.

## The modelling question — this is the slice, not the route

**Done and killed may be two states, and `archived_at` was designed for one.**

| | |
|---|---|
| **Killed** | You decided not to do it. `D-kill-means-archive` keeps the row **because the row is signal** — *"47 archived this quarter, 31 Learning"*. |
| **Done** | You did it. `D-quota-no-rollover`'s *"you did 1 of 3 runs last week"* depends on this being distinguishable. |

`T-archived-at-only` removed `status: dropped` **precisely to avoid two fields for one state**. But if these are two *states*, one timestamp cannot carry both and M8's reckoning loses its denominator before it is built.

**`T-capture-leaves-inbox-once` answered the identical question one row over**, and is where to start: `captures.triaged_at` became `left_inbox_at` — **one column, with which exit derivable from whether a `tasks` row references it**, rather than two nullable timestamps permitting an impossible both-at-once state. **Ask what plays the role of that foreign key here.**

## The canvas does not draw this control

Its complete set of `aria-label`s is `Raise priority` ×3, `Lower priority` ×3, `Minutes`, `Day`, `Save capture`, `Hours a week`, `Delete session`.

`D-four-screens` makes the canvas authoritative on **layout** — so **this is a genuine gap in the design, not a disagreement with it.** Raise it. **Do not invent a control and describe it as specified**; say what you added and that the canvas is silent, so the owner can correct it.

**One constraint that is not silent:** `pool-screen-nothing-reorders-05` asserts the absence of every reorder arrow and its QA document says finding one is a defect (#95 owns those). **A done control must not arrive as an arrow, and must not weaken that assertion.**

## Scope

**In:** marking a task done from the screen it lives on (Pool, and Committed if #94 has landed); the task leaving that screen; surviving a restart; the trip threshold reflecting it.

**Out — and each is refused by a decision, not merely deferred:**
- The decay pass, three-strike, the weekly review — **M8**
- **Un-doing.** `D-inaction-archives`: survival requires a deliberate act
- **Any browsable list of completed work.** `D-kill-means-archive`: *"the moment an archive is browsable it becomes a place to hide from decisions"*
- Quota logging (#93), reordering (#95), anything the canvas does not draw beyond the control this slice must add

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `T-capture-leaves-inbox-once` | **The precedent.** One column; the discriminator derived rather than stored. |
| `T-archived-at-only` | One signal, not two fields for one state — **and read it carefully, because it may be two states here.** |
| `D-kill-means-archive` | Keep the row. **No browsable archive.** |
| `D-inaction-archives` | Survival requires a deliberate act. **No un-do.** |
| `T-trips-are-derived-not-ranked` | Trips are derived at three items. A done task must stop counting. |
| `D-logging-is-retrospective-and-separate` | **Completing an item may offer to log time and never does it silently.** #93 meets this next — leave the seam clean. |
| `T-cross-capability-invariants-need-an-owner` | **From #92, and it bites here**: if a fixture marks a task done by writing the column directly, it skips the path the screen depends on. **Fixtures must go through the production route.** |
| `T-qa-binds-tolerantly-to-markup` · `T-latency-is-a-qa-assertion` · `T-migrations-append-only` | Unchanged. **You take the next free migration.** |

## Acceptance scenarios worth specifying

- A pool task marked done **leaves the Pool screen** and does not return after a restart.
- **A trip that drops below three items becomes loose ends, keeping its tag** — the case that makes this slice worth shipping.
- **A done task is distinguishable from a killed one**, however you modelled it — or, if you chose one state, the scenario says so and the reasoning is on the pull request.
- Done tasks are excluded from every count that a screen shows.
- **No un-do control exists**, asserted rather than inferred — the same shape as `pool-screen-nothing-reorders-05`, and for the same reason: a coder would add one in good faith.
- Hostile text stays escaped.
- **All existing features pass.**

## Known repo gotchas

1. **`trunk` moves under slices.** Merge it before your final measurement, not after.
2. **A restyle has broken QA scripts three times without CI noticing** (`T-qa-binds-tolerantly-to-markup`). You are adding a control to a styled screen.
3. **`platform/boundary.rs` asserts `capabilities.len() >= 5`** and passes by counting `platform`, documented as not a capability.
4. **Four named Gherkin traps**, and #92 hit the fourth in **unit fixtures**: a property is only as strong as the inputs its generator can produce; *if a scenario's point is that nothing happens, its parameters are not under test*; asserting only the outcome where several causes collapse into it; **reaching the right end state by the wrong path.**
5. **No browser automation, and no phone.** Say what is uncovered.
6. `trunk` expects four required status checks. Base **`trunk`**; scratch in `./tmp/`.
7. **Open the pull request when QA is done.**

## Open questions

Answer in the pull-request body, in a table, with reasoning, and **name any `T-` row you need in your handoff note.**

1. **One state or two?** See above. Start from `T-capture-leaves-inbox-once`.
2. **Where does the control go, given the canvas is silent?** Say what you added and that the design does not draw it.
3. **What does done mean for a quota item?** `D-logging-is-retrospective-and-separate` half-specifies it already — completing **may offer** to log time, never silently. **#93 meets this next; do not build quota logging, but do not leave a seam it has to unpick.**
4. **Does a done task still belong to its context tag?** It leaves the screen either way. **But the answer decides whether "done" is a filter over one list or a different list** — and that shapes what M8's reckoning can ask later.

## Dependencies

- **Blocked by #94** only for the tab bar and the Committed screen, if a done control belongs there too.
- **#93 follows this**, per the owner.

## Source

`docs/decisions.md` — `T-capture-leaves-inbox-once`, `T-archived-at-only`, `D-kill-means-archive`, `D-inaction-archives`, `T-trips-are-derived-not-ranked`, `D-logging-is-retrospective-and-separate`, `D-dogfood-first` · `docs/design/Trellis.dc.html` — silent on this · #94 — where it was found · #95 — the reorder assertion this must not weaken
