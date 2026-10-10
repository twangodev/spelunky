// Read-only telemetry observes the real Rust app; all control uses browser keys/UI.
import assert from 'node:assert/strict';

export function installMovementTrace() {
  window.__SPELUNKY_TEST_MODE__ = true;
  window.__movementTrace = [];
  let current;
  let sequence = 0;
  Object.defineProperty(window, '__spelunkyTest', {
    configurable: true,
    get: () => current,
    set(value) {
      current = { ...value, sequence: sequence++, observedAt: performance.now() };
      window.__movementTrace.push(current);
      if (window.__movementTrace.length > 1200) window.__movementTrace.shift();
    },
  });
}

function fixture(side) {
  const rows = Array.from({ length: 32 }, () => Array(40).fill('0'));
  // Map cells have a one-tile border in the runtime world. Target wall:
  // x160..223, y80..159; landing feet y80. Raised start floor primes the
  // source walking limit before the player walks across the 16px gap.
  for (let row = 4; row < 9; row++) for (let col = 9; col < 13; col++) rows[row][col] = '1';
  rows[9].fill('1');
  const start = side === 'right' ? 7 : 14;
  rows[4][start] = '1';
  rows[3][start] = '@';
  // A low ceiling cancels a real initial ground jump, allowing Classic's jump
  // key to remain held before walking off, without a narrow input-delivery race.
  // It ends before the target ledge and cannot block the hanging climb.
  rows[2][start] = '1';
  rows[8][20] = 'X';
  return [...rows.map(row => row.join('')), 'BROWSER LEDGE REGRESSION', 'CAVE', '99', '0', '0', 'NONE', '0', '0', ''].join('\n');
}

export async function snapshot(page) {
  return page.evaluate(() => window.__spelunkyTest);
}

export async function state(page, expected) {
  await page.waitForFunction(expected => window.__spelunkyTest?.screen === expected, expected, { timeout: 10000 });
  return snapshot(page);
}

// Reach the host menu through the source opening and its ordinary skip inputs.
// Reloads recreate Intro; no Rust opening/player flags are changed by the test.
export async function reachTitleMenu(page) {
  await page.waitForFunction(() => window.__spelunkyTest?.screen, null, { timeout: 15000 });
  const route = { initial: await snapshot(page), inputs: [] };
  if (route.initial.screen === 'Title') return route;
  await page.locator('canvas').focus();
  assert.equal(route.initial.screen, 'Playing');
  assert.ok(['Intro', 'Title'].includes(route.initial.opening?.kind), 'Expected the actual source opening after startup');
  if (route.initial.opening.kind === 'Intro') {
    await page.keyboard.press('Enter');
    route.inputs.push('Enter: reveal Intro');
    await page.waitForFunction(() => window.__spelunkyTest.opening?.kind === 'Intro'
      && window.__spelunkyTest.opening.fadeLevel <= 0, null, { timeout: 15000 });
    const revealed = await snapshot(page);
    // The source creates its dummy on the next tick after the fade reaches zero.
    await page.waitForFunction(tick => window.__spelunkyTest.tick >= tick + 2, revealed.tick, { timeout: 10000 });
    route.revealed = await snapshot(page);
    await page.keyboard.press('Enter');
    route.inputs.push('Enter: fade Intro to authored Title');
    await page.waitForFunction(() => window.__spelunkyTest.world?.name === 'rTitle', null, { timeout: 15000 });
  }
  route.authoredTitle = await snapshot(page);
  if (route.authoredTitle.opening) {
    await page.keyboard.press('Enter');
    route.inputs.push('Enter: skip authored Title entrance');
    await page.waitForFunction(() => window.__spelunkyTest.world?.name === 'rTitle'
      && !window.__spelunkyTest.opening, null, { timeout: 15000 });
  }
  route.hub = await snapshot(page);
  assert.equal(route.hub.world.name, 'rTitle');
  await page.keyboard.press('Escape');
  await state(page, 'Pause');
  await press(page, 'ArrowDown', 2);
  await page.keyboard.press('Enter');
  route.inputs.push('Escape, ArrowDown, ArrowDown, Enter: host Title menu');
  route.menu = await state(page, 'Title');
  return route;
}

