# Handoff brief — `context-tags`

**Date:** 2026-08-21 · **Issue:** #82 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

---

**A context tag cannot be backfilled.** Every day of capture without one is a day of items that can never be grouped by where they can be done. That is why this is first and why it was the first slice of the re-plan.

> **Halved by #88, 2026-08-21.** This issue used to carry *"and life area stops being required"*. **#88 already did that** — `life_areas` is gone, pool triage is now a single field (`kind`), and `tasks.life_area_id` stays in the schema unread per `T-migrations-append-only`. **Only the context tag remains.**
>
> **Its earlier pull request (#89) is closed.** It was built against modules #88 removed — 19,011 lines of them. **Do not resurrect that branch;** two commits worth keeping were already salvaged into #88. Specify against the tree as it is now.

## Goal

**A capture carries a free-text context tag, autocompleting on prior values.**

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

1. Quick-add **"buy screws"** with the context `@homedepot`. It saves.
2. Quick-add **"return the drill"**. Start typing `@home` — **`@homedepot` is offered from what you typed before.** Accept it.
3. Triage both as pool. **One tap each** — pool asks for nothing else now.
4. Quick-add **"pick up milk"**, tag `@supermarket`.
5. Restart. All three keep their tags.

**Step 2 is the point.** The tag is only useful if the second errand at a place lands in the same bucket as the first, and it will only do that if typing is cheap.

## Scope

**In:** a context tag on a capture — free text, **optional**, autocompleting on prior values; surviving a restart; visible wherever a capture or task is listed.

**Out:** grouping the Menu by context (#85 — this only supplies the key), the at/by distinction, quota reshaping, hour logging, and **any visual design system**. A design spec is coming from the owner and **#85 is held for it.**

## The tree you are specifying against

`#88` removed five server modules, six core modules and ten features. **`scheduler-core` is now `interval`, `schedule`, `task`, `timezone`; the server is `capture`, `dismiss`, `inbox`, `platform`, `settings`, `triage`.** Thirteen features. One screen — there is no nav, because `T-nav-is-the-site-map` renders nothing for a site map of one.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-context-tags-are-the-taxonomy` | Context tags are the product's **only** taxonomy — numerous, cheap, disposable. **Free text is deliberate**, and it is the opposite of the life-area rule that just came out: a typo costs one badly-grouped item, not an unschedulable one. Autocomplete on prior values is enough structure. **Read the row before proposing a managed set.** |
| `D-quotas-are-selected-not-typed` | **The deliberate asymmetry.** A quota is picked, never typed, because a mistyped quota name splits a week's hours across two counters and makes both wrong. A mistyped context tag mis-files one item. **Same product, opposite rules, for a reason.** |
| `D-dogfood-first` | Daily use by **2026-09-03**. |
| `T-empty-equals-absent` | Empty string and absent key report identically. The tag is **optional** — omitting it must not be a rejection. |
| `T-required-fields-are-specified-per-transport` | A field is not specified until every transport that supplies it is. #73 shipped a page whose committed form could never succeed while 21 features stayed green. **Specify the form, not only the boundary.** |
| `T-latency-is-a-qa-assertion` | The 50 ms capture budget lives in QA now, not in `capture_endpoint.feature`. **You are adding a field to the endpoint that budget describes** — if the QA reading moves, say so. |
| `T-migrations-append-only` | New migration only. `0009` is the latest; **you take `0010`.** |
| `T-forms-swap-one-fragment` · `T-422-is-product-wide` · `T-templates-take-view-models` · `T-capability-owns-its-queries` · `T-one-front-door-per-capability` | Unchanged. |

## Acceptance scenarios worth specifying

- A capture is created **with** a tag and keeps it; **without** one and is accepted.
- **Autocomplete offers prior values on a prefix.** Say what drives it — open question 2.
- Case and whitespace: decide whether `@HomeDepot` and `@homedepot` are one tag — open question 1.
- Hostile text in a tag stays escaped wherever it renders.
- **All 13 existing features pass.**

## Known repo gotchas

1. **`trunk` is green at `c95192b`.** Merge `trunk` before your final measurement, not after.
2. **Expect 13 features.** The tree shrank by 19,011 lines yesterday.
3. **PR #87's drift was invisible to CI and only QA re-running found it** — a restyle touched three templates and no test surface, breaking the exact-match regex QA scripts use to find a capture row. **You are editing `capture_row.html`. Re-run QA and believe the result.**
4. **`platform/boundary.rs` asserts `capabilities.len() >= 5` and finds six, one of which is `platform`** — documented as *"not a capability"*. It passes by counting a non-capability. **Adding a capability is fine; be aware the floor is at its limit.**
5. **DRY is 1.9%**, the lowest this project has recorded, because the denominator lost 3,650 lines. Plenty of room — report the number anyway.
6. **Four named Gherkin traps**, and the newest is live: *if a scenario's point is that nothing happens, its parameters cannot be under test* — nine mutants survived `one_screen` on exactly that. Also: a property is only as strong as the inputs its generator can produce; asserting only the outcome where several causes collapse into it; reaching the right end state by the wrong path.
7. **No browser automation.** Say what is uncovered.
8. `trunk` expects four required status checks. Base **`trunk`**; scratch in `./tmp/`.
9. **Open the pull request when QA is done.**

## Open questions

Answer in the pull-request body, in a table, with reasoning, and **name any `T-` row you need in your handoff note** — three slices running have done that and it is why this project's rationale stopped being archaeology.

1. **Are `@HomeDepot` and `@homedepot` one tag?** `T-collation-enforces-name-identity` made life-area names trim-and-case-fold **in the column**, because two indistinguishable picker entries was the failure. **That decision died with #88** — but the reasoning may transfer, and autocomplete makes near-duplicates visible in a way a picker did not. *(The closed PR #89 chose canonicalising to the first spelling used. Reach your own answer; it is available as prior art, not as precedent.)*
2. **What drives autocomplete?** A `datalist` of prior values needs no JavaScript and no endpoint, and ships the whole list every time. An endpoint filtering on a prefix scales and adds a route. **The owner will have tens of tags, not thousands** — say which, and at what size it stops being right.
3. **Is the tag on the capture, the task, or both?** A capture becomes a task at triage. If it lives only on the capture, a task's grouping is a join away and dismissed captures carry tags nothing reads. `T-capture-leaves-inbox-once` is the precedent for asking which row owns a fact.
4. **Does the tag belong on the triage form too, not just quick-add?** Tagging at capture is the fast path; the tag is often only obvious later. Cheap to allow both — say whether you did.

## Dependencies

- **Nothing blocks this.** #88 merged, `trunk` green, pipeline empty.
- **Blocks #85**, which groups by context.

## Source

`docs/decisions.md` — `D-context-tags-are-the-taxonomy`, `D-dogfood-first`, `D-quotas-are-selected-not-typed`, `T-dead-core-code-earns-its-keep` · #88 / PR #90 — the tree you build on · #89 (closed) — prior art, not precedent
