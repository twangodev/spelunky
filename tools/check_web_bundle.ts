#!/usr/bin/env bun
import { readdir, readFile, stat } from 'node:fs/promises';
import { resolve } from 'node:path';

const root = resolve(import.meta.dir, '../web/dist');
const manifest = JSON.parse(await readFile(resolve(root, 'pkg/module-path.json'), 'utf8')) as string;
if (!/^\.\/pkg\/[a-f0-9]{64}\/spelunky\.js$/.test(manifest)) {
  throw new Error('Missing content-versioned game module path');
}
const hash = new Bun.CryptoHasher('sha256');
const version = manifest.split('/')[2];
for (const name of ['spelunky.js', 'spelunky_bg.wasm']) {
  hash.update(name);
  hash.update(await readFile(resolve(root, 'pkg', version, name)));
}
if (hash.digest('hex') !== version) throw new Error('Game JS/Wasm content hash mismatch');
for (const file of ['index.html', '_headers', 'assets/LICENSE']) {
  if (!(await Bun.file(resolve(root, file)).exists())) throw new Error(`Missing ${file}`);
}
const packages = (await readdir(resolve(root, 'pkg'), { withFileTypes: true })).filter(entry => entry.isDirectory());
if (packages.length !== 1 || packages[0].name !== version) throw new Error('Unexpected old game packages');
let count = 0;
let bytes = 0;
async function inspect(directory: string): Promise<void> {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (entry.name.startsWith('.')) throw new Error(`Unexpected hidden build output: ${entry.name}`);
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) await inspect(path);
    else if (entry.isFile()) {
      const { size } = await stat(path);
      if (size > 25 * 1024 * 1024) throw new Error(`Asset exceeds Cloudflare's 25 MiB limit: ${path}`);
      count++;
      bytes += size;
    } else throw new Error(`Unexpected non-file asset: ${path}`);
  }
}
await inspect(root);
if (count > 20_000) throw new Error('Bundle exceeds the Workers Free asset count limit');
console.log(`Verified ${count} static assets (${(bytes / 1024 / 1024).toFixed(1)} MiB), game ${version}`);
