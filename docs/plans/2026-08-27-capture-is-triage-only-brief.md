# Handoff brief — `capture-is-triage-only`

**Date:** 2026-08-27 · **Issue:** #140 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> Base is `origin/trunk`, **green** at `6d2b1ae`, release `v2026.08.27.204`.
> **#153 landed a prose budget on the specifier's prompt. It applies to this slice.** Cite by slug; do not restate.

---

## Goal

The owner, 2026-08-25: *"we don't need to list the tasks on the capture page. it should just be insert and triage."*

`GET /` renders the quick-add box, **`Recent`** (untriaged captures with their triage controls), and **`Tasks`** — `templates/lists.html:16-17`, every triaged task in the database as `[kind] text tag`. **The third one goes.**

It is a leftover from **#33**, when it was `D-visible-slices`' proof that triage wrote something and there was nowhere else to look. **There are four screens for that now**, and a flat undifferentiated list is not one of them (`D-four-screens`). The canvas agrees: `Trellis.dc.html:54-171`, the `isCapture` guard, draws the box, `Recent`, and a `hasOlder` affordance. **No `Tasks` section anywhere in it.**

## Demo

**On the phone.** Label the pull request `preview`.

1. **Open Trellis.** The capture page is a box and `Recent`. **The wall of every task you have ever triaged is gone.**
2. **Type "buy screws". Tap `Pool`, give it `@homedepot`.**
3. **The row does not vanish.** It stays in `Recent`, restyled, reading what it became. **You can see that it worked without leaving the page.**
4. **Tap the Pool tab.** It is there.
5. **Triage something as a quota.** The Quota tab shows it — and **`[quota] …` no longer also appears at the bottom of Capture**, which it does today.

## The hard part is already solved on `trunk` — apply it, do not invent it

**Deleting the markup silently removes 24 assertions' ability to see anything**, and #90 recorded what that costs: *"if a scenario's point is that nothing happens, its parameters are not under test."* Nine mutants survived on that trap once.

