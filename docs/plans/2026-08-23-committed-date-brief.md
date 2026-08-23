# Handoff brief — `committed-date`

**Date:** 2026-08-23 · **Issue:** #110 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **The owner named this themselves, after using the product.** `D-dogfood-first` working exactly as intended, for the second slice running — #101 was the first.

---

## Demo

**On a phone, not in a terminal.** The live instance is on the tailnet and `ops/preview.sh` puts your branch on a second port.

1. Capture *"call the dentist"*. Triage it as **committed**.
2. Today: type `2026-08-25T17:00:00Z` — correctly, in **UTC**, on a touch keyboard. One wrong character is a `422`.
3. After: choose the day without typing a timestamp.
4. It appears on **Committed** where you expect it, in **your** timezone.

**Step 2 is the whole justification.** The owner is meant to be living in this by 2026-09-03, and committing anything is currently the most hostile interaction in the product.

## Goal and scope

**Make entering a committed date thumb-usable, in the owner's configured timezone.**

`templates/capture_row.html:13` is `<input type="text" name="deadline" placeholder="2026-08-20T17:00:00Z">`, and `scheduler_core::task::fields::parse_deadline_ms` (`fields.rs:166`) parses strictly as `jiff::Timestamp` — RFC 3339 with the `Z`. **`T-timezone-is-a-setting` already put the owner's zone in the database**, and this is the one field that ignores it.

### Out of scope — do not absorb

