#!/usr/bin/env node
// Playwright-driven phone-viewport geometry check for #101 / qa/phone_layout.md.
//
// Asserts three things at 390x844 on each screen that exists today
// (Capture, Pool, Committed): the document itself never scrolls, the
// screen seeded to overflow scrolls inside its own <main>, and the tab
// bar's bottom edge sits on the viewport's bottom edge -- found by role
// and accessible name, never by class or DOM shape
// (T-qa-binds-tolerantly-to-markup applies to a browser check as much as
// an HTTP one).
//
// On Committed, a fourth assertion: no `.committed-date` cell clips or
// wraps (#110's own "the cell does not clip, on a phone" procedure). The
// cell has no ARIA role of its own to bind to -- the same ground every
// other QA script's content extraction already stands on (`.trip-tag`,
// `.loose-text`, ...) -- so this one is a QA-owned class, not a landmark.
//
// Fails loudly, never skips: an uncaught error here exits non-zero, the
// same as an assertion failure -- a check that silently skips when Chrome
// or playwright-core is missing reproduces the exact blind spot #101
// exists to close.

const { chromium } = require('playwright-core');

const baseUrl = process.argv[2];
if (!baseUrl) {
  console.error('usage: phone_layout.cjs <base-url>');
  process.exit(1);
}

const executablePath = process.env.PHONE_LAYOUT_CHROME || '/usr/bin/google-chrome';
const VIEWPORT = { width: 390, height: 844 };
const TAB_BAR_TOLERANCE_PX = 2;

// D-four-screens names four; all four answer 200 now that #93 has shipped
// both its halves. Quota carries the densest header this product draws --
// a name, a readout, a note, two quick-log buttons and an `Other…`
// disclosure all in one row at 390px -- so a tap-target or overflow
// regression is more likely to show up here first than on the other three.
const SCREENS = [
  { path: '/', label: 'Capture', expectOverflow: true },
  { path: '/pool', label: 'Pool', expectOverflow: false },
  { path: '/committed', label: 'Committed', expectOverflow: false },
  { path: '/quota', label: 'Quota', expectOverflow: false },
];

let failures = 0;
function fail(message) {
  console.error(`FAIL: ${message}`);
  failures += 1;
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

    for (const screen of SCREENS) {
      await page.goto(`${baseUrl}${screen.path}`, { waitUntil: 'load' });

      // Assertion 1: the document itself does not scroll, on every screen
      // -- if the body grows past the viewport again, this is what
      // catches it, regardless of which screen overflows.
      const doc = await page.evaluate(() => ({
        scrollHeight: document.scrollingElement.scrollHeight,
        clientHeight: document.scrollingElement.clientHeight,
      }));
      if (doc.scrollHeight > doc.clientHeight) {
        fail(
          `[${screen.label}] the document scrolls: scrollHeight=${doc.scrollHeight} ` +
            `clientHeight=${doc.clientHeight}`,
        );
      }

      // Assertion 2: main overflows if and only if this screen is
      // expected to (seeded with enough content on Capture; short by
      // construction on the other two) -- a screen that fits must not
      // gain a scrollbar, which is the regression the fix itself could
      // introduce.
      const main = page.getByRole('main');
      const mainMetrics = await main.evaluate((el) => ({
        scrollHeight: el.scrollHeight,
        clientHeight: el.clientHeight,
      }));
      if (screen.expectOverflow) {
        if (mainMetrics.scrollHeight <= mainMetrics.clientHeight) {
          fail(
            `[${screen.label}] expected main to overflow (seed more content?): ` +
              `scrollHeight=${mainMetrics.scrollHeight} clientHeight=${mainMetrics.clientHeight}`,
          );
        }
      } else if (mainMetrics.scrollHeight > mainMetrics.clientHeight) {
        fail(
          `[${screen.label}] a screen that fits gained a scrollbar: ` +
            `scrollHeight=${mainMetrics.scrollHeight} clientHeight=${mainMetrics.clientHeight}`,
        );
      }

      // Assertion 3: the tab bar's bottom edge is the viewport's bottom
      // edge -- half the bug, present even where nothing overflows.
      // Found by the <nav> landmark's own role, never a selector.
      const nav = page.getByRole('navigation');
      const navBox = await nav.boundingBox();
      if (!navBox) {
        fail(`[${screen.label}] no navigation landmark found`);
      } else {
        const bottom = navBox.y + navBox.height;
        const delta = Math.abs(bottom - VIEWPORT.height);
        if (delta > TAB_BAR_TOLERANCE_PX) {
          fail(
            `[${screen.label}] tab bar bottom edge is ${bottom}px, expected ` +
              `~${VIEWPORT.height}px (delta ${delta}px)`,
          );
        }
      }

      // Assertion 4 (Committed only): no date cell clips or wraps. The
      // 66px cell is fixed-width and single-line by design; either
      // scrollWidth or scrollHeight exceeding the client box is a clip or
      // a wrap qa/committed_date.md names as the thing to check for.
      if (screen.path === '/committed') {
        const cellMetrics = await page.evaluate(() =>
          Array.from(document.querySelectorAll('.committed-date')).map((el) => ({
            text: el.textContent,
            scrollWidth: el.scrollWidth,
            clientWidth: el.clientWidth,
            scrollHeight: el.scrollHeight,
            clientHeight: el.clientHeight,
          })),
        );
        if (cellMetrics.length === 0) {
          fail(`[${screen.label}] no .committed-date cells found (seed a committed task?)`);
        }
        for (const cell of cellMetrics) {
          if (cell.scrollWidth > cell.clientWidth || cell.scrollHeight > cell.clientHeight) {
            fail(
              `[${screen.label}] date cell ${JSON.stringify(cell.text)} clips or wraps: ` +
                `scrollWidth=${cell.scrollWidth} clientWidth=${cell.clientWidth} ` +
                `scrollHeight=${cell.scrollHeight} clientHeight=${cell.clientHeight}`,
            );
          }
        }
      }
    }
  } finally {
    await browser.close();
  }

  if (failures > 0) {
    console.error(`FAIL: phone_layout (${failures} assertion(s) failed)`);
    process.exit(1);
  }
  console.log('PASS: phone_layout (browser assertions)');
}

main().catch((error) => {
  console.error('FAIL: phone_layout (uncaught error)');
  console.error(error);
  process.exit(1);
});
