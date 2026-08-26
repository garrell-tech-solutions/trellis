#!/usr/bin/env node
// Playwright-driven check for qa/quota_triage_validation.md's "the name box
// is the escape hatch, in a browser" procedure. #138 retired the Quota
// screen's own define form; the quota fields this project can still submit
// live in the triage panel on the inbox page, so that is what this drives.
//
// Drives the same real, headless Chrome scripts/qa/phone_layout.cjs uses,
// at a 390x844 phone viewport. Fails loudly, never skips: an uncaught error
// here exits non-zero, the same no-skip contract every browser check in
// this project carries.
//
// Args: <base-url> <finish-chapter-3-capture-id> <workout-capture-id>
// <third-capture-id> -- all three expected to exist, untriaged, before this
// script runs (the .sh wrapper submits them so their row text is known up
// front). The doc's own steps 1-4 use the first two and submit each; the
// third is reserved for the separate reload-mid-typing check, since
// submitting a capture removes its row from the inbox.

const { chromium } = require('playwright-core');

const baseUrl = process.argv[2];
const finishCaptureId = process.argv[3];
const workoutCaptureId = process.argv[4];
const reloadCaptureId = process.argv[5];
if (!baseUrl || !finishCaptureId || !workoutCaptureId || !reloadCaptureId) {
  console.error('usage: quota_triage_validation.cjs <base-url> <finish-capture-id> <workout-capture-id> <reload-capture-id>');
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

function readQuotaPanel(captureId) {
  const row = document.getElementById(`capture-row-${captureId}`);
  if (!row) return { open: false, rowPresent: false };
  const form = row.querySelector('.fields-panel form.fields');
  if (!form) return { open: false, rowPresent: true };
  const rectOf = (el) => (el ? el.getBoundingClientRect() : null);
  return {
    open: true,
    rowPresent: true,
    nameValue: form.querySelector('input[name="name"]')?.value ?? null,
    nameRect: rectOf(form.querySelector('input[name="name"]')),
    hoursRect: rectOf(form.querySelector('input[name="hours"]')),
    submitRect: rectOf(form.querySelector('button[type="submit"]')),
    hasSessionsField: /sessions/i.test(form.innerHTML),
    hasMinutesEachField: form.innerHTML.includes('minutes_each'),
    hasPeriodField: form.innerHTML.includes('name="period"'),
  };
}

function quotaRowReadout(name) {
  const rows = Array.from(document.querySelectorAll('.quota-row'));
  const row = rows.find((r) => r.querySelector('.quota-name')?.textContent === name);
  return row ? row.querySelector('.quota-readout')?.textContent ?? null : null;
}

async function tapQuota(page, captureId) {
  await page.locator(`#capture-row-${captureId} form.kind-choice`).filter({ hasText: 'Quota' }).locator('button').click();
  await page.waitForSelector(`#capture-row-${captureId} .fields-panel form.fields input[name="name"]`, { timeout: 5000 });
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
    await page.goto(`${baseUrl}/`, { waitUntil: 'load' });

    // --- Step 1: tap Quota; the name box already holds the capture's own
    // words, in full (a multi-word capture, so a prefill that survives one
    // word and drops the rest would still pass on a single-word fixture). ---
    await tapQuota(page, finishCaptureId);
    let panel = await page.evaluate(readQuotaPanel, finishCaptureId);
    if (!panel || !panel.open) {
      fail('no quota fields-panel found after tapping Quota on the "finish chapter 3" row');
      throw new Error('setup failed');
    }
    if (panel.nameValue !== 'finish chapter 3') {
      fail(`[prefill] expected the name box prefilled with "finish chapter 3" in full, got: ${JSON.stringify(panel.nameValue)}`);
    }
    if (panel.hasSessionsField || panel.hasMinutesEachField || panel.hasPeriodField) {
      fail('[retired-fields] the quota panel still carries a sessions/minutes-each/period field -- the retirement was cosmetic');
    }
    for (const [label, rect] of [
      ['name input', panel.nameRect],
      ['hours input', panel.hoursRect],
      ['submit control', panel.submitRect],
    ]) {
      if (!rect) {
        fail(`[tap-targets] ${label} not found`);
      } else if (rect.height < TAP) {
        fail(`[tap-targets] ${label} is ${rect.height.toFixed(1)}px tall, expected at least ${TAP}px`);
      }
    }

    // --- Step 2: record network activity. Edit the name to Reading. Type
    // 2 hours. Typing costs zero requests -- no live, per-keystroke check
    // exists; the warning is submit-only. ---
    const requests = [];
    page.on('request', (req) => requests.push(req.url()));
    const nameInput = page.locator(`#capture-row-${finishCaptureId} input[name="name"]`);
    await nameInput.fill('');
    for (const ch of 'Reading') {
      await nameInput.type(ch, { delay: 30 });
    }
    const hoursInput = page.locator(`#capture-row-${finishCaptureId} input[name="hours"]`);
    await hoursInput.fill('2');
    await page.waitForTimeout(200);
    if (requests.length > 0) {
      fail(`[as-you-type] expected zero network requests while typing, got ${requests.length}: ${requests.join(', ')}`);
    }

    // --- Step 3: submit. Read the Quota screen: called Reading, not
    // "finish chapter 3". ---
    await Promise.all([
      page.waitForResponse((resp) => resp.request().method() === 'POST' && resp.url().includes('/triage')),
      page.locator(`#capture-row-${finishCaptureId} .fields-panel button[type="submit"]`).click(),
    ]);
    await page.goto(`${baseUrl}/quota`, { waitUntil: 'load' });
    let readout = await page.evaluate(quotaRowReadout, 'Reading');
    if (readout !== '0m / 2h') {
      fail(`[submit] expected a "Reading" quota reading "0m / 2h", got: ${JSON.stringify(readout)}`);
    }
    const stillFinishChapter3 = await page.evaluate(quotaRowReadout, 'finish chapter 3');
    if (stillFinishChapter3 !== null) {
      fail('[submit] expected no quota named "finish chapter 3" -- the rename must replace the prefill, not add beside it');
    }

    // --- Step 4: capture workout, tap Quota, change nothing, type 3,
    // submit -- the prefill is a default you can ignore. ---
    await page.goto(`${baseUrl}/`, { waitUntil: 'load' });
    await tapQuota(page, workoutCaptureId);
    const secondPanel = await page.evaluate(readQuotaPanel, workoutCaptureId);
    if (secondPanel.nameValue !== 'workout') {
      fail(`[prefill] expected the second row's name box prefilled with "workout", got: ${JSON.stringify(secondPanel.nameValue)}`);
    }
    await page.locator(`#capture-row-${workoutCaptureId} input[name="hours"]`).fill('3');
    await Promise.all([
      page.waitForResponse((resp) => resp.request().method() === 'POST' && resp.url().includes('/triage')),
      page.locator(`#capture-row-${workoutCaptureId} .fields-panel button[type="submit"]`).click(),
    ]);
    await page.goto(`${baseUrl}/quota`, { waitUntil: 'load' });
    readout = await page.evaluate(quotaRowReadout, 'workout');
    if (readout !== '0m / 3h') {
      fail(`[submit] expected a "workout" quota reading "0m / 3h", got: ${JSON.stringify(readout)}`);
    }

    // --- Reload mid-typing, before submitting: nothing about the
    // half-typed name survives (T-ephemeral-view-state-rides-the-request).
    // A separate, still-untriaged capture, since the two above are now
    // triaged and off the inbox. ---
    await page.goto(`${baseUrl}/`, { waitUntil: 'load' });
    await tapQuota(page, reloadCaptureId);
    const reloadNameInput = page.locator(`#capture-row-${reloadCaptureId} input[name="name"]`);
    await reloadNameInput.fill('');
    for (const ch of 'Should Not Persist') {
      await reloadNameInput.type(ch, { delay: 10 });
    }
    await page.reload({ waitUntil: 'load' });
    const afterReload = await page.evaluate(readQuotaPanel, reloadCaptureId);
    if (!afterReload.open) {
      fail('[reload] expected the quota panel to still be open after reload (shown_kind persists; that is correct) but it was not');
    } else if (afterReload.nameValue === 'Should Not Persist') {
      fail('[reload] the half-typed name survived a reload -- something is storing draft state that should not exist');
    }
  } finally {
    await browser.close();
  }

  if (failures > 0) {
    console.error(`FAIL: quota_triage_validation (${failures} assertion(s) failed)`);
    process.exit(1);
  }
  console.log('PASS: quota_triage_validation (browser assertions)');
}

main().catch((error) => {
  console.error('FAIL: quota_triage_validation (uncaught error)');
  console.error(error);
  process.exit(1);
});
