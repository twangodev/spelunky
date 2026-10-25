// Real UI-generated dark rooms and readback from the actual production WebGL pass.
import assert from 'node:assert/strict';
import { snapshot, state, press, reachTitleMenu } from './movement.mjs';

export function installLightingProfile() {
  // Controlled persisted unlock, not injected room, equipment, darkness, or seed.
  localStorage.setItem('spelunky-rust.save', JSON.stringify({ version: 1, unlocks: ['shortcut_5'] }));
}

export function installLightingSeedControl() {
  const actualNow = Performance.prototype.now;
  const actualDateNow = Date.now;
  let pendingPerformanceSample = null;
  const control = window.__lightingSeedControl = { armed: null, observations: [] };
  Performance.prototype.now = function (...args) {
    // Production XORs Date.now().to_bits() with (performance.now()/1000).to_bits()
    // in the same synchronous start_game expression. Reuse one actual sample
    // for the latter so both halves describe a coherent, real browser clock.
    if (pendingPerformanceSample !== null) {
      const value = pendingPerformanceSample;
      pendingPerformanceSample = null;
      return value;
    }
    return actualNow.apply(this, args);
  };
  Date.now = function () {
    if (control.armed === null) return actualDateNow();
    const performanceMs = actualNow.call(performance);
    if (!Number.isFinite(performanceMs)) throw new Error('No real performance clock sample before source seed selection');
    const desiredSeed = BigInt(control.armed);
    const performanceBits = new DataView(new ArrayBuffer(8));
    performanceBits.setFloat64(0, performanceMs / 1000, true);
    const performanceFold = (performanceBits.getUint32(0, true) ^ performanceBits.getUint32(4, true)) >>> 0;
    const dateFold = Number(desiredSeed ^ BigInt(performanceFold));

    // Pick an ordinary finite ~1000 ms double whose folded IEEE-754 words are
    // exactly the requested Date entropy. Rust's fold_seed_entropy XORs the
    // float's low and high u32 words, so fixing its high word makes the low word
    // an exact inverse of that fold.
    const highWord = 0x408f4000; // high word of the f64 value 1000.0
    const dateBits = new DataView(new ArrayBuffer(8));
    dateBits.setUint32(0, (dateFold ^ highWord) >>> 0, true);
    dateBits.setUint32(4, highWord, true);
    const wallMs = dateBits.getFloat64(0, true);
    const actualDateFold = (dateBits.getUint32(0, true) ^ dateBits.getUint32(4, true)) >>> 0;
    pendingPerformanceSample = performanceMs;
    control.armed = null;
    control.observations.push({
      desiredSeed: String(desiredSeed), performanceMs, performanceSeconds: performanceMs / 1000,
      performanceFold, wallMs, dateFold: actualDateFold,
      predictedSeed: (actualDateFold ^ performanceFold) >>> 0, actualWallMs: actualDateNow(),
    });
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
  const sourceCircleVertices = s.lighting.circles.map(light => Array.from({ length: 24 }, (_, index) => {
    const angle = 2 * Math.PI * index / 24;
    return [light.x + light.radius * Math.cos(angle), light.y + light.radius * Math.sin(angle)];
  }));
  const insideSourceCircle = (vertices, x, y) => {
    for (let index = 0; index < vertices.length; index++) {
      const [firstX, firstY] = vertices[index];
      const [secondX, secondY] = vertices[(index + 1) % vertices.length];
      const cross = (secondX - firstX) * (y - firstY) - (secondY - firstY) * (x - firstX);
      if (cross < -1e-9) return false;
    }
    return true;
  };
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
    const coverage = sourceCircleVertices.filter(vertices => insideSourceCircle(vertices, wx, wy)).length;
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
  await reachTitleMenu(page);
  await page.locator('canvas').focus();
}

async function enableRouteSurvivalCheat(page, entry) {
  // This changes only the player-health precondition through the real Cheats
  // UI. The test still uses normal generated rooms and lets the source RNG
  // decide whether Level 6 is dark; no world, flag, or light state is written.
  await page.keyboard.press('F6');
  await state(page, 'Cheats');
  const before = await snapshot(page);
  await page.keyboard.press('Enter'); // Default selection: INFINITE HEALTH.
  await page.waitForFunction(() => window.__spelunkyTest?.player.health === 99, null, { timeout: 5000 });
  const enabled = await snapshot(page);
  await page.keyboard.press('F6');
  await state(page, 'Playing');
  entry.routeSurvivalCheat = {
    input: 'F6, Enter, F6 through the real Cheats menu',
    selectedScreen: before.screen,
    healthBefore: before.player.health,
    healthAfter: enabled.player.health,
    screenAfter: (await snapshot(page)).screen,
  };
  assert.equal(entry.routeSurvivalCheat.screenAfter, 'Playing');
}

// Test-only DOM events enter the same canvas/winit keyboard listener as normal
// keys. They are untrusted browser events, not OS input; no Rust state is set.
export function installLightingRoute(config) {
  const route = window.__lightingRoute = {
    config, phase: 'waiting', frames: [], edges: [], jumps: [], gaps: [], focusEvents: [],
    progress: { minX: null, maxX: null, startY: null, maxY: null }, error: null, done: false,
  };
  const descriptor = Object.getOwnPropertyDescriptor(window, '__spelunkyTest');
  if (!descriptor?.set || !descriptor?.get) throw new Error('Read-only movement trace must be installed before route controller');
  const canvas = document.getElementById('spelunky-canvas');
  const keys = new Set();
  const keyNames = { KeyA: 'a', KeyZ: 'z', KeyX: 'x', ShiftLeft: 'Shift' };
  const setKeys = desired => {
    const next = new Set(desired);
    for (const code of new Set([...keys, ...next])) {
      if (keys.has(code) === next.has(code)) continue;
      const down = next.has(code);
      canvas.dispatchEvent(new KeyboardEvent(down ? 'keydown' : 'keyup', {
        code, key: keyNames[code] ?? code, bubbles: true, cancelable: true, composed: true, repeat: false,
      }));
      route.edges.push({ tick: lastTick, code, down, phase: route.phase });
    }
    keys.clear();
    for (const key of next) keys.add(key);
  };
  let lastTick = -1, phaseStartedTick = 0, nextJumpTick = Infinity, endingTick = null;
  let hangReleaseTick = null, hangJumpTick = null, hangResumePhase = null, exitReleaseTick = null;
  let previousX = null, stationarySince = null;
  for (const name of ['focus', 'blur']) window.addEventListener(name, () => {
    route.focusEvents.push({ name, tick: lastTick, activeElement: document.activeElement?.id });
    if (name === 'blur') keys.clear(); // The normal host also releases input on lost focus.
  });
  const phase = (name, tick) => {
    if (route.phase === name) return;
    route.phase = name;
    phaseStartedTick = tick;
    previousX = null;
    stationarySince = null;
  };
  const directionWithJumpPulse = (direction, value, tick) => {
    const desired = [direction, 'ShiftLeft'];
    if (value.player.grounded && tick >= nextJumpTick && !keys.has('KeyZ')) {
      desired.push('KeyZ');
      nextJumpTick = tick + 28;
      route.jumps.push({ tick, x: value.player.x, y: value.player.y, stun: value.player.stun,
        grounded: value.player.grounded, phase: route.phase, reason: 'source-route-pulse' });
    }
    setKeys(desired);
  };
  const handleHangingJump = (tick, direction) => {
    if (hangReleaseTick === null) {
      hangReleaseTick = tick;
      hangJumpTick = null;
      phase('hang-release', tick);
      setKeys([direction]);
      return;
    }
    if (hangJumpTick === null) {
      if (tick <= hangReleaseTick) return setKeys([direction]);
      hangJumpTick = tick;
      phase('hang-jump', tick);
      route.jumps.push({ tick, x: route.progress.last.x, y: route.progress.last.y,
        stun: route.progress.last.stun, grounded: route.progress.last.grounded,
        phase: route.phase, reason: 'fresh-hang-jump-edge' });
      return setKeys([direction, 'KeyZ']);
    }
    if (tick <= hangJumpTick + 1) return setKeys([direction]);
    // If a jump edge leaves the player hanging, arm another only after a real
    // release tick. Successful edges leave this branch as soon as state changes.
    hangReleaseTick = tick;
    hangJumpTick = null;
    phase('hang-release', tick);
    setKeys([direction]);
  };
  function control(value) {
    if (route.done || !value?.world || value.world.level < 5) return;
    if (value.tick === lastTick) return;
    if (lastTick >= 0 && value.tick !== lastTick + 1) route.gaps.push({ from: lastTick, to: value.tick, phase: route.phase });
    lastTick = value.tick;
    const p = value.player;
    const progress = route.progress;
    progress.minX = progress.minX === null ? p.x : Math.min(progress.minX, p.x);
    progress.maxX = progress.maxX === null ? p.x : Math.max(progress.maxX, p.x);
    progress.startY ??= p.y;
    progress.maxY = progress.maxY === null ? p.y : Math.max(progress.maxY, p.y);
    progress.last = { tick: value.tick, x: p.x, y: p.y, vx: p.vx, vy: p.vy,
      stun: p.stun, grounded: p.grounded, state: p.state };
    route.frames.push({ tick: value.tick, levelTick: value.levelTick, world: value.world.name,
      level: value.world.level, dark: value.world.dark, x: p.x, y: p.y, vx: p.vx, vy: p.vy,
      grounded: p.grounded, health: p.health, stun: p.stun, state: value.state,
      phase: route.phase, ending: value.ending?.stage ?? null, sourceTimeMs: value.sourceTimeMs,
      focused: document.hasFocus(), activeElement: document.activeElement?.id });
    if (value.world.level === 6 && value.state === 'Playing') {
      progress.reachedLevel = 6;
      route.done = true;
      setKeys([]);
      return;
    }
    if (p.health <= 0 || !['Playing', 'Ending', 'Transition'].includes(value.state)) {
      route.error = `Natural route stopped at${value.tick}: ${value.state}, health${p.health}`;
      route.done = true;
      setKeys([]);
      return;
    }
    if (value.levelTick > config.waitTicks + 1450) {
      route.error = `Bounded generated route did not reach the next room; progress=${JSON.stringify(progress)}`;
      route.done = true;
      setKeys([]);
      return;
    }
    if (value.state === 'Ending' || value.state === 'Transition') {
      if (endingTick === null) {
        endingTick = value.tick;
        phase('source-exit-and-tally', value.levelTick);
      }
      // Source attack pulses hurry the tally, then acknowledge it. The full
      // authored exit and tally still run on actual simulation/elapsed time.
      setKeys((value.tick - endingTick) % 15 === 0 ? ['KeyX'] : []);
      return;
    }
    if (route.phase === 'waiting') {
      if (value.levelTick < config.waitTicks) return setKeys([]);
      progress.start = { x: p.x, y: p.y, tick: value.tick };
      phase('initial-east-run', value.levelTick);
    }
    const routeTick = value.levelTick - config.waitTicks;
    // This ordinary-input route was reproduced through Game::step on the exact
    // direct-shortcut seed. Timed opening actions reach the generated east
    // shaft; subsequent choices follow actual position/state telemetry.
    if (routeTick < 19) return setKeys(['ArrowRight', 'ShiftLeft']);
    if (routeTick < 29) return setKeys(['ArrowRight', 'ShiftLeft', 'KeyZ']);
    if (routeTick < 34) return setKeys(['ArrowRight', 'ShiftLeft']);
    if (routeTick < 35) return setKeys(['ArrowDown', 'KeyA']);
    if (routeTick < 67) return setKeys(['ArrowLeft', 'ShiftLeft']);
    if (routeTick < 192) return setKeys([]);

    if (route.phase.startsWith('initial-')) {
      phase('upper-crossing', value.levelTick);
      nextJumpTick = value.levelTick;
    }
    if (p.state === 'Hanging') {
      if (hangResumePhase === null) {
        hangResumePhase = route.phase === 'wall-first-jump' || route.phase === 'wall-contact'
          ? 'wall-contact' : route.phase === 'exit-launch' ? 'exit-crossing' : route.phase;
        hangReleaseTick = null;
        hangJumpTick = null;
      }
      handleHangingJump(value.levelTick, hangResumePhase === 'east-shaft' ? 'ArrowLeft' : 'ArrowRight');
      return;
    }
    if (hangResumePhase !== null) {
      const resume = hangResumePhase;
      hangResumePhase = null;
      hangReleaseTick = hangJumpTick = null;
      phase(resume, value.levelTick);
    }

    if (route.phase === 'upper-crossing') {
      if (p.x >= 610 && p.y <= 160) {
        phase('east-shaft', value.levelTick);
        nextJumpTick = value.levelTick + 28;
      } else {
        directionWithJumpPulse('ArrowRight', value, value.levelTick);
        return;
      }
    }
    if (route.phase === 'east-shaft') {
      // Start braking as the runner passes the lower opening. One source tick
      // continues the held leftward impulse before the next telemetry sample.
      if (p.x <= 120 && p.y >= 350) {
        phase('lower-drop', value.levelTick);
        setKeys([]);
        return;
      }
      directionWithJumpPulse('ArrowLeft', value, value.levelTick);
      return;
    }
    if (route.phase === 'lower-drop') {
      if (p.grounded && p.y >= 480) {
        phase('bottom-wall-approach', value.levelTick);
        setKeys(['ArrowRight', 'ShiftLeft']);
      } else {
        setKeys([]);
      }
      return;
    }
    if (route.phase === 'bottom-wall-approach') {
      if (p.grounded && p.x >= 122) {
        phase('wall-backoff', value.levelTick);
        setKeys(['ArrowLeft', 'ShiftLeft']);
      } else {
        setKeys(['ArrowRight', 'ShiftLeft']);
      }
      return;
    }
    if (route.phase === 'wall-backoff') {
      if (value.levelTick - phaseStartedTick < 4) return setKeys(['ArrowLeft', 'ShiftLeft']);
      phase('wall-runup', value.levelTick);
      return setKeys(['ArrowRight', 'ShiftLeft']);
    }
    if (route.phase === 'wall-runup') {
      if (value.levelTick - phaseStartedTick < 7) return setKeys(['ArrowRight', 'ShiftLeft']);
      phase('wall-first-jump', value.levelTick);
      return setKeys(['ArrowRight', 'ShiftLeft', 'KeyZ']);
    }
    if (route.phase === 'wall-first-jump') {
      if (value.levelTick - phaseStartedTick < 10) return setKeys(['ArrowRight', 'ShiftLeft', 'KeyZ']);
      phase('wall-contact', value.levelTick);
      return setKeys(['ArrowRight', 'ShiftLeft']);
    }
    if (route.phase === 'wall-contact') {
      if (p.grounded && p.x >= 150 && p.y < 470) {
        phase('exit-launch', value.levelTick);
        return setKeys(['ArrowRight', 'ShiftLeft', 'KeyZ']);
      }
      setKeys(['ArrowRight', 'ShiftLeft']);
      return;
    }
    if (route.phase === 'exit-launch') {
      if (value.levelTick - phaseStartedTick < 10) return setKeys(['ArrowRight', 'ShiftLeft', 'KeyZ']);
      phase('exit-crossing', value.levelTick);
      nextJumpTick = value.levelTick + 28;
    }
    if (route.phase === 'exit-crossing') {
      if (p.grounded && Math.abs(p.x - config.exitX) <= 8 && Math.abs(p.y - config.exitY) <= 8) {
        if (exitReleaseTick === null) {
          exitReleaseTick = value.levelTick;
          phase('exit-release', value.levelTick);
          return setKeys([]);
        }
        if (value.levelTick > exitReleaseTick) {
          phase('door-up', value.levelTick);
          return setKeys(['ArrowUp']);
        }
      }
      const direction = p.x < config.exitX - 8 ? 'ArrowRight' : 'ArrowLeft';
      if (previousX === null) previousX = p.x;
      if (Math.abs(p.x - previousX) >= .2) stationarySince = null;
      else if (stationarySince === null) stationarySince = value.levelTick - 1;
      previousX = p.x;
      if (p.grounded && stationarySince !== null && value.levelTick - stationarySince >= 4) {
        nextJumpTick = Math.min(nextJumpTick, value.levelTick);
      }
      directionWithJumpPulse(direction, value, value.levelTick);
      return;
    }
    if (route.phase === 'exit-release') {
      if (value.levelTick > phaseStartedTick) {
        phase('door-up', value.levelTick);
        return setKeys(['ArrowUp']);
      }
      return setKeys([]);
    }
    if (route.phase === 'door-up') return setKeys(['ArrowUp']);
  }
  Object.defineProperty(window, '__spelunkyTest', {
    configurable: true, get: descriptor.get,
    set(value) { descriptor.set(value); control(descriptor.get()); },
  });
}

async function enterEligibleDarkRoom(page, entry, result, screenshot) {
  await press(page, 'ArrowDown', 5);
  await page.keyboard.press('Enter');
  await state(page, 'Shortcuts');
  entry.preStart = await snapshot(page);
  await page.bringToFront();
  await page.locator('canvas').focus();
  await page.evaluate(installLightingRoute, { seed: entry.seed, waitTicks: entry.waitTicks });
  await page.evaluate(seed => { window.__lightingSeedControl.armed = seed; }, entry.seed);
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing' && window.__spelunkyTest.world.level === 5,
    null, { timeout: 15000 });
  entry.start = await snapshot(page);
  assert.ok(entry.start.world.exit && Number.isFinite(entry.start.world.exit.x)
    && Number.isFinite(entry.start.world.exit.y), 'Generated level must publish its actual exit center');
  entry.routeTarget = entry.start.world.exit;
  await page.evaluate(exit => {
    window.__lightingRoute.config.exitX = exit.x;
    window.__lightingRoute.config.exitY = exit.y;
  }, entry.routeTarget);
  entry.environmentClock = await page.evaluate(() => window.__lightingSeedControl.observations);
  assert.equal(entry.start.seed, entry.seed, 'One-shot environment clock must select the documented source seed');
  assert.equal(entry.start.world.dark, false, 'Fresh shortcut is correctly suppressed by source noDarkLevel');
  await enableRouteSurvivalCheat(page, entry);
  await screenshot(page, `lighting-level5-start-${entry.attempt}`, result);
  try {
    await page.waitForFunction(() => window.__spelunkyTest.ending?.stage === 'Transition' || window.__lightingRoute.error,
      null, { timeout: 70000 });
    entry.transition = await snapshot(page);
    const routeError = await page.evaluate(() => window.__lightingRoute.error);
    if (routeError !== null) {
      entry.rejected = routeError;
      console.log(`Lighting route rejected: ${routeError}`);
      await screenshot(page, `lighting-route-rejected-${entry.attempt}`, result);
      return false;
    }
    assert.equal(entry.transition.world.name, 'rTransition2');
    assert.ok(Math.floor(entry.transition.ending.sourceTimeMs / 1000) % 60 > 20,
      'Actual source room clock must satisfy darkness eligibility before generation');
    await screenshot(page, `lighting-eligible-${entry.attempt}-transition`, result);
    await page.waitForFunction(() => window.__lightingRoute.done, null, { timeout: 20000 });
  } finally {
    entry.route = await page.evaluate(() => window.__lightingRoute);
  }
  assert.equal(entry.route.error, null);
  // Gaps remain diagnostic evidence. This controller uses observed positions
  // and actual tick deadlines, not one callback per simulation-step timing.
  entry.next = await snapshot(page);
  assert.equal(entry.next.world.level, 6);
  entry.dark = entry.next.world.dark;
  console.log(`Lighting route seed${entry.seed} wait${entry.waitTicks}: level6 dark=${entry.dark}, health=${entry.next.player.health}, source exit=${entry.transition.ending.sourceTimeMs}ms`);
  return entry.dark;
}

