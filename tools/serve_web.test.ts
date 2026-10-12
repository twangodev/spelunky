import { afterAll, beforeAll, expect, test } from 'bun:test';
import { mkdtemp, mkdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { bundleHandler } from './serve_web';

let directory: string;
let server: ReturnType<typeof Bun.serve>;
let url: string;
beforeAll(async () => {
  directory = await mkdtemp(join(tmpdir(), 'spelunky-bundle-test-'));
  const bundle = join(directory, 'dist');
  await mkdir(join(bundle, 'pkg'), { recursive: true });
  await mkdir(join(bundle, 'empty'));
  await mkdir(join(bundle, 'nested'));
  await mkdir(join(bundle, 'escaped-index'));
  await writeFile(join(bundle, 'index.html'), '<canvas></canvas>');
  await writeFile(join(bundle, 'nested/index.html'), 'nested');
  await writeFile(join(bundle, 'pkg/spelunky_bg.wasm'), new Uint8Array([0, 97, 115, 109]));
  await writeFile(join(bundle, 'LICENSE'), 'original license');
  await writeFile(join(directory, 'private.txt'), 'outside bundle');
  await symlink(join(directory, 'private.txt'), join(bundle, 'escape.txt'));
  await symlink(join(directory, 'private.txt'), join(bundle, 'escaped-index/index.html'));
  server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch: await bundleHandler(bundle) });
  url = `http://127.0.0.1:${server.port}`;
});
afterAll(async () => { server?.stop(true); if (directory) await rm(directory, { recursive: true, force: true }); });

test('serves the index, Wasm and license with browser-compatible headers', async () => {
  for (const [path, mime, body] of [
    ['/', 'text/html', '<canvas></canvas>'],
    ['/pkg/spelunky_bg.wasm', 'application/wasm', '\0asm'],
    ['/LICENSE', 'text/plain', 'original license'],
  ]) {
    const response = await fetch(`${url}${path}`);
    expect(response.status).toBe(200);
    expect(response.headers.get('content-type')).toBe(mime);
    expect(response.headers.get('cache-control')).toBe('no-store');
    expect(response.headers.get('x-content-type-options')).toBe('nosniff');
    expect(await response.text()).toBe(body);
  }
});
test('HEAD returns the same size and MIME without a response body', async () => {
  const response = await fetch(`${url}/LICENSE`, { method: 'HEAD' });
  expect(response.status).toBe(200);
  expect(response.headers.get('content-length')).toBe('16');
  expect(response.headers.get('content-type')).toBe('text/plain');
  expect(await response.text()).toBe('');
});
test('rejects symlink escapes, index escapes and directory listings', async () => {
  for (const path of ['/escape.txt', '/escaped-index/', '/empty/', '/%2e%2e%2fprivate.txt']) {
    const response = await fetch(`${url}${path}`);
    expect(response.status).toBe(403);
    expect(await response.text()).not.toContain('outside bundle');
  }
});
test('redirects an indexed directory and preserves its query', async () => {
  const response = await fetch(`${url}/nested?a=1`, { redirect: 'manual' });
  expect(response.status).toBe(301);
  expect(response.headers.get('location')).toBe('/nested/?a=1');
  expect(await (await fetch(`${url}/nested/`)).text()).toBe('nested');
});
test('reports missing files and invalid paths and rejects unsupported methods', async () => {
  for (const [path, status] of [['/missing', 404], ['/%GG', 400], ['/%00', 400]] as const) {
    expect((await fetch(`${url}${path}`)).status).toBe(status);
  }
  const response = await fetch(url, { method: 'POST' });
  expect(response.status).toBe(405);
  expect(response.headers.get('allow')).toBe('GET, HEAD');
});
