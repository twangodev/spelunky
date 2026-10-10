// Imported Classic levels and real keyboard input; telemetry is read-only.
import assert from 'node:assert/strict';
import { snapshot, state, press, returnToEditor, restoreDefaultSettings } from './movement.mjs';

export function level({ start = 6, floor = '1', health = 9, bombs = 2, ropes = 3, arrow = false,
  next = 'NONE', entrances, exit = 7, targets = [] } = {}) {
  const rows = Array.from({ length: 32 }, () => Array(40).fill('0'));
  rows[8].fill(floor);
  for (const col of entrances || [start]) rows[7][col] = '@';
  if (exit !== null) rows[7][exit] = 'X';
  if (arrow) rows[7][2] = '>';
  return [...rows.map(row => row.join('')), 'BROWSER ADVERSARIAL REGRESSION', 'CAVE',
    String(health), String(bombs), String(ropes), next, String(targets.length), ...targets, '0'].join('\n');
}

export async function importDocument(page, name, text) {
  const chooser = page.waitForEvent('filechooser', { timeout: 10000 });
  await page.keyboard.press('Control+i');
  await (await chooser).setFiles({ name: `${name}.lvl`, mimeType: 'text/plain', buffer: Buffer.from(text) });
  await page.waitForFunction(({ name, text }) => JSON.parse(localStorage.getItem('spelunky-rust.save'))?.custom_levels?.[name] === text,
    { name, text }, { timeout: 10000 });
}

async function playtest(page) {
  await page.keyboard.press('F5');
  await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing' && window.__spelunkyTest.player.grounded,
    null, { timeout: 10000 });
  return snapshot(page);
}

async function waitTicks(page, start, count) {
  await page.waitForFunction(({ start, count }) => window.__spelunkyTest.tick >= start + count,
    { start, count }, { timeout: 15000 });
}

async function pathSince(page, sequence) {
  return page.evaluate(sequence => window.__movementTrace.filter(value => value.sequence >= sequence && value.player), sequence);
}

function finiteAndInside(path) {
  assert.ok(path.length > 0, 'No authoritative frames were observed');
  for (const frame of path) {
    for (const value of [frame.player.x, frame.player.y, frame.player.vx, frame.player.vy,
      frame.rendered.x, frame.rendered.y, frame.camera.x, frame.camera.y]) assert.ok(Number.isFinite(value));
    assert.ok(frame.player.x >= 0 && frame.player.x < frame.world.width * 16,
      `Player escaped room horizontally at tick ${frame.tick}: x=${frame.player.x}`);
  }
}

export async function startCustom(page, name) {
  await page.reload({ waitUntil: 'domcontentloaded' });
  await state(page, 'Title');
  await page.locator('canvas').focus();
  await press(page, 'ArrowDown', 3);
  await page.keyboard.press('Enter');
  await state(page, 'Custom');
  const index = await page.evaluate(name => Object.keys(JSON.parse(localStorage.getItem('spelunky-rust.save')).custom_levels).sort().indexOf(name), name);
  assert.ok(index >= 0);
  await press(page, 'ArrowDown', index);
  await page.keyboard.press('Enter');
  await page.waitForFunction(name => window.__spelunkyTest?.screen === 'Playing' && window.__spelunkyTest.world.name === name,
    name, { timeout: 10000 });
}

export async function walkThroughNextDoor(page) {
  const from = await snapshot(page);
  await page.keyboard.down('ArrowRight');
  // Holding Up lets the authoritative tick select the door as we enter its
  // mask; this avoids steering past it while a software renderer catches up.
  await page.keyboard.down('ArrowUp');
  await page.waitForFunction(name => {
    const value = window.__spelunkyTest;
    return value.state !== 'Playing' || value.world.name !== name;
  }, from.world.name, { timeout: 10000 });
  await page.keyboard.up('ArrowRight');
  await page.keyboard.up('ArrowUp');
  return snapshot(page);
}

