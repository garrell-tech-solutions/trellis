# Handoff brief — `committed-screen`

**Date:** 2026-08-21 · **Issue:** #94 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

---

**The third screen, and the one where the canvas and the decisions log disagree.** `D-four-screens` · the `isCommitted` guard in `docs/design/Trellis.dc.html`.

> **Read the canvas yourself, by construct.** The previous brief for #92 paraphrased it from a `grep` and was wrong in four places — the trip threshold, the group ordering, `poolMeta`, and twice asserting no reorder control where six are drawn. **Everything below cites the canvas by name so you can check me.**

## Goal

**Committed items in date order, on their own tab.**

## What the canvas actually draws

The `isCommitted` guard, verbatim in structure:

- **Title `Committed`**, with `committedMeta` — `committed.length + " dated"`, or **`"nothing dated"`** when empty.
- **Empty state:** *"Nothing with a time on it. That is allowed."* with **`Go to Capture →`**. **Keep that sentence verbatim** — it tells the owner an empty Committed screen is success, not neglect.
- **A row per item**, three columns:
  - `it.at` — **66px fixed, uppercase, `font-variant-numeric: tabular-nums`**
  - `it.text` — the item
  - `it.ctxLabel` — **the context tag**, right-aligned, uppercase, small

**Committed items carry a context tag.** `ctx` is `"desk"`, `"phone"`, or absent. **Context is not pool-only**, which #92 might have led you to assume.

**Ordering is `a.ord - b.ord`** — an explicit field, **not derived from the date string**. The canvas's rows carry `ord` of 3, 1, 4 with dates Thu / Tue / Fri, so `ord` and chronology agree in the fixture but are not the same thing.

## ⚠️ The canvas and the decisions log disagree, and this is the slice's central question

`D-committed-is-at-or-by` says a committed item is either an **at** — a fixed block, *2pm dentist* — or a **by** — a deadline with slack. *"They display differently and behave differently."*

**The canvas draws no such distinction.** Its three committed rows are:

```
Q3 planning doc          desk    at "Thu 17:00"
Book the dentist         phone   at "Tue 8:30"
Furnace service window   —       at "Fri 13:00"
```

**All three carry a day and a time.** There is no badge, no second column, no `isAt`, no `byDate` — the string `"at"` appears nowhere in the canvas as a concept.

`D-four-screens`: **the canvas is authoritative on layout, `docs/decisions.md` on behaviour.** The at/by distinction is behaviour, so it stands — but the canvas does not say how it looks. **Raise this rather than resolving it quietly.**

**One reading, offered as inference and not as instruction:** a 66px `tabular-nums` column suits both `THU 2:00PM` and a bare `FRI`, so *at* and *by* may differ by **whether the cell carries a time at all**. That would need no badge and no extra column. **It is a guess. Say what you chose and why.**

## Scope

**In:** a `/committed` route and screen; the at/by distinction in the data model; date-ordered listing; the context tag on the row; the empty state; the third tab.

