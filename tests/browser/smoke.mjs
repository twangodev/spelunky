#!/usr/bin/env node
// Actual browser-engine acceptance. JavaScript is test tooling, not game logic.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { access, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import http from 'node:http';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { execFileSync } from 'node:child_process';
import { installMovementTrace, runMovementAcceptance, reachTitleMenu } from './movement.mjs';
import { runAdversarialAcceptance, runOnboardingAcceptance } from './adversarial.mjs';
import { installLightingProfile, installLightingSeedControl, installLightingProbe, runLightingAcceptance } from './lighting.mjs';
import { runCinematicAcceptance } from './cinematics.mjs';
import { installBloodProbe, runBloodAcceptance } from './blood.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const webRoot = path.resolve(process.env.BROWSER_WEB_ROOT || path.join(root, 'web/dist'));
const artifacts = path.resolve(process.env.BROWSER_ARTIFACTS || path.join(root, 'artifacts/browser'));
const tools = process.env.SPELUNKY_TOOLS || path.join(root, 'target/debug/spelunky-tools');
const require = createRequire(import.meta.url);
let playwright;
try {
  playwright = process.env.PLAYWRIGHT_MODULE
    ? await import(pathToFileURL(path.resolve(process.env.PLAYWRIGHT_MODULE)).href)
    : require('playwright');
} catch (error) {
  throw new Error('Install Playwright outside the repository or set PLAYWRIGHT_MODULE to its index.mjs.', { cause: error });
}
const engines = (process.env.BROWSERS || 'chromium,firefox,webkit').split(',');
const report = {
  timestamp: new Date().toISOString(),
  suite: process.env.BROWSER_SUITE || 'full',
  productionRevision: process.env.BROWSER_PRODUCTION_REVISION || null,
  runnerRevision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  scope: 'Automated browser engine acceptance, optionally using a virtual display; no physical 240 Hz display or Safari application validation.',
  engines: [],
};
assert.ok(['full', 'gameplay', 'adversarial', 'movement', 'onboarding', 'lighting', 'cinematics', 'blood'].includes(report.suite), 'BROWSER_SUITE must be full, gameplay, adversarial, movement, onboarding, lighting, cinematics, or blood');
if (report.suite === 'gameplay') report.deferredChecks = ['Source-valid natural dark-room WebGL pixel acceptance runs separately; gameplay success does not imply lighting acceptance.'];
await mkdir(artifacts, { recursive: true });
await access(tools);
report.bundle = {
  wasmSha256: createHash('sha256').update(await readFile(path.join(webRoot, 'pkg/spelunky_bg.wasm'))).digest('hex'),
  htmlSha256: createHash('sha256').update(await readFile(path.join(webRoot, 'index.html'))).digest('hex'),
  bindingsSha256: createHash('sha256').update(await readFile(path.join(webRoot, 'pkg/spelunky.js'))).digest('hex'),
  nativeSha256: createHash('sha256').update(await readFile(tools)).digest('hex'),
};
const native = [];
for (const seed of ['42', '1337']) {
  for (const ticks of [100, 600]) {
    for (const rules of ['classic', 'assisted']) {
      const result = JSON.parse(execFileSync(tools, ['replay', seed, String(ticks), rules], { encoding: 'utf8' }));
      native.push({ seed, ticks, rules, result });
    }
  }
}
report.nativeReplays = native;

