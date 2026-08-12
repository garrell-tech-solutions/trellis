# Trellis — Decisions Log

Append-only. Newest at the bottom of each section.

This file exists to record **why**, and especially **what was rejected and on
what grounds**. The project board tracks what we are doing; it cannot tell a
future contributor that a plausible-sounding idea was already considered and
refused. If you are about to propose something listed under "Rejected", read
the reasoning first — and if you still disagree, add a dated entry arguing
against it rather than quietly reversing it.

- **State** (what is in flight, what is done): the project board —
  https://github.com/orgs/garrell-tech-solutions/projects/4
- **Phases and acceptance criteria**: GitHub Milestones and their epic issues.
- **Architecture**: `docs/design/brief.md`.
- **Rationale and rejected options**: this file.

---

## Settled — product

| # | Decision | Reasoning |
|---|---|---|
| D1 | Single user, single tenant. | "Family" is a domain of the user's own time, not a shared household schedule. |
| D2 | Pool is the default task kind; committed is the exception. | Motion's schedule-everything model is exactly why it churns. A calendar that is ~40% committed is one the user trusts, because a block exists only because it had to. |
| D3 | Guardrails never yield to deadlines. | A P1 hard deadline with Work hours full raises a conflict; it does not breach the wall. An override that exists will get used, and then the walls are decorative. |
| D4 | Silence on a check-in means **done**. | Assuming "not done" is intuitively safer but is the exact mechanism by which every abandoned task app decayed: unanswered work accumulates and the backlog inflates with phantom obligations. |
| D5 | Kill means archive. Keep the row; build no browsable archive UI. | The row feeds the reckoning ("47 archived this quarter, 31 Learning" is real signal). The moment an archive is browsable it becomes a place to hide from decisions. |
| D6 | A skipped weekly review ages everything one more week. No deeper sweep. | The week the user skips is the week their life was chaotic — the worst possible moment to punish them. Aging is self-correcting. |
| D7 | Inaction archives. Survival requires a deliberate act. | Inverts the property shared by every task app the user has abandoned, where inaction preserved tasks. |
| D8 | Three-strike: on the third deferral, "keep" is removed. Commit or kill. | A task protected weekly but never touched is a lie. This is the one place in the system where friction is a feature; everywhere else, optimize it away. |
| D9 | Pool tasks never appear on the calendar — not even as all-day chips. | The calendar's entire value is that everything on it is true. Ambient chips are the first crack. |
| D10 | The menu returns exactly three options plus a rest option. | A long ranked list is a blank page with extra steps — the same decision fatigue the product exists to eliminate. |
| D11 | Emergent gaps are offered, not filled. | Dragging Thursday's deep work into a random Tuesday hole is behaving badly. Recompute committed work at domain boundaries; surface the menu mid-stream. |
| D12 | Staleness threshold is deliberately unset; instrument first, tune at the first monthly reckoning. Per-domain config. | The pool's turnover rate is unknown until the system runs. Seeded at 21 days / ≥3 offers. |

## Settled — technical

| # | Decision | Reasoning |
|---|---|---|
| T1 | Rust. | The scheduler core is interval arithmetic over sum types, the invariants are property-testable, and the artifact is a static musl binary plus one SQLite file. |
| T2 | SQLite + `sqlx`, WAL mode. | Self-hosted Supabase is ~8 containers solving problems this product does not have. All queries stay inside `scheduler-db` so a Postgres swap stays mechanical. |
| T3 | `jiff`, not `chrono`. | Guardrails are civil wall-clock. `jiff` models zoned vs civil time as distinct types and forces an explicit decision at a DST gap. Store UTC epoch millis, convert at the boundary. |
| T4 | `scheduler-core` must not depend on tokio. | If it compiles without an async runtime, the algorithm has been kept honest. Enforced in CI from M0. |
| T5 | Hand-rolled `reqwest` + `serde` Google Calendar client. | Only six operations are needed; ~300 lines fully controlled beats fighting a generated crate. |
| T6 | Greedy-with-repair, not a constraint solver. | Explainable and fast enough for one user. Keep the interface clean so an optimizer can be swapped in behind it later. |
| T7 | Google Calendar is a render target, never a source of truth for task blocks. | |
| T8 | Capture surfaces for v1: web quick-box and Telegram. CLI and iOS Shortcut deferred. | |

## Rejected

| # | Rejected | Why |
|---|---|---|
| R1 | Any guardrail override, however well-guarded. | See D3. |
| R2 | A browsable archive UI. | See D5. |
| R3 | Pool tasks on the calendar in any form. | See D9. |
| R4 | Automatic promotion of pool → committed on aging. | A task the system unilaterally puts on the calendar because it is old is a task the user ignores, and ignored blocks destroy calendar credibility. Age raises menu ranking and flags in review; the decision stays with the user. |
| R5 | Multi-tenancy, accounts, auth, RLS. | Single user. |
| R6 | Collaboration, shared calendars, delegation, colleague-visible busy time. | Task blocks are advisory to the user, authoritative to the system. |
| R7 | A plugin or general-purpose extensibility surface. | |
| R8 | Supabase (self-hosted or otherwise). | See T2. If Postgres is ever genuinely needed, run plain Postgres. |
| R9 | Google Calendar push webhooks for v1. | `watch` channels need a public HTTPS endpoint, which fights self-hosting. Poll every 60–120s; a tunnel can be added later if instant reaction is wanted. |
| R10 | Incremental patching of the plan / mutation in place. | The scheduler is recomputed from scratch on every trigger. Same inputs, same output. |

## Open

| # | Question | Status |
|---|---|---|
| O1 | Staleness threshold and offer count. | Deliberately unset. Instrument from M1, tune at first monthly reckoning. See D12. |
| O2 | Recurring quotas in v1. | **Blocks the task schema and therefore M1.** Recommendation on the table: commit the three-variant sum type (`Committed \| Pool \| Quota`) at M1, ship quota *scheduling* at M8. Awaiting decision. |
| O3 | Weekend check-in density. | Fewer domain transitions means fewer natural check-in points; a coarser Sunday sweep may be needed. Revisit at M6. |
| O4 | Google OAuth refresh-token expiry. | Verify against current Google docs rather than trusting prior notes; policy shifts. Either way, design the token store assuming re-auth happens and surface it loudly. |
| O5 | Menu diversity scoring. | "One quick win, one that matters, one you've been circling" is a stated intent with no defined mechanism. Needs specification before M3.5. |

---

## Version notes

### 2026-08-12 — Initial log

Extracted from the settled product brief. Board created, milestones M0–M9 plus
spike S3 opened.

**Source-of-truth split adopted:** the board holds state, milestones and epic
issues hold acceptance criteria, `docs/design/brief.md` holds architecture, and
this file holds rationale. No overlap, so nothing needs syncing. A
`docs/roadmap/` document was drafted and removed as duplicative of the board —
note this departs from the `swarmforge/roles/PM.prompt` convention, which
expects a roadmap document under `docs/roadmap/`; the departure is deliberate.

**Spec corrections raised against the brief, awaiting decision** (C1–C5, tracked
as issues): pins belong in the Constraints layer rather than as a bool on Block;
the "delete every block and regenerate" property is false as written and needs
the fact/plan line drawn inside the Block table; `Block::missed` is unreachable
under D4; auto-close undo is unimplementable without storing pre-close
`remaining_minutes`; and per-domain capacity accounting is incoherent with
`allowed_windows`.
