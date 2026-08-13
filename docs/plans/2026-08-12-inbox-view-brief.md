# Handoff brief — `inbox-view`

**Date:** 2026-08-12 · **Issue:** #30 · **Milestone:** M1 — Capture + Triage · **Route:** pipeline

---

## Demo

This is the first slice under **D14**, and D14 is the reason it exists. After
this merges:

```sh
cargo run -p trellis-server -- serve --db trellis.db
```

Open `http://localhost:8080`:

1. An empty inbox with a text box, saying something more useful than nothing.
2. Type `buy milk`, press enter.
3. It appears in the list below, without a full page reload.
4. Restart the server, reload the page — it is still there.

**Before this slice you cannot open Trellis at all.** The app serves two routes,
both POST, both JSON. There is no GET route, no `askama` dependency and no
template anywhere in the tree. The only way to observe the product today is
`curl -d`. This slice is what turns it into something the owner can use as an
inbox, and every later slice's demo depends on there being a page to put things
on.

Keep that demo literally true. If the final state does not do exactly those four
steps, the slice is not done — that is what D14 means.

## Goal and scope

Render the untriaged capture queue at `GET /`, with a quick-add box that creates
a capture and updates the list in place.

That is all. This is M1 story 2 pulled forward, not new scope.

### Out of scope — do not absorb these

Triage from the UI (that is the next slice), the task list, `/stats`, the
keyword classifier, any calendar or week view, authentication, and **any visual
design system**. There is no visual direction anywhere in this repo yet, and
inventing one here would be an unreviewable decision buried in a feature slice.

**Plain and ugly is correct.** Semantic HTML, minimal or no CSS. The point is
the feedback loop, not the pixels. Styling is a later, separate decision.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| **D14** | Every slice ends in something the owner can run and see. This slice *is* the enabling case; treat the Demo section above as an acceptance criterion, not a nicety. |
| **Stack** | **Askama + HTMX, served by the backend. No Node build step** (`stack.prompt`). Do not introduce a bundler, npm, or a frontend framework. HTMX as a vendored file or a plain `<script>` tag. |
| **T15** | The module boundary: rendering is a delivery concern and lives in `trellis-server::http`. Templates and `axum` types must not leak into `trellis-server::store` — there is a test in `store/mod.rs` asserting that, though see gotcha 4 about how weak it is. |
| **T4** | Nothing here touches `scheduler-core`, and the purity gate will catch it if it does. A page render is not domain policy. |
| **T2** | Queries stay in the store layer. The handler asks the store for untriaged captures; it does not write SQL. |
| **T9** | Cyclomatic complexity threshold is **8**. |

## Acceptance scenarios worth specifying

Structure and detail are yours. These are the behaviours that matter.

- The inbox renders every untriaged capture, newest first.
- A capture submitted through the quick-add box appears in the list without a
  full page reload.
- **The box and `POST /captures` are one code path, not two.** A capture created
  either way is the same row with the same fields. The existing
  `capture_endpoint.feature` must still pass untouched — if it needed changing,
  something was rebuilt that should have been reused.
- The empty state renders a message, not a blank list.
- A capture that has been triaged into a task does **not** appear in the inbox.
  (`captures.triaged_at` exists as of PR #28 — that is the discriminator.)
- **Hostile capture text is escaped, not interpreted.** `<script>alert(1)</script>`
  and quote characters render as literal text. This one is worth a scenario of
  its own; it is the defect most likely to survive review, because the happy
  path looks identical either way.

## Known repo gotchas

These are live and they will cost you time.

1. **The generated acceptance entrypoints are gitignored** —
   `crates/acceptance-tests/tests/*_acceptance.rs`, regenerated from
   `features/*.feature` by `scripts/acceptance/run.sh`. A fresh checkout has
   none, so **`cargo test --workspace` compiles zero acceptance tests and still
   passes**, reporting green for a suite that never ran. Always run
   `scripts/acceptance/run.sh`. Tracked as #26.
2. **Do not copy `task_kinds.feature`'s step style.** Its step regexes match the
   placeholder `"<(\w+)>"` rather than the substituted value, so one Examples
   cell feeds both the request and the assertion — mutating it changes stimulus
   and expectation together and the scenario cannot fail. Gherkin mutation
   scores 2 survived / 0 killed on that file. `committed_triage_validation`
   does it correctly (3/3 killed); copy that one.
3. **The complexity gate is red** — worst is `triage.rs::dispatch` at 17 against
   a threshold of 8, in `crates/acceptance-tests/src/steps/`. Adding step
   handlers to those existing dispatchers makes it worse. **Put new rendering
   steps in a new step module.**
4. **T15's layering test is a substring grep**, not a proof. It reads
   `src/store/*.rs` non-recursively and greps for `axum`/`StatusCode`. A nested
   submodule or a type alias defeats it, and it skips `mod.rs` by design. Do not
   read a green run as evidence your layering is right.
5. **CI now exists** (`.github/workflows/ci.yml`, merged as PR #25) and runs
   fmt, clippy, `cargo test --workspace`, the T4 purity gate, and the musl
   release build. It does **not** run the acceptance suite — see gotcha 1.
6. **Merge conflict warning:** `docs/decisions.md` was modified on `trunk` after
   your current worktree state (D14, T17, and an ID-prefix legend). If you have
   local edits to that file from the `triage-validation` slice, expect a
   conflict and resolve it by keeping both — the file is append-only.
7. Base branch is **`trunk`**. Use `./tmp/` for scratch, not `/tmp`.

## Open questions for you

1. **How should an acceptance test assert on HTML?** The existing step handlers
   are all JSON-and-status-code. Options are substring assertions on the
   response body, or parsing the HTML. Substring matching is cheap and brittle;
   parsing needs a dependency. Your call, but whatever you pick becomes the
   pattern for every UI slice after this one, so pick deliberately and say why.
2. **How does the QA procedure drive a browser?** `scripts/qa/*.sh` drive real
   interfaces via `curl`. For a page with HTMX, `curl` can verify the HTML and
   the partial-update endpoint, but not that the browser actually swaps content.
   Decide what the QA script can honestly claim and do not let it claim more.
3. **Does the quick-add box post to `/captures` or a new fragment endpoint?**
   HTMX wants an HTML fragment back; `POST /captures` currently returns JSON.
   Reusing it means content negotiation; a sibling endpoint means two routes
   sharing one store call. The acceptance criterion is that the *row* is
   identical, not that the route is — but say which you chose.

## Dependencies and sequencing

- **Queued behind `triage-validation` (#29)**, which is with you now. Finish
  that first; the handoff helpers will deliver this when you are done.
- Low conflict risk with #29 — that slice touches validation and the store,
  this one touches rendering. The exception is `docs/decisions.md`, gotcha 6.
- **Unblocks the rest of M1.** S2 (triage from the UI), S3 (`/stats` on the
  page) and S4 (classifier confidence highlighting) all need a page to render
  onto. This is the one that makes them possible.

## Source

- Issue **#30** — acceptance criteria and the demo
- `docs/decisions.md` — **D14** especially, plus T2, T4, T9, T15
- `swarmforge/constitution/articles/stack.prompt` — Askama + HTMX, no Node
- **#9** — M1 epic, story 2, and the re-cut slice sequence
- **#26** — the ungated acceptance suite (gotcha 1)
