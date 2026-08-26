# QA Procedure: A committed date is chosen, not typed

Covers: `features/committed_date.feature`

## Interface used

The capture screen's committed triage control, the Committed screen, the
timezone control on... **see below — there isn't one.** Plus read-only
`sqlite3`, and `scripts/qa/phone_layout.cjs` for the one assertion that needs
a browser.

## Three things that will be got wrong

**1. The canvas draws no date input at all**, verified rather than assumed —
a gap filled at the owner's direction, argued in
`features/committed_date.feature`'s header. **Its 44px touch height is a real
precedent and is kept.**

**2. A `by` takes a day and no time on the page — but the boundary still
accepts one.** `committed-screen-at-and-by-02` asserts a `by` carrying a time
still renders as a `by`. **The page stops asking; the API must not stop
accepting.** A JSON `by` with a time being rejected is a defect: the form was
narrowed and the contract narrowed with it by accident.

**3. The display was UTC and is part of this slice.** `date_cell` said so in
its own doc comment. **A date entered correctly at 23:00 local and then shown
on the wrong day is the same bug from the other end.**

**There is no way to set the timezone from the app.**
`T-timezone-is-a-setting` put the owner's zone in the database and #88
removed the page it was edited on. Set it directly for these procedures and
**say in the report that you had to** — a QA procedure that reaches past the
interface is a finding, not a technique.

## By-hand walkthrough — on a phone

1. `ops/preview.sh` for the branch, or the tailnet instance.
2. Capture *"call the dentist"*, triage as **committed**, choose **at**.
3. **Confirm the day and time controls open your phone's own pickers** and
   that no timestamp can be typed.
4. Choose a day and 8:30. Confirm it lands on **Committed** on that day.
5. Triage another as **by**. **Confirm there is no time control at all.**
6. Confirm each control is a real 44px target.

### Expected Observable Outcomes
- All six steps hold.
- **Step 5 is the halving.** The commonest committed thing is a deadline, and
  a deadline has no time of day worth entering.
- **Step 3 is the point of the slice**: no typing, no UTC, no `422` for a
  stray character.

## Procedure — an at, and a by

1. Triage an `at` for a chosen day at `08:30`; triage a `by` for a chosen day.
2. Read the Committed screen. Query both rows.

### Expected Observable Outcomes
- The `at` cell reads its day and time; the `by` cell reads `BY` and its day.
- The **`by`'s stored instant is the end of that day in the owner's zone** —
  "by Thursday" means before Thursday is over. Check the stored millisecond
  value against the zone, not against UTC.
- **The conversion is a business rule and belongs in `scheduler-core`**
  (`T-jiff-epoch-millis`). If a local date became an instant in a handler, or
  in JavaScript, that is a defect however correct the answer looks.

## Procedure — the timezone decides the day

1. Set the zone to `America/New_York`.
2. Triage an `at` at **23:30** on a chosen day, and another at **00:30**.
3. Read the Committed screen.

### Expected Observable Outcomes
- **Both land on the day chosen**, not the day either side of it.
- **This is the assertion most likely to be missing**, and a habit of typing
  UTC hides it completely — in UTC the bug is invisible because there is no
  conversion to get wrong.
- Repeat with a second zone far from the first and confirm the rendered day
  follows the setting rather than the server.

## Procedure — the JSON transport is unchanged

1. Triage a committed task over the API with an RFC 3339 instant.
2. Triage a **`by` carrying a time** over the API.
3. Read the screen.

### Expected Observable Outcomes
- Both accepted. The `by` with a time still renders as a **`by`**.
- **The page and the endpoint are one code path**
  (`T-core-owns-validation-order`). A control only the page understands must
  not fork them, and `T-required-fields-are-specified-per-transport` is why
  both are checked here rather than one.

## Procedure — rejections still name the field

1. Submit a committed triage with the deadline omitted; then with the
   commitment omitted. Do both over **each** transport.

### Expected Observable Outcomes
- **`422`** carrying the re-rendered fragment, naming the missing field.
- A date control that makes a field un-omittable **on the page** does not
  make it optional at the boundary.

## Procedure — the cell says which day, beyond this week

1. With a pinned clock, triage a `by` three days out and another three weeks
   out.

### Expected Observable Outcomes
- Near: `BY THU`. Far: the date, e.g. `BY 17 SEP`.
- **`BY THU` cannot tell two Thursdays apart**, which was harmless only while
  nothing could be dated beyond a week. It can now.
- **Both forms must fit 66px** at 10.5px uppercase — that is what the canvas
  gives this cell. `BY THU 27 AUG` does not fit and is not what was
  specified.

## Procedure — the cell does not clip, on a phone

1. Run `scripts/qa/phone_layout.sh` with a committed task dated far enough
   out to use the long form.

### Expected Observable Outcomes
- The date cell does not overflow or wrap.
- **This is the first slice that can check this** — #105 added a browser that
  can see geometry. **If the check cannot currently assert it, say so and say
  what it would take** rather than checking it by eye and calling it verified.

## Procedure — nothing else changed

1. Run every existing QA suite and the acceptance suite.

### Expected Observable Outcomes
- **All 17 acceptance features pass untouched.**
- `committed_screen`'s own suite still passes — its date-cell assertions are
  about `at` versus `by`, which this slice does not change, only the zone the
  cell is rendered in and the long form beyond a week. **If those scenarios
  needed editing, say which and why.**

## Independent of Implementation

This procedure depends only on what the triage control accepts, what instant
is stored, and what the Committed screen renders for a given zone. It does
not depend on which input types the form uses, where the conversion lives, or
how the cell is formatted.
