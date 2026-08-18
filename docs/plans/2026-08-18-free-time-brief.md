# Handoff brief — `free-time`

**Date:** 2026-08-18 · **Issue:** #60 · **Milestone:** M2 — Guardrails + Free-slot Algebra · **Route:** pipeline

> **M2 slice 2 of 4, and the milestone's hard part.** `guardrails` (#59, PR #69) merged; the owner can now say *when* each life area's work may happen. This slice is what turns that into **when am I actually free** — and it is where M2's property tests live.
>
> **It is also the timezone's first reader.** `T-timezone-is-a-setting` landed the setting and deliberately made it do nothing observable beyond being stored and shown. That was correct, and it means this slice is the first place a wrong answer about time becomes visible.

---

## Demo

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

1. Set the timezone to your own on the **Life areas** page, and give **Work** `Mon–Fri 09:00–17:00`.
2. Click **Free time** in the header. Your next 14 days, per life area, with totals: *"Work — Mon 09:00–17:00, Tue 09:00–17:00 …, 40h"*.
3. Change Work's guardrail to end at `16:00`. Reload — **every Work day is an hour shorter and the total dropped by five.**
4. **Spring forward.** Restart just before a DST transition and reload:
   ```sh
   cargo run -p trellis-server -- serve --db trellis.db --now 2027-03-13T12:00:00-05:00
   ```
   A guardrail spanning `02:00` on transition day shows the correct wall-clock hours — no panic, no negative interval, no phantom hour.
5. **Fall back.** Same at the autumn transition; the repeated hour resolves the way this slice **documented**, not the way `jiff` happened to answer first.

## Goal and scope

**`free_intervals(guardrail, range)`** — project a life area's weekly civil-time guardrail across a date range, subtract what is taken, return **disjoint, sorted** intervals. **Correct across DST gaps and folds**, which are ordinary cases here, not edge cases.

### What `guardrails` already built, so you do not rebuild it

`scheduler_core::guardrail` exists and is yours to extend, not to duplicate:

```
Weekday, Band, BandSpan, AuthoredBand<T>
group(bands)                 bands grouped by their authoring key
overlaps(existing, candidate) within-life-area overlap check
```

`scheduler_core::timezone::validate` exists. `settings::current_timezone` is the front door for the owner's zone (`T-one-front-door-per-capability`) — **read it through that, never from `settings::store`.**

### What "busy" means at M2 — stated so it is not discovered

