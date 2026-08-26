# QA Procedure: The committed screen lists what has a date on it

Covers: `features/committed_screen.feature`

## Interface used

The committed screen, reached by its **tab** — never a typed URL. The capture
screen and its triage controls for setup, `trellis serve --now <RFC3339>` to
pin today, and read-only `sqlite3` inspection. No project library, module, or
test helper is used.

**Pin the clock for every procedure.** Half of what this screen says is
relative to today — what is past, what a date cell reads — and a suite run
against the real clock tests a different question every day.

**`T-qa-binds-tolerantly-to-markup` governs this document.** Bind to ids and
owned classes, never to attribute order, adjacency, or copy quoted verbatim.
Where a procedure quotes text it says whether the text is the contract or an
example.

## At and by

`D-committed-is-at-or-by`: an **at** is a fixed block — *2pm dentist*; a
**by** is a deadline with slack — *done by Thursday*. The canvas draws no
distinction, so how it looks is ours: a `by` carries a **BY** prefix in the
same 66px cell an `at` fills with its time. **It is an explicit choice at
triage, not derived from how precisely the deadline was typed** — the
reasoning is in `features/committed_screen.feature`'s header.

`at`/`by` replaced `hard`/`soft` on the form and **the `deadline_type` column
is still there with nothing reading it.** That is intended. **Do not report
the unread column as dead code**, and **do not expect a migration dropping
it.**

## By-hand walkthrough — do this once, in a real browser, on a phone

1. `cargo run -p trellis-server -- serve --db <fresh path> --now 2026-08-24T09:00:00Z`.
2. Capture and triage three committed items: a dentist appointment as an
   **at** with a time, a tax return as a **by**, and one with a date already
   past.
3. Tap **Committed**. Confirm you never typed a URL.
4. Confirm the list is in date order, the past one first and marked, the
   `at` showing its time and the `by` showing **BY**.
5. Confirm the committed triage form asks **at or by** and **does not ask
   hard or soft**.
6. Confirm the tab bar shows **four** tabs and marks the current one, on all
   four screens (four since #93, when the Quota route arrived to make the
   fourth link live).

### Expected Observable Outcomes
- All six steps hold literally, per `D-visible-slices`.
- **Step 4 is the slice.** The date cell is 66px of tabular numerals doing
  three jobs — a time, a BY, and a past marker. **Check it at phone width**;
  `BY THU 17:00` is the longest thing it must hold and it is the one most
  likely to wrap or clip.
- `qa/phone_layout.md` drives a real browser at 390×844 and asserts geometry
  on this screen — **so the scroll and the tab bar are checked and the date
  cell's width is not.** Report what the automated check cannot see, and do
  not claim what it does not assert.

## Setup — repeat below

1. Start the server against a fresh database with
   `--now 2026-08-24T09:00:00Z`. **Today is Monday 24 August 2026.**
2. Reach the committed screen **through the tab bar**.

## Procedure — date order, and what each row carries

1. Create three committed items dated Thursday, Tuesday and Friday, in that
   order, two with context tags and one without.
2. View the committed screen.

### Expected Observable Outcomes
- They list **Tuesday, Thursday, Friday** — chronological, not creation
  order. Create them out of order deliberately, or a build that lists by
  insertion passes.
- Each row carries its date cell, its text, and its context tag; **the one
  with no tag renders without one** rather than with a placeholder.
- The count beside the title reads `3 dated`.
- **A context tag on a committed item is not a pool-only idea.** If the row
  drops it, the screen has borrowed an assumption from the Pool slice.

## Procedure — an at, and a by

1. Create one `at` with a time and one `by`.
2. View the screen.

### Expected Observable Outcomes
- The `at` cell reads its day and time; the `by` cell says **BY** and its
  day.
- **The distinction is in the markup, not colour alone**, and it is legible
  without knowing which is which beforehand.
- Create a `by` **with a time** as well. It is a legal commitment — *by 5pm
  Friday* — and must render as a `by`, not silently become an `at`.

## Procedure — a deadline that has passed

1. Create one item dated before today and one after.
2. View the screen.

### Expected Observable Outcomes
- **Both appear.** The past one is **first** — that falls out of
  chronological order without special casing — and is **marked** as past.
- **A missed deadline that vanishes is the one failure this screen cannot
  have.** If the past item is absent, that is a defect however tidy it looks.
- The count includes it.

## Procedure — only committed work

1. Create one committed, one pool and one quota task.
2. View the screen.

### Expected Observable Outcomes
- Only the committed item appears, and the count reads `1 dated`.
- Check the pool screen still shows the pool task — this slice must not have
  moved anything.

## Procedure — nothing dated

1. View the screen against an empty database.

### Expected Observable Outcomes
- The message reads **`Nothing with a time on it. That is allowed.`** —
  **this text is the contract, not an example.** It tells the owner an empty
  Committed screen is success rather than neglect, and softening it loses the
  point.
- A way back to Capture is offered, and the count reads `nothing dated`
  rather than `0 dated`.

## Procedure — triage asks at or by, on both transports

1. Submit a committed triage over the **JSON API** with `commitment` omitted,
   every other required field valid.
2. Submit the same through the **page's own form**.
3. Query the tasks table.

### Expected Observable Outcomes
- **Both are rejected**, and each names `commitment`.
- **Both transports, every time**
  (`T-required-fields-are-specified-per-transport`): #73 shipped a page whose
  committed form could never succeed while the suite, triaging over JSON,
  stayed green.
- The form **does not ask hard or soft**, and a submission carrying
  `deadline_type` is not rejected for it — the column is unread, not
  forbidden.
- Nothing is created.

## Procedure — hostile text stays escaped

1. Create a committed item whose text is `<script>alert('boom')</script>`,
   with a context tag of the same.
2. Read the **raw HTML** of the committed screen.

### Expected Observable Outcomes
- No unescaped `<script>`; `boom` still present, escaped rather than
  stripped.
- **Check the text cell and the context cell separately** — they are
  different columns with different styling and could fail independently.

## Procedure — nothing else changed

1. Run all sixteen existing QA suites.

### Expected Observable Outcomes
- All pass.
- **`one_screen` legitimately changed** — `/committed` now answers 200.
- **`committed_triage_validation` legitimately changed** — it enumerates the
  required fields by name, and `commitment` replaced `deadline_type`.
- **`pool_screen` must still pass untouched**, including
  `pool-screen-nothing-reorders-05`. The canvas draws reorder arrows on this
  screen too; **if any appeared, that assertion is the only thing standing
  between the design and a feature nobody approved.**
- **Believe the re-run.** A restyle has broken QA scripts three times in this
  project without CI noticing.

## Independent of Implementation

This procedure depends only on what the committed screen renders for a given
set of tasks and a given today, and what the triage boundary accepts. It does
not depend on which route serves it, how at/by is stored, or whether the past
marker is a class or an element.
