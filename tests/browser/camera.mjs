// Live camera-follow regression through imported terrain and ordinary input.
import assert from 'node:assert/strict';
import { snapshot, state, restoreDefaultSettings, returnToEditor } from './movement.mjs';
import { importDocument } from './adversarial.mjs';

function fixture() {
  const rows = Array.from({ length: 32 }, () => Array(40).fill('0'));
  rows[9].fill('1');
  rows[8][4] = '@';
  rows[8][35] = 'X';
  return [...rows.map(row => row.join('')), 'BROWSER CAMERA FOLLOW', 'CAVE', '9', '0', '0', 'NONE', '0', '0'].join('\n');
}

function close(actual, expected, message) {
  assert.ok(Number.isFinite(actual) && Math.abs(actual - expected) < .0001,
    `${message}: actual=${actual}, expected=${expected}`);
}

// This authored corridor follows the default Player with source Hborder128,
// canonical width320 and no room clamp. Native follow rounds each finite
// actor endpoint nearest-even before applying that border. These positions
// stay within signed32, so the native low32 conversion cannot wrap here.
function sourceNearestEven(value) {
  assert.ok(Number.isFinite(value), 'Finite source follow endpoint');
  const lower = Math.floor(value), fraction = value - lower;
  return fraction < .5 ? lower : fraction > .5 ? lower + 1
    : lower % 2 === 0 ? lower : lower + 1;
}

export async function runCameraAcceptance({ page, result, screenshot }) {
  const entry = result.cameraFollow = { status: 'failed',
    scope: 'Actual redraw telemetry shares production presentation endpoints. Ordinary imported corridor and Right input; independent interpolation/anchor/endpoint continuity checks, preserving frozen source Draw sprite/frame. Observed software-browser alpha samples do not certify physical240Hz.' };
  await restoreDefaultSettings(page);
  await page.setViewportSize({ width: 320, height: 240 });
  await page.keyboard.press('F2');
  await state(page, 'Editor');
  const name = 'camera-follow-corridor';
  await importDocument(page, name, fixture());
  await page.keyboard.press('F5');
  await page.waitForFunction(name => window.__spelunkyTest?.world?.name === name
    && window.__spelunkyTest.screen === 'Playing' && window.__spelunkyTest.player.grounded,
  name, { timeout: 15000 });
  entry.start = await snapshot(page);
  close(entry.start.player.x, 88, 'Authored camera fixture entrance');
  await page.keyboard.down('ArrowRight');
  try {
    await page.waitForFunction(() => window.__spelunkyTest.player.x >= 380
      && window.__spelunkyTest.camera.authoritativeX >= 120, null, { timeout: 15000 });
    await screenshot(page, 'camera-follow-moving', result);
  } finally { await page.keyboard.up('ArrowRight'); }
  entry.end = await snapshot(page);
  entry.samples = await page.evaluate(({ sequence, name }) => window.__movementTrace.filter(value =>
    value.sequence >= sequence && value.world?.name === name && value.screen === 'Playing'
    && value.state === 'Playing' && value.camera.previousX > 0
    && value.camera.x > value.camera.previousX), { sequence: entry.start.sequence, name });
  assert.ok(entry.samples.length > 0, 'Actual camera-follow redraws must be observed');
  entry.fractionalSamples = 0;
  entry.sameTickInterpolatedPairs = 0;
  entry.adjacentTickBoundaries = 0;
  let previous;
  for (const value of entry.samples) {
    const { camera, rendered, alpha } = value;
    assert.ok(alpha >= 0 && alpha <= 1);
    close(camera.x, camera.authoritativeX, 'Draw camera must include this tick’s authoritative camera update');
    close(camera.y, camera.authoritativeY, 'Draw vertical camera endpoint');
    close(camera.renderedX, camera.previousX + (camera.x - camera.previousX) * alpha, 'Camera alpha interpolation');
    close(camera.renderedY, camera.previousY + (camera.y - camera.previousY) * alpha, 'Vertical camera alpha interpolation');
    close(rendered.x, rendered.previousX + (rendered.currentX - rendered.previousX) * alpha, 'Frozen Draw player interpolation');
    close(rendered.y, rendered.previousY + (rendered.currentY - rendered.previousY) * alpha, 'Frozen Draw vertical player interpolation');
    close(rendered.screenX, rendered.x - camera.renderedX, 'Actual presentation screen anchor');
    close(rendered.screenY, rendered.y - camera.renderedY, 'Actual vertical presentation anchor');
    const previousFollowX = sourceNearestEven(rendered.previousX) - 192;
    const currentFollowX = sourceNearestEven(rendered.currentX) - 192;
    close(camera.previousX, previousFollowX, 'Previous source integer follow endpoint');
    close(camera.x, currentFollowX, 'Current source integer follow endpoint');
    close(rendered.screenX,
      192 + (rendered.previousX - sourceNearestEven(rendered.previousX)) * (1 - alpha)
        + (rendered.currentX - sourceNearestEven(rendered.currentX)) * alpha,
      'Source nearest-even camera border interpolates both endpoint errors');
    assert.ok(typeof rendered.sprite === 'string' && Number.isFinite(rendered.frame));
    if (alpha > .001 && alpha < .999) {
      entry.fractionalSamples++;
      assert.ok(camera.renderedX > camera.previousX && camera.renderedX < camera.x,
        'A fractional redraw must show a fractional camera position');
    }
    if (previous?.tick === value.tick) {
      for (const key of ['previousX', 'previousY', 'currentX', 'currentY', 'sprite', 'frame']) {
        assert.equal(rendered[key], previous.rendered[key], 'Redrawing must preserve the frozen pre-Animation actor');
      }
      close(camera.x, previous.camera.x, 'Same-tick camera endpoint must remain frozen');
      if (alpha > previous.alpha + .001) {
        assert.ok(camera.renderedX > previous.camera.renderedX, 'Camera must move between fractional redraws');
        entry.sameTickInterpolatedPairs++;
      }
    } else if (previous && value.tick === previous.tick + 1) {
      close(camera.previousX, previous.camera.x, 'Consecutive camera tick endpoints must join');
      close(rendered.previousX, previous.rendered.currentX, 'Consecutive frozen player endpoints must join');
      entry.adjacentTickBoundaries++;
    }
    previous = value;
  }
  assert.ok(entry.fractionalSamples > 0, 'Must observe actual fractional camera redraws');
  assert.ok(new Set(entry.samples.map(value => value.tick)).size >= 2, 'Follow must span multiple actual source ticks');
  assert.equal(entry.end.player.health, entry.start.player.health);
  await returnToEditor(page);
  entry.status = 'passed';
  result.checks.push('Live camera follows authoritative same-tick endpoints with fractional interpolation and frozen pre-Animation actor poses');
}
