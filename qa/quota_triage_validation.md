# QA Procedure: Triaging a capture as a quota requires a name and an hour target

Covers: `features/quota_triage_validation.feature`, and the half of the name
guard that moved house from `qa/quota_screen.md`.

## Interface used

The triage panel on the inbox page and the triage endpoint over HTTP,
`sqlite3` **read-only** for corroboration, and **a real browser** driven
headless at 390×844 (the `playwright-core` and Chrome `scripts/qa/
trip_controls.cjs` already resolves).

**Bind to accessible names, ids and `data-` attributes — never to the shape of
a `<div>`, a colour or a font** (`T-qa-binds-tolerantly-to-markup`).

**Go through the routes.** Create quotas by triaging captures, never by
inserting rows. The whole point of this slice is that triage is the only door.

## What changed, and why this document was rewritten rather than edited

**This file used to test `target_count`, `target_minutes_each` and `period`.
All three are retired.** Every procedure it held would have become a procedure
that passes against any implementation whatsoever, which is #90's trap wearing
a QA document: *a check that cannot fail reads exactly like coverage.*

**The owner settled the new shape on 2026-08-26**, reversing the brief:

> *"a quota is a task and should be fully creatable from the task in queuing
> and triage screen; there should not be any sort of way to create a quota in
> the quota screen — the quota screen is only displaying the quotas that you
> have inserted as a task and triaged as a quota."*

So **triage is what creates a quota.** `+ Define a new quota` is gone from the
Quota screen. Chips, `Filed here` and filing into an existing quota are
**deferred, not built** — the owner: *"this seems like something that could
come later (combining quotas)."*

**`kind=quota` is still one of the three kinds.** The brief asked whether it
becomes an unknown kind; it does not, and `unknown_kind_rejection.feature` is
untouched. **Confirm that rather than assuming it** — post `{"kind":"quota"}`
with a name and hours and watch it succeed, then post `{"kind":"banana"}` and
watch it 422 as an unknown kind. If `quota` has quietly joined `banana`, that
is the finding, and it is a large one.

## The name guard, and the two tiers

`D-quotas-are-selected-not-typed`: **a mistyped name must not be able to
create a quota**, because a typo does not mis-file an item — it **creates a
second counter that silently splits the week's hours and makes both wrong.**

| You type, against an existing **Piano** | What happens |
|---|---|
| `piano`, `PIANO`, `Pi-ano`, `pi ano` | **Refused.** Same name once case, spaces and punctuation are ignored. |
| `Pianoo`, `Piano theory` | **Warned, and still possible.** Within two letters, or one name inside the other. The control reads **Create anyway**. |
| `Guitar` | Created, no warning. |

**The rule did not change when it moved.** `scheduler_core::quota::check_name`
already states both tiers and #147 proved them; what moved is the surface it
guards. **The refusal's wording did change**, and that is a consequence rather
than a tidy-up: it used to end *"File it there instead of making a second
one."* **With filing deferred there is nowhere to file it**, so a message that
survived unedited would be instructing the owner to use a control that does
not exist. It now names the two things that are actually possible.

## Setup — repeat before each procedure

