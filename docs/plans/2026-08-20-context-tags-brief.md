# Handoff brief — `context-tags`

**Date:** 2026-08-20 · **Issue:** #82 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **First slice of the re-plan** (`D-dogfood-first`). The target is the owner using Trellis every day within two weeks, and this is the only slice that is time-critical: **a context tag cannot be backfilled.**

---

**Dogfood slice 1 of 5, and the one that cannot wait.** A context tag cannot be backfilled: every day of capture without one is a day of items that can never be grouped by where they can be done.

Also drops the life-area requirement, because both changes are on the same triage path and shipping them apart means editing the same form twice.

## Goal

**A capture carries a free-text context tag, autocompleting on prior values — and triage stops demanding a life area.**

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

1. Quick-add **"buy screws"**. Type `@homedepot` in the context field. It saves.
2. Quick-add **"return the drill"**. Start typing `@home` — **`@homedepot` is offered from what you typed before.** Accept it.
3. Triage both as **pool**. **Neither asks for a life area.**
4. Quick-add **"pick up milk"**, tag `@supermarket`, triage as pool.
5. Restart. All three keep their tags.

**Step 2 is the point.** The tag is only useful if the second errand at a place lands in the same bucket as the first, and it will only do that if typing is cheap.

## Scope

**In:**
- A **context tag** on a capture — free text, optional, autocompleting on prior values
- **Life area is no longer required at triage** for any kind
- The tag survives a restart and is visible wherever a capture or task is listed today

**Out — later slices, do not absorb:**
- Grouping the Menu by context (Menu slice)
- Removing the life-areas, guardrail, free-time, capacity or exception modules — **deferred deliberately until after dogfooding** (`D-context-tags-are-the-taxonomy`)
- Mobile layout, viewport, home-screen install (next slice)
- at/by, quota reshaping, hour logging
- **Any visual design system.** A design spec with screen layouts is coming from the owner. **Do not invent one.**

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-context-tags-are-the-taxonomy` | Context tags are the product's only taxonomy — numerous, cheap, disposable. **Free text is deliberate**: a typo costs one badly-grouped item, not an unschedulable one, and autocomplete on prior values is enough structure. **Read the row before proposing a managed set.** |
| `D-dogfood-first` | The whole re-plan. Target is the owner using this daily by **2026-09-03**. |
| `T-life-areas-are-data` · `T-life-area-required-at-triage` | **Both superseded.** The modules stay in the tree; only the requirement goes. |
| `T-empty-equals-absent` | An empty string and an absent key report identically. A tag is **optional** — omitting it must not be a rejection. |
| `T-required-fields-are-specified-per-transport` | Changing what triage requires is not specified until **every transport** is. #73 shipped a page whose committed form could never succeed while 21 features stayed green, because the suite triages over JSON. **Specify the form, not only the boundary.** |
| `T-migrations-append-only` | New migration only. `0009` is the latest; **you take `0010`.** |
| `T-forms-swap-one-fragment` · `T-422-is-product-wide` · `T-templates-take-view-models` · `T-capability-owns-its-queries` · `T-one-front-door-per-capability` | Unchanged. |

## Acceptance scenarios worth specifying

- A capture is created **with** a context tag and keeps it.
- A capture is created **without** one — accepted, not rejected.
- **Autocomplete offers prior values**, and offers them on a prefix. Say how it is driven; see open question 2.
- **Triage succeeds with no life area, for all three kinds.** This is the change most likely to be specified at the boundary and forgotten in the form.
- Tags are **case- and whitespace-sane**: decide whether `@HomeDepot` and `@homedepot` are one tag, and say so — see open question 1.
- Hostile text in a tag stays escaped wherever it renders.
- **All 22 existing acceptance features pass** — several assert a required life area and will need their fixtures corrected. **Confirm each is fixture drift, not regression**, by reproducing it.

## Known repo gotchas

1. **`trunk` is green at `69de976`.** Rebase or merge `trunk` before your final measurement, not after.
2. **Expect 22 features.** Several will legitimately change — `life_area_triage`, `committed_triage_validation`, `task_kinds`, `triage_from_page` all assert the requirement you are removing. That is the rule working, not drift; say which and why.
3. **⚠️ DRY sits near the fail-closed 3% ceiling** and has arrived red in three of the last six slices, fixed by real deduplication each time. That is the standard.
4. **`platform/boundary.rs` enforces four rules** by walking `src/`.
5. **A new step module means a new dispatch row**, CI-gated with exact scores.
6. **Four named Gherkin traps:** a property is only as strong as the inputs its generator can produce (#73, #81); if a scenario's point is that nothing happens, its parameters are not under test (#71); asserting only the outcome where several causes collapse into it (#69); reaching the right end state by the wrong path (#70).
7. **No browser automation in this stack.** Say what is uncovered.
8. `trunk` expects four required status checks. Base **`trunk`**; scratch in `./tmp/`.
9. **Open the pull request when QA is done.**

## Open questions for you

Answer in the pull-request body, in a table, with reasoning, and **name any `T-` row you need in your handoff note.**

1. **Are `@HomeDepot` and `@homedepot` one tag?** `T-collation-enforces-name-identity` made life-area names trim-and-case-fold **in the column**, because two indistinguishable picker entries was the failure. A context tag is typed, not picked, so the same argument may or may not apply — but autocomplete makes near-duplicates visible in a way a picker did not. Say what you chose and where the check lives.
2. **What drives autocomplete?** A `datalist` of prior values is the cheapest thing that works with no JavaScript and no new endpoint, and it is the whole list every time. An endpoint filtering on a prefix scales and adds a route. **The owner will have tens of tags, not thousands** — say which you picked and at what size it stops being right.
3. **Is the tag on the capture, the task, or both?** A capture becomes a task at triage. If the tag lives only on the capture, a task's grouping is a join away and dismissed captures keep tags nothing reads. `T-capture-leaves-inbox-once` is the precedent for thinking about which row owns a fact.
4. **Does the tag belong on the triage form as well as the quick-add box?** Tagging at capture is the fast path, but the tag is often only obvious later. Cheap to allow both; say whether you did.

## Dependencies and sequencing

- **Nothing blocks this.** It is the first slice of the re-plan and the only one that is time-critical.
- **Blocks the Menu**, which groups by context.
- **Does not touch** the life-areas, guardrail, free-time, capacity or exception modules. Their removal is deferred until after dogfooding.

## Source

- `docs/decisions.md` — `D-context-tags-are-the-taxonomy`, `D-dogfood-first`, `D-menu-is-a-worklist`
- **#12** — the Menu, which this feeds · **#11** — M3, paused pending evidence
