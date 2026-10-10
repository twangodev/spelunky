// Real UI-generated dark rooms and readback from the actual production WebGL pass.
import assert from 'node:assert/strict';
import { snapshot, state, press } from './movement.mjs';
import { continueTransition } from './adversarial.mjs';

export function installLightingProfile() {
  // Controlled persisted unlock, not injected room, equipment, darkness, or seed.
  localStorage.setItem('spelunky-rust.save', JSON.stringify({ version: 1, unlocks: ['shortcut_5'] }));
}

export function installLightingSeedControl() {
  const actualNow = Performance.prototype.now;
  const actualDateNow = Date.now;
  let lastPerformanceMs;
  const control = window.__lightingSeedControl = { armed: null, observations: [] };
  Performance.prototype.now = function (...args) {
    const value = actualNow.apply(this, args);
    lastPerformanceMs = value;
    return value;
  };
  Date.now = function () {
    if (control.armed === null) return actualDateNow();
    if (!Number.isFinite(lastPerformanceMs)) throw new Error('No real performance clock sample before source seed selection');
    const desiredSeed = BigInt(control.armed);
    // Match the host's exact floating-point expression, keeping its actual
    // performance clock and every simulation/presentation elapsed time intact.
    const performanceMicros = BigInt(Math.trunc((lastPerformanceMs / 1000) * 1000000));
    const encodedMicros = desiredSeed ^ performanceMicros;
    if (encodedMicros > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error('Controlled seed exceeds exact clock encoding range');
    const wallMs = (Number(encodedMicros) + .125) / 1000;
    control.observations.push({ desiredSeed: String(desiredSeed), performanceMs: lastPerformanceMs, wallMs,
      actualWallMs: actualDateNow() });
    control.armed = null;
    return wallMs;
  };
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

async function walkGeneratedStairs(page, targetX, entry) {
  const initial = await snapshot(page), toward = Math.sign(targetX - initial.player.x);
  const direction = toward > 0 ? 'ArrowRight' : 'ArrowLeft';
  let afterTick = initial.tick;
  await page.keyboard.down(direction);
  try {
    for (let jump = 0; jump < 12; jump++) {
      await page.waitForFunction(({ targetX, toward, afterTick }) => {
        const v = window.__spelunkyTest;
        if (v.state !== 'Playing' || v.player.health <= 0 || toward * (v.player.x - targetX) >= -3) return true;
        if (v.player.stun > 0 || !(v.player.grounded || v.player.state === 'Hanging')) return false;
        const older = [...window.__movementTrace].reverse().find(s => s.tick <= v.tick - 6 && s.tick >= afterTick);
        return older && Math.abs(older.player.x - v.player.x) < .5 && Math.abs(older.player.y - v.player.y) < .5;
      }, { targetX, toward, afterTick }, { timeout: 15000 });
      const current = await snapshot(page);
      assert.equal(current.state, 'Playing', 'Generated route must remain alive before the exit');
      assert.ok(current.player.health > 0);
      if (toward * (current.player.x - targetX) >= -3) return current;
      entry.jumps.push(current);
      await page.keyboard.down('KeyZ');
      await page.waitForFunction(tick => window.__spelunkyTest.tick >= tick + 12, current.tick, { timeout: 5000 });
      await page.keyboard.up('KeyZ');
      afterTick = (await snapshot(page)).tick;
    }
    assert.fail('Generated stair route exceeded its bounded real jump attempts');
  } finally {
    await page.keyboard.up('KeyZ');
    await page.keyboard.up(direction);
  }
}

async function enterEligibleDarkRoom(page, entry, result, screenshot) {
  await press(page, 'ArrowDown', 5);
  await page.keyboard.press('Enter');
  await state(page, 'Shortcuts');
  await page.evaluate(seed => { window.__lightingSeedControl.armed = seed; }, entry.seed);
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing' && window.__spelunkyTest.world.level === 5
    && window.__spelunkyTest.levelTick >= 2, null, { timeout: 15000 });
  entry.start = await snapshot(page);
  assert.equal(entry.start.seed, entry.seed, 'One-shot environment clock must select the documented source seed');
  assert.equal(entry.start.world.dark, false, 'Fresh shortcut is correctly suppressed by source noDarkLevel');
  entry.environmentClock = await page.evaluate(() => window.__lightingSeedControl.observations);
  // Source xtime advances30ms per simulation tick. Actual wall/performance
  // clocks remain untouched; waiting21 wall seconds would be insufficient.
  await page.waitForFunction(tick => window.__spelunkyTest.levelTick >= tick, entry.waitTicks, { timeout: 45000 });
  entry.waited = await snapshot(page);
  entry.jumps = [];
  entry.shaft = await walkGeneratedStairs(page, 520, entry);
  await page.waitForFunction(() => {
    const v = window.__spelunkyTest;
    return v.player.grounded && v.player.y >= 496 && v.player.stun === 0;
  }, null, { timeout: 15000 });
  entry.landed = await snapshot(page);
  entry.door = await walkGeneratedStairs(page, 424, entry);
  assert.ok(Math.abs(entry.door.player.y - 472) <= 8, 'Real generated exit must be approached at its authored height');
  await page.keyboard.down('ArrowUp');
  try {
    await page.waitForFunction(() => window.__spelunkyTest.state === 'Ending', null, { timeout: 10000 });
  } finally { await page.keyboard.up('ArrowUp'); }
  entry.exit = await snapshot(page);
  const sourceSecondsRemainder = Math.floor(entry.exit.sourceTimeMs / 1000) % 60;
  assert.ok(sourceSecondsRemainder > 20, 'Actual completed room time must satisfy source darkness eligibility');
  entry.transition = await continueTransition({ page, result, screenshot, name: `lighting-eligible-${entry.attempt}`,
    expectedRoom: 'rTransition2' });
  await page.waitForFunction(() => window.__spelunkyTest.state === 'Playing' && window.__spelunkyTest.world.level === 6,
    null, { timeout: 15000 });
  entry.next = await snapshot(page);
  entry.dark = entry.next.world.dark;
  return entry.dark;
}

export async function runLightingAcceptance({ page, result, screenshot }) {
  const entry = result.lighting = {
    status: 'failed', attempts: [],
    scope: 'Saved shortcut unlock and disclosed one-shot clock-derived seed; real elapsed source time, generated level5 keyboard clear, authored tally and naturally random eligible level6 darkness. Actual production WebGL byte-multiply readback. Published circles are pipeline inputs, not an independent emitter-catalog oracle.',
  };
  for (let attempt = 0; attempt < 56; attempt++) {
    if (attempt) await restart(page);
    const candidate = { attempt: attempt + 1, seed: '395', waitTicks: 750 + (attempt % 8) };
    entry.attempts.push(candidate);
    if (!await enterEligibleDarkRoom(page, candidate, result, screenshot)) continue;
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
    result.checks.push('Source-eligible generated level6 dark room after real level5 clear; actual WebGL RGB-byte multiplication, circle union/overlap, black exterior, and HUD bypass');
    return;
  }
  assert.fail('No live measurable natural dark room found in56 source-eligible generated room clears');
}