The at/by model itself, the rest of the triage form (**#84**, and this slice removes the deadline field from its scope), quota fields (**#93**), recurrence, an archive view, and **any visual design system**.

## The question to settle first, because it may halve the work

**`D-committed-is-at-or-by` already splits these into two things, and they may not need the same input.**

- An **at** is a fixed block — *2pm dentist*. Date **and** time.
- A **by** is a deadline with slack. `T-commitment-is-chosen-not-derived` keeps its time out of the display deliberately, and `committed_screen::date_cell` renders **`BY THU`** — never a time.

**So a by's time is entered, stored, and never shown to anyone.** If a by needs only a day, the commonest case gets the simplest control and the at/by choice already on the form decides which input appears. **Settle it and say why in the pull-request body.**

## What the canvas draws — I have read it, and it has a gap

`D-four-screens` makes `docs/design/Trellis.dc.html` authoritative on layout. **Read it yourself; here is what I found so you can check me.**

**It draws no date input anywhere.** No `type="date"`, no `type="time"`, no `type="datetime-local"` in the file. **That is a design gap — the same class as the missing done control #103 met — so flag it; do not fill it silently.**

**What it does draw is a strong precedent, and it is consistent:**

| Construct | Where | Shape |
|---|---|---|
| `c.kindButtons` (line 92) | Capture | kind is chosen by **tapping one of three buttons**, not a `<select>` |
| `c.suggestions` chips (line 101) | Capture | tags are **chips**, with *"Or type a new one. Tags are cheap."* beneath |
| `<select aria-label="Day">` over `{{ days }}` (line 274) | Quota → *Log a session* | **`height:44px`**, seven entries from `DAYS`, paired with `<input type="number" min="5" step="5" aria-label="Minutes">` |

**The canvas's idiom for choosing is: tap from a short list, or type if you must.** It is a text box nowhere except free text and a tag.

**And the seven-day list round-trips exactly to what Committed already displays.** `date_cell` renders `BY THU`; the canvas's only day control is a seven-entry weekday `<select>`. **A `by` chosen as a weekday is stored and shown in the same vocabulary** — which is worth noticing before designing anything more elaborate.

**44px is the canvas's touch-target height.** Use it rather than inventing one.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| `D-committed-is-at-or-by` | An **at** is a fixed block; a **by** is a deadline with slack. **They display differently and behave differently** — and may deserve different inputs. |
| `T-commitment-is-chosen-not-derived` | At/by is an explicit choice at triage, **never derived from how precisely the deadline was typed.** A control that infers *at* from the presence of a time breaks this. |
| `T-timezone-is-a-setting` | The zone is data in the database, not deployment config. **This slice is the first to actually use it.** |
| `T-core-owns-validation-order` / `T-required-fields-are-specified-per-transport` | The JSON endpoint and the page are **one code path**. A control only the page understands must not fork them. |
| `D-pool-is-default` | Pool stays the cheapest path — strictly fewer inputs than committed. **Do not make pool more expensive to make committed cheaper.** |
| `D-four-screens` | The canvas is authoritative on layout. |
| `T-jiff-epoch-millis` | `jiff`; civil and zoned are distinct types. **A local date plus a zone is a conversion, and conversions are business rules.** |

## Acceptance scenarios worth specifying

- A committed task is dated **without typing a timestamp**, and lands on Committed on the day intended.
- **The owner's configured timezone decides the instant.** A date chosen near midnight lands on the right day — this is the assertion most likely to be missing, and the one a UTC-typing habit hides.
- **An at and a by are still an explicit choice**, not inferred from what was entered.
- Rejection still returns `422` with the re-rendered fragment (`T-422-is-product-wide`), and **the message still names the field**.
- The **JSON transport still works**, whatever the page now sends.
- **All 17 acceptance features pass untouched.**

## Known repo gotchas

1. **`trunk` is green at `27686e9`.** Cut from `origin/trunk`, not the local `trunk`, which is routinely stale. Confirm CI on your base commit.
2. **Expect 17 features.** Analyzers in order: `scripts/acceptance/run.sh` first, then `coverage.sh`, `crap.sh`, `complexity.sh`, `dry.sh` — the first two report nonsense without the generated entrypoints.
3. **Update `scripts/ci/complexity-baseline.json` as a step, not an afterthought.** It pins exact scores and fails when a score moves *in either direction* or a row goes stale — a stale row fails a slice that changed nothing. It arrived failing on two slices running before I made this an explicit item. **Do not regenerate blindly**: last time one stale row was a genuine new dispatch arm and the other was fixed by *extracting* code. `T-complexity-8`'s threshold is in the constitution and cannot be raised.
4. **`parse_deadline_ms` is `pub(super)` and proptested.** Whatever the form sends, **the core still owns what a valid instant is.** Do not move parsing into the adapter to make a control convenient — and a local-date-plus-zone conversion belongs in `scheduler-core`, not a handler and **not in JavaScript**.
5. **`scripts/qa/phone_layout.cjs` now exists** and drives a real Chrome at 390×844, gated in CI as of #105. **A control claiming to be thumb-usable can be checked by something that can see it, for the first time in this project.** Consider whether it should be.
6. **The company standard binds you**: `swarmforge/constitution/articles/engineering.prompt` now carries it, and `T-set-operations-execute-in-the-store` is its Trellis row. Nothing here should fetch a collection to sort in memory.
7. **A scenario asserting invariance under exactly the transformation the mutator applies is untestable by that mutator and looks fully covered.** If a scenario here is about sameness across days, write literal ones.
8. Base branch is **`trunk`**. Scratch in `./tmp/`, not `/tmp`.
9. **Open the pull request when QA is done**, before taking another brief.

## Dependencies and sequencing

- **Nothing blocks this.** `trunk` is green, the pipeline is empty, no PR is open.
- **Narrows #84** — the deadline field leaves its scope. #84's premise also needs correcting when it is briefed: it claims there is no viewport meta or stylesheet, and `base.html:5` and `:8` both have them.
- **#111** (undo) and **#93** (Quota) are queued behind this.

## Source

- Issue **#110** — the field, the parser, and the at/by question
- `crates/trellis-server/templates/capture_row.html:13` · `crates/scheduler-core/src/task/fields.rs:166`
- `docs/design/Trellis.dc.html` — lines 92, 101, 274; **and the absence, which is the finding**
- `docs/decisions.md` — `D-committed-is-at-or-by`, `T-commitment-is-chosen-not-derived`, `T-timezone-is-a-setting`, `D-pool-is-default`, `T-jiff-epoch-millis`
