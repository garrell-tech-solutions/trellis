# QA Procedure: The quota screen shows what you triaged, and lets you change or remove it

Covers: `features/quota_screen.feature` and the edited `committed-screen-tabs-06`.

## Interface used

The four screens over HTTP, `sqlite3` **read-only** for corroboration, **a
real browser** driven headless at 390×844, and **a real phone.**

**Bind to accessible names, ids and `data-` attributes — never to the shape of
a `<div>`, a colour or a font** (`T-qa-binds-tolerantly-to-markup`). **#137
re-did the entire palette and typeface**; a check bound to either will not
survive the next one.

**Go through the routes.** Get quotas onto this screen by triaging captures,
never by inserting rows.

## This screen stopped creating things, and that is most of what changed

**Settled by the owner 2026-08-26**, reversing the larger half of #147:

> *"there should not be any sort of way to create a quota in the quota screen
> — the quota screen is only displaying the quotas that you have inserted as a
> task and triaged as a quota."*

**`+ Define a new quota` and its whole form leave this screen.** Five of the
nine scenarios this feature shipped with go with them, and **not one is
deleted** — both-fields-required, target-must-be-positive, repeated-name-
refused and similar-name-warns are now `quota_triage_validation.feature`'s,
where the door is. **The rules survived; the surface moved.** Their QA lives
in `qa/quota_triage_validation.md`, including the whole name guard.

**`-09` is the one assertion that is gone outright**, and it is gone because
it has been **inverted**. It said *"quota tasks from triage are a different
thing and do not appear here"* — the boundary between two coexisting quota
concepts, asserted while that state was knowingly accepted. **#138 closes that
state.** A triaged quota is now the only thing this screen can show.

**So the first thing to check is the exact opposite of what this document used
to say.** Triage a capture as a quota, then look at this screen: **it must be
there.** Corroborate in `sqlite3` that exactly one quota exists, not two.

## Procedure — the fourth tab

1. Load each of Capture, Pool, Committed and Quota.

### Expected Observable Outcomes
- **Four tabs on every one of them**, in the order Capture, Pool, Committed,
  Quota, each marking itself as current.
- **No dead link anywhere.** Follow every tab from every screen and confirm
  none 404s.
- The Quota tab is reachable **on the phone**, not only by typing a URL.

## Procedure — an empty screen that is not a dead end

1. Empty database. Read the Quota screen.

### Expected Observable Outcomes
- The screen says **`none yet`** and explains what a quota is — *"A quota is a
  weekly hour target you keep — practice, study, running. Capture one and
  triage it."*
- **There is no `+ Define a new quota` control, and no define form behind
  anything.** Look for a route that still answers as well as a control that is
  still drawn: **a live create endpoint with no link is the same hole in the
  other direction**, and `nav.rs`'s own comment — *"a dead link is worse than
  no link"* — is the shape to check against. **If `POST /quotas` still creates
  a quota when curled directly, say so**; that is a door the owner asked to be
  closed, and closing only the button is not closing the door.
- **The screen offers a way back to Capture.** With nothing to create here,
  an empty screen that only explains itself is a dead end. `pool-screen-
  empty-07` established this shape for exactly this situation.
- **The old note said "Define one below."** If you see that sentence, the text
  is instructing the owner to use a control that is not there.

## Procedure — reading a quota

1. Triage a capture as a quota named `Piano`, `4` hours a week.
2. Read the Quota screen.
3. Triage two more: `Running` at 3, `Rust` at 5.

### Expected Observable Outcomes
- Step 2: `Piano` reads **`0m / 4h`** with the note **`4h left this week ·
  0%`**, and the meta reads **`1 quota`**.
- **Read `0m / 4h` carefully.** The canvas composes `fmtMins(mins) + " / " +
  fmtMins(target)`, so it is `0m / 4h` and not `0h of 4h`
  (`T-canvas-is-authoritative-where-it-speaks`).
- Step 3: **`Piano, Running, Rust`**, meta **`3 quotas`** — the order you
  triaged them in, oldest first, new ones at the bottom.

## Procedure — changing a quota

**#86 is the reason this exists:** *"the target is editable — that is the
intended response to a persistent shortfall."* It is the load-bearing half of
`D-quota-no-rollover`: refusing to carry a shortfall forward is only humane if
the target can change.

