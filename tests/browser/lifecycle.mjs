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

export async function runBfcacheAcceptance({ page, base, result }) {
  const start = await snapshot(page);
  assert.equal(start.screen, 'Title');
  assert.equal(start.hostOccluded, false);

  await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing', null, { timeout: 15000 });
  const playing = await snapshot(page);
  await page.waitForFunction(tick => window.__spelunkyTest?.tick >= tick + 3, playing.tick, { timeout: 10000 });
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
