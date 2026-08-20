# QA Procedure: A capture carries a context tag

Covers: `features/context_tags.feature`

## Interface used

The inbox at `GET /` and its quick-add box, the triage controls on each
capture row, and read-only `sqlite3` inspection. No project library, module,
or test helper is used. **Read the quick-add endpoint and its fields out of
the page's own markup.**

## What a context tag is, and why it is free text

**It is the product's only taxonomy now** (`D-context-tags-are-the-taxonomy`)
— numerous, cheap, disposable. Free text is deliberate: a typo costs one
badly-grouped item, not an unschedulable one, and autocomplete on prior
values is enough structure.

**If any procedure tempts you to check that a tag is validated against a
managed set, stop.** There is no set. That is the decision, not an oversight.

**The tag's whole value is that the second errand at a place lands in the
same bucket as the first.** Every procedure below is ultimately about that.

## What autocomplete can and cannot be checked here

The control offers **the whole set of prior tags**, and the browser narrows
it as the owner types. So these procedures assert **what the server sends** —
every tag used before, exactly once — and **cannot** assert prefix filtering,
which is the browser's behaviour. **There is no browser automation in this
stack; say that prefix narrowing is unverified rather than implying it was
checked.**

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Quick-add **"buy screws"** with the context field set to `@homedepot`.
   Confirm it saves and the tag shows on the row.
3. Quick-add **"return the drill"**. Start typing `@home` — **confirm
   `@homedepot` is offered** and can be accepted. This is the step that
   cannot be scripted; it is the reason the walkthrough exists.
4. Triage both as **pool**. **Confirm neither asks for a life area**, and that
   both are accepted without one.
5. Quick-add **"pick up milk"**, tag `@supermarket`, triage as pool.
6. Restart and reload. Confirm all three keep their tags.

### Expected Observable Outcomes
- All six steps hold literally, per `D-visible-slices`.
- **Step 3 is the point of the slice.** A tag nobody can retype cheaply is a
  tag that fragments, and a fragmented tag is worse than none — it looks like
  a grouping and is not.
- **Step 4 is the change most likely to be specified at the boundary and
  forgotten in the form**, which is exactly what
  `T-required-fields-are-specified-per-transport` was written about after
  #73 shipped a committed form that could never succeed.

## Setup — repeat before each procedure below

1. Start the server against a fresh database file.
2. Confirm the server is reachable and the inbox is empty.

## Procedure — a tag is kept, and is optional

1. Quick-add `buy screws` tagged `@homedepot`.
2. Quick-add `return the drill` with the context field left empty.
3. Quick-add `pick up milk` with the context field omitted from the
   submission entirely.
4. Read the inbox and query the capture rows.

### Expected Observable Outcomes
- `buy screws` shows `@homedepot`; the other two show no tag and are
  **accepted, not rejected**.
- **Empty and absent behave identically** (`T-empty-equals-absent`). Steps 2
  and 3 must produce the same stored state — not one empty string and one
  NULL, and certainly not one acceptance and one rejection.
- A tag of only whitespace is stored as no tag at all, and surrounding
  whitespace is trimmed off one that has content.

## Procedure — the suggestions offer every prior tag, once

1. Quick-add three captures tagged `@homedepot`, `@supermarket` and
   `@homedepot` again.
2. `GET /` and read the tag control's suggestion list out of the markup.

### Expected Observable Outcomes
- Exactly **two** suggestions — `@homedepot` and `@supermarket`. A tag used
  twice is offered once.
- A capture with no tag contributes nothing to the list.
- **Prefix narrowing is not checked here** and must be reported as unverified.
  What is checked is that the list the browser narrows contains the right
  things.

## Procedure — case does not split a tag

1. Quick-add `buy screws` tagged `@HomeDepot`.
2. Quick-add `return the drill` tagged `@homedepot`.
3. Read the inbox and the suggestion list.

### Expected Observable Outcomes
- **One suggestion, not two**, and it reads `@HomeDepot` — the spelling first
  used.
