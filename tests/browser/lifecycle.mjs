// Browser page lifecycle acceptance; the trace observes the real app and does
// not mutate simulation state.
import assert from 'node:assert/strict';
import { snapshot } from './movement.mjs';

export function installBfcacheTrace() {
  if (window.__bfcacheTraceInstalled) return;
  window.__bfcacheTraceInstalled = true;
  window.__bfcacheEvents = [];
  const record = (type, event) => {
    const state = window.__spelunkyTest;
    window.__bfcacheEvents.push({
      type,
      persisted: event?.persisted ?? null,
      hidden: document.hidden,
      tick: state?.tick ?? null,
      sequence: state?.sequence ?? null,
      hostOccluded: state?.hostOccluded ?? null,
    });
  };
  for (const type of ['pagehide', 'pageshow']) {
    window.addEventListener(type, event => record(type, event));
  }
  document.addEventListener('visibilitychange', event => record('visibilitychange', event));
}

async function enterPlaying(page) {
  const start = await snapshot(page);
  assert.equal(start.screen, 'Title');
  assert.equal(start.hostOccluded, false);

  await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing', null, { timeout: 15000 });
  const playing = await snapshot(page);
  await page.waitForFunction(tick => window.__spelunkyTest?.tick >= tick + 3, playing.tick, { timeout: 10000 });
}

async function runResumeFocusRegression({ page, result }) {
  // Simulate a restored document whose DOM focus and Winit's cached canvas
  // focus disagree. Dispatching persisted pageshow exercises the same Winit
  // Resumed path as BFCache without depending on browser cache eligibility.
  await page.locator('#spelunky-canvas').focus();
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === true, null, { timeout: 5000 });
  await page.locator('#attribution a').focus();
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === false, null, { timeout: 5000 });
  const focusBeforeResume = await page.evaluate(() => ({
    documentFocused: document.hasFocus(),
    activeElementIsLicense: document.activeElement?.matches('#attribution a') === true,
  }));
  assert.equal(focusBeforeResume.documentFocused, true);
  assert.equal(focusBeforeResume.activeElementIsLicense, true);
  await page.evaluate(() => {
    // Winit's canvas focus handler updates its cached state even though a
    // synthetic event leaves document.activeElement on the license link.
    document.querySelector('#spelunky-canvas').dispatchEvent(new FocusEvent('focus'));
  });
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === true, null, { timeout: 5000 });
  assert.equal(await page.evaluate(() => document.activeElement?.matches('#attribution a') === true), true);
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })));
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === false, null, { timeout: 5000 });
  assert.equal(await page.evaluate(() => document.hasFocus()), true);
  assert.equal(await page.evaluate(() => document.activeElement?.matches('#attribution a') === true), true);
  await page.locator('#spelunky-canvas').focus();
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === true, null, { timeout: 5000 });
  result.checks.push('persisted pageshow restores focus from the active DOM element, not stale Winit canvas focus');
}

async function runWindowFocusRegression({ page, result }) {
  await page.locator('#spelunky-canvas').focus();
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === true, null, { timeout: 5000 });

  await page.evaluate(() => window.dispatchEvent(new Event('blur')));
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === false, null, { timeout: 5000 });
  const blurred = await snapshot(page);
  await page.waitForTimeout(200);
  const paused = await snapshot(page);
  assert.equal(paused.hostFocused, false);
  assert.equal(paused.tick, blurred.tick, 'A browser window blur must freeze the source simulation');
  assert.equal(paused.renderSequence, blurred.renderSequence,
    'A browser window blur must stop GPU presentation as well as simulation');

  await page.evaluate(() => window.dispatchEvent(new Event('focus')));
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === true, null, { timeout: 5000 });
  await page.waitForFunction(tick => window.__spelunkyTest?.tick > tick, paused.tick, { timeout: 5000 });

  // A browser window focus is insufficient when the user has moved DOM focus
  // to another control; the canvas must own focus before gameplay resumes.
  await page.locator('#attribution a').focus();
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === false, null, { timeout: 5000 });
  const linkFocused = await snapshot(page);
  await page.evaluate(() => window.dispatchEvent(new Event('focus')));
  await page.waitForTimeout(200);
  const stillPaused = await snapshot(page);
  assert.equal(stillPaused.hostFocused, false);
  assert.equal(stillPaused.tick, linkFocused.tick, 'Window focus must not resume while a link owns DOM focus');
  assert.equal(stillPaused.renderSequence, linkFocused.renderSequence,
    'Moving DOM focus to a link must stop browser rendering');

  await page.locator('#spelunky-canvas').focus();
  await page.waitForFunction(() => window.__spelunkyTest?.hostFocused === true, null, { timeout: 5000 });
  await page.waitForFunction(tick => window.__spelunkyTest?.tick > tick, linkFocused.tick, { timeout: 5000 });
  await page.waitForFunction(sequence => window.__spelunkyTest?.renderSequence > sequence,
    stillPaused.renderSequence, { timeout: 5000 });
  result.checks.push('browser window blur freezes simulation and focus resumes only with the canvas active');
}

export async function runResumeFocusAcceptance({ page, result }) {
  await enterPlaying(page);
  await runResumeFocusRegression({ page, result });
  await runWindowFocusRegression({ page, result });
}

export async function runBfcacheAcceptance({ page, base, result }) {
  await enterPlaying(page);
  await runResumeFocusRegression({ page, result });

  await page.evaluate(installBfcacheTrace);
  await page.addInitScript(installBfcacheTrace);

  await page.goto(new URL('__bfcache-away', base).href, { waitUntil: 'domcontentloaded' });
  assert.match(await page.locator('body').textContent(), /Navigation target/);
  await page.waitForTimeout(1100);
  await page.goBack({ waitUntil: 'commit' });
  await page.waitForFunction(
    () => window.__bfcacheEvents?.some(event => event.type === 'pageshow' && event.persisted),
    null,
    { timeout: 15000 },
  );

  const lifecycle = await page.evaluate(() => window.__bfcacheEvents);
  const pagehide = lifecycle.find(event => event.type === 'pagehide' && event.persisted);
  const pageshow = lifecycle.filter(event => event.type === 'pageshow' && event.persisted).at(-1);
  assert.ok(pagehide, 'The app document must enter the browser back-forward cache');
  assert.ok(pageshow, 'The app document must be restored from the browser back-forward cache');
  assert.ok(Number.isFinite(pagehide.tick), 'The app must be ticking before navigation');

  await page.waitForFunction(
    sequence => window.__movementTrace?.some(frame => frame.sequence > sequence),
    pagehide.sequence,
    { timeout: 10000 },
  );
  const firstResumedFrame = await page.evaluate(sequence =>
    window.__movementTrace.find(frame => frame.sequence > sequence), pagehide.sequence);
  assert.equal(firstResumedFrame.hostOccluded, false, 'A visible restored document must clear stale occlusion');
  assert.ok(
    firstResumedFrame.tick - pagehide.tick <= 2,
    `Restoration must reset fixed-step timing instead of catching up (${pagehide.tick} -> ${firstResumedFrame.tick})`,
  );
  await page.waitForFunction(tick => window.__spelunkyTest?.tick > tick, pagehide.tick, { timeout: 10000 });
  result.checks.push('BFCache restore clears occlusion, resumes simulation, and discards hidden-tab elapsed time');
  result.lifecycle = lifecycle;
}
