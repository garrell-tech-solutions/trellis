# QA Procedure: The owner's timezone is one setting for the whole product

Covers: `features/timezone_setting.feature`

## Interface used

The life areas page at `GET /life-areas`, `curl` against whatever endpoint its
timezone control submits to, and read-only `sqlite3` inspection. No project
library, module, or test helper is used.

## What this setting is for, and what it does not do yet

**It does nothing observable beyond being stored and shown, and that is
correct for this slice.** A civil wall-clock guardrail is not an instant until
you know where the owner is — but nothing here converts a band to an instant.
`#60` is the first reader. **A build in which changing the zone changes what a
guardrail band displays has done `#60`'s work early and probably wrongly.**

It is one value for the whole product (`D-single-user`): not one per life
area, not one per guardrail, not one per page.

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Click **Life areas**. Confirm the page states the owner's timezone, and
   that it reads **UTC**.
3. Change it to `Europe/London`. Confirm the page now says so.
4. Change it to `Mars/Olympus`. Confirm it is **refused** and the page still
   says `Europe/London`.
5. Restart the server and reload. Confirm it still says `Europe/London`.

### Expected Observable Outcomes
- All five steps hold literally.
- **Step 2's default is UTC, and that is a decision.** Reading the host's zone
  would mean a guardrail silently meaning a different hour than the one on
  screen, on a machine the owner may not have configured — the class of
  silent-wrong-default `D-manual-triage-until-llm` rejected for the life-area
  picker. UTC is the only default that is honest about not knowing.
- Step 5 is what makes it the owner's data rather than deployment
  configuration. **If the zone can only be set by a flag at startup, this
  procedure cannot pass**, and the decision was reversed.

## Setup — repeat before each procedure below

1. Start the trellis server against a fresh database file.
2. Confirm the server is reachable.

## Procedure — the default is UTC

1. `GET /life-areas` and read the timezone the page reports.
2. Query the database for the stored value.

### Expected Observable Outcomes
- The page reports **UTC**, and the stored value agrees with the page. A page
  that displays a default the database does not hold will disagree with itself
  the first time anything else reads it.

## Procedure — the zone can be changed, and it sticks

1. Set the timezone to `Europe/London`.
2. `GET /life-areas`.
3. Stop the server. Restart against the **same** database. `GET /life-areas`.
4. Set it to `America/Denver` and re-read.

### Expected Observable Outcomes
- Each change is accepted and reported back by the page.
- The value survives the restart — it is a stored row, not process state.
- Step 4 confirms it can change more than once. **There is exactly one stored
  zone afterwards, not a history and not a second row**; the owner has one
  location at a time.

## Procedure — a name that is not a timezone is refused

1. Set the timezone to `Mars/Olympus`.
2. Observe the status and body.
3. Set it to `Europe/Londonn`.
4. `GET /life-areas` and query the database.

### Expected Observable Outcomes
- Both are rejected with `422` carrying the re-rendered fragment, and the
  rejection **echoes the value submitted** — the shape `unknown_life_area`
  established for a value that does not resolve.
- Step 3 matters more than step 1: `Europe/Londonn` is a plausible typo of a
  real zone, and a check that only rejects obvious nonsense will accept it and
  store a zone that resolves to nothing the day `#60` tries to use it.
- The stored zone is unchanged, and no second row was written.

## Procedure — hostile text stays escaped

1. Set the timezone to `<script>alert('boom')</script>`.
2. Read the **raw HTML source** of the response.

### Expected Observable Outcomes
- Rejected, no unescaped `<script>` tag in the raw response, and the word
  `boom` still present — escaped, not stripped, since the rejection echoes
  what was submitted.

## Procedure — nothing else changed

1. Run the `life_areas`, `life_area_triage` and `app_shell` QA suites.

### Expected Observable Outcomes
- All pass unchanged. The timezone control shares the life areas page with the
  life-area list and the guardrail controls; adding it must not disturb the
  fragment either of those swaps, and `app_shell` still expects exactly one
  header and three nav links.

## Independent of Implementation

This procedure depends only on what the life areas page reports, what its
timezone control accepts, and what survives a restart. It does not depend on
which route the control submits to, whether the zone lives in its own table or
a column, or which library validates the name.
