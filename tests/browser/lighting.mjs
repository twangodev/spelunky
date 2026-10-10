// Real UI-generated dark rooms and readback from the actual production WebGL pass.
import assert from 'node:assert/strict';
import { snapshot, state, press } from './movement.mjs';

export function installLightingProfile() {
  // Controlled persisted unlock, not injected room, equipment, darkness, or seed.
  localStorage.setItem('spelunky-rust.save', JSON.stringify({ version: 1, unlocks: ['shortcut_5'] }));
}

export function installLightingProbe() {
  const probe = window.__lightingProbe = { armed: false, capture: null, error: null };
  function read(gl) {
    const draw = gl.getParameter(gl.DRAW_FRAMEBUFFER_BINDING);
    const previous = gl.getParameter(gl.READ_FRAMEBUFFER_BINDING);
    const packBuffer = gl.getParameter(gl.PIXEL_PACK_BUFFER_BINDING);
    const pixels = new Uint8Array(gl.drawingBufferWidth * gl.drawingBufferHeight * 4);
    try {
      if (draw !== previous) gl.bindFramebuffer(gl.READ_FRAMEBUFFER, draw);
      if (packBuffer) gl.bindBuffer(gl.PIXEL_PACK_BUFFER, null);
      gl.readPixels(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
      const error = gl.getError();
      if (error !== gl.NO_ERROR) throw new Error(`Production framebuffer readback GL error ${error}`);
    } finally {
      if (packBuffer) gl.bindBuffer(gl.PIXEL_PACK_BUFFER, packBuffer);
      if (draw !== previous) gl.bindFramebuffer(gl.READ_FRAMEBUFFER, previous);
    }
    return pixels;
  }
  for (const name of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced']) {
    const original = WebGL2RenderingContext.prototype[name];
    WebGL2RenderingContext.prototype[name] = function (...args) {
      if (!probe.armed || this.canvas.id !== 'spelunky-canvas') return original.apply(this, args);
      const multiply = this.isEnabled(this.BLEND)
        && this.getParameter(this.BLEND_SRC_RGB) === this.DST_COLOR
        && this.getParameter(this.BLEND_DST_RGB) === this.ZERO;
      try {
        if (multiply && !probe.capture) {
          probe.capture = {
            width: this.drawingBufferWidth, height: this.drawingBufferHeight,
            viewport: Array.from(this.getParameter(this.VIEWPORT)),
            samples: this.getParameter(this.SAMPLES),
            offscreenFramebuffer: this.getParameter(this.DRAW_FRAMEBUFFER_BINDING) !== null,
            snapshot: structuredClone(window.__spelunkyTest), before: read(this), multiplyDraws: 0,
          };
        }
      } catch (error) { probe.error = String(error); probe.armed = false; }
      const value = original.apply(this, args);
      try {
        if (probe.armed && multiply && probe.capture) {
          probe.capture.after = read(this);
          probe.capture.multiplyDraws++;
        } else if (probe.armed && probe.capture?.after) {
          probe.capture.final = read(this);
          probe.capture.complete = true;
          probe.armed = false;
        }
      } catch (error) { probe.error = String(error); probe.armed = false; }
      return value;
    };
  }
}

function analyzeCapture() {
  const c = window.__lightingProbe.capture;
  const s = c.snapshot;
  const even = x => {
    const floor = Math.floor(x), fraction = x - floor;
    return fraction < .5 ? floor : fraction > .5 ? floor + 1 : floor + (floor % 2 !== 0 ? 1 : 0);
  };
  // Independent source make_color_rgb oracle: nearest-even arguments, additive
  // packing (including carry/overflow), then low 24-bit channel extraction.
  let channels = [255 - 255 * s.darkness, 255 - 255 * s.darkness, 255];
  for (const distance of s.lighting.red_lamp_distances) {
    if (distance !== null && distance <= 96) channels = [255 - distance, 24 + distance, 24 + distance];
  }
  const packed = even(channels[0]) + 256 * even(channels[1]) + 65536 * even(channels[2]);
  const tint = [packed & 255, (packed >>> 8) & 255, (packed >>> 16) & 255];
  const counts = { outside: 0, single: 0, overlap: 0, singleVisible: 0, overlapVisible: 0, hudRestored: 0 };
  let maxError = 0;
  const mismatches = [], examples = {};
  const indexFor = (x, y, rowZeroIsTop) => {
    const px = Math.floor((x + .5) * c.width / 320);
    const py = Math.floor((y + .5) * c.height / 240);
    return (((rowZeroIsTop ? py : c.height - 1 - py)) * c.width + px) * 4;
  };
  // wgpu may render into an intermediate FBO which its final presentation
  // flips. Establish readback orientation independently using the red HUD
  // heart at its known top-left coordinates, rather than choosing whichever
  // orientation happens to fit the darkness oracle.
  const orientationEvidence = [true, false].map(rowZeroIsTop => {
    let heartPixels = 0;
    for (let y = 10; y < 21; y++) for (let x = 10; x < 21; x++) {
      const i = indexFor(x, y, rowZeroIsTop);
      const [r, g, b] = c.final.slice(i, i + 3);
      if (r >= 160 && r > g * 1.6 && r > b * 1.6 && r - c.after[i] > 30) heartPixels++;
    }
    return { rowZeroIsTop, heartPixels };
  }).sort((a, b) => b.heartPixels - a.heartPixels);
  const rowZeroIsTop = orientationEvidence[0].rowZeroIsTop;
  const index = (x, y) => indexFor(x, y, rowZeroIsTop);
  for (let y = 0; y < 240; y++) for (let x = 0; x < 320; x++) {
    const wx = x + .5 + s.camera.renderedX, wy = y + .5 + s.camera.renderedY;
    const coverage = s.lighting.circles.filter(light => (wx - light.x) ** 2 + (wy - light.y) ** 2 <= light.radius ** 2).length;
    const kind = coverage === 0 ? 'outside' : coverage === 1 ? 'single' : 'overlap';
    counts[kind]++;
    const i = index(x, y), before = Array.from(c.before.slice(i, i + 3)), after = Array.from(c.after.slice(i, i + 3));
    const expected = before.map((value, channel) => coverage ? Math.round(value * tint[channel] / 255) : 0);
    const error = Math.max(...after.map((value, channel) => Math.abs(value - expected[channel])));
    maxError = Math.max(maxError, error);
    if (coverage && Math.max(...before) > 32) {
      counts[`${kind}Visible`]++;
      examples[kind] ??= { x, y, before, after, expected };
    }
    if (!coverage) examples.outside ??= { x, y, before, after, expected };
    if (error > 1 && mismatches.length < 12) mismatches.push({ x, y, coverage, before, after, expected });
    if (y >= 8 && y < 24 && x >= 8 && x < 210) {
      const final = Array.from(c.final.slice(i, i + 3));
      if (Math.max(...final) >= 200 && final.some((value, channel) => value - after[channel] > 30)) counts.hudRestored++;
    }
  }
  return { snapshot: s, width: c.width, height: c.height, viewport: c.viewport, samples: c.samples,
    offscreenFramebuffer: c.offscreenFramebuffer, orientationEvidence,
    readbackOrientation: rowZeroIsTop ? 'row-zero-is-logical-top' : 'row-zero-is-logical-bottom',
    multiplyDraws: c.multiplyDraws, sourceTintRgb: tint, publishedTintRgb: s.lighting.final_tint_rgb,
    logicalSamples: 320 * 240, counts, maxError, mismatches, examples };
}

async function restart(page) {
  await page.reload({ waitUntil: 'domcontentloaded' });
  await state(page, 'Title');
  await page.locator('canvas').focus();
}

export async function runLightingAcceptance({ page, result, screenshot }) {
  const entry = result.lighting = {
    status: 'failed', attempts: [],
    scope: 'Valid saved shortcut unlock; naturally random level5 darkness; actual production WebGL byte-multiply readback. Published circles are pipeline inputs, not an independent emitter-catalog oracle.',
  };
  for (let attempt = 0; attempt < 96; attempt++) {
    if (attempt) await restart(page);
    await press(page, 'ArrowDown', 5);
    await page.keyboard.press('Enter');
    await state(page, 'Shortcuts');
    await page.keyboard.press('Enter');
    await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing' && window.__spelunkyTest.world.level === 5
      && window.__spelunkyTest.levelTick >= 2, null, { timeout: 15000 });
    const initial = await snapshot(page);
    const candidate = { attempt: attempt + 1, seed: initial.seed, dark: initial.world.dark };
    entry.attempts.push(candidate);
    if (!initial.world.dark) continue;
    for (const direction of ['ArrowRight', 'ArrowLeft', 'ArrowRight']) {
      let current = await snapshot(page);
      if (current.darkness >= .12 || current.state !== 'Playing' || current.player.health <= 0) break;
      await page.keyboard.down(direction);
      await page.waitForFunction(tick => {
        const v = window.__spelunkyTest;
        return v.tick >= tick + 24 || v.darkness >= .12 || v.state !== 'Playing' || v.player.health <= 0;
      }, current.tick, { timeout: 10000 });
      await page.keyboard.up(direction);
    }
    let current = await snapshot(page);
    if (!(current.darkness >= .12 && current.state === 'Playing' && current.player.health > 0)) {
      candidate.rejected = 'No live measurable tint reached through bounded ordinary walking';
      continue;
    }
    await page.keyboard.press('Escape');
    await state(page, 'Pause');
    current = await snapshot(page);
    if (!(current.lighting?.active && current.darkness >= .1 && current.player.health > 0)) {
      candidate.rejected = 'Dark/live/tint prerequisite changed before pause';
      continue;
    }
    entry.selected = candidate;
    await page.evaluate(() => { window.__lightingProbe.capture = null; window.__lightingProbe.error = null; window.__lightingProbe.armed = true; });
    await page.waitForFunction(() => window.__lightingProbe.capture?.complete || window.__lightingProbe.error, null, { timeout: 10000 });
    assert.equal(await page.evaluate(() => window.__lightingProbe.error), null);
    entry.pixels = await page.evaluate(analyzeCapture);
    assert.ok(entry.pixels.orientationEvidence[0].heartPixels > 5
      && entry.pixels.orientationEvidence[0].heartPixels > entry.pixels.orientationEvidence[1].heartPixels,
    'Actual HUD heart must independently establish framebuffer readback orientation');
    assert.deepEqual(entry.pixels.publishedTintRgb, entry.pixels.sourceTintRgb, 'Published tint must match independent source packed-color oracle');
    assert.ok(entry.pixels.multiplyDraws > 0, 'Capture must observe the actual multiply pipeline');
    assert.ok(entry.pixels.counts.outside > 100 && entry.pixels.counts.singleVisible > 20 && entry.pixels.counts.overlapVisible > 20,
      'Natural dark room must expose exterior plus visible single/overlapping circles');
    assert.ok(entry.pixels.maxError <= 1, `Actual production byte multiplication differs: ${JSON.stringify(entry.pixels.mismatches)}`);
    assert.ok(entry.pixels.counts.hudRestored > 5, 'HUD must draw bright pixels after the world darkness mask');
    await screenshot(page, 'dark-room-paused', result);
    await page.keyboard.press('Escape');
    await state(page, 'Playing');
    await screenshot(page, 'dark-room-playing', result);
    entry.status = 'passed';
    result.checks.push('Natural shortcut dark room; actual WebGL RGB-byte multiplication, hard circle union/overlap, black exterior, and HUD bypass');
    return;
  }
  assert.fail('No live measurable natural dark room found in 96 legitimate shortcut starts');
}