**`quota-filing` (#138, PR #150) hit exactly this problem yesterday and answered it.** `features/quota_triage_validation.feature:96-101` replaced *"the task list is still empty"* with a pair:

```gherkin
Then the triage is rejected
And the quota screen offers no quotas          # the destination saw nothing
And the capture is still waiting in the untriaged queue   # the source kept it
```

**8 of 8 mutants killed.** That is the pattern, written, mutation-tested, and on `trunk`.

**And the second line is strictly stronger than what it replaces, which is the argument for the whole slice.** *"The task list is still empty"* is a global fact: it passes whenever nothing was written **anywhere** — including when the capture was silently consumed and no task created. *"The capture is still waiting"* names the specific thing. **A rejection that eats the capture passes the old assertion and fails the new one.** This slice is not preserving coverage through a deletion; it is trading a weak global assertion for a precise local one, 24 times.

## The 24, by family — say the mapping out loud per family, not per scenario

`grep -E '^\s+(Then|And) .*task list' features/` — **24 real assertions across 9 files.** The 21 `Given … empty task list` backgrounds are setup, not observation, and are not affected.

| shape | n | what it proves | where it moves |
|---|---|---|---|
| `the task list is still empty` | 14 | a rejected triage wrote nothing | the pair above |
| `the task list has "<tasks>" tasks` | 5 | dismissal created nothing | `dismiss_capture` — the capture left and no screen gained a row |
| `the task list shows "…"` / `tagged "…"` | 4 | the write landed with the right tag | `/pool` or `/committed`, which already have steps for exactly this |
| `does not contain an unescaped "<script>"` + `contains the word "boom"` | 2 | **the XSS assertion** | **see below — do not fold this into the row above** |

Affected: `committed_field_domains`, `committed_date`, `unknown_kind_rejection`, `committed_screen`, `committed_triage_validation`, `context_tags`, `disclosures`, `dismiss_capture`, `triage_from_page`.

## The XSS assertion is the one to be slowest about

`triage_from_page.feature:65-69` captures `<script>alert('boom')</script>`, triages it, and asserts the **rendered** page does not contain an unescaped tag **and does contain the word `boom`**. The second half is what stops the assertion passing because the text was dropped rather than escaped — **a pair, and both halves are load-bearing.**

**It must land somewhere hostile text is actually rendered**, which after this slice is `/pool` (and `Recent`'s own restyled row, which is new surface this slice creates and therefore new escaping to prove). **Assert it in both places rather than choosing**, and keep the two halves together wherever it goes.

## The product half, which the issue calls the second problem

**Remove the list and the capture page has no feedback at all.** `inbox::lists::build_capture_rows` lists **untriaged captures only**: you type a thing, tap `Pool`, the row vanishes, and with `Tasks` gone nothing says where it went.

**The canvas answers it.** `Trellis.dc.html:84-92`: a capture that has just been triaged **stays in `Recent`, restyled** — `c.triaged` renders `c.meta` in place of the kind buttons that `c.untriaged` carries. **One row template, two states**, the shape #94 used for at/by.

**Two questions that follow, and they are yours to settle:** how long a triaged row stays before `Recent` is only recent again, and whether it survives a reload. `T-ephemeral-view-state-rides-the-request` is the rule, and this project has now got that call right twice and wrong twice — **read `D-a-trip-survives-being-tidied`'s row before reaching for a column.**

## Watch

1. **`[quota]` rows are currently duplicated and this slice removes the duplicate.** Since #150, quota triage writes a `tasks` row **and** a `quotas` row (`triage/http.rs:51-66`), and `inbox::store::list_tasks` has no `WHERE` — so the owner's `workout` appears both on the Quota screen and at the bottom of Capture. **Deleting the list resolves it. Migration `0016` left the old rows in place deliberately; leave them.**
2. **`scripts/qa/phone_layout.cjs:41` seeds `/` with `expectOverflow: true`.** This slice removes most of that page's content. **If `main` stops overflowing at 390px the gate goes red for the right reason — re-seed, do not relax the assertion.**
3. **`T-a-check-must-be-seen-to-fail`.** Twenty-four assertions are being rewritten. **A rewritten assertion that has only ever been green is not evidence.** Break each family once — one per family, not one per scenario — and record the message. **#149 is the open example of not doing this.**
4. **#145** — the manifest cannot record a survivor. Report what the tool printed.
5. **Two stale pointers, two lines, while you are in the file:** `scripts/ci/complexity_baseline.sh:12` and `:232` cite `docs/decisions.md, 2026-08-12` for the do-not-flatten-the-dispatchers ruling, which moved to `docs/decisions-history.md` at `ed58454`. **They are the only two in the repository.**
6. **Two CI gates run but do not block.** `capabilities go through front doors` and `every feature has an entrypoint` are not in `trunk`'s required-checks list. **Owner action, flagged, not yours** — but do not read their green as gating.
7. Scratch in `./tmp/`. **New pull request, labelled `preview`.**

## Out of scope

**#154** (a capture cannot reach an existing quota) · **#148** · **#111** · **#136** · **#118** · **#95** · any visual design system · and **retiring `list_tasks` itself** if another caller needs it — check before deleting the function as well as the markup.

## Source

- **#140** — the issue, and the owner's words
- `features/quota_triage_validation.feature:96-101` — **the pattern to copy, and the reason this slice is a strengthening rather than a substitution**
- `crates/trellis-server/templates/lists.html:16-17` — what goes
- `crates/trellis-server/src/inbox/store.rs:107` — `list_tasks`, no `WHERE`
- `crates/trellis-server/src/inbox/lists.rs` — `build_capture_rows`, untriaged only
- `docs/design/Trellis.dc.html:54-171`, `:84-92` — the `isCapture` guard, and `c.triaged` beside `c.untriaged`
- `features/triage_from_page.feature:65-69` — the XSS pair
- `docs/decisions.md` — `D-four-screens`, `D-visible-slices`, `T-ephemeral-view-state-rides-the-request`, `T-a-check-must-be-seen-to-fail`, `T-qa-binds-tolerantly-to-markup`
- **#90** — nine mutants lost to an assertion whose subject stopped existing
