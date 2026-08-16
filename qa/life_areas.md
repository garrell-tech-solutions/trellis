# QA Procedure: Life areas are user-managed rows, added and retired from the running app

Covers: `features/life_areas.feature`

## Interface used

The page at `GET /life-areas`, the page at `GET /`, `curl` against whatever
endpoints those pages' own controls submit to, and read-only `sqlite3`
inspection of persisted state. No project library, module, or test helper is
used.

Management lives on its own route rather than on `/`. `#30` made `/` the inbox
and `#33` added the task list to it; `#45` settled that a further concern
belongs on its own route, and this is the same call. The **picker** is on `/`,
because that is where triage happens.

**What this procedure can and cannot honestly claim**, as established for
`inbox_view` (#30): `curl` verifies every server-side fact that enables the
page's behaviour, but not that a real browser's DOM updates without a visible
reload. The by-hand walkthrough covers that once, literally; the procedures
after it are `curl`-only.

## By-hand walkthrough — do this once, in a real browser

This is the slice's demo and covers both halves, so
`qa/life_area_triage.md` does not repeat it.

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Open `http://localhost:8080/life-areas`. Confirm five life areas are listed:
   **Work, Fitness, Learning, Family, Home**.
3. Add **Side project**. Confirm it appears immediately, without the page
   visibly reloading and without restarting the server.
4. Go to `http://localhost:8080` and capture `sketch the landing page`.
5. Triage it, choosing **Side project**. Confirm the task list shows it tagged
   with that life area.
6. Restart the server and reload both pages. **Side project** is still listed,
   and the task is still tagged with it.

### Expected Observable Outcomes
- All six steps hold literally, per `D-visible-slices`.
- **Step 3 is the one worth the trip.** `T-life-areas-are-data` settled that
  adding a life area is never a development task — no rebuild, no config file,
  no migration. This is where that stops being an assertion in a decision log.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.
3. `GET /life-areas` and confirm the five seeded areas are listed.

## Procedure — a fresh database is seeded with five life areas

1. On a fresh database, `GET /life-areas`.
2. Query the life areas table.

### Expected Observable Outcomes
- Exactly five are listed: **Work, Fitness, Learning, Family, Home** — no more,
  no fewer. The count matters as much as the membership; a sixth would mean the
  seed drifted from the four cited in settled decisions plus Home.
- The same five are present as rows, so the seed is data and not a hard-coded
  render.

## Procedure — adding a life area, with no restart

1. Submit the page's add control with the name `Side project`.
2. Observe the response status.
3. `GET /life-areas` **without restarting the server**.
4. `GET /` and read the triage controls.

### Expected Observable Outcomes
- The add response is not a redirect (not a `3xx`) — the server-side half of
  "appears immediately"; see the walkthrough for the browser-side half.
- `Side project` is listed sixth, after the five seeded areas. New areas append;
  the seed order is the order the decisions cite, not alphabetical.
- The triage picker on `/` offers `Side project` too, on the same server
  process. This is the acceptance criterion — no restart, no rebuild, no
  migration.

## Procedure — a life area cannot be added twice

1. Submit the add control with `Work`.
2. Observe the status and body.
3. Repeat with `work`, then with `WORK`.
4. `GET /life-areas` and count.
5. Query the life areas table and count its rows.

### Expected Observable Outcomes
- All three are rejected, and each rejection names the submitted text as
  already being a life area.
- Still exactly five listed, and still exactly five rows. Case does not make a
  new life area — two entries a reader cannot tell apart in a picker would
  scatter tasks between them and split one category in M8's reckoning.
- The rejection re-renders the fragment carrying the error, per
  `T-forms-swap-one-fragment`; it is not a redirect and not a blank page.

## Procedure — names are trimmed, and a blank name is refused

1. Submit the add control with `␣␣Side project␣␣` (leading and trailing
   spaces).
2. `GET /life-areas`.
3. Query the stored name for that row.
4. Submit the add control with `␣␣␣` (whitespace only).
5. `GET /life-areas` and count.

### Expected Observable Outcomes
- After step 2 the list shows `Side project` with no leading or trailing
  space, and step 3 shows the stored name is trimmed — not merely displayed
  trimmed.
- Step 4 is rejected and names the name field. Step 5 still shows six.
- Order matters here: trim happens **before** the blank check. An
  implementation that validated first and trimmed second would accept `␣␣␣`
  and create a life area that renders as nothing.

## Procedure — archiving takes a life area out of circulation without erasing it

1. Capture `sketch the landing page` and triage it as pool into `Learning`.
2. Archive `Learning` through the page's own control.
3. `GET /life-areas`.
4. `GET /` and read both the triage picker and the task list.
5. Query the life areas table.

### Expected Observable Outcomes
- `Learning` no longer appears in the list at step 3, nor in the triage picker
  at step 4 — it cannot be chosen for anything new.
- The task **still shows, still tagged `Learning`**. Archiving retires a
  choice; it does not orphan or rewrite the tasks already in it.
- The row is still present at step 5, with an archive timestamp set. Nothing is
  deleted — `D-kill-means-archive` keeps the row, and `T-archived-at-only`
  makes that one timestamp the single archive signal rather than a second
  status field.
- **Known limitation, deliberate:** there is no un-archive in this slice, so an
  archived life area cannot be brought back from the page. Flagged rather than
  discovered.

## Procedure — hostile text in a life area name stays escaped

1. Add a life area named `<script>alert('boom')</script>`.
2. `GET /life-areas` and read the **raw HTML source**.
3. `GET /` and read the raw source of the triage picker.

### Expected Observable Outcomes
- Neither response contains an unescaped `<script>` tag.
- The word `boom` is present in both — the content survived, escaped, rather
  than being stripped.
- `#30` proved this for the inbox and `#33` for the task list. A life area name
  renders on two further surfaces, each with its own template, and the same
  defect could exist in either independently.

## Procedure — life areas survive a restart

1. Add `Side project`. Archive `Learning`.
2. Stop the server. Restart it against the **same** database.
3. `GET /life-areas`.

### Expected Observable Outcomes
- `Side project` is still listed and `Learning` is still absent. Both the
  addition and the archival are durable, not process state.

## Independent of Implementation

This procedure depends only on what `/life-areas` and `/` render, how their
controls respond, and the durable state left behind. It does not depend on
which routes the controls submit to, how the seed is applied, whether the
uniqueness check runs in SQL or in Rust, or which template renders the list.
