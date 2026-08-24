# Handoff brief — `trip-controls`

**Date:** 2026-08-24 · **Issues:** #120 **and** #125 · **Milestone:** Dogfood — daily use by 2026-09-03 · **Route:** pipeline

> **Two issues, one slice, because the header would otherwise be designed three times by three slices that never saw each other.** #127 already put a **`✕` Clear done** on it; #125 adds a **complete-group** button; #120 reworks the **expand** control just below. On a 390px screen.
>
> **And they share an unanswered question** — where toggle state lives — which is the question that bought a permanent column in #126 when it was answered at the wrong tier.

---

## Demo

**On the phone.** Label the pull request `preview`.

1. A trip of eight. It shows three and a control saying five more are hidden.
2. **Expand it.** The list *continues* — one list, not two — and the control now reads as something you can collapse. Today it still says *"Show 5 more"* directly above the five it is showing.
3. **Complete the group.** One deliberate act, not a mistap.
4. Collapse, expand, complete, undo. **Nothing about the panel surprises you.**

## Part one — the expand control (#120)

Three problems, one cause: `pool_body.html:33-41` used `<details>`/`<summary>` where the canvas draws a button.

1. **The label never changes** — a `<summary>` is static text, so after expanding it still reads *"Show 3 more"* above the items it is showing.
2. **The disclosure triangle stays**, pointing down, above a list it no longer hides.
3. **The revealed items are a second list**, nested inside the `<details>` with its own spacing, rather than continuing the one above. `trellis.css:772-780` styles only `.trip-more { margin-top: 4px }` and a summary colour, so nothing reconciles them.

**The canvas draws this** (line 210, gated on `g.hasMore`): a borderless `<button>` with `g.onToggle`, `color: var(--color-primary-800)`, `letter-spacing: 0.02em`, and a **state-driven label** — expanding swaps `g.visible` for the full set. **One list, and a label that can change.**

**The reasoning that produced the wrong control is recorded and is not silly.** `pool/view.rs:22` says a native `<details>` *"costs no request and no server-held state."* True — and it is the tell.

## Part two — the complete-group button (#125)

Settled by the owner: **a "complete group" button at the top of the group**, completing the group's remaining items. Struck-through items stay as they are.

`D-bulk-completion-is-explicit` permits **exactly this and nothing else** — *"a control whose only purpose is completing that group."* **What that rule forbids is bulk completion arriving as a side effect of another gesture**, so this must be unmistakably deliberate, not reachable by a mistap while scrolling past a header. **A confirmation step is a legitimate answer and so is a control that is simply hard to hit by accident** — but say which and why. The failure it exists to prevent is **losing six items whose only shared property is that they mention `@homedepot`.**

## Why these are one slice

**#125's central question is unanswerable without #120.** *Does completing the group complete the items behind "show more"?* — *"hidden"* is a state the expand control defines. **Design them together and the question resolves itself; design them apart and one of them guesses.**

Both also need the same answer to: **where does the expand state live?** — and #125 needs to know whether the header it is joining has one control or two.

## The question both parts turn on

**Where does toggle state live, and do not answer it the way #126 did.**

That slice bought `captures.shown_kind` — a permanent, append-only column for a display preference — and traced why afterwards: *"the acceptance suite speaks only HTTP; to make that assertion true over HTTP the state had to be server-rendered; to be server-rendered across a request it had to be stored."* **The tier you assert in decides what the implementation must store.**

**Two things this project already has, which that slice did not use:**

- **`scripts/qa/phone_layout.cjs` is gated in CI and drives real Chrome.** It can observe rendered state no HTTP assertion can. It does **not** look at panel or expand state today.
- **The product already runs JavaScript.** `base.html:16-17` loads htmx and an inline script on **every page**. *"A toggle needs client state this product does not have"* has never been true.

**Nothing here should need a migration.** #127's `cleared_at` was justified — a durable consequence of a deliberate act. **An expand state is not that.** If you find yourself reaching for a column, **stop and say why in the handoff before writing it.**

## Get the reorder rule right — both previous briefs had it wrong

`T-trips-are-derived-not-ranked` records that the PM's briefs dissented **twice**, *"having read the canvas by grep rather than reading it."*

**Reorder controls are approved — for loose ends** (**#95**). `D-menu-is-a-worklist` names manual priority and the canvas draws six of them. **What must not appear is a reorder control on a trip**, and `pool-screen-nothing-reorders-05` asserts that absence. The reason is worth keeping rather than restating as a ban:

> **A trip is a unit you clear in one stop, so the order of its items is noise; a loose end is a thing you decide about, so it earns a control.**

**One thing the canvas genuinely draws that this product refuses:** a per-group note — *"One stop clears all 3."* — computed from a hardcoded list of which tags are **places** and which are **sittings**. That needs Trellis to know `@homedepot` is a shop, **which is the managed taxonomy `D-context-tags-are-the-taxonomy` exists to refuse.** If you touch the header, that is the thing not to copy.

## Watch

1. **Undo matters more for the group action than anywhere.** In the pool, unchecking is the undo (`D-a-trip-survives-being-worked`) — **but undoing six at once by unchecking six is not undo.** Say what reverses a group completion. #111 was narrowed to Committed precisely because the pool had per-item unchecking; this reopens it for the group case.
2. **The empty state.** A group with nothing open left now exists and #127 asserts it. **The complete-group button must not appear, or must not act, in it** — and a panel reading *"0 of 3 done"* with nothing to do is #103's half-pass trap wearing the new label.
3. **`T-cross-capability-invariants-need-an-owner`** — a group completion goes through `mark_done`'s front door, not a second write path. `mark_done` is the only capability declaring `mod store;` privately; keep it that way.
4. **`T-set-operations-execute-in-the-store`** — completing N tasks is one statement, not N round trips through a loop in a handler.
5. **A fixture that archives rows directly proves nothing.** Go through the route.
6. **#129 is open on this same panel** — it changes when a group stops being a trip. **No scope overlap, but say which you expect to land first.**
7. **An htmx round trip to expand a list is a real cost on a phone over a tailnet.** If you go that way, say what it costs.
8. **DRY headroom is zero until #130 lands** (in flight, `scripts/analyzers/dry.sh` only). #127 finished at exactly 3.00%.
9. **All 21 acceptance features pass untouched**, and #127's scenarios must still hold.
10. Base branch is **`trunk`**; cut from `origin/trunk`. Scratch in `./tmp/`. **Open the pull request when QA is done and label it `preview`.**

## Out of scope

The trip threshold and `VISIBLE_TRIP_ITEMS` (three is settled), loose ends, dismissing a group (**this is completion** — nothing in Trellis can kill a task and `T-archived-at-only` still has no discriminator), Committed, Quota, when a group stops being a trip (**#129**), and **any visual design system**.

## Source

- Issues **#120** and **#125** · `crates/trellis-server/templates/pool_body.html` · `crates/trellis-server/src/pool/view.rs` · `crates/trellis-server/static/trellis.css:772-780`
- `docs/design/Trellis.dc.html:210` — the button the canvas draws
- `docs/decisions.md` — `D-bulk-completion-is-explicit`, `D-a-trip-survives-being-worked`, `T-trips-are-derived-not-ranked`, `T-canvas-is-authoritative-where-it-speaks`, `T-a-check-must-be-seen-to-fail`
- PR **#126** — what asserting at the wrong tier costs, permanently · PR **#127** — the `✕` already on this header
