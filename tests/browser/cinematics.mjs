// Source-authored cinematic routes reached through UI and real keyboard input.
import assert from 'node:assert/strict';
import { snapshot, state, press, restoreDefaultSettings } from './movement.mjs';
import { importDocument, startCustom, walkThroughNextDoor, continueTransition, level } from './adversarial.mjs';

export async function runCinematicAcceptance({ page, result, screenshot }) {
  await restoreDefaultSettings(page);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await state(page, 'Title');
  await page.locator('canvas').focus();
  await page.keyboard.press('F2');
  await state(page, 'Editor');
  const name = '000-browser-final-cinematic';
  await importDocument(page, name, level({ health: 9, next: 'NONE' }));
  result.cinematics = [];
  const custom = { name: 'custom-final-summary', status: 'failed' };
  result.cinematics.push(custom);
  await startCustom(page, name);
  custom.start = await snapshot(page);
  await walkThroughNextDoor(page);
  custom.transition = await continueTransition({ page, result, screenshot, name: 'custom-final' });
  await page.waitForFunction(() => window.__spelunkyTest?.state === 'Ending'
    && window.__spelunkyTest.ending?.kind === 'Custom'
    && window.__spelunkyTest.ending.stage === 'Summary', null, { timeout: 15000 });
  custom.summary = await snapshot(page);
  assert.equal(custom.summary.screen, 'Playing');
  assert.equal(custom.summary.world.name, 'rEndCustom');
  assert.equal(custom.summary.ending.recordReady, false);
  const frozenTime = custom.summary.ending.sourceTimeMs;
  await page.waitForFunction(() => window.__spelunkyTest.ending?.drawStatus === 7,
    null, { timeout: 30000 });
  custom.counted = await snapshot(page);
  assert.equal(custom.counted.ending.bonusAwarded, true);
  assert.equal(custom.counted.money, custom.start.money + 50000);
  assert.equal(custom.counted.ending.moneyCount, custom.counted.money);
  assert.equal(custom.counted.ending.sourceTimeMs, frozenTime, 'Summary must freeze the source run clock');
  assert.equal(custom.counted.ending.recordReady, false, 'A custom ending must not become a campaign record');
  await screenshot(page, 'custom-ending-summary', result);
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.__spelunkyTest.ending?.drawStatus === 8,
    null, { timeout: 10000 });
  custom.faded = await snapshot(page);
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.__spelunkyTest?.state === 'Playing'
    && window.__spelunkyTest.world.name === 'rHighscores', null, { timeout: 15000 });
  custom.scores = await snapshot(page);
  await screenshot(page, 'custom-ending-scores', result);
  custom.status = 'passed';

  await page.reload({ waitUntil: 'domcontentloaded' });
  await state(page, 'Title');
  await page.locator('canvas').focus();
  await press(page, 'ArrowDown', 2);
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.__spelunkyTest?.world.name === 'rTitle', null, { timeout: 10000 });
  const credits = { name: 'authored-rope-title-credits', status: 'failed', hub: await snapshot(page) };
  result.cinematics.push(credits);
  assert.equal(credits.hub.player.x, 600);
  await page.keyboard.down('ArrowUp');
  await page.keyboard.down('z');
  await page.waitForFunction(() => window.__spelunkyTest?.player.state === 'Climbing',
    null, { timeout: 10000 });
  credits.grab = await snapshot(page);
  await page.keyboard.up('z');
  await page.waitForFunction(() => window.__spelunkyTest?.state === 'Ending'
    && window.__spelunkyTest.ending?.kind === 'TitleCredits', null, { timeout: 15000 });
  await page.keyboard.up('ArrowUp');
  credits.exit = await snapshot(page);
  await page.waitForFunction(() => window.__spelunkyTest.ending?.stage === 'Credits'
    && window.__spelunkyTest.world.name === 'rCredits1'
    && !window.__spelunkyTest.ending.fadeIn, null, { timeout: 15000 });
  credits.visible = await snapshot(page);
  assert.equal(credits.visible.screen, 'Playing');
  assert.equal(credits.visible.state, 'Ending');
  assert.equal(credits.visible.ending.recordReady, false);
  await screenshot(page, 'title-rope-credits', result);
  await page.keyboard.down('x'); // Actual bound attack: source Credits accepts held attack.
  try { await state(page, 'Title'); } finally { await page.keyboard.up('x'); }
  credits.returned = await snapshot(page);
  assert.equal(credits.returned.player, null);
  await screenshot(page, 'title-after-rope-credits', result);
  credits.status = 'passed';
  result.checks.push('Linked custom source transitions, final custom bonus summary/Scores, and authored Title rope→Credits1→browser Title');
}