**Out:** Quota (#93), reordering (#95 — **and `pool-screen-nothing-reorders-05` asserts the absence of every arrow; do not add one here**), the scheduler, and anything the canvas does not draw.

## `deadline_type` is owed a reading

Today a committed task carries `deadline` plus `deadline_type(hard | soft)`. `T-hard-refuses-soft-slips` gave that field its **only** behaviour — hard refuses, soft slips with a projected finish — **and that behaviour lives in the scheduler, which `D-dogfood-first` paused.**

So `deadline_type` is once again a field that changes nothing, which is the exact state U2 complained about before `T-hard-refuses-soft-slips` fixed it. **Does it survive alongside at/by, or does at/by replace it?** Deletion is on the table; this project has removed `same_name`, `scheduler_core::ratio` and a `TriageRejection` payload on precisely that ground. **Answer deliberately and name the `T-` row you need.**

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-committed-is-at-or-by` | **At** is a fixed block; **by** is a deadline with slack. They display and behave differently. **The behaviour stands even though the canvas does not draw it.** |
| `D-four-screens` | Canvas authoritative on layout, decisions log on behaviour. **Raise disagreements.** |
| `T-cross-capability-invariants-need-an-owner` | **New, from #92, and it applies directly**: `ctxLabel` here means this screen consumes the same canonicalised tag key, so it inherits the same cross-capability dependency — and **fixtures that seed a tag with its spelling already final skip the step the screen depends on.** |
| `T-qa-binds-tolerantly-to-markup` | Bind QA to ids and owned classes, never attribute order or copy. |
| `T-trips-are-derived-not-ranked` | Trips and groups never carry a priority control. **Committed is neither**, but the canvas's `pri` field is a trap — do not wire it. |
| `D-dogfood-first` | Daily use by 2026-09-03. **A *by* item is exactly what a scheduler would have slack to place into**, so this screen is where M3's evidence accumulates. |
| `T-latency-is-a-qa-assertion` · `T-required-fields-are-specified-per-transport` · `T-migrations-append-only` | Unchanged. **You take the next free migration.** |

## Acceptance scenarios worth specifying

- Committed items appear **in date order**, each with its date cell, text, and context tag where it has one.
- **An item with no context tag renders without one** — the canvas's third row has `ctx: null`.
- **At and by are distinguishable**, however you chose to draw them.
- The empty state renders verbatim.
- **Pool and quota tasks do not appear here.**
- The tab bar shows **three** tabs and marks the current one — `every_header_link_reaches_the_page_it_names` walks `nav::ALL` over the real router, so a typo'd path fails rather than shipping a dead link.
- **`one_screen`'s `path` column gains a third `200` row.**
- Hostile text stays escaped.
- **All 15 existing features pass.**

## Known repo gotchas

1. **`trunk` moves under slices constantly.** Merge it before your final measurement, not after.
2. **Expect 15 features** (Pool's PR #96 may merge first — check).
3. **A restyle has broken QA scripts three times without CI noticing.** You are adding a screen to that stylesheet.
4. **`platform/boundary.rs` asserts `capabilities.len() >= 5`** and passes by counting `platform`, which is documented as not a capability. Adding one helps.
5. **Four named Gherkin traps**, and #92 hit the fourth in **unit fixtures**: a property is only as strong as the inputs its generator can produce; *if a scenario's point is that nothing happens, its parameters are not under test*; asserting only the outcome where several causes collapse into it; **reaching the right end state by the wrong path.**
6. **No browser automation, and no phone.** This is the third phone-first screen shipped unverified on a real device. **Say so.**
7. `trunk` expects four required status checks. Base **`trunk`**; scratch in `./tmp/`.
8. **Open the pull request when QA is done.**

## Open questions

Answer in the pull-request body, in a table, with reasoning, and **name any `T-` row you need in your handoff note.**

1. **How do at and by differ on screen?** See above. The canvas does not say; the decision says they must.
2. **Does `deadline_type` survive?** See above.
3. **What orders the list?** The canvas sorts by `ord`, a field with no derivation shown. **Chronological is the obvious answer and the canvas does not quite say it** — the fixture's `ord` agrees with date order but is a separate field. Say which you built and whether `ord` has any meaning beyond it.
4. **Does an *at* in the past still show?** *"Nothing with a time on it"* is the empty state; a dentist appointment from last Tuesday is neither nothing nor useful. The canvas has no notion of past.

---
## Source
`docs/design/Trellis.dc.html` — the `isCommitted` guard, its three fixture rows, `committedMeta`, and the `a.ord - b.ord` sort · `docs/design/README.md` · `docs/decisions.md` — `D-committed-is-at-or-by`, `D-four-screens`, `T-hard-refuses-soft-slips`, `T-cross-capability-invariants-need-an-owner`, `D-dogfood-first` · #92 / PR #96 — the tab bar you extend
