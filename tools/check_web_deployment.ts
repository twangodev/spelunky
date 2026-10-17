#!/usr/bin/env bun
import { resolve } from 'node:path';

const root = resolve(import.meta.dir, '../web/dist');
const origin = new URL(process.argv[2] ?? 'https://spelunky.twango.dev');
const modulePath = await Bun.file(resolve(root, 'pkg/module-path.json')).json() as string;
const paths = ['index.html', modulePath.slice(2), modulePath.slice(2).replace(/spelunky\.js$/, 'spelunky_bg.wasm')];
async function verify(): Promise<void> {
  for (const path of paths) {
    const response = await fetch(new URL(path === 'index.html' ? '/' : path, origin), {
      headers: { 'Cache-Control': 'no-cache' }, signal: AbortSignal.timeout(30_000),
    });
    if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
    const hash = (data: ArrayBuffer) => new Bun.CryptoHasher('sha256').update(data).digest('hex');
    if (hash(await response.arrayBuffer()) !== hash(await Bun.file(resolve(root, path)).arrayBuffer())) {
      throw new Error(`${path}: deployed bytes do not match the CI artifact`);
    }
    const cache = response.headers.get('cache-control') ?? '';
    if (path === 'index.html' ? !cache.includes('must-revalidate') : !cache.includes('max-age=31536000') || !cache.includes('immutable')) {
      throw new Error(`${path}: unexpected Cache-Control ${cache}`);
    }
    if (path.endsWith('.wasm') && !response.headers.get('content-type')?.startsWith('application/wasm')) {
      throw new Error('Wasm response has the wrong MIME type');
    }
  }
  const missing = await fetch(new URL('/missing-deployment-check.wasm', origin), { signal: AbortSignal.timeout(30_000) });
  if (missing.status !== 404) throw new Error(`Missing asset returned HTTP ${missing.status}, expected 404`);
}
for (let attempt = 1; ; attempt++) {
  try {
    await verify();
    console.log(`Verified deployed HTML, JS and Wasm match the CI artifact at ${origin.origin}`);
    break;
  } catch (error) {
    if (attempt === 6) throw error;
    console.warn(`Deployment verification attempt ${attempt}: ${error instanceof Error ? error.message : String(error)}`);
    await Bun.sleep(5000);
  }
}
