# Mutation testing — making it affordable

**Date:** 2026-08-20 · **Owner:** architect, at the owner's direction · **Status:** implemented

Mutation testing was slow enough to be shaping how the swarm worked. This
records what was actually costing the time, measured rather than assumed,
and what changed.

## What it cost, before

A full pass over the workspace was 1296 mutants. Per-mutant cost is
dominated by the test run, not the rebuild:

| crate | mutants | `cargo test` | rebuild (incremental) |
|---|---|---|---|
| `scheduler-core` | 345 | 1.8 s | 3.0 s |
| `trellis-server` | 301 | 67.7 s | 2.4 s |
| `acceptance-tests` | 650 | **251.8 s** | 9.2 s |

**Half the mutants were in the acceptance harness**, and because
`cargo test -p acceptance-tests` runs 23 generated entrypoints, that one
crate was roughly 90% of the total cost.

## Four findings

**1. Most of the budget went on second-order work.** The harness mutants are
things like `replace then_html_body_contains -> Result<(), String> with
Ok(())` and `replace dispatch_band_removed -> ... with Ok(())`. Killing one
means writing a unit test proving that an *assertion helper* fails when it
should — a test of a test of the product. The property being bought, "do
these steps assert anything", is what `gherkin-mutator --level soft` answers
directly by mutating example values in the `.feature` files. It was already
running every slice.

**2. The test runner was the bottleneck, and `cargo test` is the slow one.**
It runs test binaries one after another. nextest runs them concurrently and
stops at the first failure — which, for a *caught* mutant, is immediately.

| | `cargo test` | nextest | |
|---|---|---|---|
| `acceptance-tests` | 251.8 s | 46.9 s | 5.4× |
| `trellis-server` | 67.7 s | 3.3 s | 20.7× |
| `scheduler-core` | 1.8 s | 0.6 s | 2.9× |

**3. Hang detection was derived from the baseline, so it inherited the
slowness.** cargo-mutants sets the per-mutant timeout to 5× the baseline
test run, floored at 20s. With a 53s `cargo test` baseline that is **268
seconds** before a hung mutant is noticed. A ~3s nextest baseline puts it at
the 20s floor — 13× faster, with no policy change.

**4. Timeouts were counted as kills.** `killed = caught + timeout` meant a
mutant that merely made a slow suite run long inflated the score. A timeout
is a fact about the machine, not the mutant: during this work the *same*
mutant timed out under load and was caught on an idle run.

## What changed

- `.cargo/mutants.toml` — `exclude_globs` for `crates/acceptance-tests`.
  **1296 → 646 mutants.**
- `.config/nextest.toml` — a `mutants` profile with `fail-fast` and
  per-test `terminate-after`, so a hanging *test* dies well before the
  suite timeout. The default profile is untouched, so ordinary verification
  behaves exactly as before.
- `scripts/analyzers/mutation.sh` — runs nextest with that profile;
  timeouts are reported as their own category and fail the gate rather than
  counting as kills.
- `stack.prompt`, `hardener.prompt` — record the scope, the tool, the cache
  rule, and why timeouts are not kills.

## The trap that cost an hour

cargo-mutants tests a **patched copy** of the tree. Exporting
`CARGO_TARGET_DIR` — which the constitution's "prefer project-local cache
paths" actively encourages — makes two different source trees share one
build cache. The last mutant's artifacts stay behind, and a later
`cargo test` links a mutated library while `git status` is clean. The suite
goes red for a change that does not exist.

The analyzer now sets `target-mutants/` itself. It is deliberately
*persistent*, because the alternative is worse: with no shared cache every
parallel build directory is cold, which is what makes `-j > 1` unusable for
`trellis-server` (every mutant timed out).

## What was left alone, and why

**The parallelism in `stack.prompt` — `scheduler-core: 8`, everything else
`1`.** This looked like an unexamined default and is not. `-j > 1` gives
each mutant its own build directory, so each build is cold. That is cheap
for a small pure crate and ruinous for `trellis-server`'s axum/sqlx/tokio
tree. The other crates get their speed from the warm cache instead, which
requires `-j 1`. Both readings were tested; the recorded numbers are right.

**Linker and debuginfo.** No `.cargo/config.toml` and no `[profile]` tuning
exist, so builds use GNU `ld` with full debuginfo. `mold`/`lld` plus
`debug = 0` would cut link time further. Unmeasured, and secondary: rebuild
is 3–12 s against test runs that were 47–252 s. Worth revisiting if
`scheduler-core`, where the build is the larger half, becomes the tall pole.

## Effect

Measured on `scheduler-core/src/interval.rs`, 28 mutants, full
(non-differential) run through the new analyzer: **97.7 s, 100% kill rate.**
Differential re-run of the same file: **0.26 s**. Extrapolating the
per-mutant cost across the remaining 646 mutants puts a full workspace pass
in the tens of minutes rather than the tens of hours.

## What this gives up

A real bug in step-handler logic — a parser, a comparison, an extraction —
is now caught by the acceptance suite and by Gherkin mutation rather than by
per-line mutation. That is the trade, made deliberately, and it is
reversible: deleting four lines from `.cargo/mutants.toml` restores the old
scope.