1. Start the server against a fresh database file.
2. Confirm the Quota screen offers no quotas. **The `Tasks` list on the capture page is gone (#140)**; the destination screens are where a write is visible.
3. Submit a capture with raw text `go to the gym`.
4. Confirm it is present in the untriaged queue.

## Procedure — a required field with no usable value

Four combinations: `name` and `hours`, each **omitted entirely** and each
**submitted as `""`**.

For each: triage as kind `quota` supplying the other field validly, then read
the response, the Quota screen, and the untriaged queue.

### Expected Observable Outcomes
- **All four rejected, each naming the field that had no usable value.**
- **Omitted and empty are the same rejection** (`T-empty-equals-absent`) —
  compare the two responses for a field and confirm they say the same thing.
  A client rarely intends the distinction and the product must not invent one.
- **No quota is created.** Check the Quota screen *and* `sqlite3` after each.
- The capture is still waiting, untriaged.

## Procedure — an hour target that is not a positive number of minutes

Rows: `0`, `-2`, `four`, `0.004`.

### Expected Observable Outcomes
- **All four rejected, each identifying `hours` as invalid**, and no quota
  created.
- **`0.004` is the row to watch.** It is positive, it parses as a number, and
  it rounds to **zero minutes** — so it is the only row that gets past
  `parse_positive_hours` and reaches `WeeklyTarget::from_minutes`'s own guard.
  **If `0.004` is accepted and `0` is not, you have found the gap this row
  exists for**, and a quota with a zero-minute target is a divide waiting to
  happen in `quota::progress`.
- **`four` proves the parse is a parse.** A field read with a permissive
  coercion would land on `0` and report the wrong thing about the wrong field.

## Procedure — the name guard, over HTTP

1. Triage a capture as a quota named `Piano`, 4 hours.
2. Capture something new. Triage it naming `piano`, then `PIANO`, then
   `Pi-ano`, then `pi ano`.
3. Triage naming `Pianoo`, 2 hours. Then submit the same thing again.
4. Triage naming `Guitar`, 3 hours.

### Expected Observable Outcomes
- Step 2: **all four refused**, each warning naming `Piano` **and its target**
  — *"already exists at 4 h a week. Log your time against that one, or give
  this a different name."* **Still one quota** in `sqlite3` after all four,
  and **the capture is still untriaged each time** — a refused triage must not
  consume the capture, or the owner loses the thing they were filing.
- Step 3: the first submit is **refused with the "reads a lot like" warning**
  and the control now reads **`Create anyway`**; the second submit **creates
  it.** Two quotas.
- Step 4: created immediately, **no warning.**
- **Corroborate the backstop.** `T-collation-enforces-name-identity` says a
  constraint the database enforces cannot be bypassed by a write path that
  forgot to call something, so the rule living in Rust does **not** excuse the
  column. **Try inserting `piano` directly with `sqlite3` on a scratch copy
  and confirm the database itself refuses it.** If only the Rust refuses, say
  so.

## Procedure — the name box is the escape hatch, in a browser

**Renaming a quota is #148 and is not built**, so whatever name triage writes
is the name forever. The editable box is the only thing standing between a
one-off capture and a permanent quota called `finish chapter 3`.

1. Capture `finish chapter 3`. Load the inbox at 390×844 and tap `Quota`.
2. **Record network activity.** Edit the name to `Reading`. Type `2` hours.
3. Submit. Read the Quota screen.
4. Capture `workout`, tap `Quota`, change nothing, type `3`, submit.

### Expected Observable Outcomes
- Step 1: **the name box already holds `finish chapter 3`.**
- Step 2: **zero requests while typing.** Typing is free — an htmx round trip
  per character on a phone over a tailnet is a real cost, and this project
  settled that this kind of state is held client-side.
- Step 3: **the quota is called `Reading`, not `finish chapter 3`.**
- Step 4: **the quota is called `workout`** — the prefill is a default you can
  ignore, so the common case stays "type the hours and go".
- **Every control is at least 44px** (`--tap`), including the name box, the
  hours box and the create control.
- **Reload mid-typing before submitting.** Nothing about the half-typed name
  survives, and **no column holds a draft name or a warning state — if one
  exists, say that before anything else in the report**
  (`T-ephemeral-view-state-rides-the-request`; `T-migrations-append-only`
  means it can never be taken back).

## Procedure — prove the checks can fail

**`T-a-check-must-be-seen-to-fail`.** #149 is open because three QA procedures
in #147 ran green and were **never seen red**. **Do not repeat the shape.**
Each breakage below names **the assertion it must trip** — confirm *that one*
goes red, not merely that something did.

1. **Compare names with `==` on the raw string** — revert `check_name`'s
   normalization only. → **all four rows of `-03` fail, and they fail on the
   *warning text***: with the exact tier broken, `piano` still matches `Piano`
   through the **similar** tier's containment check, so the product warns
   *"reads a lot like"* instead of refusing.
   **⚠️ Do not predict a 2-of-4 split here.** An earlier document did, on the
   theory that the collation would rescue `piano` and `PIANO`; **QA proved
   that wrong by running it.** `is_similar` is evaluated on normalized strings
   and matches long before the database is consulted, so **this breakage never
   reaches the collation at all** — which is exactly why breakage 4 exists.
2. **Drop the near-match tier entirely.** → **`-04` fails at the first
   submit**: `Pianoo` is created with no warning. **Nothing in `-03` moves** —
   check that too, because a breakage that reddens everything proves nothing
   about which tier did the work.
3. **Make the near-match refuse instead of warn.** → **`-04` fails at the
   second submit** — `Piano theory` can never be created. **This is the
   failure mode that looks like a stricter, better product and is not.**
4. **Drop the `UNIQUE COLLATE NOCASE` column, keeping the Rust check.** →
   **every acceptance scenario still passes.** Only the `sqlite3`
   direct-insert step above fails. **This is the one no other tier can see.**
5. **Accept a blank name instead of rejecting it.** → **`-01` fails on both
   `name` rows and only those** — `hours` omitted and `hours` empty still
   reject. If the `hours` rows move too, the two fields share a check that
   should be two.
6. **Remove `WeeklyTarget::from_minutes`'s own positive guard**, leaving
   `parse_positive_hours` in place. → **`-02` fails on the `0.004` row and no
   other.** `0`, `-2` and `four` are still refused upstream. **This is the
   breakage that proves `0.004` earns its row**; if `-02` stays green, that
   row is decoration and should be said so in the report.

### Expected Observable Outcomes
- **Six breakages, six distinct messages, then restore and a clean pass with a
  clean `git status`. Say which you ran.**
- **Breakages 4 and 6 are the two to do if you do only two.** Each is
  invisible to every other check in the project.

## By-hand walkthrough — on a real phone

**Label the pull request `preview`.**

1. **Capture `workout`. Tap `Quota`.** The name is already there. Type `3`.
2. **Tap the fourth tab.** `workout` is on it, reading `0m / 3h`.
3. **Capture `workuot`** — a genuine typo — and try to make it a quota.
4. **Capture `Piano theory`** with a `Piano` quota already there.
5. **Capture `finish chapter 3`.** Tap `Quota`, rename it to `Reading`.

### Expected Observable Outcomes
- Step 2 is **the whole slice**: the thing you triaged is on the screen that
  exists for it. This is the complaint that opened #138, answered.
- Step 3 warns you rather than quietly creating a second counter beside
  `workout` — **which would split the week's hours and make both wrong.**
- Step 4 warns you *differently*, and **lets you through if you insist.**
- Step 5 is the escape hatch working; without it that quota is called
  `finish chapter 3` forever.
- **There are no up/down arrows on a quota row.** The canvas draws them; they
  are **#139** and deliberately not built. **Do not report their absence as a
  defect, and do not report it as a rule either** — on a quota that absence is
  **undecided**. If the order feels wrong on the phone, *that* is worth saying.

## Independent of Implementation

This procedure depends only on what the triage panel offers, what the triage
endpoint accepts and refuses, and what the Quota screen shows afterwards. It
does not depend on how names are compared, where the comparison lives, which
table a quota is stored in, or how the panel is composed.
