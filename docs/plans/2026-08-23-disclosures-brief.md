# Handoff brief — `disclosures`

**Date:** 2026-08-23 · **Issue:** #119 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **Found by the owner using the product** — the third such defect in two days, after the scroll and the UTC timestamp. **Small, and it is pure friction on the thing you do most often.**

---

## Demo

**On the phone.** Label your pull request `preview` and the box puts it on `:8443` within two minutes.

1. Capture two things, so the inbox has **two rows**.
2. On the first row, open **Committed**. Then open **Quota**. Today: **both stay open.**
3. Inside Committed, open **At a time**, then **By a day**. Today: **both stay open too.**
4. So one capture row can stack three sets of fields on a 390px screen.
5. After: choosing one input method puts the others away. **And opening Committed on the second row leaves the first row alone.**

**Step 5's second sentence is the whole risk in this slice.** See below.

## Goal and scope

**Choosing one input method closes the others, within a capture row and only within that row.**

`templates/capture_row.html` has four disclosures and **none carries a `name` attribute**:

| line | | |
|---|---|---|
| 9 | `<details class="kind">` | Committed |
| 51 | `<details class="kind">` | Quota |
| 11 | `<details class="commitment-choice">` | At a time |
| 31 | `<details class="commitment-choice">` | By a day |

A `<details>` without a `name` is independent of every other one. **`<details name="…">` makes siblings an exclusive accordion natively — no JavaScript, no state.** Two groups: the two `kind` disclosures share one name, the two `commitment-choice` disclosures share another.

## The trap, and it will ship if it is not named

**The name must be unique per capture row.** `lists.html` renders one of these blocks **per capture**, so a bare `name="kind"` shared across rows means **opening Committed on one capture closes the one you already had open on another** — a worse bug than the one being fixed.

**A single-capture fixture cannot fail this way.** That is `T-a-check-must-be-seen-to-fail` in its second shape — a check that cannot fail looks like coverage. **Prove it with at least two captures in the inbox.**

## What is not wrong, and must stay that way

**There is no submit ambiguity today, and this must not create one.** Each disclosure wraps **its own `<form>`** — five in the row — each with its own hidden `kind`/`commitment` and its own submit button. Two open disclosures never merge their inputs.

**Do not consolidate the forms while tidying this.** That separation is what makes the current behaviour merely ugly rather than incorrect, and a slice that "cleans it up" into one form would turn a cosmetic defect into a data defect.

## Browser support — state it, do not assume it

`<details name>` is Chrome 120+, Safari 17.2+, Firefox 130+. The phone-layout check drives **Chrome 151** on this box. **Older browsers ignore the attribute and get today's behaviour — degradation is to the current bug, not to something worse.** Say so in the pull request rather than leaving it inferred.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-committed-is-at-or-by` | At and by are an explicit choice at triage. **This changes how the choice is presented, never what it means.** |
| `D-pool-is-default` | Pool stays the cheapest path — its form is not inside a disclosure at all and **must not gain one.** |
| `T-a-check-must-be-seen-to-fail` | Break it, watch it fail, restore it, say so. |
| `T-qa-binds-tolerantly-to-markup` | Bind by role and accessible name, **never by `.kind` or `.commitment-choice`.** |
| `D-four-screens` | The canvas is authoritative on layout — and per `T-canvas-is-authoritative-where-it-speaks`, **check what it draws here before assuming it is silent.** It was silent on the done control, the date input, a settings surface and an app icon; that is four, and a fifth should be flagged, not assumed. |

## Acceptance scenarios worth specifying

- Opening one kind disclosure closes the other, **in the same row.**
- Opening one commitment disclosure closes the other, **in the same row.**
- **Opening a disclosure on one capture leaves every other capture's disclosures exactly as they were.** Two captures minimum.
- **Every form still submits what it submitted before** — same `kind`, same `commitment`, same fields. This is presentation only.
- **All 19 acceptance features pass untouched.**

## Known repo gotchas

1. **`trunk` is green at `0f47e9b`.** Cut from `origin/trunk`, not the local `trunk`.
2. **Expect 19 features.** Analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh`.
3. **Update `scripts/ci/complexity-baseline.json` as a step**, not an afterthought.
4. **`cargo test --workspace` skips the `#[ignore]`d property tests** locally — CI runs them via `--include-ignored` since #113, your local run does not unless you pass it.
5. **`scripts/qa/phone_layout.cjs` is gated in CI** and drives a real Chrome. **Disclosure state is exactly what it can see and no HTTP assertion can.** Consider whether it belongs there — and if you assert it, prove the assertion can fail.
6. **#122 and #120 are open on the Pool screen**, not this one. No overlap, but they may land around you.
7. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
8. **Open the pull request when QA is done, and label it `preview`.**

## Dependencies and sequencing

- **Nothing blocks this.** `trunk` is green, pipeline empty, no pull request open.
- Behind it: **#122** (a trip survives being worked), **#120** (show more), **#93** (Quota), **#111** (undo on Committed), **#118** (timezone surface).

## Source

- Issue **#119** · `crates/trellis-server/templates/capture_row.html:9,11,31,51` · `crates/trellis-server/templates/lists.html`
- `docs/decisions.md` — `D-committed-is-at-or-by`, `D-pool-is-default`, `T-a-check-must-be-seen-to-fail`, `T-canvas-is-authoritative-where-it-speaks`