**Nothing supplies `busy`, `pins` or `buffers` yet.** Pins arrive at M3 (`T-pins-in-constraints`), calendar busy at M4, and **dated exceptions in the very next slice (#61)**. So:

- The **algebra** is built and property-tested here against **generated** interval sets. A proptest supplies its own inputs and does not need a production source.
- The **only user-visible subtraction at M2** arrives in #61. That is deliberate sequencing.
- **Do not stub a fake busy source to make the demo richer.** The demo above is the mask projected correctly across a range and a DST boundary, which is the genuinely hard part.

### Out of scope — do not absorb

Dated exceptions (#61), the capacity view (#62), `allowed_windows` / cross-life-area borrowing (M3), placing anything, pins, calendar busy, and **any visual design system**. **Do not build a scheduler** — this answers *when are you free*, never *what should go there*.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `T-jiff-epoch-millis` | **The load-bearing decision for this slice, and the reason it was made.** *"Guardrails are civil wall-clock. `jiff` models zoned vs civil time as distinct types and forces an explicit decision at a DST gap."* Store UTC epoch millis, convert at the boundary. |
| `T-timezone-is-a-setting` | One stored zone for the product, defaulting to **UTC**, read through `settings::current_timezone`. **You are its first reader.** |
| `D-life-area-owns-its-time` | Guardrails **may overlap between life areas**, and overlap is not an error — both life areas offer that time and they compete at scheduling, which does not exist yet. At this slice they simply both show it. |
| `T-guardrail-well-formedness` | A life area's own bands may **touch but not overlap**, so within one life area you can rely on disjointness. A life area marked *never scheduled* has no hours — see the empty-answer criterion. |
| `T-nav-is-the-site-map` | **Every page in the route table gets a header link.** This slice adds a page, so it adds a `nav::Page` variant **and a row in `nav::ALL`** — see gotcha 3. |
| `T-422-is-product-wide` | `422` means exactly one thing: a validation rejection whose body is the re-rendered fragment. An endpoint that cannot honour that must not return 422. |
| **Core vs adapter** | `free_intervals` is the clearest case in this product of *a rule that survives changing HTTP*. It belongs in **`scheduler-core`**, the enforced-pure boundary — `cargo tree -p scheduler-core` must still contain no `tokio` and no `sqlx`. |
| `T-templates-take-view-models` | The page renders a view model, not a core type verbatim if the shapes differ. |
| `T-complexity-8` | Threshold 8, CI-gated. **Interval arithmetic is where a genuine branching thicket is most likely** — this is the slice most likely to earn a legitimate baseline row, and *"adding a row here is a decision, not a formality."* |
| **No visual design system** | Plain and ugly remains correct. |

## Acceptance scenarios worth specifying

- `free_intervals(guardrail, range)` returns intervals that are **disjoint**, **sorted**, and each a subset of `mask − busy − pins − buffers` — asserted by **proptest, ≥1000 cases**.
- **DST gap:** a guardrail spanning a spring-forward transition yields correct wall-clock intervals, with no panic and no negative-length interval.
- **DST fold:** a guardrail spanning a fall-back transition resolves to a **documented, tested choice**. The choice goes in `docs/decisions.md`, not only in a comment.
- **Empty is a real answer**, not a failure: a life area marked *never scheduled*, or a range containing none of a guardrail's bands, returns no intervals.
- **Overlapping guardrails each report their own free time** — two life areas sharing clock time both offer it.
- Changing the timezone changes what the page reports, and **the same guardrail reports different instants in different zones** — the first observable consequence of `T-timezone-is-a-setting`.
- **All 18 existing acceptance features pass untouched.**

## Known repo gotchas

1. **`trunk` is green at `1a52305`** (the #69 merge). Confirm CI on your base commit.
2. **Expect 18 features.** Run the analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh` — the first two report nonsense without the generated acceptance entrypoints.
3. **Adding a page: `platform::nav::Page`'s `label` and `path` are exhaustive matches, so the compiler sends you there — but `nav::ALL` is the one step it cannot force.** A page missing from that list compiles and ships with **no link**. `app::every_header_link_reaches_the_page_it_names` walks `ALL` through the real router.
4. **⚠️ DRY has almost no room, and the number understates the risk.** Measured on the merged tree at `8d539d8`: aggregate **2.87%** against threshold 3 — but **Rust alone is 3.43%**, over the line, diluted under it by a 5,196-line shell corpus. **This slice adds Rust and comparatively little shell**, so it pushes the aggregate toward the threshold faster than the headroom suggests. #68 also closed the gate's fail-open, so it can no longer pass by measuring nothing. Raised as **#52**; not yours to fix, but budget for it and expect to have to deduplicate rather than discovering it at the gate.
5. **`platform/boundary.rs` enforces four rules**, walking `src/`: no persistence module names a delivery type; nothing outside a `store.rs` or `platform/db.rs` writes production SQL; no top-level directory carries a technical-role name; **no capability names another capability's `store` in production**. Read the timezone through `settings::current_timezone`, not `settings::store`.
6. **You probably need no migration.** This slice computes and renders; it stores nothing. `0006` is the latest. If you find yourself writing `0007`, something has gone wrong — say what.
7. **A new step module means a new dispatch row, CI-gated.** `scripts/ci/complexity-baseline.json` pins exact scores and fails when the set grows, when a score moves *in either direction*, or when a row goes stale. **Keep every dispatch arm a one-line delegation.** Do not flatten any dispatcher into a `(Regex, handler)` table.
8. **Two Gherkin traps, both of which bit the last slice** — twelve mutants survived #59's own scenarios:
   - **Do not assert only the outcome when the implementation collapses several causes into it.** #59 had two scenarios asserting *"rejected, and still shows no guardrail"* while an unparseable time, an `end <= start`, and a missing weekday all produced that — so **no Examples value was under test**. Assert the *reason*, or write literal scenarios.
   - **Do not echo a placeholder in both the input and the assertion** (`task_kinds.feature`'s anti-pattern), and check the mutator's axis: `jiff` resolves zone names case-insensitively, so a case-flip mutation on a zone name is a no-op by construction.
9. **`cargo test --workspace` compiles zero acceptance tests on a fresh checkout** — entrypoints are gitignored. Run `scripts/acceptance/run.sh` yourself.
10. **`trellis serve --now <RFC3339>` is an offset, not a freeze** — the server starts believing it is that instant and time advances normally. `scripts/qa/lib.sh`'s `qa_start_server` takes it as an optional fourth argument. **This slice needs it**, for the DST demos.
11. **#66 is open and not yours:** the 50 ms capture budget is load-sensitive. If it fails under load, re-run quiet and say so.
12. **There is no browser automation in this stack.** Anything only visible in a browser is uncovered; say so rather than implying otherwise.
13. **`trunk` now expects four required status checks** (#68).
14. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
15. **Open the pull request when QA is done**, before taking another brief.

## Open questions for you

Answer them **in the pull-request body, in a table, with reasoning**. Four consecutive slices have now done this and it is why this project's rationale stopped being written from archaeology. #59 went further and reported a weakness in its own scenarios that no reader would have found — that is the bar.

1. **What does a fold do?** A guardrail covering `01:00–03:00` on a fall-back day meets `01:30` **twice**, and both instants are legitimately inside the civil band. Offer both hours, or the first only? **This wants recording in `docs/decisions.md` with its reasoning** — it is precisely the class of thing `T-jiff-epoch-millis` says must be an explicit decision, and once made it is invisible in the code.
2. **What is the range's default and shape?** M2's epic says 14 days. Fixed constant, parameter, or config? `/stats` already hard-codes a 14-day window **for a different purpose** — say whether these are the same number **by rule or by coincidence**, because #62's capacity view makes it three.
3. **Where does the interval type live, and does it already want to be shared?** M3's `schedule()` consumes free intervals and M4's calendar sync produces busy ones. An interval type invented here is one both inherit — which is an argument for putting it somewhere deliberate now rather than moving it twice.
4. **Is the free-time page permanent, or scaffolding?** It is the thinnest surface that exposes the algebra, which is why it exists — and it may also be genuinely useful forever. Say which you think it is; it changes how much it deserves.

## Dependencies and sequencing

- **Nothing blocks this.** #59 merged, `trunk` green, pipeline empty.
- **Blocks #61** (dated exceptions — its entire visible effect is on these intervals) and **#62** (capacity — its supply number is the sum of these).
- Its interval algebra is consumed by **M3** (#11) and **M4** (#14).
- **#63 and #65 are ops PRs** that may land independently; if #63 lands first, your new code is inside a stricter module boundary.

## Source

- Issue **#60** — acceptance criteria and the demo
- Issue **#10** — M2 epic, the cut, and the two costs flagged during it
- `docs/decisions.md` — `T-jiff-epoch-millis` above all, plus `T-timezone-is-a-setting`, `D-life-area-owns-its-time`, `T-guardrail-well-formedness`, `T-nav-is-the-site-map`
- `docs/design/architecture.md` — Guardrails and free time; the module boundary
- **#59** / PR #69 — what landed, and the twelve mutation survivors worth not repeating
- **#52** — why the DRY number understates the risk
