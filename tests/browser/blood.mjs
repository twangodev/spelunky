// Actual snake-whip input and production framebuffer pixels, not injected effects.
import assert from 'node:assert/strict';
import { snapshot, state, reachTitleMenu } from './movement.mjs';
import { importDocument } from './adversarial.mjs';

export function installBloodProbe() {
  const probe = window.__bloodProbe = { armed: false, frames: [], error: null };
  const palette = new Set(['64,0,0', '128,0,0', '192,0,0', '255,64,64']);
  for (const name of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced']) {
    const original = WebGL2RenderingContext.prototype[name];
    WebGL2RenderingContext.prototype[name] = function (...args) {
      const result = original.apply(this, args);
      if (!probe.armed || this.canvas.id !== 'spelunky-canvas') return result;
      const draw = this.getParameter(this.DRAW_FRAMEBUFFER_BINDING);
      const read = this.getParameter(this.READ_FRAMEBUFFER_BINDING);
      const pack = this.getParameter(this.PIXEL_PACK_BUFFER_BINDING);
      try {
        if (read !== draw) this.bindFramebuffer(this.READ_FRAMEBUFFER, draw);
        if (pack) this.bindBuffer(this.PIXEL_PACK_BUFFER, null);
        const width = this.drawingBufferWidth, height = this.drawingBufferHeight;
        const pixels = new Uint8Array(width * height * 4);
        this.readPixels(0, 0, width, height, this.RGBA, this.UNSIGNED_BYTE, pixels);
        const error = this.getError();
        if (error !== this.NO_ERROR) throw new Error(`Blood framebuffer GL error ${error}`);
        const red = [];
        // Exclude both possible HUD edges independently of framebuffer flip.
        // The imported room has only neutral terrain, a green snake and hero.
        for (let y = 40; y < 200; y++) for (let x = 24; x < 240; x++) {
          const px = Math.floor((x + .5) * width / 320), py = Math.floor((y + .5) * height / 240);
          const i = (py * width + px) * 4;
          if (palette.has(`${pixels[i]},${pixels[i + 1]},${pixels[i + 2]}`)) red.push(y * 320 + x);
        }
        probe.frames.push({ tick: window.__spelunkyTest?.tick, red });
        if (probe.frames.length > 500) probe.frames.shift();
      } catch (error) { probe.error = String(error); probe.armed = false; }
      finally {
        if (pack) this.bindBuffer(this.PIXEL_PACK_BUFFER, pack);
        if (read !== draw) this.bindFramebuffer(this.READ_FRAMEBUFFER, read);
      }
      return result;
    };
  }
}

function fixture() {
  const rows = Array.from({ length: 32 }, () => Array(40).fill('0'));
  rows[9].fill('1');
  rows[8][4] = '@';
  rows[8][6] = 'n';
  rows[8][20] = 'X';
  return [...rows.map(row => row.join('')), 'BROWSER NATURAL SNAKE BLOOD', 'CAVE', '9', '0', '0', 'NONE', '0', '0'].join('\n');
}

export async function runBloodAcceptance({ page, result, screenshot }) {
  const entry = result.blood = { status: 'failed',
    scope: 'Imported snake fixture, ordinary Right+X whip, actual WebGL source-red pixel emergence outside HUD. Core regressions separately prove lethal snake/death-burst semantics; pixels do not independently expose enemy health.' };
  await page.setViewportSize({ width: 320, height: 240 });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await reachTitleMenu(page);
  await page.keyboard.press('F2');
  await state(page, 'Editor');
  await importDocument(page, 'natural-snake-blood', fixture());
  await page.keyboard.press('F5');
  await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing' && window.__spelunkyTest.player.grounded,
    null, { timeout: 15000 });
  entry.start = await snapshot(page);
  assert.equal(entry.start.player.x, 88);
  assert.equal(entry.start.player.y, 152);
  await page.evaluate(() => { window.__bloodProbe.armed = true; });
  await page.waitForFunction(() => window.__bloodProbe.frames.length >= 3 || window.__bloodProbe.error,
    null, { timeout: 10000 });
  entry.baseline = await page.evaluate(() => [...new Set(window.__bloodProbe.frames.flatMap(frame => frame.red))]);
  await screenshot(page, 'snake-before-whip', result);
  await page.evaluate(() => { window.__bloodProbe.frames = []; });
  await page.keyboard.down('ArrowRight');
  await page.keyboard.press('x');
  await page.waitForFunction(baseline => {
    const old = new Set(baseline);
    const ticks = new Set(window.__bloodProbe.frames.filter(frame => frame.red.filter(pixel => !old.has(pixel)).length >= 8).map(frame => frame.tick));
    return ticks.size >= 3 || window.__bloodProbe.error;
  }, entry.baseline, { timeout: 10000 });
  await page.keyboard.up('ArrowRight');
  await screenshot(page, 'snake-whip-blood-visible', result);
  await page.keyboard.press('Escape');
  await state(page, 'Pause');
  entry.visible = await snapshot(page);
  entry.frames = await page.evaluate(() => window.__bloodProbe.frames);
  assert.equal(await page.evaluate(() => window.__bloodProbe.error), null);
  const baseline = new Set(entry.baseline);
  entry.newRedByTick = Object.fromEntries(entry.frames.map(frame => [frame.tick, frame.red.filter(pixel => !baseline.has(pixel)).length])
    .reduce((map, [tick, count]) => map.set(tick, Math.max(count, map.get(tick) || 0)), new Map()));
  assert.ok(Object.values(entry.newRedByTick).filter(count => count >= 8).length >= 3,
    'Source blood palette must emerge in production pixels across at least three real ticks');
  assert.equal(entry.visible.player.health, entry.start.player.health, 'Player damage must not explain these blood pixels');
  await screenshot(page, 'snake-whip-blood-paused', result);
  await page.evaluate(() => { window.__bloodProbe.armed = false; });
  entry.status = 'passed';
  result.checks.push('Natural snake-whip blood: new source-red production WebGL pixels over multiple ticks, without player damage');
}