let server;
let base = process.env.BASE_URL;
if (!base) {
  const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.json': 'application/json', '.txt': 'text/plain' };
  server = http.createServer(async (request, response) => {
    try {
      const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
      if (pathname === '/favicon.ico') {
        response.writeHead(204).end();
        return;
      }
      const filename = path.resolve(webRoot, `.${pathname === '/' ? '/index.html' : pathname}`);
      if (!filename.startsWith(`${webRoot}${path.sep}`) || !(await stat(filename)).isFile()) {
        response.writeHead(404).end();
        return;
      }
      response.writeHead(200, { 'Content-Type': mime[path.extname(filename)] || 'application/octet-stream', 'Cache-Control': 'no-store' });
      createReadStream(filename).pipe(response);
    } catch {
      response.writeHead(404).end();
    }
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  base = `http://127.0.0.1:${server.address().port}/`;
}
report.url = base;
report.webRoot = webRoot;
report.runnerSha256 = Object.fromEntries(await Promise.all(
  ['smoke.mjs', 'movement.mjs', 'adversarial.mjs', 'lighting.mjs', 'cinematics.mjs', 'blood.mjs'].map(async filename => [
    filename, createHash('sha256').update(await readFile(path.join(root, 'tests/browser', filename))).digest('hex'),
  ]),
));

function instrument() {
  const probe = window.__smoke = { contexts: [], draws: 0, callbacks: 0, contextLost: 0 };
  const getContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (type, ...args) {
    const context = getContext.call(this, type, ...args);
    if (this.id === 'spelunky-canvas' && context && type === 'webgl2') {
      const debug = context.getExtension('WEBGL_debug_renderer_info');
      probe.contexts.push({ type, version: context.getParameter(context.VERSION), renderer: context.getParameter(debug ? debug.UNMASKED_RENDERER_WEBGL : context.RENDERER) });
      this.addEventListener('webglcontextlost', () => probe.contextLost++);
    }
    return context;
  };
  for (const name of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced']) {
    const original = WebGL2RenderingContext.prototype[name];
    WebGL2RenderingContext.prototype[name] = function (...args) {
      if (this.canvas.id === 'spelunky-canvas') probe.draws++;
      return original.apply(this, args);
    };
  }
  const raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = callback => raf(time => {
    probe.callbacks++;
    callback(time);
  });
}

async function ready(page) {
  await page.goto(base, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => document.querySelector('#status')?.hidden || document.querySelector('#status.error'), null, { timeout: 120000 });
  assert.equal(await page.locator('#status').evaluate(element => element.hidden), true, await page.locator('#status').textContent());
  await page.waitForFunction(() => window.__smoke.draws > 2, null, { timeout: 120000 });
  await page.locator('canvas').focus();
  return reachTitleMenu(page);
}

async function press(page, key, count = 1) {
  for (let n = 0; n < count; n++) {
    await page.keyboard.press(key);
    await page.waitForTimeout(45);
  }
  await page.waitForTimeout(150);
}

async function saved(page) {
  return page.evaluate(() => JSON.parse(localStorage.getItem('spelunky-rust.save')));
}

async function screenshot(page, name, result) {
  // Host telemetry is published before its GPU submission. Let the browser
  // present the new scene before capturing a transition's visible evidence.
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const filename = `${result.engine}-${name}.png`;
  const png = await page.screenshot({ path: path.join(artifacts, filename) });
  const hash = createHash('sha256').update(png).digest('hex');
  result.screenshots.push({ filename, bytes: png.length, sha256: hash });
  assert.ok(png.length > 2500, `${name}: unexpectedly small rendered screenshot`);
  return hash;
}

async function canvasClick(page, x, y, options = {}) {
  const box = await page.locator('canvas').boundingBox();
  const scale = Math.min(box.width / 320, box.height / 240);
  await page.mouse.click(box.x + (box.width - 320 * scale) / 2 + x * scale,
    box.y + (box.height - 240 * scale) / 2 + y * scale, options);
  await page.waitForTimeout(100);
}

async function importLevel(page, name, text) {
  const chooser = page.waitForEvent('filechooser', { timeout: 10000 });
  await press(page, 'Control+i');
  await (await chooser).setFiles({ name, mimeType: 'text/plain', buffer: Buffer.from(text) });
  await page.waitForTimeout(500);
}

try {
  for (const engine of engines) {
    const result = { engine, status: 'failed', checks: [], screenshots: [], errors: [], consoleErrors: [], warnings: [] };
    report.engines.push(result);
    let browser;
    let activePage;
    try {
      assert.ok(playwright[engine], `Unknown browser engine ${engine}`);
      const options = { headless: process.env[`${engine.toUpperCase()}_HEADLESS`] !== '0' };
      result.headless = options.headless;
      const executable = process.env[`${engine.toUpperCase()}_EXECUTABLE`];
      if (executable) options.executablePath = executable;
      if (engine === 'chromium') options.args = ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--no-sandbox', '--disable-dev-shm-usage'];
      if (engine === 'firefox') {
        options.firefoxUserPrefs = {
          'webgl.force-enabled': true,
          'webgl.disabled': false,
          'webgl.allow-software-renderer': true,
          'webgl.out-of-process': false,
          'gfx.webrender.software': true,
          'gfx.webrender.all': true,
        };
        options.env = { ...process.env, LIBGL_ALWAYS_SOFTWARE: '1', MOZ_WEBRENDER: '1', MOZ_X11_EGL: '1' };
      }
      browser = await playwright[engine].launch(options);
      result.version = browser.version();
      const context = await browser.newContext({ viewport: { width: 960, height: 720 }, acceptDownloads: true });
      await context.addInitScript(instrument);
      await context.addInitScript(installMovementTrace);
      await context.addInitScript(installBloodProbe);
      if (report.suite === 'lighting') {
        await context.addInitScript(installLightingProfile);
        await context.addInitScript(installLightingSeedControl);
        await context.addInitScript(installLightingProbe);
      }
      const page = await context.newPage();
      activePage = page;
      page.on('pageerror', error => result.errors.push(String(error)));
      page.on('console', message => {
        if (message.type() === 'error') result.consoleErrors.push(message.text());
        if (message.type() === 'warning') result.warnings.push(message.text());
      });
      result.openingRoute = await ready(page);
      assert.match(await page.title(), /Spelunky Classic/);
      assert.match(await page.locator('#attribution').textContent(), /Derek Yu \/ Mossmouth/);
      const license = await page.request.get(new URL('assets/COPYING.txt', base).href);
      assert.equal(license.status(), 200);
      assert.match(await license.text(), /Spelunky/);
      result.graphics = await page.evaluate(() => window.__smoke);
      assert.ok(result.graphics.contexts.length > 0, 'Actual application canvas must use WebGL 2');
      const title = await screenshot(page, 'title', result);
      result.checks.push('Wasm startup and real WebGL 2 draw calls');

      if (!['full', 'gameplay'].includes(report.suite)) {
        if (report.suite === 'adversarial') await runAdversarialAcceptance({ page, result, screenshot });
        else if (report.suite === 'movement') await runMovementAcceptance({ page, result, screenshot });
        else if (report.suite === 'onboarding') await runOnboardingAcceptance({ page, result, screenshot });
        else if (report.suite === 'blood') await runBloodAcceptance({ page, result, screenshot });
        else if (report.suite === 'cinematics') await runCinematicAcceptance({ page, result, screenshot });
        else await runLightingAcceptance({ page, result, screenshot });
        assert.deepEqual(result.errors, []);
        assert.deepEqual(result.consoleErrors.filter(message => !message.includes('favicon.ico')), []);
        result.status = 'passed';
        await context.close();
        continue;
      }

      result.replays = await page.evaluate(async cases => {
        const module = await import('./pkg/spelunky.js');
        return cases.map(({ seed, ticks, rules }) => JSON.parse(module.replay(seed, ticks, rules === 'assisted')));
      }, native);
      assert.deepEqual(result.replays, native.map(entry => entry.result));
      const invalid = await page.evaluate(async () => {
        const module = await import('./pkg/spelunky.js');
        return [['invalid', 1, false], ['42', 100001, false]].map(args => {
          try { module.replay(...args); return false; } catch { return true; }
        });
      });
      assert.deepEqual(invalid, [true, true]);
      result.checks.push('Eight native/Wasm replay results match for both presets; invalid inputs rejected');

      // Settings is fourth from the end, independent of additional title entries.
      await press(page, 'ArrowUp', 4);
      await press(page, 'Enter');
      await press(page, 'ArrowRight');
      await press(page, 'ArrowDown');
      await press(page, 'ArrowRight');
      await press(page, 'ArrowDown');
      await press(page, 'ArrowLeft');
      await press(page, 'ArrowDown', 9); // row 11: jump binding
      await press(page, 'Enter');
      await press(page, 'j');
      const settings = (await saved(page)).settings;
      assert.equal(settings.rules, 'Assisted');
      assert.equal(settings.pixel_snap, true);
      assert.ok(Math.abs(settings.music_volume - 0.5) < 0.001);
      assert.equal(settings.bindings.jump, 'KeyJ');
      await screenshot(page, 'settings', result);
      await press(page, 'Escape');
      result.checks.push('Settings, volume, Assisted preset, pixel snap, and jump rebinding saved');

      await press(page, 'Enter');
      const gameplay = await screenshot(page, 'campaign', result);
      assert.notEqual(gameplay, title);
      await page.keyboard.down('ArrowRight');
      await page.waitForTimeout(160);
      await page.keyboard.press('j');
      await page.waitForTimeout(160);
      await page.keyboard.up('ArrowRight');
      await press(page, 'Escape');
      const pause = await screenshot(page, 'pause', result);
      await page.waitForTimeout(300);
      assert.equal(await screenshot(page, 'pause-stable', result), pause, 'Paused rendered world must remain stable');
      // Campaign pause menu: resume, settings, title.
      await press(page, 'ArrowDown', 2);
      await press(page, 'Enter');
      result.checks.push('Campaign startup, movement/jump input exercise, stable pause, return to title');

      for (const [row, name] of [[1, 'tutorial'], [2, 'original-hub']]) {
        await press(page, 'ArrowDown', row);
        await press(page, 'Enter');
        assert.notEqual(await screenshot(page, name, result), title);
        await press(page, 'Escape');
        await press(page, 'ArrowDown', 2);
        await press(page, 'Enter');
      }
      result.checks.push('Authored tutorial and original hub startup and return to title');

      await press(page, 'F2');
      await canvasClick(page, 20, 40);
      await press(page, 'Control+s');
      let level = (await saved(page)).custom_levels.custom;
      assert.equal(level.split('\n')[11][2], '1');
      await press(page, 'Control+z');
      await press(page, 'Control+s');
      assert.equal((await saved(page)).custom_levels.custom.split('\n')[11][2], '0');
      await press(page, 'Control+y');
      await press(page, 'Control+s');
      level = (await saved(page)).custom_levels.custom;
      assert.equal(level.split('\n')[11][2], '1');
      await screenshot(page, 'editor', result);
      const downloadEvent = page.waitForEvent('download', { timeout: 10000 });
      await press(page, 'Control+e');
      const download = await downloadEvent;
      assert.equal(download.suggestedFilename(), 'custom.lvl');
      assert.equal(await readFile(await download.path(), 'utf8'), level);
      await importLevel(page, 'roundtrip.lvl', level);
      assert.equal((await saved(page)).custom_levels.roundtrip, level);
      const beforeBadImport = (await saved(page)).custom_levels;
      await importLevel(page, 'broken.lvl', 'not a valid Classic level');
      assert.deepEqual((await saved(page)).custom_levels, beforeBadImport);
      await press(page, 'F5');
      await screenshot(page, 'editor-playtest', result);
      await page.keyboard.down('ArrowRight');
      await page.keyboard.press('j');
      await page.waitForTimeout(150);
      await page.keyboard.up('ArrowRight');
      await press(page, 'Escape');
      await press(page, 'ArrowDown', 2);
      await press(page, 'Enter');
      await screenshot(page, 'editor-return', result);
      result.checks.push('Editor paint, undo/redo, save, exact download/import round trip, invalid import preservation, playtest return');

      const persisted = await saved(page);
      await ready(page);
      assert.deepEqual(await saved(page), persisted);
      await press(page, 'F2');
      for (const viewport of [{ width: 480, height: 800 }, { width: 1440, height: 600 }]) {
        await page.setViewportSize(viewport);
        await page.waitForTimeout(300);
        const dimensions = await page.locator('canvas').evaluate(canvas => ({ width: canvas.width, height: canvas.height }));
        assert.equal(dimensions.width, viewport.width);
        assert.equal(dimensions.height, viewport.height);
        await screenshot(page, `editor-${viewport.width}x${viewport.height}`, result);
      }
      result.checks.push('Reload preserves settings/custom levels; portrait and wide canvas resizing');
      result.finalGraphics = await page.evaluate(() => window.__smoke);
      assert.equal(result.finalGraphics.contextLost, 0);
      assert.deepEqual(result.errors, []);
      assert.deepEqual(result.consoleErrors.filter(message => !message.includes('favicon.ico')), []);
      await runMovementAcceptance({ page, result, screenshot });
      await runAdversarialAcceptance({ page, result, screenshot });
      await runCinematicAcceptance({ page, result, screenshot });
      await runBloodAcceptance({ page, result, screenshot });
      assert.deepEqual(result.errors, []);
      assert.deepEqual(result.consoleErrors.filter(message => !message.includes('favicon.ico')), []);
      await context.close();

      const onboarding = await browser.newContext({ viewport: { width: 960, height: 720 } });
      await onboarding.addInitScript(instrument);
      await onboarding.addInitScript(installMovementTrace);
      const onboardingPage = await onboarding.newPage();
      activePage = onboardingPage;
      onboardingPage.on('pageerror', error => result.errors.push(String(error)));
      onboardingPage.on('console', message => {
        if (message.type() === 'error') result.consoleErrors.push(message.text());
        if (message.type() === 'warning') result.warnings.push(message.text());
      });
      await ready(onboardingPage);
      await runOnboardingAcceptance({ page: onboardingPage, result, screenshot });
      assert.deepEqual(result.errors, []);
      assert.deepEqual(result.consoleErrors.filter(message => !message.includes('favicon.ico')), []);
      await onboarding.close();

      if (report.suite === 'full') {
        const lighting = await browser.newContext({ viewport: { width: 960, height: 720 } });
        await lighting.addInitScript(instrument);
        await lighting.addInitScript(installMovementTrace);
        await lighting.addInitScript(installLightingProfile);
        await lighting.addInitScript(installLightingSeedControl);
        await lighting.addInitScript(installLightingProbe);
        const lightingPage = await lighting.newPage();
        activePage = lightingPage;
        lightingPage.on('pageerror', error => result.errors.push(String(error)));
        lightingPage.on('console', message => {
          if (message.type() === 'error') result.consoleErrors.push(message.text());
          if (message.type() === 'warning') result.warnings.push(message.text());
        });
        await ready(lightingPage);
        await runLightingAcceptance({ page: lightingPage, result, screenshot });
        assert.deepEqual(result.errors, []);
        assert.deepEqual(result.consoleErrors.filter(message => !message.includes('favicon.ico')), []);
        await lighting.close();
      }

      // Failure paths run in fresh storage, separate from the happy-path session.
      const corrupt = await browser.newContext();
      await corrupt.addInitScript(instrument);
      await corrupt.addInitScript(installMovementTrace);
      await corrupt.addInitScript(() => localStorage.setItem('spelunky-rust.save', '{invalid'));
      const corruptPage = await corrupt.newPage();
      corruptPage.on('pageerror', error => result.errors.push(String(error)));
      await ready(corruptPage);
      await press(corruptPage, 'F2');
      await press(corruptPage, 'Control+s');
      assert.equal(await corruptPage.evaluate(() => localStorage.getItem('spelunky-rust.save')), '{invalid');
      await screenshot(corruptPage, 'corrupt-storage-preserved', result);
      await corrupt.close();
      const denied = await browser.newContext();
      await denied.addInitScript(instrument);
      await denied.addInitScript(installMovementTrace);
      await denied.addInitScript(() => Object.defineProperty(window, 'localStorage', { get() { throw new DOMException('Test denial', 'SecurityError'); } }));
      const deniedPage = await denied.newPage();
      deniedPage.on('pageerror', error => result.errors.push(String(error)));
      await ready(deniedPage);
      await press(deniedPage, 'F2');
      await press(deniedPage, 'Control+s');
      await screenshot(deniedPage, 'storage-denied', result);
      await denied.close();
      const unavailable = await browser.newContext();
      await unavailable.addInitScript(() => {
        const original = HTMLCanvasElement.prototype.getContext;
        HTMLCanvasElement.prototype.getContext = function (type, ...args) {
          return type === 'webgl2' ? null : original.call(this, type, ...args);
        };
      });
      const unavailablePage = await unavailable.newPage();
      await unavailablePage.goto(base);
      await unavailablePage.locator('#status.error').waitFor();
      assert.match(await unavailablePage.locator('#status').textContent(), /WebGL 2 is required/);
      await screenshot(unavailablePage, 'webgl-unavailable', result);
      await unavailable.close();
      assert.deepEqual(result.errors, []);
      result.checks.push('Corrupt saves preserved, storage denial handled, clear WebGL 2 unavailable message');
      result.status = 'passed';
    } catch (error) {
      result.failure = error.stack || String(error);
      if (activePage && !activePage.isClosed()) {
        result.failureMovementTrace = await activePage.evaluate(() => window.__movementTrace?.slice(-120)).catch(() => null);
        await activePage.screenshot({ path: path.join(artifacts, `${engine}-failure.png`) }).catch(() => {});
      }
    } finally {
      await browser?.close();
      await writeFile(path.join(artifacts, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
      console.log(`${engine}: ${result.status}${result.failure ? `: ${result.failure.split('\n')[0]}` : ''}`);
    }
  }
} finally {
  if (server) await new Promise(resolve => server.close(resolve));
}
if (report.engines.some(engine => engine.status !== 'passed')) process.exitCode = 1;