export async function runLightingAcceptance({ page, result, screenshot }) {
  // Exercise the production pipeline at its native logical resolution, reducing
  // software-renderer cost without reducing any of the76,800 logical samples.
  // The separate gameplay suite already covers larger/responsive viewports.
  await page.setViewportSize({ width: 320, height: 240 });
  const entry = result.lighting = {
    status: 'failed', attempts: [], viewport: { width: 320, height: 240 },
    scope: 'Saved shortcut unlock and disclosed one-shot clock-derived seed; real elapsed clocks; telemetry-timed untrusted DOM keyboard events through normal canvas/winit input; Infinite Health is enabled through the actual F6 cheat menu only to keep the route alive. Generated level5 clear, authored tally and naturally random eligible level6 darkness remain unmodified. Actual production WebGL byte-multiply readback. Published circles are pipeline inputs, not an independent emitter-catalog oracle.',
  };
  for (let attempt = 0; attempt < 1; attempt++) {
    if (attempt) await restart(page);
    const candidate = { attempt: attempt + 1, seed: '395', waitTicks: 10 };
    entry.attempts.push(candidate);
    if (!await enterEligibleDarkRoom(page, candidate, result, screenshot)) continue;
    let current = await snapshot(page);
    if (!(current.darkness >= .12 && current.state === 'Playing' && current.player.health > 0)) {
      candidate.rejected = 'No live measurable tint reached through bounded ordinary walking';
      continue;
    }
    // The accepted room is naturally dark, but its entrance can put the player
    // just outside every overlap. Walk toward the nearest source emitter using
    // normal input until the player's real light disk overlaps it with margin.
    // This creates no light, changes no world flags, and writes no game state.
    const overlapsWithMargin = value => {
      const [player, ...emitters] = value.lighting?.circles ?? [];
      return Boolean(player && emitters.some(light => {
        const distance = Math.hypot(player.x - light.x, player.y - light.y);
        return distance < player.radius + light.radius - 8;
      }));
    };
    candidate.overlapApproach = { start: current.player, starts: current.lighting?.circles ?? [], steps: [] };
    for (let step = 0; step < 4 && !overlapsWithMargin(current); step++) {
      const playerLight = current.lighting?.circles?.[0];
      const target = current.lighting?.circles?.slice(1).sort((a, b) =>
        Math.hypot(playerLight.x - a.x, playerLight.y - a.y) - playerLight.radius
          - (Math.hypot(playerLight.x - b.x, playerLight.y - b.y) - b.radius))[0];
      if (!playerLight || !target) break;
      const direction = target.x >= playerLight.x ? 'ArrowRight' : 'ArrowLeft';
      const before = current;
      await page.keyboard.down(direction);
      await page.waitForFunction(tick => {
        const value = window.__spelunkyTest;
        return value.tick >= tick + 24 || value.state !== 'Playing' || value.player.health <= 0;
      }, before.tick, { timeout: 10000 });
      await page.keyboard.up(direction);
      current = await snapshot(page);
      candidate.overlapApproach.steps.push({ direction, before: before.player, after: current.player,
        circles: current.lighting?.circles ?? [] });
      if (current.state !== 'Playing' || current.player.health <= 0) break;
    }
    assert.ok(overlapsWithMargin(current),
      `Natural room must permit a real player/emitter overlap: ${JSON.stringify(candidate.overlapApproach)}`);
    if (!(current.darkness >= .12 && current.state === 'Playing' && current.player.health > 0)) {
      candidate.rejected = 'Darkroom overlap approach did not retain a live measurable tint';
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
  assert.fail('No live measurable natural dark room found in three diagnosed source-eligible room clears');
}
