# Trellis

A single-user, single-tenant scheduling assistant. See `docs/decisions.md`
for the settled product and technical decisions (and what was rejected, and
why) before proposing changes to either.

## Getting started

Prerequisites:

- Rust via `rustup` — the toolchain is pinned in `rust-toolchain.toml`
  (stable) and installs itself automatically on first `cargo` invocation.
- [Babashka](https://babashka.org/) (`bb`) — used by the acceptance test
  generator.
- The [Acceptance Pipeline Specification](https://github.com/garrelj1) tools
  (`gherkin-parser`, `gherkin-ir-dry-checker`, `gherkin-mutator`) on your
  `PATH`.

Build and unit-test everything:

```sh
cargo build
cargo test
```

## Project layout

- `crates/scheduler-core` — the pure scheduling algorithm. No I/O, no
  `tokio` dependency (enforced in CI) — this is what makes it property-testable.
- `crates/trellis-server` — the `axum`/`sqlx` HTTP server, binary name `trellis`.
- `crates/acceptance-tests` — generated from `features/*.feature`; do not
  hand-edit anything under `crates/acceptance-tests/tests/`, it's
  regenerated on every acceptance run.
- `features/*.feature` — Gherkin acceptance specs; the source of truth for
  what `crates/acceptance-tests` contains.
- `qa/*.md` + `scripts/qa/*.sh` — one QA procedure per script, each driving
  the project through a real user-facing interface (HTTP or CLI) rather than
  mocking internals.
- `scripts/analyzers/` — complexity, coverage, CRAP, dry, lint, and mutation
  checks used as CI/quality gates.
- `docs/decisions.md` — append-only decision log: settled product and
  technical calls, plus rejected alternatives and why. Read before
  re-proposing something under "Rejected".

## Running the acceptance suite

Regenerates test entry points from `features/*.feature` and runs them:

```sh
./scripts/acceptance/run.sh
```

## Running QA scripts

Runs every procedure in `qa/*.md` via its corresponding script in
`scripts/qa/`:

```sh
./scripts/qa/run.sh
```

## Running the analyzers

Each analyzer is standalone, e.g.:

```sh
./scripts/analyzers/lint.sh
./scripts/analyzers/complexity.sh
./scripts/analyzers/coverage.sh
```

## Swarm tooling

This repo also carries `swarmforge/` config and a `./swarm` entrypoint for
multi-agent development sessions. Unrelated to building or testing the
product itself — see `swarmforge/swarmforge.conf` if you're picking that up.
