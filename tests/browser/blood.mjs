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
  rows[8][8] = '1'; // Ordinary terrain prevents a fleeing snake escaping the bounded approach.
  rows[8][20] = 'X';
  return [...rows.map(row => row.join('')), 'BROWSER NATURAL SNAKE BLOOD', 'CAVE', '9', '0', '0', 'NONE', '0', '0'].join('\n');
}

// As with the lighting route, these untrusted DOM events enter the normal
// canvas/winit listener. Only observed telemetry controls their timing.
function installBloodRoute(baseline) {
  const route = window.__bloodRoute = { edges: [], frames: [], done: false, error: null };
  const canvas = document.getElementById('spelunky-canvas');
  const old = new Set(baseline), keys = new Set();
  const setKeys = desired => {
    const next = new Set(desired);
    for (const code of new Set([...keys, ...next])) {
      if (keys.has(code) === next.has(code)) continue;
      const down = next.has(code);
      canvas.dispatchEvent(new KeyboardEvent(down ? 'keydown' : 'keyup', {
        code, key: code === 'KeyX' ? 'x' : code, bubbles: true,
      }));
      route.edges.push({ tick: window.__spelunkyTest.tick, code, down });
    }
    keys.clear();
    for (const key of next) keys.add(key);
  };
  route.stop = () => { route.done = true; setKeys([]); };
  let startTick, previousTick;
  const update = () => {
    if (route.done) return;
    const s = window.__spelunkyTest;
    if (s.screen === 'Playing') {
      startTick ??= s.tick;
      if (previousTick !== s.tick) {
        route.frames.push({ tick: s.tick, x: s.player.x, health: s.player.health });
        previousTick = s.tick;
      }
      const ticks = new Set(window.__bloodProbe.frames.filter(frame =>
        frame.red.filter(pixel => !old.has(pixel)).length >= 8).map(frame => frame.tick));
      if (ticks.size >= 3 || window.__bloodProbe.error) route.stop();
      else if (s.player.health !== 9 || s.tick - startTick > 180) {
        route.error = s.player.health !== 9 ? 'Player damaged during snake approach' : 'Bounded whip route exhausted';
        route.stop();
      } else setKeys([
        ...(s.player.x < 92 ? ['ArrowRight'] : []),
        ...((s.tick - startTick) % 18 < 9 ? ['KeyX'] : []),
      ]);
    }
    if (!route.done) requestAnimationFrame(update);
  };
  requestAnimationFrame(update);
}

export async function runBloodAcceptance({ page, result, screenshot }) {
  const entry = result.blood = { status: 'failed',
    scope: 'Imported snake with ordinary containment wall; real pause/resume and telemetry-timed untrusted DOM Right/X events through normal input, bounded approach and repeated fresh whip presses. Actual WebGL source-red pixel emergence outside HUD. Core regressions separately prove lethal snake/death-burst semantics; pixels do not independently expose enemy health.' };
  await page.setViewportSize({ width: 320, height: 240 });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await reachTitleMenu(page);
  await page.keyboard.press('F2');
  await state(page, 'Editor');
  await importDocument(page, 'natural-snake-blood', fixture());
  await page.keyboard.press('F5');
  await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing' && window.__spelunkyTest.player.grounded,
    null, { timeout: 15000 });
  await page.keyboard.press('Escape');
  await state(page, 'Pause');
  entry.start = await snapshot(page);
  assert.equal(entry.start.player.x, 88);
  assert.equal(entry.start.player.y, 152);
  assert.equal(entry.start.player.health, 9);
  try {
    await page.evaluate(() => { window.__bloodProbe.armed = true; });
    await page.waitForFunction(() => window.__bloodProbe.frames.length >= 3 || window.__bloodProbe.error,
      null, { timeout: 10000 });
    entry.baseline = await page.evaluate(() => [...new Set(window.__bloodProbe.frames.flatMap(frame => frame.red))]);
    await screenshot(page, 'snake-before-whip', result);
    await page.evaluate(() => { window.__bloodProbe.frames = []; });
    await page.evaluate(installBloodRoute, entry.baseline);
    await page.keyboard.press('Enter');
    await page.waitForFunction(() => window.__bloodRoute.done, null, { timeout: 15000 });
    const routeError = await page.evaluate(() => window.__bloodRoute.error);
    assert.equal(routeError, null);
    await screenshot(page, 'snake-whip-blood-visible', result);
    await page.keyboard.press('Escape');
    await state(page, 'Pause');
    entry.visible = await snapshot(page);
    assert.equal(await page.evaluate(() => window.__bloodProbe.error), null);
    entry.frames = await page.evaluate(() => window.__bloodProbe.frames);
    const baseline = new Set(entry.baseline);
    entry.newRedByTick = Object.fromEntries(entry.frames.map(frame => [frame.tick, frame.red.filter(pixel => !baseline.has(pixel)).length])
      .reduce((map, [tick, count]) => map.set(tick, Math.max(count, map.get(tick) || 0)), new Map()));
    assert.ok(Object.values(entry.newRedByTick).filter(count => count >= 8).length >= 3,
      'Source blood palette must emerge in production pixels across at least three real ticks');
    assert.equal(entry.visible.player.health, entry.start.player.health, 'Player damage must not explain these blood pixels');
    await screenshot(page, 'snake-whip-blood-paused', result);
    entry.status = 'passed';
    result.checks.push('Natural snake-whip blood: new source-red production WebGL pixels over multiple ticks, without player damage');
  } finally {
    entry.route = await page.evaluate(() => {
      window.__bloodRoute?.stop();
      return window.__bloodRoute ? { edges: window.__bloodRoute.edges, frames: window.__bloodRoute.frames,
        done: window.__bloodRoute.done, error: window.__bloodRoute.error } : null;
    });
    entry.frames = await page.evaluate(() => { window.__bloodProbe.armed = false; return window.__bloodProbe.frames; });
    entry.final = await snapshot(page);
  }
}
