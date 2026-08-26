# QA Procedure: A capture row offers three kinds, and shows one set of fields

Covers: `features/disclosures.feature`

## Interface used

The inbox and its capture rows, over HTTP and — for the half that depends on
what is *visible* rather than what is *present* — `scripts/qa/phone_layout.cjs`
driving a real Chrome. Read every control by role and accessible name, never
by class (`T-qa-binds-tolerantly-to-markup`); the old `.kind` and
`.commitment-choice` selectors are gone and were never the contract.

## What changed, and why it is bigger than the brief scoped

The brief proposed one attribute per `<details>` to make them mutually
exclusive. **Checking the canvas first — as `T-canvas-is-authoritative-where-it-speaks`
requires — showed it draws no `<details>` at all**, but three 44px kind
buttons with the chosen one filled, and a bordered panel below for the kind
that needs fields.

So the row is now **three buttons and at most one panel**, not four
collapsibles.

## The one thing the canvas draws that this deliberately does not do

**The canvas files a committed task on one tap, undated** — its script sets
`at: "Unset"`, `ord: 99`, to be dated later. **This slice does not.**
Committed triage still requires a deadline, a commitment, a priority and an
estimate; that is *behaviour*, and `D-four-screens` gives behaviour to the
decisions log while giving the canvas layout.

**So do not report the presence of the committed fields as a divergence from
the design.** It is a deliberate, recorded split, and one-tap-and-date-later
remains a live product option the owner may take on purpose later.

**The committed panel's layout is invented** — the canvas draws a panel for
quota only. That is the fifth gap it has left, and the largest. It borrows
the quota panel's own drawing so the invention is as small as possible.
**Report how it looks; that is the only judgement available, since there is
nothing to compare it against.**

## By-hand walkthrough — on a phone

1. Label the pull request `preview`; the box puts it on `:8443` within two
   minutes.
2. Capture **two** things.
3. On the first row, tap **Committed**. Confirm its fields appear and the
   button is visibly the chosen one.
4. Tap **Quota** on the same row. **Confirm the committed fields are gone**,
   not merely pushed down.
5. **Tap Committed on the second row. Confirm the first row still shows
   Quota.** This is the trap; see below.
6. Tap **Pool** on either row. Confirm it files immediately with no fields
   at all.

### Expected Observable Outcomes
- All six steps hold.
- **Step 5 is the risk in this slice**, and it survived the change of
  mechanism. `lists.html` renders one row per capture, so whatever makes a
  row's choice exclusive must be **scoped to that row**. If choosing on one
  row disturbs another, the fix has produced a worse bug than the one it
  cured.
- **Step 6 is `D-pool-is-default`.** Pool is the cheapest path: one tap, no
  fields, no panel. **A panel for Pool added to make the three buttons look
  symmetrical is a defect.**

## Procedure — one panel, and only within the row

1. Put **two** captures in the inbox.
2. Choose Committed on the first; read the whole inbox.
3. Choose Quota on the first; read it again.
4. Choose Committed on the second; read it again.

### Expected Observable Outcomes
- After 2: the first row shows committed fields and **no quota fields**.
- After 3: the first row shows quota fields and **no committed fields** —
  replaced, not stacked.
- After 4: the second row shows committed fields **and the first row still
  shows quota fields.**
- **Two captures is the minimum that can fail.** A single-capture fixture
  passes whether the scoping is right or wrong, which is
  `T-a-check-must-be-seen-to-fail` in its second shape: a check that cannot
  fail looks like coverage.

## Procedure — present versus visible

**Which tool proves this depends on how it was built, and you must check
which.**

1. Read the raw HTML of a row with Committed chosen.
2. If the other kinds' fields are **absent** from the markup, HTTP is
   sufficient and `scripts/qa/disclosures.sh` can assert it alone.
3. If they are **present but hidden** by CSS, HTTP cannot tell you anything —
   **the browser check is the only thing that can**, and the assertion must
   be about visibility, not presence.

