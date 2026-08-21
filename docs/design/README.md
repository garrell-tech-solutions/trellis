# Design

**`Trellis.dc.html` is the design source.** Four screens on the GTS design
system, authored by the owner in Claude Design and checked in here so the
pipeline can read it from a worktree.

| | |
|---|---|
| **Project** | `620d68b2-3394-4dd0-8553-1441f106e895` — claude.ai/design |
| **Design system** | `_ds/gts-design-system-2e718b41-01f6-4aca-a2a4-28e4f49a0c4c/` |
| **Read it with** | the `claude_design` MCP — `DesignSync` `get_file`, or open the project |

**The four screens are `Capture`, `Pool`, `Quota`, `Committed`**, guarded in the
source by `isCapture` / `isPool` / `isQuota` / `isCommitted`. The bottom tab bar
is styled by `header nav a` in `crates/trellis-server/static/trellis.css`.

## How to read it

It is a Claude Design canvas: ordinary HTML with `<x-dc>` components,
`{{ binding }}` placeholders and `<sc-if>` / `<sc-for>` guards. **The
`hint-placeholder-*` attributes are sample data, not specification** — they
exist so the canvas renders with something in it.

**What it does not carry**: it is a *layout*, not a behaviour spec. Where the
canvas and `docs/decisions.md` disagree about behaviour, the decisions log
wins and the disagreement is worth raising rather than resolving quietly.

## Its dependencies are not checked in

`_ds_bundle.js`, `styles.css` and `support.js` live in the Claude Design
project. **The canvas will not render from this repo** — it is here to be
*read*. `crates/trellis-server/static/trellis.css` is the implemented subset,
and `crates/trellis-server/static/fonts/` carries the typeface.

## History

PR #87 implemented the tokens, the typeface, the app shell and the **Capture**
screen, and said at the time: *"The canvas source lives in the Claude Design
project, not in this repo. Worth checking into `docs/design/` before the Menu
slice needs it."* That moment arrived on 2026-08-21 and this is that check-in.
