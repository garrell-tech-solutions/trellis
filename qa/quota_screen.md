# QA Procedure: The quota screen, and defining a quota

Covers: `features/quota_screen.feature`, the edited `committed-screen-tabs-06`,
and the half of the name guard no acceptance scenario can hold — **the warning
as you type.**

## Interface used

The four screens over HTTP, `sqlite3` **read-only** for corroboration, **a
real browser** driven headless at 390×844 (the `playwright-core` and Chrome
`scripts/qa/trip_controls.cjs` already resolves), and **a real phone.**

**Bind to accessible names, ids and `data-` attributes — never to the shape of
a `<div>`, a colour or a font** (`T-qa-binds-tolerantly-to-markup`). **#137
re-did the entire palette and typeface this week**; a check bound to either
will not survive the next one.

**Go through the routes.** Define quotas through the form, never by inserting
rows. The whole slice is about what that path refuses.

## A quota is not a task, and that is the finding

`TaskKind::Quota { target_count, target_minutes_each, period }` makes a quota
a **triaged capture counting sessions**. This screen's quota is a **named
container with a weekly hour target**, created without a capture. They are
different things and **both exist after this slice** — #138 closes that.

**So the first thing to check is that they do not touch.** Triage a capture as
a quota task the old way, then look at this screen: it must not be there, must
not have been migrated, and must not have altered any count. `-09` asserts it
over HTTP; corroborate in `sqlite3` that the old row is untouched and still
carries its `target_count` / `target_minutes_each` / `period`.

## The name guard, and the two tiers

`D-quotas-are-selected-not-typed`: **a mistyped name must not be able to
create a quota**, because a typo does not mis-file an item — it **creates a
second counter that silently splits the week's hours and makes both wrong.**

Settled by the owner 2026-08-25, from `Trellis.dc.html:427-436`:

| You type, against an existing **Piano** | What happens |
|---|---|
| `piano`, `PIANO`, `Pi-ano`, `pi ano` | **Refused.** Same name once case, spaces and punctuation are ignored. |
| `Pianoo`, `Piano theory` | **Warned, and still possible.** Within two letters, or one name inside the other. The control reads **Create anyway**. |
| `Guitar` | Created, no warning. |

**The second tier is not politeness, it is the harder half.** Refusing
`Piano theory` would be wrong — it may genuinely be a second quota — so the
product has to say something and then get out of the way.

## Procedure — the fourth tab

1. Load each of Capture, Pool, Committed and Quota.

### Expected Observable Outcomes
- **Four tabs on every one of them**, in the order Capture, Pool, Committed,
  Quota, each marking itself as current.
- **No dead link anywhere.** `nav.rs`'s comment — *"a dead link is worse than
  no link"* — is why the tab could not ship before the route. Follow every tab
  from every screen and confirm none 404s.
- The Quota tab is reachable **on the phone**, not only by typing a URL.

## Procedure — defining a quota

1. Empty database. Read the Quota screen.
2. Tap `+ Define a new quota`. Submit with **no name**. Then with **no hours**.
   Then with **0** hours.
3. Define `Piano`, `4`.

### Expected Observable Outcomes
- Step 1: the screen says **`none yet`** and explains what a quota is — *"A
  quota is a weekly hour target you keep — practice, study, running. Define one
  below."* — and offers `+ Define a new quota`.
- Step 2: **each rejection is a `422` whose body is the re-rendered form**
  (`T-422-is-product-wide`), the form still holds what you typed, and **no
  quota is created.** Check `sqlite3` after each: still zero rows.
- Step 3: the quota appears reading **`0m / 4h`** with the note **`4h left this
  week · 0%`**, and the meta reads **`1 quota`**.
- **Read `0m / 4h` carefully.** The brief's fallback line calls it `0h of 4h`;
  the canvas composes `fmtMins(mins) + " / " + fmtMins(target)`, so it is
  `0m / 4h`. **The canvas wins** (`T-canvas-is-authoritative-where-it-speaks`)
  and the brief's phrasing was informal. Say if you think the canvas is wrong.

## Procedure — the name guard, over HTTP

1. Define `Piano`, 4 hours.
2. Submit `piano`, then `PIANO`, then `Pi-ano`, then `pi ano`.
3. Submit `Pianoo`. Then submit it a second time.
4. Submit `Guitar`.

### Expected Observable Outcomes
- Step 2: **all four refused**, each with the warning naming `Piano` **and its
  target** — *"already exists at 4 h a week. File it there instead of making a
  second one."* **Still one quota** in `sqlite3` after all four.
- Step 3: the first submit is **refused with the "reads a lot like" warning**
  and the control now reads **`Create anyway`**; the second submit **creates
  it.** Two quotas.
- Step 4: created immediately, **no warning**.
- **Corroborate the backstop.** `T-collation-enforces-name-identity` says a
  constraint the database enforces cannot be bypassed by a write path that
  forgot to call something, so the rule being in Rust does **not** excuse the
  column. Confirm a `UNIQUE COLLATE NOCASE` (or equivalent) exists on the name
  — **try inserting `piano` directly with `sqlite3` on a scratch copy and
  confirm the database itself refuses it.** If only the Rust refuses, say so:
  that is the exact shape that decision row was written against.

## Procedure — the warning as you type, in a browser

