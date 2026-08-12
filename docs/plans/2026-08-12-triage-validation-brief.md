# Handoff brief — `triage-validation`

**Date:** 2026-08-12 · **Issue:** #29 · **Milestone:** M1 — Capture + Triage · **Route:** pipeline

---

## Sequencing gate — read first

**Do not start until PR #28 (`m1-triage`) is merged into `trunk`.**

This slice modifies the `tasks` table and the triage handler that #28 creates. If
you branch from `trunk` before #28 lands, there is no `tasks` table, no
`TaskKind`, and no triage endpoint to harden. Check with `gh pr view 28` — if it
is still open, stop and say so rather than working around it.

## Goal

Triage currently accepts and durably stores arbitrary strings in fields whose
domains are closed. Close that, and give the existing `422` rejection behaviour
the acceptance coverage it never got.

Verified over HTTP against a running server — every one of these returns **201**
and persists:

```
deadline="banana"                           -> 201, stored
deadline="2026-13-45T99:99:99Z"             -> 201, stored
deadline="'); DROP TABLE tasks;--"          -> 201, stored
deadline_type="squishy"  priority="P9"      -> 201, stored
deadline=""  deadline_type=""  priority=""  -> 201, stored as a committed task
kind="quota" with no target fields at all   -> 201, stored as a quota with no target
```

## Scope

1. **Parse `deadline` to epoch millis at the boundary, with `jiff`.** Store
   `INTEGER`. Reject unparseable input the way unknown `kind` is already
   rejected. This is T3 compliance — see "Decisions already made".
2. **`deadline_type` and `priority` become sum types in `scheduler-core`**,
   alongside `TaskKind`, validated at the same boundary. `hard`/`soft` and
   `P1`–`P4` currently exist only as Examples values and doc comments.
3. **`require()` must reject empty strings.** It is currently
   `value.clone().ok_or(...)`, so `Some("")` satisfies a required field.
4. **Decide and enforce the quota case.** `TaskKind::Quota` takes
   `target_count: Option<i64>` and `target_minutes_each: Option<i64>` with no
   validation, so `{"kind":"quota"}` alone creates a quota task with no target.
   See "Open questions" — this one needs a call before you specify it.
5. **`CHECK` constraints** on `tasks.kind`, `deadline_type`, `priority`.
   Defence in depth behind the boundary check, not a replacement for it.
6. **Acceptance + QA coverage for every rejection**, including the *existing*
   `422 {"unknown_kind": <submitted>}` behaviour from T16, which today has no
   Gherkin and no QA procedure — it lives in two unit tests only.

### Out of scope

Anything else in M1 — the pool 3-POST path, the untriaged queue UI, the keyword
classifier, `/stats`. Those are separate slices. Do not absorb them.

Also out of scope: fixing the complexity violations described under "Gotchas".
They are real and they are someone's problem, but not this slice's.

## Decisions already made — confirm, don't re-litigate

| | |
|---|---|
| **T3** | `jiff`, not `chrono`. **Store UTC epoch millis, convert at the boundary.** This is the whole basis for item 1. `created_at_ms` and `archived_at` are already `INTEGER`; `deadline TEXT` is the outlier inside the same table. |
| **T11** | `kind` is a three-variant sum type — `Committed \| Pool \| Quota`. Quota *scheduling* waits for M8; quota *creation* is M1. |
| **T16** | Unrecognised `kind` is rejected at the boundary with `422 {"unknown_kind": <submitted>}`. Keep this behaviour and its response shape. You are adding coverage for it, not redesigning it. |
| **T15** | The module boundary: `scheduler-core` holds the rules, `trellis-server::http` translates requests into core inputs, `trellis-server::store` translates core types into rows. New validation types belong in `scheduler-core`, next to `TaskKind`. Adapters name core types; the core names neither. |
| **T9** | Cyclomatic complexity threshold is **8**. |
| **D3** | Guardrails never yield to deadlines. Relevant because `priority` and `deadline_type` are the fields the M3 scheduler branches on — that is why their domains have to be closed now. |

`tasks` has no production data, so `deadline TEXT` → `INTEGER` is a free change
today and a data migration after the first real row. That is the reason this
slice is being run now rather than later.

## Acceptance scenarios worth specifying

Your call on structure and detail — these are the behaviours that matter, not a
prescription of your feature files.

- A committed triage with a well-formed ISO-8601 deadline is accepted, and the
  stored value round-trips to the same instant.
- A committed triage with an unparseable deadline is rejected, names the field,
  and creates nothing. The capture stays in the untriaged queue.
- A syntactically-plausible but invalid instant (`2026-13-45T99:99:99Z`) is
  rejected — this is the case a naive regex passes and a real parse catches.
