# Trellis — Architecture Reference

**What this is:** the vocabulary the rest of the project refers to. Type shapes,
layer names, the scheduler's signature, the closed enums. Nothing else lives
here — the board holds state, milestones hold acceptance criteria, and
`docs/decisions.md` holds rationale.

**What this is not:** the product brief. That is the settled narrative of what
Trellis is for, and it is not in this repo. Every epic's `## Source` line points
at `docs/design/brief.md`, which has never existed; those links should point
here instead.

Every row is marked with what it is:

| | |
|---|---|
| **built** | exists in `crates/`, verifiable today |
| **specified** | fixed by a decision or an epic's acceptance criteria; not built |
| **removed** | was built and is deleted. The reasoning is kept; the code is gone. |
| **GAP** | referred to by name elsewhere and defined nowhere. Needs a decision. |

---

## What #88 removed, and how to read this file now

**2026-08-20, `remove-unused`.** Nine capabilities were deleted outright:
life areas, guardrails, free time, exceptions, capacity, stats, the nav, the
app shell's header, and the server-side schedule. `D-dogfood-first` decided
it: the product had six pages, four of them read-only instruments feeding a
solver that did not exist, and none of it on the path to daily use.

**Sections below marked *removed* describe code that is gone.** They are kept
rather than deleted because the demolition was a decision about *what the
owner uses*, not a judgement that the reasoning was wrong — M3 is paused, not
cancelled, and `docs/decisions.md` still holds every `T-` and `D-` row those
sections cite. A revival reads them; a reader asking "what does Trellis do
today" must skip them.

**What is left is one path**: capture a thought, see it in the inbox, triage
it into a task, or dismiss it. That is the whole product.

> **This file is the thing most likely to be wrong after a demolition**, and
> it was: #88 deleted nine capabilities and changed not one line here, so the
> reference described a product that no longer existed. Marked up rather than
> rewritten, so the gap between "what we decided" and "what exists" stays
> visible instead of being quietly closed.

## Crates — built

```
crates/scheduler-core     pure rules. no tokio, no sqlx, no async (T-core-no-tokio)
                          interval · schedule · task · timezone
crates/trellis-server     axum + sqlx + askama. binary name `trellis`
crates/acceptance-tests   APS runtime + step handlers; entrypoints generated from features/
```

**`scheduler_core::schedule` and `::interval` have no caller** since #88
deleted `trellis-server/src/schedule/`. Kept deliberately, and said out loud
in `trellis-server/src/lib.rs` so it is not mistaken for an oversight: M3 is
paused rather than cancelled, and 775 subtle lines carrying 1000-case
property tests are expensive to rebuild.

> **GAP — un-pausing M3 is not just re-attaching a caller. `ScheduleTask`
> requires two fields the domain has stopped producing.**
>
> | field | required by | last written |
> |---|---|---|
> | `life_area_id: i64` | `ScheduleTask`, and `LifeAreaWindow` is keyed by it | #88 deleted life areas; triage writes `NULL` |
> | `deadline_type: DeadlineType` | `ScheduleTask`, and `T-hard-refuses-soft-slips` branches on it | #94 retired it; triage writes `NULL` |
>
> `triage::store::insert_task`'s `INSERT` binds a literal `NULL` for both,
> and a test pins the `deadline_type` one. So every task triaged since #94
> carries neither, while the code that would place it needs both — and
> `LifeAreaWindow`, the shape the whole placement search is organised
> around, names a concept with no table behind it.
>
> **`#94`'s own note that "schedule.rs carries its own independent copy of
> that field" is exactly right about the code and is what creates this:**
> independence means the field has no source. Nothing is broken today
> because nothing runs. What it costs is that reviving M3 needs *decisions*
> first — does at/by map onto hard/soft, or does the backward pass branch on
> commitment instead; does the scheduler get life areas back, or group by
> context tag now that tags are the taxonomy (`D-context-tags-are-the-
> taxonomy`) — and those are the specifier's, not a re-attachment.
>
> Worth stating because the cost of keeping those 775 lines was recorded as
> "expensive to rebuild", which is still true, and reads as though the only
> thing standing between them and use is a caller. `::ratio` went with `stats` on the
opposite reasoning — 223 lines whose settled numbers live in
`docs/decisions.md`, so deleting the code destroyed nothing.

The brief specified six crates. Three exist. `scheduler-db` collapsed into
`trellis-server` — settled as a module boundary rather than a crate split, and
ratified by `T-package-by-business-domain`: the brief's other three
(`scheduler-web`, `scheduler-bot`, `scheduler-bin`) are role names too, and a
crate earns existence when a capability needs a real boundary — a purity gate,
an independent dependency set — not because a layer has a name. That closes
#24. Renaming the crates that exist is deliberately not part of it:
`scheduler-core` is named twice in the constitution, which no agent may edit.

## How it runs — built (`#98`, `#101`)

**Until 2026-08-22 this section could not have been written**, and that was
the problem: the server was a debug binary launched by hand on `127.0.0.1`,
the newest release build on disk was 179 commits old, and nothing restarted
it or survived a reboot. `#83` closed "Capture works from a phone, anywhere"
on a viewport and a manifest — a stylesheet, with nothing on the network to
load it.

```
ops/trellis.service   systemd --user unit; ExecStart trellis serve --addr 127.0.0.1:8080
ops/install.sh        first install; runs loginctl enable-linger
ops/update.sh         fetches CI's release artifact; no local compile
ops/backup.sh         the SQLite file
```

**A `--user` unit with linger, not a system unit.** The host defaults to
`Linger=no`, so a user unit would die when the desktop session ended —
silently, with nothing connecting cause to effect. `enable-linger` keeps it
running from boot with nobody logged in, and costs root exactly once instead
of on every future edit to the unit file.

**The binary is CI's artifact, not a local build.** `ci.yml`'s gate job
already builds `--release --target x86_64-unknown-linux-musl` and asserts it
is static; `update.sh` fetches that exact artifact from the last green run
on `trunk`. So the running binary is byte-identical to the one CI proved,
and no musl release build competes with a mutation swarm for CPU on the same
box.

**Reachable over the tailnet, still bound to loopback.** `tailscale serve`
reverse-proxies to `127.0.0.1:8080`; the unit binds nothing wider.
`R-multi-tenancy` holds — single user, local only — and being reachable from
a phone does not weaken it.

> **Two things already recorded here stopped being theoretical the day this
> landed.** The 50ms capture budget is now measured against a real server
> over a network rather than a loopback socket (`T-latency-is-a-qa-assertion`
> put that assertion in the QA suite, which is where it can be measured
> honestly). And the committed screen's UTC date cell, filed as a gap under
> `#94`, is now something the owner can *see* — a weekday and an hour they
> never typed — rather than a latent disagreement between a comment and a
> settled decision.

## Two things are called "layers". They are unrelated.

### Domain layers — specified (`T-fact-plan-line`)

Which rows the scheduler may destroy. The line runs **inside** the `Block`
table, drawn by block state.

| Layer | Members | Behaviour |
|---|---|---|
| **Plan** | `proposed`, `published` future blocks | Disposable. Deleted wholesale and regenerated on every recompute. |
| **Constraints** | `in_progress`, `completed`, `missed` blocks; pins | Immutable inputs. Read by the engine, never written by it. |

"Facts" is informal shorthand for the immutable block subset. It is not a third
layer.

### The module boundary — built (`T-module-boundary`, `T-templates-take-view-models`)

