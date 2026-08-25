#!/usr/bin/env node
// Playwright-driven check for #120/#125, qa/trip_controls.md: the half of
// the trip panel's own controls that no HTTP assertion can hold --
// expanding and collapsing (client state, never stored, so a request log
// and a rendered box are the only things that can prove it), and the
// complete-group control's placement relative to the destructive `x Clear
// done` beside it.
//
// Drives the same real, headless Chrome scripts/qa/phone_layout.cjs uses,
// at a 390x844 phone viewport, against a Pool screen seeded with a trip of
// eight (@homedepot) and a trip of five (@supermarket) so independence is
// observable.
//
// Fails loudly, never skips: an uncaught error here exits non-zero, the
// same as an assertion failure -- qa/trip_controls.md's own warning, the
// same one qa/phone_layout.md and qa/colour.md already carry.

const { chromium } = require('playwright-core');

const baseUrl = process.argv[2];
if (!baseUrl) {
  console.error('usage: trip_controls.cjs <base-url>');
  process.exit(1);
}

const executablePath = process.env.PHONE_LAYOUT_CHROME || '/usr/bin/google-chrome';
const VIEWPORT = { width: 390, height: 844 };
const TAP = 44; // --tap

let failures = 0;
function fail(message) {
  console.error(`FAIL: ${message}`);
  failures += 1;
}

// Note: page.evaluate(fn) serializes only fn's own source text -- a
// reference to another top-level function in this file (like an
// `isPainted` helper) is undefined in the browser, not shared closure
// state. Every function passed to page.evaluate below is self-contained
// for that reason.

// Everything this script needs to read off a trip panel in one round trip:
// which <li>s are actually painted (not just present), the show-more
// control's own facts, whether any <details>/<summary> survives anywhere
// in the panel, and (for the complete-group placement check) every
// relevant control's bounding box.
function readTrip(tagText) {
  const panels = Array.from(document.querySelectorAll('.trip.panel'));
  const panel = panels.find((p) => {
    const tagEl = p.querySelector('.trip-tag');
    return tagEl && tagEl.textContent.trim() === tagText;
  });
  if (!panel) return null;

  const items = Array.from(panel.querySelectorAll('.trip-items > li')).map((li) => {
    const rect = li.getBoundingClientRect();
    return {
      text: li.querySelector('.trip-item-text')?.textContent.trim() || '',
      done: li.classList.contains('done'),
      painted: rect.width > 0 && rect.height > 0,
      rect: { top: rect.top, left: rect.left, bottom: rect.bottom, height: rect.height },
    };
  });

  const moreToggle = panel.querySelector('.trip-more-toggle');
  const hasDetailsOrSummary = !!panel.querySelector('details, summary');
  const clearDone = panel.querySelector('.clear-done');
  const completeButton = panel.querySelector('.trip-complete');

  const rectOf = (el) => (el ? el.getBoundingClientRect() : null);

  return {
    expanded: panel.classList.contains('expanded'),
    items,
    moreToggleTag: moreToggle ? moreToggle.tagName : null,
    moreToggleText: moreToggle ? moreToggle.textContent.trim() : null,
    hasDetailsOrSummary,
    clearDoneRect: rectOf(clearDone),
    completeButtonRect: rectOf(completeButton),
    checkboxRects: Array.from(panel.querySelectorAll('.done-check input[type="checkbox"]')).map((cb) => {
      const r = cb.getBoundingClientRect();
      return { left: r.left, right: r.right, top: r.top, bottom: r.bottom };
    }),
  };
}

function clickMoreToggle(tagText) {
  const panels = Array.from(document.querySelectorAll('.trip.panel'));
  const panel = panels.find((p) => {
    const tagEl = p.querySelector('.trip-tag');
    return tagEl && tagEl.textContent.trim() === tagText;
  });
  const toggle = panel && panel.querySelector('.trip-more-toggle');
  if (!toggle) return false;
  toggle.click();
  return true;
}

// page.evaluate(fn, arg) passes exactly one argument to fn -- takes
// {tagText, index} rather than two positional parameters, which a
// two-parameter signature here would silently receive as
// (arrayOrObject, undefined) instead of erroring.
function clickNthCheckbox({ tagText, index }) {
  const panels = Array.from(document.querySelectorAll('.trip.panel'));
  const panel = panels.find((p) => {
    const tagEl = p.querySelector('.trip-tag');
    return tagEl && tagEl.textContent.trim() === tagText;
  });
  const boxes = panel ? panel.querySelectorAll('.done-check input[type="checkbox"]') : [];
  if (!boxes[index]) return false;
  boxes[index].click();
  return true;
}