export async function press(page, key, count = 1) {
  for (let i = 0; i < count; i++) {
    await page.keyboard.press(key);
    await page.waitForTimeout(35);
  }
}

export async function returnToEditor(page) {
  await page.keyboard.press('Escape');
  await state(page, 'Pause');
  await press(page, 'ArrowDown', 2);
  await page.keyboard.press('Enter');
  await state(page, 'Editor');
}

async function importFixture(page, side, name) {
  const chooser = page.waitForEvent('filechooser', { timeout: 10000 });
  await page.keyboard.press('Control+i');
  await (await chooser).setFiles({ name: `${name}.lvl`, mimeType: 'text/plain', buffer: Buffer.from(fixture(side)) });
  await page.waitForFunction(name => {
    const save = JSON.parse(localStorage.getItem('spelunky-rust.save'));
    return typeof save?.custom_levels?.[name] === 'string';
  }, name, { timeout: 10000 });
}

function currentPositionIsRendered(value, label) {
  assert.ok(Math.abs(value.rendered.x - value.player.x) < 0.0001, `${label}: rendered x rewound`);
  assert.ok(Math.abs(value.rendered.y - value.player.y) < 0.0001, `${label}: rendered y rewound`);
}

async function pauseWhileMoving(page, direction, toward, result) {
  await page.keyboard.down(direction);
  await page.waitForFunction(() => {
    const value = window.__spelunkyTest;
    return value?.screen === 'Playing' && value.player.x !== value.player.previousX && value.alpha < 0.99;
  }, null, { timeout: 10000 });
  const moving = await snapshot(page);
  await page.keyboard.press('Escape');
  const paused = await state(page, 'Pause');
  currentPositionIsRendered(paused, 'pause');
  assert.ok(toward * (paused.rendered.x - moving.rendered.x) >= -0.0001, 'Entering pause must not move the player backward');
  await page.waitForTimeout(180);
  const still = await snapshot(page);
  assert.equal(still.tick, paused.tick, 'Simulation advanced while paused');
  assert.equal(still.player.x, paused.player.x);
  assert.equal(still.player.y, paused.player.y);
  await page.keyboard.press('Escape');
  await state(page, 'Playing');
  const resumed = await page.evaluate(sequence => window.__movementTrace.find(value => value.sequence > sequence && value.screen === 'Playing'), paused.sequence);
  assert.ok(resumed, 'No resumed frame was observed');
  assert.ok(resumed.tick >= paused.tick, 'Resuming moved simulation time backward');
  assert.ok(toward * (resumed.rendered.x - paused.rendered.x) >= -0.0001, 'Resuming must not render an older player position');
  if (resumed.tick === paused.tick) {
    currentPositionIsRendered(resumed, 'resume before next tick');
    assert.equal(resumed.player.x, paused.player.x);
    assert.equal(resumed.player.y, paused.player.y);
  } else if (resumed.tick === paused.tick + 1) {
    // A software renderer may not draw until after the next 33ms tick. Its
    // interpolation must start from the paused position, not an older snapshot.
    assert.equal(resumed.player.previousX, paused.player.x);
    assert.equal(resumed.player.previousY, paused.player.y);
  }
  await page.keyboard.up(direction);
  result.pause = { moving, paused, resumed };
}

export async function restoreDefaultSettings(page) {
  await page.setViewportSize({ width: 960, height: 720 });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await reachTitleMenu(page);
  await page.locator('canvas').focus();
  // A new app instance starts the settings cursor at row zero. Restore defaults
  // through the actual menu to use Classic rules, unsnapped motion, and Z jump.
  await press(page, 'ArrowUp', 4);
  await page.keyboard.press('Enter');
  await state(page, 'Settings');
  await press(page, 'ArrowDown', 21);
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => {
    const value = JSON.parse(localStorage.getItem('spelunky-rust.save'))?.settings;
    return value?.rules === 'Classic' && value?.pixel_snap === false && value?.bindings?.jump?.includes('KeyZ');
  });
  await page.keyboard.press('Escape');
  await state(page, 'Title');
}

