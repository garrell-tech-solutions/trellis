# QA Procedure: A capture carries a context tag

Covers: `features/context_tags.feature`

## Interface used

The inbox at `GET /`, its quick-add box, the triage controls on each capture
row, and read-only `sqlite3` inspection. No project library, module, or test
helper is used. **Read the quick-add endpoint and its fields out of the
page's own markup.**

## Three things to hold before you start

**A tag is free text and there is no managed set**
(`D-context-tags-are-the-taxonomy`; the deliberate opposite of
`D-quotas-are-selected-not-typed`, argued in
`features/context_tags.feature`'s header). **If any procedure tempts you to
check that a tag is validated against a set, stop.**

**Prefix narrowing cannot be checked here.** These procedures assert what the
server sends — every prior tag, exactly once. `qa/phone_layout.md`'s browser
automation asserts page *geometry* and does not drive typing. **Report prefix
narrowing as unverified** rather than assuming that check covers it.

**The 50 ms capture budget's subject must not move**
(`T-latency-is-a-qa-assertion`). Adding a nullable column does not change
that endpoint's work; computing the suggestion set would. **Check it: if
capture slowed measurably, the distinct-values query has leaked out of the
page render and into `POST /captures`.**

## By-hand walkthrough — do this once, in a real browser

1. `cargo run -p trellis-server -- serve --db <fresh path>`.
2. Quick-add **"buy screws"** with the context field set to `@homedepot`.
   Confirm it saves and the tag shows on the row.
3. Quick-add **"return the drill"**. Start typing `@home` — **confirm
   `@homedepot` is offered** and can be accepted. **This is the step that
   cannot be scripted, and it is why the walkthrough exists.**
4. Triage both as **pool** — one tap each; pool asks for nothing else now.
   Confirm both keep their tags in the task list.
5. Quick-add **"pick up milk"**, tag `@supermarket`.
6. Restart and reload. Confirm all three keep their tags.

### Expected Observable Outcomes
- All six steps hold literally, per `D-visible-slices`.
- **Step 3 is the point of the slice.** A tag nobody can retype cheaply is a
  tag that fragments, and a fragmented tag is worse than none — it looks like
  a grouping and is not.
- **The quick-add box must stay one-handed.** `enterkeyhint="done"` and
  autofocus are there because capture happens on a phone; a tag field that
  costs a second tap before the text can be typed has made the fast path
  slower. Confirm the raw-text field still takes focus first.

## Setup — repeat before each procedure below

1. Start the server against a fresh database file.
2. Confirm the server is reachable and the inbox is empty.

## Procedure — a tag is kept, and is optional

1. Quick-add `buy screws` tagged `@homedepot`.
2. Quick-add `return the drill` with the context field left **empty**.
3. Quick-add `pick up milk` with the context field **omitted from the
   submission entirely**.
4. Quick-add `renew the passport` tagged with **only spaces**.
5. Read the inbox and query the capture rows.

### Expected Observable Outcomes
- `buy screws` shows `@homedepot`; the other three show no tag and are
  **accepted, not rejected**.
- **Empty, absent and whitespace-only produce the same stored state**
  (`T-empty-equals-absent`) — not one empty string and one NULL, and
  certainly not one acceptance and one rejection.
- Surrounding whitespace is trimmed off a tag that has content.
- **Check both transports.** The quick-add box posts a form; the capture
  endpoint also takes JSON. `T-required-fields-are-specified-per-transport`
  exists because #73 shipped a page whose form could never succeed while
  every feature stayed green.

## Procedure — the suggestions offer every prior tag, once

1. Quick-add four captures: two tagged `@homedepot`, one `@supermarket`, one
   with no tag.
2. `GET /` and read the tag control's suggestion list out of the markup.

### Expected Observable Outcomes
- Exactly **two** suggestions — `@homedepot` and `@supermarket`. A tag used
  twice is offered once; an untagged capture contributes nothing.
- **Prefix narrowing is not checked here** and must be reported as unverified.
  What is checked is that the list the browser narrows contains the right
  things.
- **Time the capture endpoint before and after populating tags.** If it slows
  as the tag set grows, the suggestion query has leaked into the capture path
  — see the budget note above.

## Procedure — case does not split a tag

1. Quick-add `buy screws` tagged `@HomeDepot`.
2. Quick-add `return the drill` tagged `@homedepot`.
3. Read the inbox and the suggestion list.

### Expected Observable Outcomes
- **One suggestion, not two**, reading `@HomeDepot` — the spelling first used.
- **Both rows display `@HomeDepot`**, including the one typed in lower case.
  The tag is one thing; the display is the spelling it was introduced with.
- **The failure this prevents**: a Menu showing two Home Depot lists and
  sending the owner twice.
- Confirm where identity is enforced. If it is a column constraint, try it
  **through the database** as well as the page; if it is application code,
  say so, because the two fail differently.

## Procedure — a tag can be given at triage, and survives it

1. Quick-add `buy screws` tagged `@homedepot`; triage it as pool.
2. Quick-add `return the drill` with no tag; triage it as pool supplying
   `@homedepot` on the triage control.
3. Read the task list and query both rows.

### Expected Observable Outcomes
- Both tasks show `@homedepot`.
- **The tag is stored once, on the capture**, and the task reads it through
  the capture it came from. Confirm the tag is **not** duplicated onto the
  task row: two copies of one fact is two places to edit and two chances to
  disagree.
- Tagging at capture is the fast path; tagging at triage is the second
  chance, for when the context is only obvious later. Both write the same
  field.

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
- **The suggestion list is a new render surface and the easy one to miss** —
  it renders tag text into markup attributes rather than element bodies,
  which escapes differently. Check it specifically.

## Procedure — nothing else changed

1. Run all thirteen existing QA suites.

### Expected Observable Outcomes
- **All pass.** This slice adds an optional field; it changes no existing
  behaviour.
- **PR #87's drift was invisible to CI and only a QA re-run found it**: a
  restyle touched three templates and no test surface, so CI stayed green
  while `capture_row.html`'s `<li>` gained `class="row"` and broke the
  exact-match regex every script uses. **You are editing those templates
  again. Re-run everything and believe the result, not the expectation.**
- `one_screen` still holds: no header renders, and the five removed routes
  still 404. Adding a field adds no route.

## Independent of Implementation

This procedure depends only on what the inbox renders, what its controls
accept, and what survives a restart. It does not depend on which row stores
the tag, whether identity is a column collation or application code, or how
the suggestion list is delivered.
