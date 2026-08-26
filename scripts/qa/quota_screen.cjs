#!/usr/bin/env node
// Playwright-driven check for the half of qa/quota_screen.md's "the
// warning as you type" procedure that an HTTP-level script cannot hold:
// tap-target size, and whether typing costs a network request.
//
// FINDING, established here rather than assumed: the built define form
// (crates/trellis-server/templates/quota_body.html) has no hx-trigger on
// its name/hours inputs and no client script anywhere on this screen --
// the only round trip is the form's own `hx-post="/quota"` on submit. The
// warning is real (`quota-screen-repeated-name-refused-06`,
// `-similar-name-warns-07`, both HTTP-asserted and covered by
// scripts/qa/quota_screen.sh) but it is a SUBMIT-time check, not a live,
// as-you-type one -- qa/quota_screen.md's framing ("the warning panel
// appears and updates as the name changes") describes a feature that was
// not built. This script verifies the actual behaviour (zero requests
// while typing, nothing submitted until the button is pressed) rather
// than asserting the doc's literal text, and reports the mismatch.
//
// Drives the same real, headless Chrome scripts/qa/phone_layout.cjs uses,
// at a 390x844 phone viewport. Fails loudly, never skips: an uncaught
// error here exits non-zero, the same no-skip contract every browser
// check in this project carries.

const { chromium } = require('playwright-core');

const baseUrl = process.argv[2];
if (!baseUrl) {
  console.error('usage: quota_screen.cjs <base-url>');
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

function readDefineForm() {
  const form = document.querySelector('.quota-define-form');
  if (!form) return null;
  const rectOf = (el) => (el ? el.getBoundingClientRect() : null);
  return {
    nameRect: rectOf(form.querySelector('input[name="name"]')),
    hoursRect: rectOf(form.querySelector('input[name="hours"]')),
    submitRect: rectOf(form.querySelector('.quota-define-submit')),
    warningPresent: !!form.querySelector('.quota-warning'),
  };
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

    await page.goto(`${baseUrl}/quota`, { waitUntil: 'load' });

    // --- Procedure: every control on the define form is a real tap target ---
    const before = await page.evaluate(readDefineForm);
    if (!before) {
      fail('no .quota-define-form found on the Quota screen');
      throw new Error('setup failed');
    }
    for (const [label, rect] of [
      ['name input', before.nameRect],
      ['hours input', before.hoursRect],
      ['define/submit control', before.submitRect],
    ]) {
      if (!rect) {
        fail(`[tap-targets] ${label} not found`);
      } else if (rect.height < TAP) {
        fail(`[tap-targets] ${label} is ${rect.height.toFixed(1)}px tall, expected at least ${TAP}px`);
      }
    }

    // --- Procedure: the warning as you type ---
    // qa/quota_screen.md asks whether the live check costs a request per
    // keystroke. The finding: it costs none, because there is no live
    // check -- confirmed by typing slowly and recording every request the
    // page issues, per its own instruction to "record network activity
    // throughout".
    const requests = [];
    page.on('request', (req) => requests.push(req.url()));

    const nameInput = page.locator('input[name="name"]');
    await nameInput.fill('');
    for (const ch of 'Piano') {
      await nameInput.type(ch, { delay: 50 });
    }
    await page.waitForTimeout(300);

    if (requests.length > 0) {
      fail(
        `[as-you-type] expected zero network requests while typing (no live check exists), got ${requests.length}: ${requests.join(', ')}`,
      );
    } else {
      console.log('FINDING: [as-you-type] typing "Piano" issued zero requests -- confirmed no live, per-keystroke check exists; the warning is submit-only (POST /quota, HTTP-asserted in quota_screen.sh). qa/quota_screen.md\'s "the warning panel appears and updates as the name changes" describes a feature that was not built; flagged for the specifier rather than asserted as written.');
    }

    const afterTyping = await page.evaluate(readDefineForm);
    if (afterTyping.warningPresent) {
      fail('[as-you-type] a warning panel appeared before the form was ever submitted');
    }

    // Nothing about the half-typed name is stored: reload mid-typing and
    // confirm the form is back to empty (a plain, un-scripted HTML input
    // has no persistence of its own, and nothing here gives it one).
    await page.reload({ waitUntil: 'load' });
    const afterReload = await page.evaluate(() => {
      const input = document.querySelector('input[name="name"]');
      return input ? input.value : null;
    });
    if (afterReload !== '') {
      fail(`[as-you-type] expected the name field empty after a mid-typing reload, got: ${JSON.stringify(afterReload)}`);
    }
  } finally {
    await browser.close();
  }

  if (failures > 0) {
    console.error(`FAIL: quota_screen (${failures} assertion(s) failed)`);
    process.exit(1);
  }
  console.log('PASS: quota_screen (browser assertions)');
}

main().catch((error) => {
  console.error('FAIL: quota_screen (uncaught error)');
  console.error(error);
  process.exit(1);
});
