# Capture categorization — handoff for PM

Written for whoever picks up the PM role next. A same-day brainstorming session
(2026-08-13) designed an LLM-based auto-categorization feature for captures,
then discovered mid-design that it overlaps an existing epic. Nothing has been
executed against GitHub or `docs/decisions.md` — this doc is the reconciliation
proposal for the PM to review, adjust, and turn into a specifier brief.

## Use cases driving this

- User drops "call dentist" into the web quick-box. Today it just sits in the
  captures table as raw text. The ask is for it to arrive at triage already
  tagged (e.g. Health) and with a cleaned-up title, instead of the user typing
  a category by hand for every single capture.
- Weekly reckoning ("47 archived this quarter, 31 Learning" — the pattern D5 and
  T14 are built around) is only meaningful if captures are consistently
  domain-tagged. Right now nothing tags them at all.
- The domain set needs to be easy to extend — "add a domain" should not require
  a schema migration or a Rust enum change, because the user expects to tune it
  as the system runs (mirrors D12's stance on tuning thresholds from real data
  rather than guessing up front).

## What's already on the board

Two open issues already describe a closely related mechanism:

- **M1 — Capture + Triage (#9, unblocked).** One of its stories is "Keyword
  classifier + per-field confidence" — a rules-based classifier run at triage,
  storing per-field confidence so the UI can highlight low-confidence fields.
- **M9 — LLM Enrichment (#19).** *"Replaces the keyword classifier behind the
  same trait. Deliberately last — rules get ~80%."* Acceptance criteria:
  classifier is a trait with two implementations (swap requires no change
  outside wiring); accuracy measured against ≥100 hand-labeled captures
  collected since M1; API failure/timeout falls back to the keyword
  implementation with no capture lost.

**The gap:** as scoped, that classifier predicts task **kind** (committed vs.
pool vs. quota) plus per-field confidence on **deadline/priority** — nothing
about domain or title. What was asked for this session is a different pair of
fields on what is otherwise the same shape of problem (classify a raw capture,
cheap rule-based first, LLM later, fall back safely).

## Decision reached with the user this session

Fold `domain` and a cleaned-up `title` into the **same** classifier trait
instead of building a second, parallel categorization pipeline. Rationale: M9
already specifies the exact pattern this needs — trait with swappable impls,
keyword-first with LLM later, fallback-on-failure, evaluated against labeled
data — and duplicating that machinery for a second field set would be pure
churn. This is a recommendation from this session, not yet applied anywhere.

## Proposed changes (none applied yet — for PM review)

### 1. Issue #9 (M1 epic) — extend the keyword-classifier story

Add `domain` and `title` to what the keyword classifier produces, even though
the keyword impl's guesses will be crude (simple keyword/phrase matching for
domain; raw passthrough, no cleanup, for title). The reason to do this at M1
rather than waiting for M9 is the same reason `kind` is a three-variant sum
type from M1 (T11): retrofitting a trait's output shape after M9 already
depends on it is the expensive order to do it in.

Suggested addition to the story list:
> - [ ] Keyword classifier also produces `domain` (keyword-matched against the
>   configured domain list) and `title` (passthrough of raw_text at M1) —
>   same trait, same per-field confidence mechanism.

### 2. Issue #19 (M9 epic) — broaden the acceptance criteria

Suggested additions:
> - [ ] Classifier predicts `domain` and a cleaned-up `title`, in addition to
>   kind/deadline/priority, through the same trait.
> - [ ] Domain accuracy measured against the same hand-labeled capture set
>   used for committed/pool accuracy.

Description could gain a clause noting the trait now covers two originally
separate asks (task-kind classification, and domain/title enrichment) that
turned out to be the same mechanism.

### 3. `docs/decisions.md` — new entry

Recommend a `T`-numbered entry (next number after the current max) recording:
domain/title categorization was folded into the existing M1/M9 classifier
trait rather than built as a standalone system, because M9 already commits to
the trait/fallback/eval pattern this needs and a second pipeline would
duplicate it. Worth an entry specifically because a future contributor could
plausibly propose "just add a categorization worker" again without knowing
M9 already covers this ground — that's exactly what this file is for.

## Technical constraints already settled this session

For whoever specs the classifier's LLM implementation:

- **Domain list is plain data, not a Rust enum.** Provisional set: Work,
  Health, Home, Learning, Social. Stored as a string column / a
  `const &[&str]` list, deliberately decoupled from the future scheduler
  `Domain` sum type (T9/T11, which governs guardrails/capacity and has its
  own reasons to be a strict enum). Adding a domain here should be a one-line
  edit, no migration. Note `docs/design/brief.md` still doesn't exist, so
  this list has no other home right now — flagged in the 2026-08-12 PM
  handoff as the biggest documentation gap, still true.
- **Provider: OpenRouter**, not a direct Anthropic client, specifically so the
  model is swappable without a code change — pin via an env var
  (`OPENROUTER_MODEL`) with a cheap default, not hardcoded.
- **Hand-rolled `reqwest` + `serde` client**, consistent with T5's precedent
  for the Google Calendar client (no generated SDK, small surface).

## Open question — not resolved, flagged for the PM/specifier

The original design (before this reconciliation) assumed a background
`tokio` worker polling the captures table asynchronously, so `POST /captures`
stays inside its 50ms budget. But M9's AC — "swapping requires no change
outside wiring" for a **trait** — reads like a synchronous
`fn classify(&self, raw_text: &str) -> Classification` call made at triage
time, not an async background poller. Whether classification happens
synchronously at triage (blocking on the LLM call there) or asynchronously
right after capture (a background worker that fills in fields before the
user reaches triage) is a real design decision with different latency and
UX implications, and it wasn't settled this session. Whoever specs M1's
classifier story should decide this explicitly rather than inheriting the
async-worker assumption by default.

## Process note

If any of this work involves editing the GitHub Project's single-select
fields (e.g. reassigning Status), note: `updateProjectV2Field` on a
single-select **wipes that field's value on every item in the project** and
regenerates option IDs — confirmed on this board 2026-08-12. Snapshot
`gh project item-list` before touching any single-select field's options.

## Not done in this session

- No GitHub issues edited (#9, #19 unchanged).
- No `docs/decisions.md` entry added.
- No code written, no schema changed.