export async function continueTransition({ page, result, screenshot, name, expectedRoom = 'rTransition1' }) {
  const entry = { exit: await snapshot(page) };
  await page.waitForFunction(() => window.__spelunkyTest?.state === 'Ending'
    && window.__spelunkyTest.ending?.kind === 'Transition'
    && window.__spelunkyTest.ending?.stage === 'Transition', null, { timeout: 15000 });
  entry.scene = await snapshot(page);
  assert.equal(entry.scene.screen, 'Playing');
  assert.equal(entry.scene.world.name, expectedRoom);
  assert.equal(entry.scene.ending.recordReady, false);
  await screenshot(page, `${name}-transition`, result);
  await page.keyboard.press('Enter'); // Source hurry-up action, not a room mutation.
  await page.waitForFunction(() => window.__spelunkyTest.ending?.drawLoot === 2
    && window.__spelunkyTest.ending.moneyCount === window.__spelunkyTest.ending.levelMoney,
  null, { timeout: 15000 });
  entry.ready = await snapshot(page);
  await page.keyboard.press('Enter');
  return entry;
}

function resources(value) {
  return { health: value.player.health, bombs: value.player.bombs, ropes: value.player.ropes };
}

export async function runAdversarialAcceptance({ page, result, screenshot }) {
  await restoreDefaultSettings(page);
  result.adversarial = [];
  const generated = { name: 'generated-room-left-approach', status: 'failed',
    scope: 'Bounded live-hazard keyboard route; authored cases below separately require side-wall contact.' };
  result.adversarial.push(generated);
  await page.keyboard.press('Enter');
  await state(page, 'Playing');
  generated.start = await snapshot(page);
  await page.keyboard.down('ArrowLeft');
  await page.keyboard.down('ShiftLeft');
  for (let cycle = 0; cycle < 3; cycle++) {
    const begin = await snapshot(page);
    await page.keyboard.down('z');
    await waitTicks(page, begin.tick, 12);
    await page.keyboard.up('z');
    await waitTicks(page, begin.tick, 24);
  }
  await page.keyboard.up('ArrowLeft');
  await page.keyboard.up('ShiftLeft');
  generated.path = await pathSince(page, generated.start.sequence);
  finiteAndInside(generated.path);
  generated.reachedLeftWall = generated.path.some(value => value.player.x <= 21);
  generated.end = await snapshot(page);
  await screenshot(page, 'generated-boundary-route', result);
  generated.status = 'passed';
  await page.reload({ waitUntil: 'domcontentloaded' });
  await state(page, 'Title');
  await page.locator('canvas').focus();
  await page.keyboard.press('F2');
  await state(page, 'Editor');

  for (const side of ['left', 'right']) {
    const name = `boundary-${side}`;
    const entry = { name, status: 'failed' };
    result.adversarial.push(entry);
    await importDocument(page, name, level({ start: side === 'left' ? 0 : 39, exit: null }));
    entry.start = await playtest(page);
    const direction = side === 'left' ? 'ArrowLeft' : 'ArrowRight';
    await page.keyboard.down(direction);
    await page.keyboard.down('ShiftLeft');
    for (let cycle = 0; cycle < 3; cycle++) {
      const begin = await snapshot(page);
      await page.keyboard.down('z');
      await waitTicks(page, begin.tick, 12);
      await page.keyboard.up('z');
      await waitTicks(page, begin.tick, 24);
    }
    await page.keyboard.up(direction);
    await page.keyboard.up('ShiftLeft');
    entry.end = await snapshot(page);
    entry.path = await pathSince(page, entry.start.sequence);
    finiteAndInside(entry.path);
    assert.ok(entry.path.every(value => value.player.x >= 16 && value.player.x < (value.world.width - 1) * 16),
      'Player center crossed an authored hard border');
    assert.ok(entry.path.some(value => side === 'left' ? value.player.x <= 21 : value.player.x >= 651),
      'The approach must reach the authored side wall');
    assert.ok(entry.path.some(value => value.player.y < entry.start.player.y), 'The wall exercise must include an actual jump');
    await screenshot(page, name, result);
    entry.status = 'passed';
    await returnToEditor(page);
  }

  for (const floor of ['1', 'i']) {
    const name = `stun-${floor === 'i' ? 'ice' : 'stone'}`;
    const entry = { name, status: 'failed' };
    result.adversarial.push(entry);
    await importDocument(page, name, level({ floor, arrow: true, exit: null }));
    entry.start = await playtest(page);
    // Approach the live arrow rather than racing its floor impact. Holding
    // until the observed hit avoids depending on a two-tick key release.
    await page.keyboard.down('ArrowLeft');
    await page.waitForFunction(() => window.__spelunkyTest.player.stun > 0 && window.__spelunkyTest.player.health < 9,
      null, { timeout: 10000 });
    await page.keyboard.up('ArrowLeft');
    entry.hit = await snapshot(page);
    assert.ok(entry.hit.player.health < entry.start.player.health, 'Arrow must actually damage the player');
    await screenshot(page, `${name}-hit`, result);
    await page.waitForFunction(() => {
      const value = window.__spelunkyTest;
      return value.state === 'Playing' && value.player.stun === 0 && value.player.grounded && value.player.state !== 'Stunned';
    }, null, { timeout: 15000 });
    entry.recovered = await snapshot(page);
    await page.keyboard.down('ArrowRight');
    await page.waitForFunction(x => window.__spelunkyTest.player.x > x + 4, entry.recovered.player.x, { timeout: 10000 });
    await page.keyboard.up('ArrowRight');
    const ground = await snapshot(page);
    await page.keyboard.down('z');
    await page.waitForFunction(y => window.__spelunkyTest.player.y < y && window.__spelunkyTest.player.vy < 0, ground.player.y,
      { timeout: 10000 });
    entry.jump = await snapshot(page);
    await page.keyboard.up('z');
    assert.notEqual(entry.jump.player.state, 'Stunned');
    entry.path = await pathSince(page, entry.start.sequence);
    finiteAndInside(entry.path);
    await screenshot(page, `${name}-recovered-jump`, result);
    entry.status = 'passed';
    await returnToEditor(page);
  }

  const first = '000-browser-a';
  const second = '000-browser-b';
  const firstText = level({ targets: ['NONE', `${second.toUpperCase()}.LVL`] });
  const secondText = level({ entrances: [2, 14], exit: 17, health: 1, bombs: 0, ropes: 0,
    targets: ['NONE', `${first.toUpperCase()}.LVL`, `${first.toUpperCase()}.LVL`] });
  await importDocument(page, second, secondText);
  await importDocument(page, first, firstText);
  const editor = { name: 'editor-exit-priority', status: 'failed' };
  result.adversarial.push(editor);
  editor.start = await playtest(page);
  await walkThroughNextDoor(page);
  await state(page, 'Editor');
  editor.end = await snapshot(page);
  assert.equal(editor.end.world.name, first, 'Editor playtest must return before following the linked room');
  editor.status = 'passed';
  await screenshot(page, 'editor-exit-priority', result);

  const linked = { name: 'linked-custom-roundtrip', status: 'failed' };
  result.adversarial.push(linked);
  await startCustom(page, first);
  linked.start = await snapshot(page);
  await page.keyboard.press('b');
  await page.waitForFunction(() => window.__spelunkyTest.player.bombs === 1, null, { timeout: 10000 });
  linked.spent = await snapshot(page);
  await walkThroughNextDoor(page);
  linked.outboundTransition = await continueTransition({ page, result, screenshot, name: 'linked-custom-outbound' });
  await page.waitForFunction(name => window.__spelunkyTest.world.name === name && window.__spelunkyTest.state === 'Playing', second,
    { timeout: 10000 });
  linked.second = await snapshot(page);
  assert.deepEqual(resources(linked.second), resources(linked.spent), 'A linked header must not replace ongoing run resources');
  assert.ok(Math.abs(linked.second.player.x - 248) <= 6, 'Previous-room link must select the matching second entrance');
  await screenshot(page, 'linked-custom-second', result);
  await walkThroughNextDoor(page);
  linked.returnTransition = await continueTransition({ page, result, screenshot, name: 'linked-custom-return' });
  await page.waitForFunction(name => window.__spelunkyTest.world.name === name && window.__spelunkyTest.state === 'Playing', first,
    { timeout: 10000 });
  linked.returned = await snapshot(page);
  assert.deepEqual(resources(linked.returned), resources(linked.spent), 'Returning must not refill the original header inventory');
  await screenshot(page, 'linked-custom-return', result);
  linked.status = 'passed';

  // Reload resets the menu while preserving documents; import a real lethal trap.
  await page.reload({ waitUntil: 'domcontentloaded' });
  await state(page, 'Title');
  await page.locator('canvas').focus();
  await page.keyboard.press('F2');
  await state(page, 'Editor');
  const death = { name: 'lethal-arrow-death-reveal', status: 'failed' };
  result.adversarial.push(death);
  await importDocument(page, 'death-arrow', level({ health: 1, arrow: true, exit: null }));
  death.start = await playtest(page);
  await page.keyboard.down('ArrowLeft');
  await page.waitForFunction(() => window.__spelunkyTest.state === 'Dead', null, { timeout: 10000 });
  await page.keyboard.up('ArrowLeft');
  death.hit = await snapshot(page);
  assert.equal(death.hit.screen, 'Playing', 'Death must show its reveal before the menu');
  assert.ok(death.hit.deathTicks < 100);
  await screenshot(page, 'death-reveal', result);
  await state(page, 'Finished');
  death.finished = await snapshot(page);
  assert.ok(death.finished.deathTicks >= 100);
  await page.waitForTimeout(180);
  death.later = await snapshot(page);
  assert.ok(death.later.tick > death.finished.tick, 'The source death menu must leave the corpse/world simulating');
  await screenshot(page, 'death-menu', result);
  await page.keyboard.press('Enter');
  await state(page, 'Editor');
  death.restarted = await playtest(page);
  assert.equal(death.restarted.player.health, 1);
  assert.equal(death.restarted.player.stun, 0);
  assert.equal(death.restarted.deathTicks, 0);
  death.status = 'passed';
  await returnToEditor(page);
  result.checks.push('Bounded generated route, authored side-wall run/jump bounds, real arrow stun recovery on stone/ice, custom room resource/entrance roundtrip, editor exit priority, and live death reveal/restart');
}