- Empty string is rejected for each of the three committed-required fields,
  distinctly from the field being absent. Both are rejections; specify whether
  they report the same way.
- `deadline_type` outside `hard`/`soft` is rejected. `priority` outside
  `P1`–`P4` is rejected.
- An unrecognised `kind` is rejected with `422 {"unknown_kind": <submitted>}` —
  **existing behaviour, currently unspecified.** Cover the absent-`kind` case
  too; it currently yields `{"unknown_kind": null}`.
- Whatever the quota answer turns out to be (see below).
- Every rejection leaves the task list unchanged and the capture untriaged.
  This is the invariant that makes the others worth having.

## Known repo gotchas

These cost me real time this session. Some are live defects you will trip over.

1. **The generated acceptance entrypoints are gitignored** —
   `crates/acceptance-tests/tests/*_acceptance.rs`, regenerated by
   `scripts/acceptance/run.sh` from `features/*.feature`. A fresh checkout has
   none, so **`cargo test --workspace` alone compiles zero acceptance tests and
   still passes.** It will tell you your suite is green when it never ran. Run
   `scripts/acceptance/run.sh`. Tracked as #26.
2. **Step regexes match the placeholder, not the substituted value.**
   `task_kinds.feature` matches `"<(\w+)>"`, so a single Examples cell feeds
   both the request body and the assertion. Mutating the cell changes stimulus
   and expectation together and the scenario cannot fail — Gherkin mutation
   reports **2 survived, 0 killed** on that feature. `committed_triage_validation`
   does not have this problem (3/3 killed). **Do not copy `task_kinds`' step
   style.** Assert against a literal expectation, or the mutation gate will pass
   on scenarios that verify nothing.
3. **The complexity gate is red on `trunk`** — 3 violations, all in
   `crates/acceptance-tests/src/steps/`, worst is `triage.rs::dispatch` at 17
   against a threshold of 8. `docs/decisions.md` currently claims these are
   "identical before and after" #28; they are not — `trunk` before #28 had 1.
   Adding more step handlers to those dispatchers makes it worse. Prefer a new
   step module over extending `triage.rs::dispatch`.
4. **No CI on branches cut before #25.** The CI workflow (#25) may or may not be
   on `trunk` when you start. If `gh pr checks` reports nothing on your PR, that
   is why — run the gates locally and do not read silence as green.
5. **`then_triage_is_rejected` accepts any status in `400..500`.** An axum
   JSON-extractor 400 would satisfy "the triage is rejected". If you assert a
   specific code, assert it.
6. **`serde_json::from_slice(&bytes).ok()`** in the triage steps swallows parse
   failures into `None`, which then reports as "no rejection body recorded".
   Misleading diagnostics when a step fails for an unrelated reason.
7. Base branch is **`trunk`**, not `main`. Use `./tmp/` for scratch, not `/tmp`.

## Open questions for you

1. **The quota gap.** `{"kind":"quota"}` with no target fields creates a quota
   task with `target_count: None`. Is a quota task without a target meaningful,
   or are `target_count` / `target_minutes_each` / `period` required for kind
   `quota` the way deadline/type/priority are for `committed`? T11 says the
   *columns* are nullable — that is a schema statement and does not settle
   whether the *triage path* may omit them. My read is they should be required
   at triage, because a quota with no target cannot be scheduled at M8 and
   cannot be reported on at the reckoning. **Raise it rather than assuming; if
   you and the user settle it, it belongs in `docs/decisions.md`.**
2. **Do absent and empty report identically?** `{"deadline": ""}` and a missing
   `deadline` key are both rejections. Same error shape, or distinguished? The
   existing rejection body is `{"missing_field": <name>}`.
3. **Is `period` a closed set too?** It is a free string today. If quota targets
   become required, `period` probably wants `week`/`month` rather than anything.

## Dependencies and sequencing

- **Blocked by PR #28** — see the gate at the top.
- **Blocks nothing directly**, but M2 is where guardrail interval arithmetic
  starts consuming deadlines and M3's forward pass sorts on `priority`. A typed
  deadline arriving before M2 means the DST work in M2 starts from a real
  instant rather than a string, which is T3's entire point.
- Unticks **#9 AC-3**, which I left unticked on the M1 epic precisely because of
  the empty-string hole.

## Source

- Issue **#29** — the defect, with the full HTTP transcript
- PR **#28** review comment — how this was found
- `docs/decisions.md` — T3, T9, T11, T15, T16, D3
- **#9** — M1 epic and its acceptance criteria
- **#26** — the ungated acceptance suite (gotcha 1)
