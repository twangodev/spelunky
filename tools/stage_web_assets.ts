#!/usr/bin/env bun
import { copyFile, mkdir, rm, readFile, rename } from 'node:fs/promises';
import { resolve } from 'node:path';

const root = resolve(import.meta.dir, '..');
const destination = resolve(root, 'web/public/assets');
await rm(destination, { recursive: true, force: true });
await mkdir(destination, { recursive: true });
for (const name of ['LICENSE', 'ORIGINAL_README.txt']) {
  await copyFile(resolve(root, 'assets', name), resolve(destination, name));
}

// Version the JS/Wasm pair together, preserving wasm-pack's relative Wasm URL.
const pkg = resolve(root, 'web/public/pkg');
const hash = new Bun.CryptoHasher('sha256');
for (const name of ['spelunky.js', 'spelunky_bg.wasm']) {
  hash.update(name);
  hash.update(await readFile(resolve(pkg, name)));
}
const version = hash.digest('hex');
const versioned = resolve(pkg, version);
await mkdir(versioned, { recursive: true });
for (const name of ['spelunky.js', 'spelunky_bg.wasm']) {
  await rename(resolve(pkg, name), resolve(versioned, name));
}
await Bun.write(resolve(pkg, 'module-path.json'), JSON.stringify(`./pkg/${version}/spelunky.js`));

// wasm-pack metadata and TypeScript declarations are build inputs, not web assets.
for (const name of ['.gitignore', 'package.json', 'spelunky.d.ts', 'spelunky_bg.wasm.d.ts']) {
  await rm(resolve(pkg, name), { force: true });
}