async function enterOriginalHub(page) {
  await state(page, 'Title');
  await page.locator('canvas').focus();
  await press(page, 'ArrowDown', 2);
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing' && window.__spelunkyTest.world.name === 'rTitle',
    null, { timeout: 10000 });
}

async function walkToAuthoredStart(page) {
  await page.keyboard.down('ArrowLeft');
  // Pass Scores (x432..447) before holding Up, so the source door queries can
  // select Start (x384..399) without a precise frame-timed key press.
  await page.waitForFunction(() => window.__spelunkyTest.player.x <= 420, null, { timeout: 15000 });
  await page.keyboard.down('ArrowUp');
  await page.waitForFunction(() => window.__spelunkyTest.world.name !== 'rTitle', null, { timeout: 10000 });
  await page.keyboard.up('ArrowLeft');
  await page.keyboard.up('ArrowUp');
  return snapshot(page);
}

export async function runOnboardingAcceptance({ page, result, screenshot }) {
  const entry = { name: 'fresh-authored-start-onboarding', status: 'failed' };
  result.adversarial ??= [];
  result.adversarial.push(entry);
  await enterOriginalHub(page);
  entry.freshHub = await snapshot(page);
  assert.equal(entry.freshHub.player.x, 600);
  await screenshot(page, 'fresh-onboarding-hub', result);
  entry.tutorial = await walkToAuthoredStart(page);
  assert.equal(entry.tutorial.world.level, 0, 'Fresh authored Start must route to Tutorial');
  assert.equal(entry.tutorial.player.bombs, 0);
  assert.equal(entry.tutorial.player.ropes, 2);
  await screenshot(page, 'fresh-onboarding-tutorial', result);
  await page.keyboard.press('Escape');
  await state(page, 'Pause');
  await press(page, 'ArrowDown', 2);
  await page.keyboard.press('Enter');
  await enterOriginalHub(page);
  entry.consumedHub = await snapshot(page);
  await screenshot(page, 'consumed-onboarding-hub', result);
  entry.campaign = await walkToAuthoredStart(page);
  assert.equal(entry.campaign.world.level, 1, 'Consumed authored Start must route to campaign');
  assert.equal(entry.campaign.player.bombs, 4);
  assert.equal(entry.campaign.player.ropes, 4);
  await screenshot(page, 'consumed-onboarding-campaign', result);
  entry.status = 'passed';
  result.checks.push('Fresh authored Start routes to Tutorial with 0 bombs/2 ropes; consumed Start routes to campaign through actual hub walking');
}