export async function runMovementAcceptance({ page, result, screenshot }) {
  await restoreDefaultSettings(page);
  result.movement = [];

  for (const rules of ['Classic', 'Assisted']) {
    if (rules === 'Assisted') {
      // Reload resets the settings cursor; toggling rule row zero persists via UI.
      await page.reload({ waitUntil: 'domcontentloaded' });
      await reachTitleMenu(page);
      await page.locator('canvas').focus();
      await press(page, 'ArrowUp', 4);
      await page.keyboard.press('Enter');
      await state(page, 'Settings');
      await page.keyboard.press('ArrowRight');
      await page.waitForFunction(() => JSON.parse(localStorage.getItem('spelunky-rust.save')).settings.rules === 'Assisted');
      await page.keyboard.press('Escape');
      await state(page, 'Title');
    }
    await page.keyboard.press('F2');
    await state(page, 'Editor');
    for (const side of ['right', 'left']) {
      const repeats = rules === 'Classic' ? 2 : 1;
      for (let attempt = 0; attempt < repeats; attempt++) {
        const name = `ledge-${rules.toLowerCase()}-${side}-${attempt + 1}`;
        const entry = { name, rules, side, status: 'failed' };
        result.movement.push(entry);
        const toward = side === 'right' ? 1 : -1;
        const direction = side === 'right' ? 'ArrowRight' : 'ArrowLeft';
        await importFixture(page, side, name);
        await page.keyboard.press('F5');
        await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing' && window.__spelunkyTest.player.grounded, null, { timeout: 10000 });
        entry.start = await snapshot(page);
        assert.equal(entry.start.player.y, 72);
        if (rules === 'Classic') {
          await page.keyboard.down('z');
          await page.waitForFunction(tick => {
            const value = window.__spelunkyTest;
            return value.tick >= tick + 12 && value.player.grounded;
          }, entry.start.tick, { timeout: 10000 });
          entry.heldJumpPrimed = await snapshot(page);
          assert.equal(entry.heldJumpPrimed.player.y, 72, 'The ceiling must cancel the initial ground jump');
        }
        await page.keyboard.down(direction);
        // Classic is already holding jump after the blocked ground jump;
        // Assisted approaches without jump to avoid its intentional coyote jump.
        await page.waitForFunction(() => window.__spelunkyTest?.player.state === 'Hanging', null, { timeout: 10000 });
        entry.hanging = await snapshot(page);
        assert.equal(entry.hanging.player.y, 88);
        assert.equal(entry.hanging.player.x, side === 'right' ? 155 : 229);
        await screenshot(page, `${name}-hang`, result);
        await page.keyboard.up('z');
        await page.waitForFunction(tick => window.__spelunkyTest.tick > tick, entry.hanging.tick, { timeout: 10000 });
        const released = await snapshot(page);
        assert.equal(released.player.state, 'Hanging');
        await page.keyboard.down('z');
        await page.waitForFunction(() => window.__spelunkyTest.player.state !== 'Hanging', null, { timeout: 10000 });
        entry.launch = await snapshot(page);
        assert.ok(entry.launch.player.y < entry.hanging.player.y, 'Fresh hanging jump must move upward');
        assert.ok(entry.launch.player.vy < 0, 'Fresh hanging jump must have upward velocity');
        await page.waitForFunction(() => {
          const value = window.__spelunkyTest;
          return value.player.grounded && value.player.y === 72 && value.player.x >= 155 && value.player.x <= 229;
        }, null, { timeout: 10000 });
        await page.keyboard.up(direction);
        await page.keyboard.up('z');
        entry.landed = await snapshot(page);
        assert.equal(entry.landed.player.y + 8, 80, 'Feet must land above the original ledge');
        const path = await page.evaluate(sequence => window.__movementTrace.filter(value => value.sequence >= sequence && value.screen === 'Playing'), entry.hanging.sequence);
        assert.ok(path.some(value => value.player.y < 72), 'Player must clear the top before landing');
        entry.path = path;
        await screenshot(page, `${name}-landed`, result);
        await pauseWhileMoving(page, direction, toward, entry);
        await screenshot(page, `${name}-resumed`, result);
        entry.status = 'passed';
        await returnToEditor(page);
      }
    }
  }
  result.checks.push('Six natural ledge climbs across both sides/presets, repeated Classic held-jump release/repress climbs, and moving pause/resume without rewind');
}
