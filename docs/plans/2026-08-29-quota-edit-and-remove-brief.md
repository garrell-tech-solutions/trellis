# Handoff brief — `quota-edit-and-remove`

**Date:** 2026-08-29 · **Issue:** #148 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> Base is `origin/trunk` at `c3159a7`, green. **The owner hit this in real use on 2026-08-29:** *"I realized I can't edit or delete a quota."*

---

## The state, confirmed rather than quoted

`platform/app.rs:89-92` exposes four quota routes: `GET /quota`, `POST /quota/{id}/sessions`, `POST /quota/sessions/{id}`, `POST /quota/sessions/{id}/delete`.

**Every session is correctable and deletable. The quota holding them is neither.** No rename, no retarget, no removal. `quota_body.html` after #157's redesign carries only session controls — checked, not assumed.

**#86 named this:** *"**The target is editable** — that is the intended response to a persistent shortfall, not a carried debt."* It is the load-bearing half of `D-quota-no-rollover`: refusing to carry a shortfall forward is only humane if the target can change.

## The filing is stale in two ways, both from #150

**#148 was written 2026-08-26, before the owner changed the creation model.** Two things in it no longer hold:

- *"Define `Piano` at 4 hours with a typo"* — **you cannot define a quota on the screen any more.** `POST /quota` is gone. A quota is created by triaging a capture, with its name prefilled from the capture's own words. **So the typo arrives through triage, and the name you are stuck with is one you typed once, in a hurry, on a phone.** That makes the case stronger, not weaker.
- *"the rename half waits on the identity ruling that belongs with #138"* — **#138 delivered it.** `quota::NameStanding { Free, Taken, Resembles }` is a working front door. **The rename is unblocked and the guard already exists.**

## Demo

**On the phone.** Label the pull request `preview`.

1. **Open Quota.** `workout` reads *0 of 2h 15m*, `learning with lev` *0 of 30m* — the two the migration brought across.
2. **Change `workout` to 3 hours.** The bar and the readout move. **You did not have to delete it and capture it again.**
3. **Fix a name.** Rename one. **Try renaming it to the other one's name — it is refused, naming what already exists**, the same way triage refuses it.
4. **Remove one you no longer keep.** It goes.
5. **Reload.** All three stick.

## The four questions this slice must answer, and one is a trap

**1. What happens to a removed quota's sessions?** `0015`: `quota_id INTEGER NOT NULL REFERENCES quotas(id)` — **with no `ON DELETE` clause.**

**⚠️ And `PRAGMA foreign_keys` is set nowhere in the tree.** SQLite defaults it **OFF, per connection**, so that `REFERENCES` is documentation and not enforcement. **A `DELETE FROM quotas` today silently orphans every session row rather than failing.** The owner's two quotas have zero sessions each, so **the live database will not show you this bug** — it appears the first week someone deletes a quota they had been logging against.

**Decide it explicitly**: cascade, refuse-while-sessions-exist, or archive rather than delete. **Do not leave it to a pragma nobody set.**

**2. Delete, or archive?** `D-kill-means-archive` — *"Kill means archive. Keep the row"* — is about **tasks**, and a quota is not one. But a quota now has a `tasks` row beside it (triage writes both, `triage/http.rs`), so **removing a quota has to say what happens to that task.** `R-browsable-archive` should be read before proposing anywhere to see removed quotas.

**3. Does a rename go through `NameStanding`?** It must. `quotas.name` is `UNIQUE COLLATE NOCASE` and `check_name` folds punctuation and measures resemblance. **A rename path that skips the guard lets you rename `workout` into `Workout` and produce the duplicate the create path refuses** — the same rule enforced at one door and not the other is the failure `T-one-front-door-per-capability` exists for.

