#!/usr/bin/env node
// Playwright-driven check for qa/quota_sessions.md's "the disclosures, in a
// browser" procedure: which quota row is expanded, and whether its
// Other... panel is open, are ephemeral view state
// (T-ephemeral-view-state-rides-the-request) that an HTTP-level script
// cannot observe -- a <details> element's own open/closed state lives in
// the DOM, not in any response body.
//
// Drives the same real, headless Chrome scripts/qa/phone_layout.cjs uses,
// at a 390x844 phone viewport, against a Quota screen seeded with three
// quotas so independence between rows is observable.
//
// Fails loudly, never skips: an uncaught error here exits non-zero, the
// same no-skip contract every browser check in this project carries.

const { chromium } = require('playwright-core');

const baseUrl = process.argv[2];
if (!baseUrl) {
  console.error('usage: quota_sessions.cjs <base-url>');
  process.exit(1);
}

const executablePath = process.env.PHONE_LAYOUT_CHROME || '/usr/bin/google-chrome';
const VIEWPORT = { width: 390, height: 844 };

let failures = 0;
function fail(message) {
  console.error(`FAIL: ${message}`);
  failures += 1;
}

function readQuotaRows() {
  const rows = Array.from(document.querySelectorAll('.quota-row'));
  return rows.map((row) => {
    const nameEl = row.querySelector('.quota-name');
    const expand = row.querySelector('details.quota-expand');
    const other = row.querySelector('details.quota-other');
    const sessionLabels = Array.from(row.querySelectorAll('.quota-session-label')).map((el) =>
      el.textContent.trim(),
    );
    return {
      id: row.id,
      name: nameEl ? nameEl.textContent.trim() : null,
      expanded: !!(expand && expand.open),
      otherOpen: !!(other && other.open),
      sessionLabels,
    };
  });
}

function clickExpand(name) {
  const rows = Array.from(document.querySelectorAll('.quota-row'));
  const row = rows.find((r) => r.querySelector('.quota-name')?.textContent.trim() === name);
  if (!row) return false;
  const summary = row.querySelector('details.quota-expand > summary');
  if (!summary) return false;
  summary.click();
  return true;
}

function clickOther(name) {
  const rows = Array.from(document.querySelectorAll('.quota-row'));
  const row = rows.find((r) => r.querySelector('.quota-name')?.textContent.trim() === name);
  if (!row) return false;
  const summary = row.querySelector('details.quota-other > summary');
  if (!summary) return false;
  summary.click();
  return true;
}

function clickQuickLog30m(name) {
  const rows = Array.from(document.querySelectorAll('.quota-row'));
  const row = rows.find((r) => r.querySelector('.quota-name')?.textContent.trim() === name);
  if (!row) return false;
  const button = row.querySelector('.quota-quick-log-30m');
  if (!button) return false;
  button.click();
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
    await page.goto(`${baseUrl}/quota`, { waitUntil: 'load' });

    let rows = await page.evaluate(readQuotaRows);
    if (rows.length < 3) {
      fail(`expected at least 3 seeded quotas, found ${rows.length}`);
      throw new Error('setup failed');
    }
    const [first, second] = rows;
    if (rows.some((r) => r.expanded || r.otherOpen)) {
      fail('expected every row collapsed on a fresh load, found one already expanded/open');
    }

    // --- Step 2: expand one; the others stay collapsed ---
    await page.evaluate(clickExpand, first.name);
    rows = await page.evaluate(readQuotaRows);
    const firstRow = rows.find((r) => r.name === first.name);
    if (!firstRow.expanded) {
      fail('[step2] expected the clicked row to be expanded');
    }
    if (rows.some((r) => r.name !== first.name && r.expanded)) {
      fail('[step2] expected every other row to stay collapsed');
    }

    // --- Step 3: open Other... on a different row; confirm the first's state ---
    await page.evaluate(clickOther, second.name);
    rows = await page.evaluate(readQuotaRows);
    const firstAfterOther = rows.find((r) => r.name === first.name);
    const secondAfterOther = rows.find((r) => r.name === second.name);
    if (!secondAfterOther.otherOpen) {
      fail('[step3] expected the second row\'s Other panel to be open');
    }
    console.log(
      `FINDING: [step3] opening Other... on "${second.name}" leaves "${first.name}" expanded=${firstAfterOther.expanded} -- independent client state, each <details> toggles on its own.`,
    );

    // --- Step 4: with a row expanded, log a session in it ---
    // This is qa/quota_sessions.md's own named hard part: an hx-swap of
    // #quota-body's outerHTML replaces every <details> in the fragment,
    // and nothing found anywhere in quota/http.rs or quota/view.rs echoes
    // back which row was expanded the way base.html's own
    // htmx:configRequest hook does for #pool-body's trips (that hook is
    // itself scoped to `poolBody.contains(event.detail.elt)` and does
    // nothing for a request originating inside #quota-body).
    await page.goto(`${baseUrl}/quota`, { waitUntil: 'load' });
    await page.evaluate(clickExpand, first.name);
    let before = (await page.evaluate(readQuotaRows)).find((r) => r.name === first.name);
    if (!before.expanded) {
      fail('[step4] setup: expected the row expanded before logging a session in it');
    }
    const sessionCountBefore = before.sessionLabels.length;

    await page.evaluate(clickQuickLog30m, first.name);
    await page.waitForResponse((resp) => resp.request().method() === 'POST', { timeout: 5000 }).catch(() => {});
    await page.waitForTimeout(200);

    const after = (await page.evaluate(readQuotaRows)).find((r) => r.name === first.name);
    if (!after) {
      fail('[step4] the row disappeared after logging a session');
    } else {
      if (!after.expanded) {
        fail(
          `[step4] THE ROW COLLAPSED: expected "${first.name}" to still be expanded after logging +30m in it, ` +
            'but the fragment swap reset every <details> to closed -- this is D-a-trip-survives-being-worked\'s ' +
            'failure arriving on the Quota screen: the panel closes under your thumb and takes the session you ' +
            'just logged off the screen with it.',
        );
      }
      if (after.sessionLabels.length !== sessionCountBefore + 1) {
        fail(
          `[step4] expected the newly logged session to be visible after the swap (was ${sessionCountBefore} sessions, now ${after.sessionLabels.length}), ` +
            'but even if it is present in the response, it is not visible because the row is collapsed.',
        );
      }
    }

    // --- Step 5: reload; a fresh load starting collapsed is correct ---
    await page.reload({ waitUntil: 'load' });
    rows = await page.evaluate(readQuotaRows);
    if (rows.some((r) => r.expanded || r.otherOpen)) {
      fail('[step5] expected every row collapsed on a fresh reload, found one still expanded/open');
    }
  } finally {
    await browser.close();
  }

  if (failures > 0) {
    console.error(`FAIL: quota_sessions (${failures} assertion(s) failed)`);
    process.exit(1);
  }
  console.log('PASS: quota_sessions (browser assertions)');
}

main().catch((error) => {
  console.error('FAIL: quota_sessions (uncaught error)');
  console.error(error);
  process.exit(1);
});