async function main() {
  const browser = await chromium.launch({
    executablePath,
    headless: true,
    args: ['--no-sandbox'],
  });

  try {
    const page = await browser.newPage();
    await page.setViewportSize(VIEWPORT);

    const requests = [];
    page.on('request', (req) => requests.push(req.url()));

    await page.goto(`${baseUrl}/pool`, { waitUntil: 'load' });

    // --- Procedure: the expand control, in a browser ---

    let big = await page.evaluate(readTrip, '@homedepot');
    let small = await page.evaluate(readTrip, '@supermarket');
    if (!big || !small) {
      fail(`could not find both seeded trips (@homedepot=${!!big}, @supermarket=${!!small})`);
      throw new Error('setup failed');
    }

    // Step 2: collapsed, exactly three of eight painted.
    const paintedCount = big.items.filter((i) => i.painted).length;
    if (paintedCount !== 3) {
      fail(`[collapsed] expected exactly 3 painted items of 8, got ${paintedCount}`);
    }
    if (big.items.length !== 8) {
      fail(`[collapsed] expected all 8 items present in the document, got ${big.items.length}`);
    }

    // Step 3: a button, not a <details>/<summary>, reading "Show 5 more".
    if (big.moreToggleTag !== 'BUTTON') {
      fail(`[control] expected the show-more control to be a <button>, got <${big.moreToggleTag}>`);
    }
    if (big.hasDetailsOrSummary) {
      fail('[control] found a <details> or <summary> in the panel -- the old disclosure control survived');
    }
    if (big.moreToggleText !== 'Show 5 more') {
      fail(`[control] expected the show-more control to read "Show 5 more", got ${JSON.stringify(big.moreToggleText)}`);
    }

    // Step 4/7: record requests, tap it, confirm zero requests fired.
    const requestsBefore = requests.length;
    const clicked = await page.evaluate(clickMoreToggle, '@homedepot');
    if (!clicked) fail('[expand] could not find the show-more control to click');
    await page.waitForTimeout(300);
    const requestsAfterExpand = requests.length - requestsBefore;
    if (requestsAfterExpand !== 0) {
      fail(
        `[expand] expanding made ${requestsAfterExpand} request(s), expected 0 -- ` +
          `client state went to the server: ${requests.slice(requestsBefore).join(', ')}`,
      );
    }

    // Step 5: expanded, all eight painted, label flips.
    big = await page.evaluate(readTrip, '@homedepot');
    const paintedAfterExpand = big.items.filter((i) => i.painted).length;
    if (paintedAfterExpand !== 8) {
      fail(`[expanded] expected all 8 items painted, got ${paintedAfterExpand}`);
    }
    if (big.moreToggleText !== 'Show fewer') {
      fail(`[expanded] expected the control to read "Show fewer", got ${JSON.stringify(big.moreToggleText)}`);
    }

    // Step 6: one list -- uniform vertical spacing and a shared left edge
    // across all eight, not a nested second list with its own margin.
    const gaps = [];
    for (let i = 1; i < big.items.length; i++) {
      gaps.push(big.items[i].rect.top - big.items[i - 1].rect.bottom);
    }
    const distinctGaps = new Set(gaps.map((g) => Math.round(g)));
    if (distinctGaps.size > 1) {
      fail(`[one-list] inconsistent vertical spacing between items, expected uniform: ${gaps.map((g) => g.toFixed(1)).join(', ')}`);
    }
    const lefts = new Set(big.items.map((i) => Math.round(i.rect.left)));
    if (lefts.size > 1) {
      fail(`[one-list] items do not share a left edge: ${[...lefts].join(', ')}`);
    }

    // Step 8: tap again, back to three, label restored.
    const requestsBeforeCollapse = requests.length;
    await page.evaluate(clickMoreToggle, '@homedepot');
    await page.waitForTimeout(300);
    if (requests.length - requestsBeforeCollapse !== 0) {
      fail('[collapse] collapsing made a request, expected 0');
    }
    big = await page.evaluate(readTrip, '@homedepot');
    const paintedAfterCollapse = big.items.filter((i) => i.painted).length;
    if (paintedAfterCollapse !== 3) {
      fail(`[collapse] expected 3 painted items again, got ${paintedAfterCollapse}`);
    }
    if (big.moreToggleText !== 'Show 5 more') {
      fail(`[collapse] expected the control to read "Show 5 more" again, got ${JSON.stringify(big.moreToggleText)}`);
    }

    // Step 9: independence -- expand @homedepot, @supermarket stays collapsed.
    await page.evaluate(clickMoreToggle, '@homedepot');
    await page.waitForTimeout(100);
    small = await page.evaluate(readTrip, '@supermarket');
    const smallPainted = small.items.filter((i) => i.painted).length;
    if (smallPainted !== 3) {
      fail(`[independence] expected @supermarket to stay collapsed (3 painted of 5), got ${smallPainted}`);
    }
    big = await page.evaluate(readTrip, '@homedepot');
    if (!big.expanded) {
      fail('[independence] expected @homedepot to still be expanded');
    }

    // Step 10: expanded survives being worked -- tick the sixth item (one
    // of the five that only became visible on expansion) and confirm the
    // panel is STILL expanded, all eight still painted, and the ticked
    // item struck in place, after the #pool-body outerHTML swap.
    const beforeTick = await page.evaluate(readTrip, '@homedepot');
    const sixthText = beforeTick.items[5].text;
    const clickedCheckbox = await page.evaluate(clickNthCheckbox, { tagText: '@homedepot', index: 5 });
    if (!clickedCheckbox) fail('[survives-being-worked] could not find the sixth item\'s checkbox');
    await page.waitForResponse((resp) => resp.request().method() === 'POST', { timeout: 5000 }).catch(() => {});
    await page.waitForTimeout(200);
    const afterTick = await page.evaluate(readTrip, '@homedepot');
    if (!afterTick.expanded) {
      fail('[survives-being-worked] the panel collapsed when an item inside it was ticked -- the swap destroyed client state');
    }
    const paintedAfterTick = afterTick.items.filter((i) => i.painted).length;
    if (paintedAfterTick !== 8) {
      fail(`[survives-being-worked] expected all 8 items still painted after the tick, got ${paintedAfterTick}`);
    }
    const struckItem = afterTick.items.find((i) => i.text === sixthText);
    if (!struckItem || !struckItem.done) {
      fail(`[survives-being-worked] expected ${JSON.stringify(sixthText)} struck in place, got: ${JSON.stringify(struckItem)}`);
    }

    // --- Procedure: the complete-group control, placement (step 3) ---
    // Re-navigate fresh so the completed sixth item above does not affect
    // this measurement, then mark one item done through the route so
    // `x Clear done` renders alongside the complete-group control -- the
    // placement claim only means something once both controls exist.
    await page.goto(`${baseUrl}/pool`, { waitUntil: 'load' });
    await page.evaluate(clickNthCheckbox, { tagText: '@homedepot', index: 0 });
    await page.waitForResponse((resp) => resp.request().method() === 'POST', { timeout: 5000 }).catch(() => {});
    await page.waitForTimeout(200);
    const placed = await page.evaluate(readTrip, '@homedepot');
    if (!placed.completeButtonRect) {
      fail('[placement] no .trip-complete control found after marking one item done');
    } else if (!placed.clearDoneRect) {
      fail('[placement] no .clear-done control found after marking one item done');
    } else {
      const complete = placed.completeButtonRect;
      const clear = placed.clearDoneRect;
      // "Same row" means their vertical ranges overlap, not that their
      // tops align -- two controls of different heights
      // (.clear-done: 22px, .trip-complete: --tap, 44px) sitting in a
      // flex row with align-items: center are visually adjacent with
      // different `top`s, since the row centers them rather than
      // top-aligning them. A top-only comparison missed exactly this.
      const sameRow = complete.top < clear.bottom && complete.bottom > clear.top;
      let clearSpace = Infinity;
      if (sameRow) {
        clearSpace = complete.left >= clear.right ? complete.left - clear.right : clear.left - complete.right;
      }
      if (sameRow && clearSpace < TAP) {
        fail(
          `[placement] complete-group control is on the same row as Clear done with only ${clearSpace.toFixed(1)}px ` +
            `between them, expected a different row or at least ${TAP}px`,
        );
      }
      // "The horizontal band the item checkboxes occupy" only creates a
      // mistap hazard where a thumb could actually land on both -- true
      // 2D overlap, not merely sharing an x-range. A full-width button
      // on its own row (this product's own .trip-actions, entirely above
      // .trip-items) shares the checkboxes' x-range by construction and
      // is not the hazard qa/trip_controls.md is naming; a button placed
      // so its box physically overlaps a checkbox's box is.
      const overlapsACheckbox = placed.checkboxRects.some(
        (cb) =>
          complete.left < cb.right &&
          complete.right > cb.left &&
          complete.top < cb.bottom &&
          complete.bottom > cb.top,
      );
      if (overlapsACheckbox) {
        fail('[placement] the complete-group control physically overlaps an item checkbox');
      }
    }
  } finally {
    await browser.close();
  }

  if (failures > 0) {
    console.error(`FAIL: trip_controls (${failures} assertion(s) failed)`);
    process.exit(1);
  }
  console.log('PASS: trip_controls (browser assertions)');
}

main().catch((error) => {
  console.error('FAIL: trip_controls (uncaught error)');
  console.error(error);
  process.exit(1);
});