1. `Piano` at 4 hours, with 30 minutes logged. **Change the target to 2.**
2. **Rename it** to `Piano theory`.
3. With `Running` also present, **rename `Running` to `piano`** — then to
   `Pi-ano`.

### Expected Observable Outcomes
- Step 1: reads **`30m / 2h`**, note **`1h 30m left this week · 25%`**. **The
  logged 30 minutes survived the retarget** — changing the target must not
  touch what you recorded.
- Step 2: the listing shows `Piano theory`, still reading **`30m / 4h`**. **The
  sessions came with the name.**
- Step 3: **both refused**, each naming what already exists, and the listing is
  unchanged. **Same guard as triage** (`NameStanding`) — a rename path that
  skipped it would let you produce the duplicate the create path refuses, which
  is the failure `T-one-front-door-per-capability` exists for.
- Each refusal is a **`422` whose body is the re-rendered fragment**
  (`T-422-is-product-wide`), matching triage's own refusal.

## Procedure — removing a quota

**Remove means archive**, settled by the owner 2026-08-29. It stops appearing
and stops counting; the hours stay, with nowhere to browse them — the bargain
`D-kill-means-archive` already struck for tasks.

1. `Piano` (30 minutes logged) and `Running`. **Remove `Piano`.**
2. Read the screen, then **reload**.
3. **Capture `Piano` again and triage it as a quota at 2 hours.**

### Expected Observable Outcomes
- Step 1–2: `Piano` is gone, meta reads **`1 quota`**, and it stays gone across
  a reload. **`Running` is untouched.**
- **Nothing was deleted.** Check `sqlite3` on a copy: the `quotas` row and its
  `quota_sessions` rows are **both still there**. If either is gone, removal is
  destroying data the owner was told it would keep.
- Step 3: **`Piano` comes back with its 30 minutes**, at the new 2-hour target
  — **one quota, not two**, meta `1 quota`, reading `30m / 2h`.
- **There is nowhere to browse removed quotas** (`R-browsable-archive`). If you
  find one, that is the finding.

## Procedure — the foreign key, and why removal is not a delete

**⚠️ The brief said a `DELETE FROM quotas` silently orphans its sessions
because no `PRAGMA foreign_keys` is set in the tree. That is the opposite of
what happens, and it was proved by running it.** `sqlx` sets
`foreign_keys = ON` on every connection it opens, so the constraint is live.

1. On a **scratch copy**, with a quota that has sessions, run
   `DELETE FROM quotas WHERE id = ...` through the app's own connection.

### Expected Observable Outcomes
- **It is refused: SQLite error 787, `FOREIGN KEY constraint failed`.**
- **This is why removal archives rather than deletes.** A delete-based removal
  would have passed every test — the owner's two quotas have **zero sessions
  each** — and then failed in the shop on exactly the quota they had been
  logging against for a month. **Confirm no code path deletes a `quotas` row.**

## Procedure — prove the checks can fail

**`T-a-check-must-be-seen-to-fail`.** #149 is open because three procedures in
#147 ran green and were **never seen red.** Each breakage names **the
assertion it must trip** — confirm *that one* goes red.

1. **Delete the quota row instead of archiving it.** → **`-08` fails**, and
   read *how*: with sessions present it is a 787 constraint error, not a wrong
   listing. **That is the whole reason this slice archives**, and it is
   invisible on the owner's own data.
2. **Let a rename skip `NameStanding`.** → **`-07` fails on both rows.**
   `Pi-ano` is the row that matters: the column's `UNIQUE COLLATE NOCASE` would
   still catch `piano`, so a breakage that only reddens the first row means the
   Rust guard is gone and only the collation is holding.
3. **Free the name on removal instead of reviving.** → **`-09` fails**: you get
   two quotas, or a fresh one starting at zero instead of `30m / 2h`.
4. **Let a retarget clear the sessions.** → **`-05` fails on the readout** —
   `0m / 2h` instead of `30m / 2h`. The listing does not move, which is what
   makes this one easy to miss by eye.
5. **Filter triaged quotas back out of the screen's query** — the exact
   behaviour `-09` used to assert. → **`quota-screen-reads-its-target-03` and
   `-triaged-order-04` both fail**, and the screen is empty after a triage.
   **This is the defect the owner reported**, reproduced on purpose: *"it
   brings up the page but it doesn't show any of the things that I've triaged
   as a quota."*