**4. Where do the controls live at 390px?** The row already carries a toggle, a bar, a readout, a note, and three quick-log buttons. **`scripts/qa/phone_layout.cjs` measures tap targets on `/quota` now** (since #147), so this is checkable rather than arguable.

## The canvas is not the authority here, and that is a change

`D-four-screens` was bounded on 2026-08-24: *"the canvas is authoritative on layout for a slice being specified against it, at that moment. **Once a slice ships and the owner has used it, the implementation is the record.**"*

**#157 (`quota-screen-redesign`) redesigned this screen outside the pipeline** and the canvas was not updated. **So `Trellis.dc.html` describes a screen that no longer exists**, and it draws no edit or delete control either way. **Read `quota_body.html` and the running app; do not spec against the canvas for this one.** Say so in the handoff if that reading turns up anything the log cannot explain — #157 left no decisions-log trace.

## The assertion family, handed over whole

**This is the part my last two briefs got wrong, so it is enumerated rather than sampled.** `grep -rhoE '(Then|And) the quota screen [a-z ]+' features/` — **22 assertions across 6 files:**

| shape | n | note |
|---|---|---|
| `offers the quotas "…"` | 7 | the listing; a rename and a removal both change it |
| `reports "…"` | 6 | the readout; a retarget changes it |
| `offers no quotas` | 6 | the empty state |
| `offers no way to define a quota` | 1 | **#150's assertion that the screen creates nothing — a control that edits is not a control that defines, but say so rather than assuming it** |
| `offers a way back to "…"` | 1 | |
| `notes "…"` | 1 | |

Files: `quota_screen`, `quota_sessions`, `quota_migration`, `quota_triage_validation`, `triage_from_page`, `task_kinds`.

**Give a verdict per occurrence, not per file.** #90: a scenario whose subject changes does not start failing — it stops asserting.

## Watch

1. **CRAP headroom, per module** — the gate fails at `>= 30` and has **no exception list** (`T-crap-has-no-baseline-and-complexity-does`). Today: `pool_screen` **29 — full**, `mod` 26, `quota_screen` 13, `quota_sessions` 10, `mark_done` 9. **New steps belong in `quota_screen.rs`, which has room. Do not put them anywhere near `pool_screen`.**
2. **Run the analyzers before handoff.** `complexity_baseline` has been red at QA handoff for **three consecutive slices**. **#156** covers the five `scripts/ci/` gates the constitution never names; `crap.sh` is *not* one of them — it is listed in `stack.prompt` and simply was not run.
3. **`T-a-check-must-be-seen-to-fail`.** Break each new rule once and record the message. **#149** is the standing example of not doing it.
4. **`T-set-operations-execute-in-the-store`** for whatever a removal does to sessions — one statement, not a fetch-and-loop.
5. **`T-422-is-product-wide`** — a rename to a taken name is a `422` whose body is the re-rendered fragment it failed against, matching triage's own refusal.
6. **`T-migrations-append-only`** — `0016` is applied. Anything new is `0017`.
7. Scratch in `./tmp/`. **New pull request, labelled `preview`.**

## Out of scope

**#154** (a capture cannot reach an existing quota) — **but a removal has to say what it would mean for a quota with items, even though none can have them yet.** Also out: **#118**, **#136**, **#95**, **#156**, and any visual design system.

## Source

- **#148** — the issue; **read the two stale paragraphs above before its body**
- `crates/trellis-server/src/platform/app.rs:89-92` — the four routes, and what is not among them
- `crates/trellis-server/migrations/0015_quota_sessions.sql` — `REFERENCES quotas(id)`, no `ON DELETE`, and no `PRAGMA foreign_keys` anywhere in the tree
- `crates/trellis-server/src/quota/mod.rs` — `NameStanding`, the guard a rename must reuse
- `crates/trellis-server/templates/quota_body.html` — the row as #157 left it
- **#86** — *"the target is editable"* · **#150** — the creation model this brief is written against · **#157** — the redesign the canvas does not know about
- `docs/decisions.md` — `D-quota-no-rollover`, `D-kill-means-archive`, `R-browsable-archive`, `T-collation-enforces-name-identity`, `T-one-front-door-per-capability`, `T-set-operations-execute-in-the-store`, `T-422-is-product-wide`, `T-crap-has-no-baseline-and-complexity-does`, `D-four-screens`
