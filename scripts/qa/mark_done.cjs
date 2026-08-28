#!/usr/bin/env node
// Playwright-driven check for qa/mark_done.md's "the way back is at least
// 44px" outcome. #111's way-back line is transient -- it exists only in
// the tick's own response fragment -- so this is the one thing about it
// that a static seed-and-navigate check (phone_layout.cjs's own shape)
// cannot see: it has to actually click the checkbox in a real browser and
// measure the control the click produced.
//
// Drives the same real, headless Chrome scripts/qa/phone_layout.cjs uses,
// at a 390x844 phone viewport. Fails loudly, never skips: an uncaught
// error here exits non-zero, the same no-skip contract every browser
// check in this project carries.
//
// Args: <base-url> <pool-task-id> <committed-task-id> -- both tasks
// expected to exist, open, before this script runs (the .sh wrapper seeds
// them so their ids are known up front).

const { chromium } = require('playwright-core');

const baseUrl = process.argv[2];
const poolTaskId = process.argv[3];
const committedTaskId = process.argv[4];
if (!baseUrl || !poolTaskId || !committedTaskId) {
  console.error('usage: mark_done.cjs <base-url> <pool-task-id> <committed-task-id>');
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

async function checkWayBackTapTarget(page, path, checkboxSelector, label) {
  await page.goto(`${baseUrl}${path}`, { waitUntil: 'load' });
  await Promise.all([
    page.waitForResponse((resp) => resp.request().method() === 'POST' && resp.url().includes('/done')),
    page.locator(checkboxSelector).click(),
  ]);
  const rect = await page.evaluate(() => {
    const el = document.querySelector('.way-back-undo');
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { width: r.width, height: r.height };
  });
  if (!rect) {
    fail(`[${label}] no .way-back-undo control found after ticking the checkbox`);
    return;
  }
  if (rect.height < TAP) {
    fail(`[${label}] the way-back-undo control is ${rect.height.toFixed(1)}px tall, expected at least ${TAP}px`);
  }
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

    await checkWayBackTapTarget(
      page,
      '/pool',
      `input[type="checkbox"][hx-post="/pool/tasks/${poolTaskId}/done"]`,
      'pool',
    );
    await checkWayBackTapTarget(
      page,
      '/committed',
      `input[type="checkbox"][hx-post="/committed/tasks/${committedTaskId}/done"]`,
      'committed',
    );
  } finally {
    await browser.close();
  }

  if (failures > 0) {
    console.error(`FAIL: mark_done (${failures} assertion(s) failed)`);
    process.exit(1);
  }
  console.log('PASS: mark_done (browser assertions)');
}

main().catch((error) => {
  console.error('FAIL: mark_done (uncaught error)');
  console.error(error);
  process.exit(1);
});