### Expected Observable Outcomes
- **Say in the report which of the two it is**, and which tool proved it. A
  procedure that asserts absence against a CSS-hidden panel passes for the
  wrong reason and would keep passing if the CSS broke.

## Procedure — nothing about submission changed

1. Choose Committed and submit with each required field omitted in turn:
   deadline, commitment, priority, estimate.
2. Choose Quota and submit with a target field omitted.
3. Submit a complete committed triage and a complete quota triage.

### Expected Observable Outcomes
- **Every rejection is `422`, names the missing field, and creates nothing** —
  exactly as before. This slice is presentation.
- **Each kind still submits its own form.** Two panels never merged their
  inputs because two forms never merged. **If the forms have been
  consolidated while tidying, a cosmetic defect has become a data defect** —
  check that a committed submission carries no quota field and vice versa.

## Procedure — prove the check can fail

`T-a-check-must-be-seen-to-fail`.

1. Break the row scoping so a choice on one row affects the other — however
   the implementation scopes it.
2. Run the suite. **Confirm it fails, and that the message names the row that
   was disturbed** rather than reporting a generic mismatch.
3. Restore; confirm a clean diff and a clean pass.
4. **Say in the report that you did this, and how you broke it.**

## Procedure — nothing else changed

1. Run all nineteen existing QA suites and the acceptance suite.

### Expected Observable Outcomes
- **All 19 acceptance features pass untouched.**
- `triage_from_page`'s procedures bound to the old two-`<details>` committed
  form **will need rewriting** — that is drift from a deliberate change, not
  regression. **Reproduce each failure before fixing it**, and say which.
- `phone_layout` still passes: the row is taller with a panel open, and the
  document must still not scroll.

## Independent of Implementation

This procedure depends only on what a capture row offers, what it shows once
a kind is chosen, and what each form submits. It does not depend on whether
the panel is swapped from the server or revealed by CSS, nor on how the
chosen kind is remembered.

## Procedure — the quota panel's name box (#138)

**The quota panel changed its fields and kept its shape.** It used to ask
`target_count`, `target_minutes_each` and `period`; it now asks **a name and
hours a week**, because triaging as a quota is what **creates** the quota.
**Nothing about which-panel-is-open moved**, which is why the procedures above
are unedited — they assert which panel is showing, never what is inside it.

1. Capture `learning with lev`. Tap `Quota` on its row.
2. Capture `practise piano`. Tap `Quota` on that row too.
3. Edit the first row's name box to `Reading`. Do not submit.
4. Tap `Committed` on the first row, then `Quota` again.

### Expected Observable Outcomes
- Steps 1–2: **each name box already holds that row's own capture text, in
  full.** `learning with lev` carries spaces — **a prefill that survives one
  word and drops the rest would pass on a single-word fixture**, which is
  `T-a-check-must-be-seen-to-fail` in the shape #90 named.
- **The box is editable**, and that is the half that matters: **renaming a
  quota is #148 and is not built**, so the name triage writes is the name
  forever. Without an editable box, `finish chapter 3` becomes a permanent
  quota called that.
- Step 2 must not disturb step 1's row — **rows are independent**, and the
  name box is new per-row state that could break that.
- Step 4: **switching away and back does not leave `Reading` behind.** An
  unsubmitted edit is ephemeral (`T-ephemeral-view-state-rides-the-request`).
  **Check the schema for a column holding a draft name — if one exists, say
  that before anything else in the report**; `T-migrations-append-only` means
  it can never be taken back.
- **The panel shows no sessions field, no minutes-each field and no period
  dropdown.** If any survives, the retirement was cosmetic.

### Prove it can fail
- **Prefill the name box with a constant instead of the row's text.** →
  **`disclosures-quota-offers-the-captures-words-07` fails on both rows**, and
  it must fail on *both* — a breakage that reddens only one row means the
  scenario is reading a fixture rather than the capture.
- **Make the name box read-only.** → **`-07` fails on its "can be changed"
  step alone**, with the prefill steps still green. **That is the pairing to
  confirm**: if the prefill steps go red too, the two assertions are entangled.