6. **Put `+ Define a new quota` back on the screen.** → **`-02` fails on its
   "offers no way to define a quota" step alone.** The note, the meta and the
   Capture link are unmoved — if they move too, that step is entangled with
   something it should not be.
7. **Restore the old empty-state note** (*"Define one below"*). → **`-02`
   fails on the note, and only the note.**
8. **Sort the screen's query by name instead of by id.** → **`-04` fails** —
   `Piano, Running, Rust` happens to be alphabetical, so **use the triage
   order `Rust`, `Piano`, `Running` when running this one**, or the breakage
   passes and proves nothing. **A fixture that cannot distinguish two orderings
   is not testing the ordering.**
9. **Revert `nav.rs` to three pages.** → **`committed-screen-tabs-06` fails on
   all four rows**, and the Quota tab vanishes while `/quota` still answers.

### Expected Observable Outcomes
- **Nine breakages, nine distinct messages, then restore and a clean pass with
  a clean `git status`. Say which you ran.**
- **Breakage 1 is the one to do if you do only one.** It is the failure the
  owner's own data cannot show you, and the reason removal archives.
- **Breakage 8 has a trap.** `Piano, Running, Rust` is alphabetical, so use the
  triage order `Rust`, `Piano`, `Running` or it passes and proves nothing.

## By-hand walkthrough — on a real phone

**Label the pull request `preview`.**

1. **Tap the fourth tab** on a fresh database. It explains itself and points
   you at Capture. **There is nothing to create here.**
2. **Capture `workout`, triage it as a quota at 3 hours a week.**
3. **Tap the fourth tab again.** `workout` is there.
4. **Tap `+1h` on it.** The bar moves.

### Expected Observable Outcomes
- **Step 3 is the whole slice.** This is #138 answered.
- Step 4 confirms everything #147 shipped still works on a quota that arrived
  this way rather than through the form that no longer exists.
- **There are no up/down arrows on a quota row.** They are **#139** and
  deliberately not built. **Do not report their absence as a defect, and do
  not report it as a rule either** — on a quota that absence is **undecided**,
  unlike the define control, which the owner settled today.

## Procedure — nothing else changed

1. Run the acceptance suite and every QA suite.
2. Run `scripts/analyzers/dry.sh` and **say what it measured.**

### Expected Observable Outcomes
- **All 27 acceptance features pass** — 26 before this slice, plus
  `quota_migration`. **Five are edited**: `quota_triage_validation`,
  `task_kinds`, `quota_screen`, `disclosures` and `triage_from_page`. **If any
  other needed editing, the change leaked.**
- **`quota_sessions.feature` is untouched, and that is worth confirming
  rather than assuming.** Not one of its scenarios cares how a quota came to
  exist, so its `Given a quota named "Piano"…` keeps its wording and changes
  only which door its handler goes through. **If that file needed editing, a
  Given was written as implementation rather than as state**, and saying so is
  worth more than the edit.
- **`unknown_kind_rejection.feature` is untouched too**, because `quota` is
  still one of the three kinds. The brief expected this to move. **It did
  not** — confirm it, do not assume it.
- **DRY: report the number and the formats.** It was **2.51% at #147**, up
  from 2.23%, against a **3% product-code** threshold
  (`T-dry-measures-product-code`). **Two slices in a row raised it.** Say whether a shared
  family was extracted early or bolted on at the end.
- **`/quota` is inside `colour.cjs` and `phone_layout.cjs` now** (#147). The
  triage panel's new name box is new markup on a gated screen — **expect the
  palette and contrast gates to have an opinion and treat what they find as a
  finding rather than something to style around.**
- **`phone_layout.cjs` seeds `/` with `expectOverflow: true`.** This slice
  shortens the triage form from three fields to two. **If `/` stops
  overflowing at 390px that gate goes red for the right reason — re-seed it,
  do not relax the assertion.**
- **A skipped check is not a blocked check.** `migrations are append-only` is
  `if: github.event_name == 'pull_request'` (`ci.yml:73`) and skips on every
  `trunk` push **by design**. **Read the `if:` before reporting a skip.**

## Independent of Implementation

This procedure depends only on what the four screens show, what an empty Quota
screen offers, and what appears there after a triage. It does not depend on
which table a quota lives in, what the routes are called, or how the row is
composed — only that the fourth tab exists everywhere, that this screen
creates nothing, and that what you triaged as a quota is on it.