- **Both rows display `@HomeDepot`**, including the one typed in lower case.
  The tag is one thing; the display is the spelling it was introduced with.
- This is `T-collation-enforces-name-identity`'s argument applied to something
  typed rather than picked, where the slip is likelier. **The failure it
  prevents is a Menu showing two Home Depot lists and sending the owner
  twice** — and that failure is invisible until the Menu exists, which is why
  it is settled now rather than then.
- Confirm where identity is enforced by trying it **through the database**
  as well as the page, if the check is a column constraint rather than
  application code.

## Procedure — a tag can be given at triage

1. Quick-add `buy screws` with no tag.
2. Triage it as pool, supplying `@homedepot` on the triage control.
3. Read the task list.

### Expected Observable Outcomes
- The task shows `@homedepot`.
- **The tag is stored once**, on the capture, and the task reads it through
  the capture it came from. Query both rows and confirm the tag is not
  duplicated onto the task: two copies of one fact is two places to edit and
  two chances to disagree, which is `T-archived-at-only`'s shape.
- Tagging at capture is the fast path and tagging at triage is the
  second chance; both write the same field.

## Procedure — triage no longer requires a life area

1. Quick-add three captures.
2. Triage the first as **pool**, the second as **committed** (with its
   deadline, deadline type, priority and estimate), the third as **quota**
   (with its targets and period) — **none supplying a life area**.
3. Do it once over the JSON API and once through the page's own controls.
4. Read the task list and query the rows.

### Expected Observable Outcomes
- **All three succeed, on both transports.** `T-life-area-required-at-triage`
  is superseded.
- The tasks are listed with **no life area**, not with a defaulted one. A
  silent default is the failure `D-manual-triage-until-llm` refused for the
  picker.
- **Step 3 is not optional.** #73 shipped a page whose committed form could
  never succeed while every acceptance feature stayed green, because the suite
  triages over JSON. Checking one transport here proves nothing about the
  other.
- The picker itself is unchanged: it still offers every active life area and
  still preselects nothing. Supplying a life area still works.

## Procedure — the tags survive a restart

1. Create tagged captures, triage one.
2. Stop the server. Restart against the **same** database.
3. Read the inbox, the task list and the suggestion list.

### Expected Observable Outcomes
- Every tag is still there, on captures and on tasks.
- The suggestion list is rebuilt from stored values, not from anything the
  process remembered.

## Procedure — hostile text in a tag stays escaped

1. Quick-add a capture tagged `<script>alert('boom')</script>`.
2. Read the **raw HTML source** of the inbox.
3. Triage it and read the task list, and read the suggestion list.

### Expected Observable Outcomes
- No unescaped `<script>` tag on **any** of the three surfaces; `boom` still
  present, escaped rather than stripped.
- **The suggestion list is a new render surface and an easy one to miss** — it
  renders tag text into markup attributes rather than element bodies, which
  escapes differently. Check it specifically.

## Procedure — nothing else changed

1. Run every other QA suite.
2. Run `life_area_triage` **expecting it to have changed**.

### Expected Observable Outcomes
- **Expect fixture drift wherever a script supplies a life area at triage.**
  It is still accepted, so most should pass untouched — but any script that
  asserted a *rejection* without one is now asserting the opposite of the
  rule. **Reproduce each failure and confirm it is drift before fixing it.**
- **`life_area_triage` legitimately changed**: its two requirement scenarios
  were **inverted, not deleted**. Deleting them would have left "triage
  succeeds with no life area" asserted nowhere, on exactly the path
  `T-required-fields-are-specified-per-transport` exists for.
- The life-areas, guardrail, free-time, capacity, exception and schedule
  suites are **untouched by design**. Those modules stay in the tree;
  `D-context-tags-are-the-taxonomy` defers their removal until after
  dogfooding, and this slice must not start it.

## Independent of Implementation

This procedure depends only on what the inbox renders, what its controls
accept, and what survives a restart. It does not depend on which row stores
the tag, whether identity is a column collation or application code, or how
the suggestion list is delivered.