Which code may name what. Dependencies point inward, and the tree names
business domains rather than technical roles (`T-package-by-business-domain`,
#44). What went away is `http/` and `store/` as *top-level directories*; the
dependency rule itself is unchanged.

```
crates/trellis-server/src/
  capture/    mod.rs  http.rs  store.rs
  dismiss/    mod.rs  http.rs
  committed/  mod.rs  body.rs  http.rs  store.rs  view.rs
  inbox/      mod.rs  http.rs  lists.rs  store.rs  view.rs
  mark_done/  mod.rs  store.rs
  pool/       mod.rs  body.rs  http.rs  store.rs  view.rs
  settings/   mod.rs  http.rs  store.rs
  triage/     mod.rs  http.rs  store.rs
  platform/   app.rs  assets.rs  boundary.rs  clock.rs  db.rs
              mod.rs  nav.rs  request.rs  response.rs  test_support.rs
```

**Eight capabilities and one bucket.** #88 took it from ten to five —
`capacity`, `exceptions`, `free_time`, `life_areas` and `stats` are gone —
then `#92` added `pool`, `#94` added `committed`, and `#97` added
`mark_done`.

**A `body.rs` is a fragment a screen can swap on its own**, and two
capabilities grew one at `#97` for the same reason `inbox::lists` has one:
the response to a mark-done `POST` is not a page. A capability earns one
when something other than a full page render needs its content — the same
"only when its page shape is its own" test that decides whether it gets a
`view`.

**`mark_done` is a capability rather than a function on either screen**
because both screens need the write and the write has nothing to do with
either screen's shape — `T-inbox-owns-membership`'s reasoning, one exit
over. `pool::http` and `committed::http` keep their own routes and re-render
their own fragments, because only they know what "done" removed a row from.

> **`mark_done` declares `mod store;` — private — and it is the only
> capability that does.** Every other one is `pub mod store`, which today
> buys nothing: no capability names another's store in production, and the
> only external caller of any of them is `platform::test_support`'s shared
> `insert_capture` fixture. `platform/boundary.rs` enforces the rule as a
> substring lint that architecture.md itself calls "a lint, not a proof";
> a private module is the proof. `inbox` and `capture` have taken the same
> step function-by-function (`pub(super)` on the writes that matter).
> **`mark_done` is the shape to copy, not the exception.**

**`platform/nav.rs` is back**, deleted by #88 and rebuilt for `#92`, now
three pages of the design's four: `ALL` names only what `app::build_app`
can actually route to, because a dead link is worse than no link. The guard
that makes the nav and the route table agree came back with it —
`app::every_header_link_reaches_the_page_it_names` walks `ALL`, fetches each
page over the real router, and asserts it resolves and marks exactly itself
current. It walks the list rather than restating it, which is what makes it
cover the third and fourth tabs when they arrive.

> **`settings` is now reachable from nowhere.** Its only reader was
> `free_time`, so `POST /timezone` has no UI path to it. Stated in the
> demolition rather than left to be discovered — a one-way door held open
> until `#85`, not an oversight.

> **The `capabilities.len() >= 5` floor in `platform/boundary.rs` had no
> slack at all after #88 and has one capability's worth again after `#92`.** That check asserts a minimum so a walk that stops covering
> the tree fails loudly instead of passing vacuously; with ten capabilities
> it had slack, and with five it has none. The next legitimate deletion trips
> it and forces someone to lower it consciously, which is the forcing
> function working — but it is no longer also a smoke alarm.
`scheduler-core` holds the rules and names neither adapter; a domain's `http`
turns requests into core inputs and core types into view models; its `store`
turns core types into rows and is the only production SQL; its `view` is what
a template renders, never a `store` row type. `capture`, `triage` and
`dismiss` reach into the inbox because the inbox is the surface they act on.

**Two capabilities have no `store`, and that is the shape rather than an
omission.** Everything `dismiss` does to the database is *take this capture
out of the inbox* — the inbox's own fact, reached through the inbox's front
door. `free_time` stores nothing at all: it computes what it shows from
`life_areas::guardrails` and `settings::current_timezone` plus the clock. A
capability owns the SQL it issues; these two issue none, so what is left of
each is one handler.

A domain has a `view` only when its page shape is its own: `inbox`'s rows are
assembled from two queries and carry a slot for an in-flight rejection, while
`stats` renders `scheduler_core::ratio`'s answer directly, and a struct
copying that field for field would be a view model in name only.

**A capability read by other capabilities gets one front door.** Three
capabilities need the pickable life areas — the management page lists them,
the inbox fragment offers them in each capture's triage forms, and a
quick-added capture renders its own row with the same forms. They call
`life_areas::active_options`, in that capability's `mod.rs`, rather than each
composing `store::list_active` with `LifeAreaOption::from` themselves: a
reader that knows which query *and* which mapping to combine is holding a
copy of another capability's internals, and three of those drift. Reaching
across for a *type* (`inbox::view::CaptureRow`, `life_areas::view::
LifeAreaOption`) stays fine; it is reaching across for the recipe that does
not.

**`life_areas` answers "does this name a life area work may be filed
under?" for everyone who asks.** `life_areas::active_id_for_name` exists
because triage (before writing a task) and exceptions (before writing a
dated exception scoped to one life area) both validate a submitted name at
their own boundary, and arrived as two byte-identical `find_active_life_area_id`
functions in two `store.rs` files. `T-capability-owns-its-queries` licenses
separate copies for *two facts*; this is one fact asked twice for the same
reason, and the answer belongs to the capability that owns the table, the
name-identity rule and the archived/active distinction. What "active" means
was about to grow a second dimension — `pool_only` — which is exactly the
kind of change two copies get wrong.

The **inbox** is the second capability to need one, and it needs three
things in its door:

```
inbox::capture_is_open(pool, id)              is it still in the inbox?
inbox::close_capture(pool, id, at_ms)         take it out, stamping when
inbox::render_lists(pool, status, error)      the #lists fragment as a response
inbox::CAPTURE_NOT_OPEN_MESSAGE               the prose for !capture_is_open
```

**Why membership is the inbox's and not the asker's.** The worked example
under `T-capability-owns-its-queries` was this very table — *"`triage` stamps
`triaged_at`"* — and it was right while the column meant "triage happened".
Migration `0005` renamed it `left_inbox_at` and widened it to "left the
inbox, by either exit", and the rename moved the fact: `left_inbox_at IS
NULL` is now the definition of `list_untriaged`'s `WHERE` clause, asked about
one row instead of all of them. Triage and dismissal ask the same question
for the same reason, which is one fact, not the two facts that decision
licenses separate copies of. `dismiss` shipped with a byte-identical copy of
both the check and the write; the third exit would have made a third.

`inbox::lists` is private to the inbox for the same reason: `ListsTemplate`'s
three fields were named by two other capabilities, so adding a fourth list to
the fragment was a four-file change. `render_lists` is also the single
implementation of `T-forms-swap-one-fragment`'s response contract — a 422
whose body is not the re-rendered fragment breaks the whole page, since the
422 swap is configured globally.

## The app shell — removed (#88; was `#58`)

> `platform/nav.rs` and `base.html`'s `<header>` are deleted. With the Menu
> unbuilt there is one screen, so there is nothing to navigate between and
> the header should not render at all until `#85`. `T-nav-is-the-site-map`
> needed no superseding: removing the routes collapsed the header on its own.

Every page carries the same header, from one definition. Settled by the
`app-shell` slice, 2026-08-17; recorded here because the shell is the frame
every page Trellis will ever have inherits, and it was answered in a commit
message.

```
templates/base.html          <head>, the 422 config, <header><nav>, {% block content %}
platform/nav.rs              Page, NavLink, links(current) -> Vec<NavLink>, ALL
```

**Inheritance, not an included partial.** `{% extends "base.html" %}` makes
"every page carries the header" structural: a page that forgets an
`{% include %}` still compiles and renders, while one with no `{% extends %}`
has no frame at all and cannot pass review by accident.

**A page declares only which page it is.** A handler passes
`nav: nav::links(Page::X)`; `platform::nav` decides `current` per link and
`base.html` only iterates, never comparing a page's name to anything
(`T-templates-take-view-models`). No page's template struct encodes another
page's identity, so a fourth page is one enum variant, one row in `nav::ALL`,
and one handler — and no page already shipped changes. Askama makes the
field itself compulsory: a template extending `base.html` without a `nav`
field does not compile.

**The nav is a second statement of the route table, and a test is what makes
them agree.** `nav::Page::path` and `app::build_app` name the same three
paths independently; rename one and the header offers a link that 404s.
Unifying them would cost the route table its readability — it is meant to be
the shortest statement of what this server does — so instead
`app::every_header_link_reaches_the_page_it_names` walks `nav::ALL`, fetches
each page over the real router, and asserts it resolves and marks exactly
itself current. It walks the list rather than restating it, which is what
makes it cover page five: a typo'd `path`, or a handler copied from another
page and still declaring that page current, fails the moment the variant
joins `ALL`. The acceptance suite asserts the same thing from a
hand-maintained Examples table, which covers what someone remembered to add.

**Every page in the route table is in the header** — worked-in or read-only.
`/stats` is an instrument rather than a place you work, and it is in the nav
anyway, because the alternative makes it reachable only by typing a URL,
which is the failure this slice exists to close. Stated as the rule rather
than the instance so `#62`'s capacity page inherits the answer.

**Links are plain `href`, not `hx-boost`.** Boosting turns navigation into
htmx requests, and an endpoint answering 422 mid-navigation would have its
body swapped into the DOM — the failure the 422 contract depends on keeping
confined to actual form responses.

**The 422 override moved into `base.html`, which widened it.**
`T-forms-swap-one-fragment` recorded it as a one-line global in `inbox.html`
and named its cost; by the third page it was in two templates, and the next
page to grow a form would have been the one that forgot it. It is now
declared once and carried by every page, `stats.html` included, which has no
forms. That is a widening, not a move: **the guard — 422 means exactly
"validation rejection, body is the re-rendered fragment" — is now owed by
every endpoint any page can reach**, and an endpoint that cannot honour it
must not use 422.

**The rule that decides what goes in the core**, and the one this tree is
easiest to get wrong: a rule that survives changing HTTP for something else
belongs in `scheduler-core`. The window's length, the sample floor, the
fifty-percent line and the share arithmetic are all that, so they are
`scheduler_core::ratio`, not `stats/`. The core is the *enforced* pure
boundary — `cargo tree -p scheduler-core` is a gate with an acceptance test
behind it — while a pure-by-convention module sitting beside the adapters is
one import away from stopping being pure. `platform/boundary.rs` checks all of it by **walking** `src/` rather
than naming a directory — the previous check globbed `src/store/*.rs` and
would have stopped covering anything the moment that directory dissolved,
which is the failure `T-module-boundary` named against itself. It asserts that
no persistence module names `axum` or `StatusCode` (the old rule); that
nothing outside a `store.rs` or `platform/db.rs` writes production SQL (the
half the old check never had); that no top-level directory carries a
technical-role name; that **no capability names another capability's `store`
in production**, which is the front-door rule above; and a floor on each, so
an empty walk fails rather than passes. "Production" means what
`production_source` has always meant — outside a `#[cfg(test)]` module —
now read at file granularity too, so a module its parent declares
`#[cfg(test)] mod x;` is skipped. That set is *derived* from the parent's
own declaration rather than hand-listed, and a test asserts it stays small,
because a walk that quietly began exempting everything would disable three
of the four rules. It is still a substring scan over
source text — defeated by a type alias or a macro, so treat it as a lint, not
a proof. The front door's *own* enforcement is stronger than the lint:
`inbox::store::capture_is_open`, `close_capture` and the whole of
`inbox::lists` are `pub(super)` or private, so reaching past the door does
not compile.

## Three things are called "domain". They are unrelated.

The same problem as "layer", one word over, and it cost a full round trip
between the PM and the owner on 2026-08-14 before anyone noticed the two sides
were discussing different subjects.

| Term | Means | Where |
|---|---|---|
| **domain** (bare) | A **life area** — Work, Fitness, Learning, Family, Home. The thing guardrails wall off, capacity counts, and the reckoning groups by. | `T-life-areas-are-data`, `D-single-user`, `D-pool-is-default`, #47 |
| **business domain** (always both words) | The **code-packaging axis**. A capability the product provides, used to name directories and crates. | `T-package-by-business-domain`, #44 |
| **domain model** | Ordinary usage: the shapes below. Not a third concept, just the phrase. | this file |

Bare "domain" is always the life area. If you mean packaging, write both words.

## And "overlap" means guardrails, never blocks — vocabulary for removed code

The fourth word, pinned after conflating three things cost a round trip on
2026-08-19.

| Term | Means |
|---|---|
| **overlap** | Two **guardrails** claiming the same hours. Legal and expected — the life areas compete for that time (`D-life-area-owns-its-time`). |
| ~~overlapping blocks~~ | **Cannot happen.** Invariant 1. Two blocks never share an instant, and never share one with a fact or a pin. |
| **adjacency** | Two guardrails **meeting at a seam** — Work ending at 17:00 where Learning begins. Not overlap. This is what U4 was about, and a block never crosses one (`T-blocks-do-not-cross-guardrail-seams`). |

## And "window" is not used at all — vocabulary for removed code

The fourth overloaded word, caught before M2 wrote it into code rather than
after. It was already ambiguous inside M2's own acceptance criteria — a mask in
`free_intervals(window, range)`, a rolling time span in "14-day capacity view",
and a time span again in `/stats`.

| Term | Means |
|---|---|
| **guardrail** | A life area's weekly mask: the hours in which its work may be scheduled (`D-life-area-owns-its-time`). |
| **range** | A span of time — the 14 days a capacity view covers, the fortnight `/stats` counts over. |
| ~~window~~ | **Do not use.** Say which of the two you mean. |

The signature is `free_intervals(guardrail, range)`. `allowed_windows` keeps its
name for now because it is the field #6 and #11 both cite; when it is built it
should be `borrowed_guardrails` or similar.

---

### The clock — built

`Clock` is a value the composition root hands to `build_app`, not a global a
handler reaches for. It carries an offset from the real wall clock, which is
what makes `trellis serve --now <RFC3339>` an offset rather than a freeze: the
server starts believing it is that instant and time advances normally from
there. One clock per server, so two servers in one process — or two tests —
disagree without disturbing each other.

## Domain model — built

Source of truth: `crates/scheduler-core/src/task.rs`.

```rust
pub enum TaskKind {
    Pool,
    Committed { deadline: i64, commitment: Commitment, priority: Priority,
                estimated_minutes: i64 },
    Quota     { target_count: i64, target_minutes_each: i64, period: Period },
}

pub enum Commitment   { At, By }          // #94, replaces DeadlineType
pub enum DeadlineType { Hard, Soft }      // retired from the domain; see below
pub enum Priority     { P1, P2, P3, P4 }
pub enum Period       { Week, Month }
```

`deadline` is **UTC epoch milliseconds** (`T-jiff-epoch-millis`), parsed with
`jiff` at the boundary. Never a string, in the type or the column.

**`commitment` (at | by) replaced `deadline_type` (hard | soft) at `#94`**
(`D-committed-is-at-or-by`), retired from the domain outright after the owner
was asked: `TaskKind::Committed`, `TriageFields` and `TaskAttributes` all drop
it, and `Field::DeadlineType` leaves the rejection vocabulary. The *column*
stays (`T-migrations-append-only`) holding what was already typed, and every
new row writes `NULL` — the same shape `life_area_id` took at #88. A leftover
`deadline_type` in a submission is tolerated and unread, which a test pins.

> They are not the same axis. *At* and *by* say **what kind of appointment
> this is**; hard and soft said **whether it may slip**. Nothing maps one to
> the other, and nothing needs to yet.

**Committed tasks carry a `splittable` flag, unticked by default**
(`T-splitting-is-opt-in`). Unticked, the task is one block or it does not fit —
and is *told* so, rather than quietly chopped. Ticked, the scheduler may chunk
it under the chunk policy. Nothing orders the pieces of a task the owner splits
by hand: **ordering between tasks is a dependency and Trellis has no such
concept.**

Each variant carries exactly the fields that kind means, so "a pool task has no
deadline" is a fact about the type rather than a claim about one payload.
`TaskAttributes` is the nullable row-shaped projection; at most one attribute
group is ever populated.

## The committed screen — built (`#94`, `D-committed-is-at-or-by`)

**What has a date on it**, listed chronologically — the Menu's second tab.

```
scheduler_core::committed_screen::order(Vec<CommittedTask>, now_ms) -> Vec<CommittedRow>
trellis_server::committed/  store.rs · view.rs · http.rs
```

**A past deadline still shows, marked, first**, and that falls out of
chronological order with no special case: past items sort earliest. The one
failure this screen must not have is a missed deadline that silently
vanishes.

**One 66px cell does both jobs.** An *at* renders `"TUE 8:30"`, a *by*
renders `"BY THU"` — a *by* carrying a time still renders as a *by* rather
than silently becoming an *at*.

> **Two capabilities answer "where does presentation text live" differently,
> and both are defensible.** `pool` keeps its labels (`"3 things"`,
> `"Show 2 more"`) in `pool/view.rs` and its core module returns domain
> shapes; `committed` formats its date cell inside `scheduler_core::
> committed_screen`. The core version buys pure, hand-verifiable
> epoch-and-weekday math, which is worth having; the cost is a core module
> named after a screen. Recorded so the third tab's author chooses rather
> than copies whichever they read last.

> **GAP — the date cell is always UTC, and the owner's zone is data nothing
> reads.** `date_cell` resolves the deadline against `TimeZone::UTC` on a
> comment saying the product "has no per-user timezone applied to display
> yet". That is true of the code and contradicts `T-timezone-is-a-setting`,
> which settled the owner's zone as a stored row they edit from the running
> app. The row still exists and `settings::store::get_timezone` still reads
> it; what #88 deleted was the front door, and `POST /timezone` has had no
> UI path since.
>
> **Correct by default, wrong once set.** The column defaults to `UTC`, so a
> fresh database renders right. An owner whose zone is stored as anything
> else sees a weekday and an hour they never typed — and #88's own brief
> records the owner's value as `America/New_York`. The fix is the shape the
> deleted `free_time` already used: pass the zone in, as
> `free_intervals(guardrail, range)` did, rather than have the core assume
> one. Not done here because it changes what the screen displays, which is
> the specifier's call and not a tidy-up.

## The pool screen — built (`#92`, `D-menu-is-a-worklist`)

**The first consumer of context tags, and the first Menu tab.** Pool work
grouped by where it can be done. `D-no-pool-on-calendar` makes this the only
place pool work is offered, which is why `pool::store`'s one query carries
`WHERE tasks.kind = 'pool'` and nothing else has to.

```
scheduler_core::pool::group(Vec<PoolTask>) -> PoolGroups   trips + loose ends
                     TRIP_THRESHOLD                        3
                     VISIBLE_TRIP_ITEMS                    what shows unexpanded

trellis_server::pool/  store.rs  the one query · view.rs  the rendered shape
```

**A tag becomes a trip only once three things wait under it.** Fewer, and
they are strays that share a place rather than a reason to leave the house,
so they fall to loose ends still showing their tag. An untagged task always
falls to loose ends — there is no threshold for having nothing to group by.

**Nothing here writes.** No reorder, no solver, no pins. When a manual
reorder arrives it belongs to loose ends alone, never to a trip, whose rank
is derived from how many things it clears and must stay derived.

> **Two capabilities hold one invariant between them, and a test is what
> ties them.** `scheduler_core::pool::group` buckets by **plain string
> equality**, which is correct *only* because `capture::resolve_tag`
> canonicalized the spelling on the way in. Nothing in the type system says
> so, and every fixture on the pool path seeds tags through
> `capture::store::insert` with the spelling already final — convenient, and
> it skips the exact step the screen depends on.
>
> **The failure is not subtle-but-harmless.** Three items under one tag is
> exactly `TRIP_THRESHOLD`; split across two spellings they are groups of 2
> and 1, both under it, so both fall to loose ends and **the trip disappears
> from the screen entirely**. `pool::http::case_variant_spellings_of_one_tag
> _make_one_trip_not_none` is the one test on this path that goes through
> `capture::create`, and it fails exactly this way when canonicalization is
> removed. Same shape as `removing_a_band_removes_exactly_the_rows_that_band
> _displayed` before #88: two independent statements of one rule, held
> together by the only thing that can hold them.

## Context tags — built (`D-context-tags-are-the-taxonomy`, #82)

**The product's only taxonomy**, and the only one left after #88. Free text,
optional, numerous, cheap, disposable — deliberately the opposite call from a
life area's: a typo costs one badly-grouped item rather than an unschedulable
one.

```
scheduler_core::context_tag::normalize(Option<&str>) -> Option<String>

trellis_server::capture::resolve_tag(pool, raw)      the front door
trellis_server::capture::create(pool, text, source, raw_tag, at_ms)
trellis_server::capture::retag(pool, capture_id, raw_tag)
trellis_server::capture::distinct_tags(pool)         the suggestions datalist
```

**The rule splits across the boundary, and the split is the point.** What one
submitted string means on its own is decidable without a database, so
`normalize` is in the core: trim, and collapse blank-or-absent to `None`
(`T-empty-equals-absent`). Deciding *which existing spelling wins* needs a
lookup against every tag stored before, which a pure crate cannot do
(`T-core-no-tokio`), so canonicalization is the adapter's.

**A tag lives on the capture, not the task** — one fact, one row. A task
reads it through the capture it came from.

**`@HomeDepot` and `@homedepot` are one tag, stored as first typed.**
`captures.context_tag` is `COLLATE NOCASE`, which settles only how two
spellings *compare* — never which is stored. No `UNIQUE`: many captures share
one tag by design.

> **NOCASE folds ASCII only.** `@CAFÉ` and `@café` are two tags. The same
> limitation `life_areas.name` carried before #88, and the same escape hatch:
> if the rule outgrows a collation the fold moves into the core and the
> constraint becomes the backstop.

**Two write paths reach the tag, so the door is shut by the compiler.** A
fresh capture and a triage-time retag both go through `resolve_tag`; if they
canonicalized differently the same typed word would land as two tags, which
is the one thing this capability's central claim rests on.
`store::set_context_tag` is `pub(super)`, so "decided in exactly one place"
is enforced rather than described — the same move `T-inbox-owns-membership`
made for `inbox::store::close_capture`.

**A tagged capture costs one lookup and one insert**; an untagged one costs
the insert alone. The tag is bound into the `INSERT` that creates the row
rather than written by a follow-up `UPDATE`: three round trips on a path
budgeted at 50ms where two will do, and the `UPDATE` form left the row
briefly on disk *untagged* while `create` had already handed its caller the
tag it was supposed to carry.

> **`canonical_tag` is an unindexed scan.** `WHERE context_tag = ?` has no
> index behind it, and it runs on every tagged capture write. Correct and
> irrelevant at the scale `D-single-user` implies — tens of tags, captures
> that arrive by hand — and it is the thing to look at first if the 50ms
> budget is ever missed, before anything cleverer.

**What ties `normalize` to the column** is a property, not a comment.
Migration `0010`'s `CHECK` says a tag is `NULL` or non-empty-and-trimmed and
its own comment says it "mirrors" `normalize`. `context_tag_properties.rs`
states that postcondition over generated whitespace — including the
non-ASCII kinds `str::trim` strips and SQLite's `TRIM` does not — and
`capture`'s `a_tag_is_stored_as_the_spelling_it_was_first_given` asserts the
schema accepts every resolved tag, which is the half a pure crate has no
database to check.

### Schema — built

```sql
captures(id, raw_text, source, created_at_ms, left_inbox_at, context_tag)
tasks(id, capture_id, kind, deadline, deadline_type, commitment, priority,
      target_count, target_minutes_each, period, life_area_id,
      archived_at, created_at_ms)
```

> **#88 deleted code, not tables.** `life_areas`, `guardrail_bands`,
> `exceptions`, `block` and `schedule_unplaceable` all still exist, created
> by migrations `0004`, `0006`, `0007` and `0009`, and nothing reads or
> writes any of them. **Settled deliberately, not overlooked**
> (`T-migrations-append-only`): `tasks.life_area_id` carries a foreign key
> into `life_areas`, so dropping that table means rebuilding `tasks` — the
> only construction the demolition would have contained, buying nothing,
> since unused tables cost SQLite nothing idle and leaving them keeps the
> owner's data as a second safety net beside git. A revival unpauses against
> real rows rather than an empty schema.
>
> `0010_context_tags.sql` was parked with `#82` by `454580d` and returned
> when the slice was rebuilt against the demolished tree — it is `0010`
> again, adding `captures.context_tag`.

`kind`, `deadline_type`, `priority` and `period` carry `CHECK` constraints.
`deadline` does **not** — SQLite's INTEGER affinity does not reject text, so a
non-HTTP write path can still store a string there. Tracked in #33.

`archived_at` is the single archive signal; there is no `status` column
(`T-archived-at-only`). **`#97` gave it its first writer** — marking a task
done stamps it, and both Menu screens filter `archived_at IS NULL`. The rest
of the archive routes still arrive at
M8.

A capture row is never deleted. It leaves the inbox by one of **two exits**,
and both stamp the same `left_inbox_at` (`0005`, #48): triage, which also
writes a `tasks` row, and dismissal, which writes nothing else. **Which exit
it was is derivable rather than stored** — a `tasks` row references the
capture, or it does not. A second `dismissed_at` column beside the first
would permit a row claiming both exits at once, which is the shape
`T-archived-at-only` exists to forbid; one column makes the impossible state
unrepresentable instead of policing it with a `CHECK`.

**Migrations are append-only** (`T-migrations-append-only`) and CI enforces it.
SQLite has no `ALTER COLUMN`, so a type change means the table-rebuild pattern
inside a *new* migration.

---

## The scheduler — specified, not built

### Signature (C2, #3)

```
schedule(tasks, busy, guardrails, pins, facts, prior_plan, now) -> placements
```

Seven parameters. `T-fact-plan-line` renders it inline with five; that is an
abbreviation written before C2 was ratified, and this form is the contract.

`prior_plan` exists because the move penalty makes the objective depend on
previous placements. `facts` exists because past blocks constrain the next run.

### Determinism (M3, #11)

> Delete every `proposed`/`published` future block, re-run with the same facts,
> get byte-identical placements.

Plus idempotence: two consecutive runs on unchanged inputs agree byte for byte.
The plan is recomputed from scratch on every trigger — never patched
(`R-incremental-patching`).

### Passes

**Backward pass** over **hard-deadline tasks only**, placed at latest-feasible
start **minus a safety margin** (`T-backward-pass-with-margin`). The margin is
best-effort: it pulls placement earlier, falls back to latest-feasible-start
where there is no room, and **never turns a placeable task into
`deadline_unreachable`**. Seeded at 20% of the task's estimate. **Forward pass**
by least slack with priority as tiebreak — a P1 with a *soft* deadline enters
here, at the front of the tiebreak, not in the backward pass. Then splitting
under chunk policy.

**Chunk policy — specified (`T-minimum-session-per-life-area`).** Each life area
carries a **minimum session length**, the smallest piece of a split worth
scheduling. **It governs only tasks the owner marked splittable** (`T-splitting-is-opt-in`),
and committed tasks only: pool is never placed, and a quota task's
`target_minutes_each` already defines its session. A task shorter
than the minimum is placed **whole**; every chunk of a split must clear it, or
the task is `chunk_policy_unsatisfiable`. Seeded Work 90 · Learning 45 ·
Family 30 · Fitness 20 · Home 15, new life areas defaulting to 30. **No
maximum** — if six contiguous hours exist, use them. Approach is
greedy-with-repair, not a solver (`T-greedy-with-repair`) — the interface stays
clean so an optimiser can be swapped in behind it.

### Infeasibility report — closed enum (M3, #11) — half removed at #88

> The enum and its four reasons survive in `scheduler_core::schedule`. What
> is gone is everything that stored or rendered them: `trellis-server/src/
> schedule/`, its `block` and `schedule_unplaceable` writes, and the page.
> The `CHECK` naming the four words is still in migration `0009`, so the
> schema and the enum still agree — with nothing left to hold them to it.

```
no_window · capacity_exceeded · deadline_unreachable · chunk_policy_unsatisfiable
```

Every unplaceable task is named with one of these. The placed/unplaceable
partition is total.

**Which one is reported when more than one is true is part of the contract**,
and it is the order `UnplaceableReason`'s variants are declared in — a life
area with no hours at all is `no_window` before it is anything else, and a
hard deadline unreachable *even placed first* outranks losing the hours to
competition.

**The enum owns its own spelling** — `UnplaceableReason::as_str` and
`::parse`, beside the type, like `DeadlineType`, `Priority`, `Period` and
`guardrail::Weekday` before it. It shipped at `#75` with the four words
written out in `trellis-server`, which made a closed core vocabulary a fact
about one delivery mechanism: the enum said there were four reasons, the
server said what they were called, migration `0009`'s `CHECK` said it again,
and nothing tied the three. The read path re-closes too, so
`view::UnplaceableRow::reason` is a `&'static str` from `as_str` rather than
whatever text the row held — a stored column reaching a template is the
shape `T-templates-take-view-models` exists to keep out of a view model.

The `CHECK` and the enum are still two statements in two languages, and what
ties them is a test: `every_reason_the_core_can_produce_is_a_reason_the_
schema_accepts` walks `UnplaceableReason::ALL`, stores each, and reads it
back. Same shape as `removing_a_band_removes_exactly_the_rows_that_band_
displayed` over in guardrails, and for the same reason — a fifth reason
added to the enum and not to a migration fails there rather than at the
first run that produces it.

### Pins — specified, lands at M3 (`T-pins-in-constraints`)

```
pin { task_id, start, end, source }
```

A first-class Constraints-layer entity, not a column on `Block`. **One interval
— there is no recurrence rule on a pin** (`D-recurrence-is-re-commitment`).

**A pin binds a task, never a chunk** (`T-pin-binds-a-task-not-a-chunk`). For a
non-splittable task that is the whole block; for a splittable one, a piece of
exactly that interval is placed there and the remainder schedules around it.
Chunks have no identity — `R-incremental-patching` regenerates them — so a pin
that referenced one could not survive a recompute. **A pin is exempt from the
minimum session**: the minimum stops the *engine* manufacturing fragments and
has no business overruling the owner.

**Recurring commitments** — *"Learning with a friend every Tuesday 20:00"* —
compose from three things that already exist rather than a fourth: **cadence**
from quota's `period`, **placement** from a pin, and **survival** from
re-committing at the weekly review, prompted at each task's own period boundary.
A skipped review changes nothing; the commitment ends the first time the owner
attends a review and does not re-commit (U7). **So M3 builds the pin above
unchanged; the recurrence is M8's.**

One question it raises is still unanswered: **which chunk a pin binds when a
task splits** across days, given chunks have no identity to refer to — or
whether pinning suppresses splitting entirely. The reason enum above has no code
for "a stale pin is in the way". *(Pin lifetime is answered for recurring pins —
they die at the period boundary unless renewed — but not for one-off pins.)*

### Guardrails and free time — removed (#88; was built at M2 slices 1–2, `#59`, `#60`)

```
free_intervals(guardrail, range) -> disjoint, sorted intervals
                                    each a subset of (mask - busy - pins - buffers)
```

> **`buffers` is a GAP** (#80). It appears in this formula and **nowhere else in
> the project** — no decision, no acceptance criterion, no code. `busy` is M4's,
> `pins` are M3's, exceptions are built; `buffers` has never had a definition or
> an owner. Nothing supplies it, so subtracting it is a no-op today. **Do not
> reason from it** until #80 either defines it or removes it.

`free_intervals` exists, in `scheduler_core::free_time`, and today subtracts
nothing: busy, pins and buffers arrive with the calendar and the scheduler,
so what it projects now is the mask itself, resolved to instants. A
`Guardrail` is bands **plus a zone**, bundled because a band with no zone is
not yet something work could be placed into; a `Range` is civil dates,
half-open, and its length is the caller's — `/stats`'s fortnight and this
one's are the same number by coincidence, not by rule, so neither module
owns a constant the other reads.

`Interval` is instants (`start_ms`, `end_ms`), not civil times, because
intervals that are adjacent or overlapping only make sense compared as
instants — a DST transition can reorder civil clock readings. That is the
type M3's `schedule()` consumes and M4's calendar sync will produce.

**It lives in `scheduler_core::interval`, with the algebra over it**, and
not in `free_time` where it started (moved 2026-08-19, at M3 S1). The type's
own doc had always said it "was never named after a producer"; it was still
*housed* by one, and that cost twice. `schedule` had to name `free_time` in
its imports while having nothing to do with projecting a guardrail. And the
paragraph below — written at `#61`, predicting that pins and calendar busy
"need interval-minus-interval subtraction, which does not exist here yet" —
came true in the shape it did not predict: `#75` built exactly that
subtraction as two private helpers inside `schedule`, where the second
producer cannot reach it and would write it again. A value type and its
operations belong together, in a module named after the value.

```
interval::Interval { start_ms, end_ms }   duration_ms · contains · subtract
interval::subtract_all(intervals, remove) -> Vec<Interval>
interval::total_duration_ms(intervals)    -> i64
```

`subtract_all` **preserves order rather than restoring it**: each interval's
pieces come out in start order and in its own place, so a sorted disjoint
input stays sorted and disjoint and the function owes no sort. `schedule`'s
`earliest_fit` depends on that; `tests/interval_properties.rs` states it,
along with subtraction itself — an instant survives iff it was in the input
and in nothing removed.

**Exceptions narrow, and only narrow** (`#61`). A dated exception is the
owner saying a week is not normal; it removes civil dates from what a
guardrail projects and there is no code path that adds an interval, which
is what keeps it from being `D-guardrails-never-yield` breached under a
friendlier name. Overlapping exceptions subtract their union, not their
sum. Dates are stored as ISO civil text, never epoch millis — a date is not
an instant. Removing one is a hard `DELETE`: configuration is removed, not
retired, which is the opposite call from `D-kill-means-archive`'s and
deliberately so, because an exception is not a record of anything that
happened.

> **The subtrahend is date-granular, and pins and busy are not.**
> `Guardrail.excluded` is `&[DateRange]`, and `free_intervals` skips a whole
> civil date rather than subtracting an interval from one. That is the
> simplest thing that supports whole-date exceptions, which is what was
> asked for — but it means M3's pins and M4's calendar busy, which are
> partial-day, **cannot arrive as more of the same input**. They need
> interval-minus-interval subtraction, which does not exist here yet. Worth
> writing down because this slice's own reasoning recorded the opposite
> ("one subtrahend shape for M3's pins and M4's calendar busy to inherit"),
> and the next slice would otherwise inherit that sentence rather than the
> code. Building the general form now would be speculative: the fold rule
> means a civil day is not a fixed span, so date exclusion is not simply a
> special case of interval subtraction, and getting it right needs the real
> second producer to design against.

The mask it reads:

```sql
guardrail_bands(id, life_area_id, weekday, start_minutes, end_minutes)
life_areas.pool_only                     -- a column, not the absence of bands
settings(id CHECK (id = 1), timezone)    -- one row; the owner's zone
exceptions(id, life_area_id NULL, start_date, end_date, label)
                                         -- NULL life_area_id means every one
```

**A band is one weekday and a civil span**, minutes since midnight, never an
instant (`T-jiff-epoch-millis` — turning a band into instants is `#60`'s
work, and DST is its problem to have). "Mon–Fri 09:00–17:00" is *five rows
sharing one span*: storage stays one row per weekday, and only the form
spares the owner five repeats.

**Which rows are one band is `scheduler_core::guardrail::group`'s call.** The
span — `BandSpan`, the start and end without the weekday — is what makes two
rows the same band, and it is named because **two places act on it and must
agree**: the page groups rows into bands to display them, and removing a band
deletes every row sharing its span. Those were two independent statements of
one rule, one in Rust and one in SQL, with nothing tying them, so the band a
reader saw and the band Remove took were defined separately. The Rust half
now lives in the core; the SQL half cannot be tied to it by any signature, so
a property test (`removing_a_band_removes_exactly_the_rows_that_band_displayed`)
holds them together instead.

Two submissions choosing the same times merge into one band afterwards. They
are indistinguishable once stored, and the alternative is a page showing two
rows whose Remove buttons each take both.

**The owner's timezone is data, not configuration** — a `settings` row the
owner edits from the running app, not a flag beside `--now`. It is what a
zone actually is: something that changes when the owner moves. `D-single-user`
makes it one value for the whole product rather than one per guardrail, and
`settings` is a capability rather than `platform` machinery for the same
reason a life area's name is. It defaults to `UTC`, which is the only default
that cannot silently mean the wrong hour.

### Capacity — removed (#88; was built at M2 slice 4, `#62`)

`scheduler_core::capacity` compares demand against supply, both in minutes:
committed work is the sum of its tasks' `estimated_minutes`, quota work is
prorated over the horizon and **rounded up**, and `percent_used` is reported
on every row while `over_minutes` is `Some` only strictly above 100% — an
exact threshold, not a cushion nobody has a fortnight of real numbers to
pick. A task with no estimate is counted as missing, never as zero: it is
the one direction this number must not be wrong in, so the count of
estimate-less tasks rides along and the total says it is incomplete.

**A never-scheduled life area reports no capacity at all**, not
zero-available-and-over (`T-guardrail-well-formedness`). This is the second
purpose `pool_only` serves and the reason it now rides on
`life_areas::LifeAreaGuardrail` beside the already-resolved `bands`:
"opted out" and "walled but full" are different answers and `bands` alone
cannot tell them apart.

> **GAP — `end_minutes` has three different bounds.** The
> `guardrail_bands` CHECK permits `end_minutes <= 1440`; the guardrail
> form's `parse_minutes` caps hours at 23 and so cannot exceed 1439; and
> `scheduler_core::free_time::band_interval` **panics** on 1440, since 24 is
> not a civil hour ("guardrail minutes are always within a single day").
> Found by strengthening the free-time property generator, which produced a
> midnight-ending band the schema calls legal.
>
> Latent today — no write path creates one, because the form is the only
> writer. But the schema advertises it as valid, and `free_intervals`'
> own doc says its guarantees "follow from what is guaranteed upstream";
> upstream does not guarantee this. Fixing it is a decision, not a tidy-up:
> tighten the CHECK in a new migration (a table rebuild, `T-migrations-append-only`),
> give `Band` a checked constructor, or teach the core that 1440 means
> end-of-day. Needs settling before a second writer exists — an import, or
> an API.

**Each life area carries one guardrail** — the hours its work may be scheduled
in (`D-life-area-owns-its-time`). A task goes in its own life area's hours by
default and may not go outside them; borrowing another's is an explicit
per-task permission. **Guardrails may overlap in clock time**, and where they
do their life areas compete, resolved by deadline and priority. A life area
with no guardrail must be marked **pool-only** — never placed, only offered by
the menu (`T-life-areas-are-data`'s well-formedness rule, made concrete).

> **GAP — walled and pool-only are not exclusive, and pool-only cannot be
> taken back.** The model reads "a guardrail, **or** pool-only", and the
> acceptance criterion says a life area may *instead* be marked pool-only.
> Nothing enforces the *or*. Marking a life area pool-only sets the column
> and leaves any bands it already had; adding a band afterwards succeeds and
> leaves the column set. Verified against the running handlers: pool-only
> then a band gives `pool_only = 1` with one stored band, both writes
> accepted. `life_area_row.html` tests `pool_only` first, so those bands
> render as "never scheduled" with no way to see or remove them, and
> `store::set_pool_only` is only ever called with `true` — submitting the
> pool-only form unchecked is `ChooseOne` (`422`), so there is no way back.
>
> This is the shape `T-archived-at-only` and `T-capture-leaves-inbox-once`
> both legislated against, arriving from the other direction: not two fields
> spelling one state, but two states with no rule saying they exclude each
> other.
>
> **The reader side is closed; the authoring side is not.** It was predicted
> to reach `#60` and did: `life_areas::guardrails` handed out the stored
> bands regardless of the column, on a doc comment asserting that a pool-only
> life area's bands "is always empty", and `/free-time` reported **32h** for
> a life area the owner had marked never scheduled. `guardrails` now applies
> `D-life-area-owns-its-time`'s "pool-only means never placed" itself, in the
> one place every reader passes through, so no reader has to remember a
> second field. That is the front door made true to its own contract, not a
> product decision.
>
> **What is still open** is how the state is reachable at all, and it is a
> product call: marking pool-only could clear the bands, adding a band could
> clear the column, or either could be refused. Choosing one is the
> specifier's. Until then the impossible state is representable, and only
> the reader-side rule stops it being visible.

Reservation is the default and sharing is opt-in, which is what lets one
mechanism serve both jobs the settled decisions demand: **containment**, since
`D-guardrails-never-yield` means a P1 hard deadline against a full guardrail
raises a conflict rather than breaching the wall; and **reservation**, since
`T-three-task-kinds` warns that *a Fitness guardrail nothing is scheduled into
is a wall protecting an empty room*. Fitness's 06:00 is protected by nothing
having claimed it, not by a protection rule.

Guardrails are civil wall-clock, so DST gaps and folds are real cases, not edge
cases (`T-jiff-epoch-millis`).

### Capacity — specified (M2, #10; `T-capacity-two-axes`)

**Consumed from the guardrail occupied, attributed to the task's life area.**

```
Work: 5h of 8h used — 2h of that is Learning you allowed in.
Learning: 2h done this week.
```

Two numbers because there are two questions: *how much of this wall is left* is
about the clock, *how much Learning did I do* is about the work. Charging a
borrowed hour to only one of them makes the other lie — and the availability
lie is the expensive one, since it reports free time that is physically
occupied. Quota demand **counts** toward capacity (`T-three-task-kinds`); pool
consumes **nothing**, because pool is never placed
(`D-no-pool-on-calendar`).

---

## The invariants — settled (`T-invariants-one-to-five`)

`#11`'s strongest acceptance criterion asserts *"Invariants 1–5 hold under
`proptest`, ≥1000 cases"*. **Ratified 2026-08-18.** Invariants 1, 3 and 4 had
never existed anywhere in this project and were reconstructed; 2 and 5 were
recovered from U4 (#7) and #11's own text.

| # | Statement | |
|---|---|---|
| 1 | **No two blocks overlap** — with each other, or with `facts` (`in_progress`, `completed`, `missed` blocks) or pins, **other than the pin binding a block's own task** (`T-pin-binds-a-task-not-a-chunk`). | specified |
| 2 | A block lies entirely within **one** allowed window. | specified (U4, #7) |
| 3 | **Conservation under splitting**: a placed task's chunks sum **exactly** to its estimate. | specified |
| 4 | Every **placed** task with a **hard** deadline finishes at or before it; one that cannot is unplaceable, never placed late. Soft deadlines may be overrun and carry a projected finish. | specified (`T-hard-refuses-soft-slips`) |
| 5 | The placed/unplaceable partition is **total**; every unplaceable task carries a reason from the closed enum. | specified (#11) |

They cover **where** a block may be (1, 2), **how much** work survives (3),
**when** it must finish (4), and that **nothing is silently dropped** (5).

Invariant 1 names `facts` deliberately: `free_intervals` computes
`mask − busy − pins − buffers`, which does not include them, while `schedule()`
takes them as a separate input — so without saying so, the engine could place
work over the block being worked right now.

Invariant 3 is well-defined only because of `D-placed-whole-or-not-at-all`:
a task is scheduled entirely or not at all, so chunks can be required to sum
*exactly*.

**Not invariants, on purpose:** *guardrails never yield*, *determinism* and
*idempotence* are separate acceptance criteria on #11. An invariant here is a
property a proptest can falsify from a `schedule()` output alone; those three
are a different shape.

**Invariant 2's cost, corrected 2026-08-19** (`T-blocks-do-not-cross-guardrail-seams`).
It was recorded as *"a 2h task … cannot use those two contiguous free hours"*,
which is only true of a task that cannot be **split**. The ordinary case places
the work at exactly the hours it wanted, as two adjacent blocks — 16:00–17:00 in
Work's hours, 17:00–19:00 in Learning's, worked straight through. **The cost
that survives is narrow:** a task whose minimum chunk forbids the smaller piece
is unplaceable where a crossing block would have fitted.

---|---|---|
| 1 | — | **GAP** |
| 2 | A block lies entirely within **one** allowed window. | specified (U4, #7) |
| 3 | — | **GAP** |
| 4 | — | **GAP** |
| 5 | The placed/unplaceable partition is **total**; every unplaceable task carries a reason from the closed enum. | specified (#11) |

Reconstruction offered and **not yet ratified** — do not build against these:
non-overlap; conservation under splitting; hard deadlines hold.

Invariant 2 has a known cost, accepted deliberately: a 2h task allowed in both
Work and Personal, with Work ending and Personal starting at 17:00, cannot use
those two contiguous free hours.

---

## Life areas — removed (#88; was built at `#47`)

> `T-life-areas-are-data` and `T-life-area-required-at-triage` are both
> superseded. `tasks.life_area_id` stays in the schema and every row keeps
> whatever it held; nothing reads it.

Settled 2026-08-14, resolving #36. **Life areas are user-managed rows, editable
from the running app** — not a Rust enum, not a config file. There is no
canonical list to ratify; there is a seed, and then it is the user's.

```sql
life_areas(id, name, archived_at)        -- name is UNIQUE COLLATE NOCASE
seed        Work · Fitness · Learning · Family · Home
tasks.life_area_id                       -- nullable column, required at triage
```

**Two names are the same name when they match once trimmed and case-folded**,
and the column's `UNIQUE COLLATE NOCASE` is where that is enforced — not a
function in `scheduler_core`. "Work" and "work" as two indistinguishable
picker entries is the failure the rule exists to prevent, and a constraint
the database checks cannot be bypassed by a write path that forgot to call
something. Two consequences: a database swap has to carry the collation
across (`T-sqlite-sqlx`'s "mechanical" Postgres move needs `CITEXT` or a
functional unique index), and if the rule ever outgrows a collation — Unicode
folding — it moves into the core and the constraint becomes the backstop.

`tasks.life_area_id` is nullable for the reason `T-quota-targets-required`
already established: SQLite cannot add a `NOT NULL` column without a default
to a table that may hold rows, so the requirement lives at the triage
boundary instead. `None` therefore means "written before this migration", not
"has no life area".

The seed keeps every life area cited in a settled decision's reasoning — Fitness
(`T-three-task-kinds`), Learning (`D-kill-means-archive`), Family
(`D-single-user`), Work (`D-pool-is-default`) — and adds Home for errand and
admin traffic, which neither candidate set housed.

**One concept, not two.** The capture-tagging vocabulary and the scheduler's
life area are the same list. Every consumer of one is a scheduler concern —
guardrails, capacity, the reckoning, menu diversity (#41) — so two lists would
make the mapping between them the real list.

**Why not a closed enum**, given `kind`, `period`, `deadline_type` and `priority`
all are: those are fields the scheduler **branches on**, a `match` with a
different body per variant. A life area is a **lookup key, not a discriminant** —
every one is handled identically. A set you `match` on must be closed; a set you
index by need not be.

**The invariant that replaces the enum:** a life area is well-formed only once
it has a guardrail, or is explicitly marked pool-only. Enforced at M2, when
guardrails exist. A non-exhaustive `match` never expressed this — it reports
missing arms, not a missing wall.

`T-complexity-8`'s threshold of 8 is unaffected: it is derived from *"the largest
enums (`Domain`, `BlockState`) have 5 variants"*, and `BlockState` still has
exactly five, so the derivation stands on that one alone.

---

## Capture and triage — built

```
GET  /                        the inbox: untriaged captures, newest first
GET  /static/htmx.min.js      vendored; no Node build step
POST /captures                raw text in. JSON -> 201 JSON; form-encoded -> 201
                              HTML fragment. One route, content-negotiated.
                              50ms budget, asserted by capture_endpoint.feature.
POST /captures/{id}/triage    capture -> task
POST /captures/{id}/dismiss   the inbox's "no" (#48). The capture leaves the
                              inbox, no task is created, the row stays. No
                              confirmation (D-three-strike), no un-dismiss and
                              no archive view (D-kill-means-archive).
GET  /stats                   the committed share of the last fourteen days
                              (R2, #45). Rules in `scheduler_core::ratio`.
GET  /capacity                hours you have against hours you promised, per life
                              area, over the same fourteen days (#62, closes M2).
                              Computes only; the arithmetic is
                              `scheduler_core::capacity`.
GET  /free-time               what each life area's guardrail projects to over
                              the next fourteen civil days (#60). Computes only;
                              stores nothing. Carries the exceptions controls,
                              being the only page whose numbers they change.
POST /exceptions              mark a civil date range away, globally or for one
                              life area (#61). 422 on a backwards range or a
                              name that resolves to nothing.
POST /exceptions/{id}/remove  a hard DELETE, not an archive — configuration is
                              removed, not retired.
GET  /life-areas              manage the set: list, add, archive (#47)
POST /life-areas              add one. Duplicate or blank -> 422 + the list
                              fragment carrying the message.
POST /life-areas/{id}/archive retire one. Never deleted (D-kill-means-archive).
```

The three `GET` pages — `/`, `/life-areas`, `/stats` — are exactly the three
the header links, and `nav::ALL` decides their order. Every other route is a
fragment or an asset, and none carries a header.

Triage requires a life area for every kind. The name is validated in two
steps that meet at `scheduler_core::task::WellFormedTriage`: the core decides
kind first and then that *some* life area was named, and the adapter resolves
that name against the `life_areas` table — a question needing the database,
so `unknown_life_area` is the one rejection the core does not produce.

The picker carries **no preselection** (`D-manual-triage-until-llm`), which is
what makes the requirement honest: the blank option submits empty and lands
on the `missing_field` path that already existed — no new rejection variant.

Rejections are `422`. An unrecognised `kind` reports
`{"unknown_kind": <submitted>}` (`T-unknown-kind-rejected`); a missing or empty
required field reports `{"missing_field": <name>}` — absent and empty are the
same submitter mistake and report identically (`T-empty-equals-absent`). A
capture that has already left the inbox, by *either* exit, reports
`{"capture_not_open": …}` — triage and dismissal both refuse it, and neither
says which exit it took, because nothing downstream needs to tell them apart.

### Classification — specified, **M9 only** (`T-classifier-covers-domain`, `D-manual-triage-until-llm`)

**Nothing classifies a capture before M9. Triage is fully manual until then**,
and the life-area picker carries no preselection — a default that is merely
first-by-id is a silent wrong answer the user never chose
(`D-manual-triage-until-llm`, 2026-08-17). The keyword implementation
`T-classifier-covers-domain` scheduled for M1 is cancelled, not deferred.

What lands at M9: one trait, output covering `kind`, `deadline`, `priority`,
`domain` and `title`, each with per-field confidence. Invoked by a **background
worker between capture and triage** — not inside `POST /captures`, which has a
50ms budget no LLM round trip fits, and not synchronously at triage, which
would put the wait in front of the user. Failure or timeout falls back to
**empty fields and manual triage**, which is the shipped M1 product rather than
a second classifier nobody validated.

`captures` therefore carries no classification columns and gains none until M9 —
a schema element with no observable behaviour has nothing to specify against.

---

## Gaps index

Everything above marked **GAP**, in the order it blocks work.

**A gap in deleted code is struck through and marked moot, not removed.** It
stops blocking anything the moment the code goes, but it was a real
disagreement between a schema, a form and a rule — and two of the three
schemas are still there. A revival inherits the gap along with the tables.

| Gap | Blocks | Tracked |
|---|---|---|
| ~~`end_minutes` bounds disagree: schema allows 1440, form caps at 1439, core panics on 1440~~ | ~~nothing today; a second writer~~ | **moot** — #88 deleted all three. `guardrail_bands` keeps its `CHECK`; no code reads it |
| ~~Walled and pool-only are not exclusive; pool-only cannot be un-marked~~ | ~~#60~~ | **moot** — #88 deleted life areas. The impossible state is still representable in the surviving table, and unreachable |
| ~~Invariants 1, 3, 4 undefined~~ | ~~M3 cannot be specified~~ | **closed** — `T-invariants-one-to-five`, 2026-08-18 |
| ~~Which five domains, and one concept or two~~ | ~~M1 S4, M9~~ | **closed** — `T-life-areas-are-data`, #47 |
| ~~Per-life-area capacity vs `allowed_windows`~~ | ~~M2~~ | **closed** — `T-capacity-two-axes` + `D-life-area-owns-its-time`, #6 |
| `Block::missed` unreachable under silence-means-done | M6 | #4 — the `block` table survives #88; nothing writes it |
| `ScheduleTask` needs `life_area_id` and `deadline_type`; triage writes `NULL` for both | M3 revival | #88 · #94 — needs a decision, not a re-attachment |
| The committed screen's date cell is always UTC; `T-timezone-is-a-setting` says the zone is stored data | correct on a fresh database, wrong once the zone is set | #94 · #85 (which brings the control back) |
| U3 — backward-pass input | M3 | #7 · ~~U2~~ `T-hard-refuses-soft-slips` · ~~U4~~ `T-blocks-do-not-cross-guardrail-seams` |
| ~~Crate layout ratification~~ | ~~nothing; cost grows~~ | **closed** — `T-package-by-business-domain`, #44 |