**This is why the suite exists at this tier.** A warning you have not submitted
is ephemeral (`T-ephemeral-view-state-rides-the-request`) — asserting it over
HTTP would push it into a stored field, which is #126's mistake with a new
subject.

1. Seed `Piano`, 4 hours. Load Quota at 390×844 and open the new-quota form.
2. Type `Pian` slowly. Then `Piano`. Then `Pianoo`.
3. **Record network activity throughout.**

### Expected Observable Outcomes
- The warning panel **appears and updates** as the name changes, and the
  create control's label tracks it.
- **Nothing about the half-typed name is stored.** Reload mid-typing: the form
  is empty and no row exists. **Check the schema for a column holding a draft
  name or a warning state — if one exists, that is the finding, and say it
  before anything else in the report.**
- **Report whether the live check costs a request per keystroke.** An htmx
  round trip per character on a phone over a tailnet is a real cost;
  `trip_controls`' toggle established that this project prefers client state
  for this kind of thing. **This is a finding either way — say which it does.**
- **Every control is at least 44px** (`--tap`), including the create and cancel
  controls and the `+ Define a new quota` button.

## Procedure — prove the checks can fail

**`T-a-check-must-be-seen-to-fail`.** `0416241` is the standard: six breakages,
six distinct messages, and a real bug found by doing it. **Each breakage below
is paired with the assertion it must trip** — confirm the named one goes red,
not merely that something did.

1. **Compare names with `==` on the raw string.** → `-06`'s `Pi-ano` and
   `pi ano` rows fail while `piano` and `PIANO` still pass, because the
   collation still catches those two. **That split is the point** — it shows
   which tier is doing which half of the work.
2. **Drop the near-match tier entirely.** → `-07` fails: `Pianoo` is created on
   the first submit with no warning. Nothing in `-06` moves.
3. **Make the near-match refuse instead of warn.** → `-07` fails at the second
   submit — `Piano theory` can never be created. **This is the failure mode
   that looks like a stricter, better product and is not.**
4. **Drop the `UNIQUE COLLATE NOCASE` column, keeping the Rust check.** →
   **every acceptance scenario still passes.** Only the `sqlite3` direct-insert
   step above fails. **This is the one no other tier can see**, and it is the
   reason that step is in this document.
5. **Let a triaged `TaskKind::Quota` row into the screen's query.** → `-09`
   fails: `practise piano` appears on a screen it has no business on.
6. **Revert `nav.rs` to three pages.** → `committed-screen-tabs-06` fails on
   all four rows, and the Quota tab vanishes while `/quota` still answers —
   **a live route with no link, which is the mirror of the thing that comment
   was guarding against.**

### Expected Observable Outcomes
- **Six breakages, six messages, then restore and a clean pass with a clean
  `git status`.** Say which you ran.
- **Breakage 4 is the one to do if you do only one.** It is invisible to every
  other check in the project.

## By-hand walkthrough — on a real phone

**Label the pull request `preview`.**

1. **Tap the fourth tab.** It exists now.
2. **Define a quota** — `Piano`, `4` hours a week. Both fields are required and
   the form says so.
3. **Try to define `piano` again.** It warns you rather than quietly creating a
   second counter.
4. **Try `Pianoo`.** It warns you differently — and lets you through if you
   insist.
5. Define two more. **Confirm they list in the order you defined them**, newest
   at the bottom.

### Expected Observable Outcomes
- All five hold.
- **Step 3 is `D-quotas-are-selected-not-typed` made visible**, and step 4 is
  the half that had to be judged rather than enforced.
- **There are no up/down arrows on a quota row.** The canvas draws them; they
  are **#139** and deliberately not built. **Do not report their absence as a
  defect, and do not report it as a rule either** — unlike a trip, where
  `pool-screen-nothing-reorders-05` settles it, on a quota that absence is
  **undecided**. If the order feels wrong on the phone, *that* is worth saying.

## Procedure — nothing else changed

1. Run the acceptance suite and every QA suite.
2. Run `scripts/analyzers/dry.sh` and **say what it measured.**

### Expected Observable Outcomes
- **All 26 acceptance features pass** — 24 before this slice, plus
  `quota_screen` and `quota_sessions`. **`committed_screen.feature` is the one
  edited**, scenario 06 only. If any other needed editing, the change leaked.
- **The brief said 23 and said "untouched".** Both were stale by the time it
  was written: `trip_persistence` landed as the 24th, and the fourth tab cannot
  arrive without changing the scenario that counts tabs. **Named, not quiet.**
- **DRY: report the number and the formats.** The gate measures product code
  only (`T-dry-measures-product-code`) and the last two slices ran 1.86% and
  1.81% against 3%. **A whole new screen with its own step module is the most
  likely thing yet to cross it** — say whether a shared family was extracted
  early or bolted on at the end.
- **#108 is untouched** — nothing here goes near the pool query.

## Independent of Implementation

This procedure depends only on what the four screens show, what the define form
does with what you type, and what the database holds afterwards. It does not
depend on how names are compared, where the comparison lives, what the routes
are called, or how the form is composed — only that the fourth tab exists
everywhere, that a repeated name cannot create a second counter, that a similar
one is surfaced and still possible, and that a triaged quota task is a
different thing that stays where it is.
