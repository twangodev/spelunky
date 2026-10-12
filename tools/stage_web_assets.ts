#!/usr/bin/env bun
import { copyFile, mkdir, rm } from 'node:fs/promises';
import { resolve } from 'node:path';

const root = resolve(import.meta.dir, '..');
const destination = resolve(root, 'web/public/assets');
await rm(destination, { recursive: true, force: true });
await mkdir(destination, { recursive: true });
for (const name of ['LICENSE', 'ORIGINAL_README.txt']) {
  await copyFile(resolve(root, 'assets', name), resolve(destination, name));
}
