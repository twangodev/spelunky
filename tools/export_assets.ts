#!/usr/bin/env bun
/** Deterministic official Classic asset export. Requires Bun and no packages. */
import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { basename, extname, join, relative, sep } from 'node:path';
import { parseArgs } from 'node:util';
import { deflateSync, inflateSync } from 'node:zlib';
import { readXml, type XmlElement } from './lib/xml.ts';
import { crc32 } from './lib/crc32.ts';

type Bounds = [number, number, number, number] | null;
type Image = { width: number; height: number; pixels: Buffer };
type Frame = { x: number; y: number; w: number; h: number; bounds?: Bounds;
  mask_offset?: number; mask_stride?: number; mask_width?: number; mask_height?: number; mask_bounds?: Bounds };
type Mask = { shape: string; separate: boolean; alpha_tolerance: number; bounds_mode: string };
type Asset = { id: number; origin: [number, number]; frames: Frame[]; mask?: Mask; bounds?: Bounds; use_as_tileset?: boolean };
type Atlas = { version: number; width: number; height: number; white_pixel: number[];
  collision_file: string; collision_bit_order: string; sprites: Record<string, Asset>; backgrounds: Record<string, Asset> };
type Audio = Record<string, { path: string; bytes: number; sha256: string }>;
type Manifest = { source_archives: Record<string, string>; source_files: Record<string, string>; generated_files: Record<string, string> };
type Xml = XmlElement;

const PNG_SIGNATURE = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
function readPng(path: string): Image {
  const data = readFileSync(path);
  if (!data.subarray(0, 8).equals(PNG_SIGNATURE)) throw new Error(`Not a PNG: ${path}`);
  let offset = 8, width = 0, height = 0, channels = 0;
  const compressed: Buffer[] = [];
  while (offset < data.length) {
    if (offset + 12 > data.length) throw new Error(`Truncated PNG: ${path}`);
    const length = data.readUInt32BE(offset);
    if (offset + 12 + length > data.length) throw new Error(`Truncated PNG chunk: ${path}`);
    const kind = data.subarray(offset + 4, offset + 8);
    const payload = data.subarray(offset + 8, offset + 8 + length);
    if (crc32(Buffer.concat([kind, payload])) !== data.readUInt32BE(offset + 8 + length))
      throw new Error(`Invalid PNG CRC: ${path}`);
    const type = kind.toString('ascii');
    if (type === 'IHDR') {
      if (length !== 13) throw new Error(`Invalid PNG header: ${path}`);
      width = payload.readUInt32BE(0); height = payload.readUInt32BE(4);
      if (payload[8] !== 8 || ![2, 6].includes(payload[9]!) || payload[10] || payload[11] || payload[12])
        throw new Error(`Unsupported PNG encoding: ${path}`);
      channels = payload[9] === 6 ? 4 : 3;
    } else if (type === 'IDAT') compressed.push(payload);
    else if (type === 'IEND') break;
    offset += 12 + length;
  }
  if (!channels) throw new Error(`Missing PNG header: ${path}`);
  const stride = width * channels;
  const raw = inflateSync(Buffer.concat(compressed));
  if (raw.length !== (stride + 1) * height) throw new Error(`Invalid PNG pixel length: ${path}`);
  const pixels = Buffer.alloc(width * height * 4);
  let previous = Buffer.alloc(stride);
  for (let y = 0; y < height; y++) {
    const start = y * (stride + 1), method = raw[start]!;
    if (method > 4) throw new Error(`Invalid PNG filter ${method}: ${path}`);
    const row = Buffer.from(raw.subarray(start + 1, start + 1 + stride));
    for (let x = 0; x < stride; x++) {
      const left = x >= channels ? row[x - channels]! : 0;
      const above = previous[x]!, corner = x >= channels ? previous[x - channels]! : 0;
      let predictor = 0;
      if (method === 1) predictor = left;
      else if (method === 2) predictor = above;
      else if (method === 3) predictor = Math.floor((left + above) / 2);
      else if (method === 4) {
        const p = left + above - corner;
        const a = Math.abs(p - left), b = Math.abs(p - above), c = Math.abs(p - corner);
        predictor = a <= b && a <= c ? left : b <= c ? above : corner;
      }
      row[x] = (row[x]! + predictor) & 255;
    }
    if (channels === 4) row.copy(pixels, y * width * 4);
    else for (let x = 0; x < width; x++) {
      row.copy(pixels, (y * width + x) * 4, x * 3, x * 3 + 3);
      pixels[(y * width + x) * 4 + 3] = 255;
    }
    previous = row;
  }
  return { width, height, pixels };
}
function pngBytes(width: number, height: number, pixels: Buffer): Buffer {
  function chunk(kind: string, payload: Buffer): Buffer {
    const type = Buffer.from(kind), length = Buffer.alloc(4), crc = Buffer.alloc(4);
    length.writeUInt32BE(payload.length); crc.writeUInt32BE(crc32(Buffer.concat([type, payload])));
    return Buffer.concat([length, type, payload, crc]);
  }
  const header = Buffer.alloc(13); header.writeUInt32BE(width, 0); header.writeUInt32BE(height, 4);
  header[8] = 8; header[9] = 6;
  const stride = width * 4, rows = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) pixels.copy(rows, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  return Buffer.concat([PNG_SIGNATURE, chunk('IHDR', header), chunk('IDAT', deflateSync(rows, { level: 9 })), chunk('IEND', Buffer.alloc(0))]);
}
function alphaBounds({ width, height, pixels }: Image, tolerance = 0): Bounds {
  let left = width, top = height, right = -1, bottom = -1;
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    if (pixels[(y * width + x) * 4 + 3]! > tolerance) {
      left = Math.min(left, x); top = Math.min(top, y); right = Math.max(right, x); bottom = Math.max(bottom, y);
    }
  }
  return right < 0 ? null : [left, top, right, bottom];
}
function unionBounds(values: Bounds[]): Bounds {
  const bounds = values.filter((value): value is NonNullable<Bounds> => value !== null);
  return bounds.length ? [Math.min(...bounds.map(b => b[0])), Math.min(...bounds.map(b => b[1])),
    Math.max(...bounds.map(b => b[2])), Math.max(...bounds.map(b => b[3]))] : null;
}
function sha256(path: string): string { return createHash('sha256').update(readFileSync(path)).digest('hex'); }
// Python's sort_keys + ensure_ascii output is part of the committed manifest contract.
function jsonText(value: unknown, depth = 0): string {
  if (typeof value === 'string') return JSON.stringify(value).replace(/[\u007f-\uffff]/g, c => `\\u${c.charCodeAt(0).toString(16).padStart(4, '0')}`);
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  const indent = '  '.repeat(depth), next = indent + '  ';
  if (Array.isArray(value)) return value.length ? `[\n${value.map(v => next + jsonText(v, depth + 1)).join(',\n')}\n${indent}]` : '[]';
  const entries = Object.entries(value as Record<string, unknown>).sort(([a], [b]) => compare(a, b));
  return entries.length ? `{\n${entries.map(([key, item]) => `${next}${jsonText(key)}: ${jsonText(item, depth + 1)}`).join(',\n')}\n${indent}}` : '{}';
}
function writeJson(path: string, value: unknown): void { writeFileSync(path, jsonText(value) + '\n'); }
function same(a: unknown, b: unknown): boolean { return jsonText(a) === jsonText(b); }
function compare(a: string, b: string): number { return a < b ? -1 : a > b ? 1 : 0; }
function files(directory: string, recursive: boolean): string[] {
  const result: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory() && recursive) result.push(...files(path, true));
    else if (entry.isFile()) result.push(path);
  }
  return result.sort(compare);
}
function stem(path: string): string { return basename(path, extname(path)); }
function portableRelative(base: string, path: string): string { return relative(base, path).split(sep).join('/'); }
function suffix(path: string, extension: string): string { return path.slice(0, -extname(path).length) + extension; }
function integer(value: string | undefined): number {
  if (value === undefined || !/^[+-]?\d+$/.test(value.trim())) throw new Error(`Invalid integer: ${value}`);
  const result = Number(value); if (!Number.isSafeInteger(result)) throw new Error(`Integer out of range: ${value}`);
  return result;
}
function child(xml: Xml, tag: string): Xml {
  const value = xml.children.find(node => node.tag === tag);
  if (!value) throw new Error(`Missing XML element: ${xml.tag}/${tag}`);
  return value;
}
function maskOf(root: Xml): Mask {
  const mask = child(root, 'mask'), bounds = child(mask, 'bounds');
  return { shape: child(mask, 'shape').text!, separate: child(mask, 'separate').text === 'true',
    alpha_tolerance: integer(bounds.attrib.alphaTolerance ?? '0'), bounds_mode: bounds.attrib.mode! };
}
function framePaths(path: string): string[] {
  return files(suffix(path, '.images'), false).filter(p => extname(p) === '.png')
    .sort((a, b) => integer(stem(a).trim().split(/\s+/).at(-1)) - integer(stem(b).trim().split(/\s+/).at(-1)));
}
function maskMetadata(images: Image[], mask: Mask, offset: number): Pick<Frame, 'mask_offset' | 'mask_stride' | 'mask_width' | 'mask_height' | 'mask_bounds'> {
  const width = Math.max(...images.map(image => image.width)), height = Math.max(...images.map(image => image.height));
  return { mask_offset: offset, mask_stride: Math.ceil(width / 8), mask_width: width, mask_height: height,
    mask_bounds: unionBounds(images.map(image => alphaBounds(image, mask.alpha_tolerance))) };
}
function appendPreciseMasks(asset: Asset, images: Image[], collision: Buffer[]): void {
  const mask = asset.mask!; if (mask.shape !== 'PRECISE') return;
  const groups = mask.separate ? images.map(image => [image]) : [images];
  let offset = collision.reduce((total, bytes) => total + bytes.length, 0);
  const masks = groups.map(group => {
    const metadata = maskMetadata(group, mask, offset), stride = metadata.mask_stride!;
    const encoded = Buffer.alloc(stride * metadata.mask_height!);
    for (const image of group) for (let y = 0; y < image.height; y++) for (let x = 0; x < image.width; x++) {
      if (image.pixels[(y * image.width + x) * 4 + 3]! > mask.alpha_tolerance)
        encoded[y * stride + Math.floor(x / 8)]! |= 1 << (x % 8);
    }
    collision.push(encoded); offset += encoded.length; return metadata;
  });
  asset.frames.forEach((frame, index) => Object.assign(frame, masks[mask.separate ? index : 0]));
}
function verifyPreciseMasks(asset: Asset, images: Image[], collision: Buffer, expectedOffset: number): number {
  const fields = ['mask_offset', 'mask_stride', 'mask_width', 'mask_height', 'mask_bounds'] as const;
  const mask = asset.mask!;
  if (mask.shape !== 'PRECISE') {
    if (asset.frames.some(frame => fields.some(key => Object.hasOwn(frame, key)))) throw new Error('Non-precise frame unexpectedly has a bitmask');
    return expectedOffset;
  }
  const unionOffset = expectedOffset;
  for (const [index, frame] of asset.frames.entries()) {
    const group = mask.separate ? [images[index]!] : images;
    const expected = maskMetadata(group, mask, mask.separate ? expectedOffset : unionOffset);
    if (fields.some(key => !Object.hasOwn(frame, key) || !same(frame[key], expected[key])))
      throw new Error(`Precise mask metadata mismatch: resource ${asset.id}, frame ${index}`);
    const offset = expected.mask_offset!, stride = expected.mask_stride!, width = expected.mask_width!, height = expected.mask_height!;
    if (offset < 0 || offset + stride * height > collision.length)
      throw new Error(`Precise mask outside collision.bin: resource ${asset.id}, frame ${index}`);
    for (let y = 0; y < height; y++) for (let x = 0; x < stride * 8; x++) {
      const expectedBit = x < width && group.some(image => x < image.width && y < image.height && image.pixels[(y * image.width + x) * 4 + 3]! > mask.alpha_tolerance);
      const actualBit = Boolean(collision[offset + y * stride + Math.floor(x / 8)]! & (1 << (x % 8)));
      if (actualBit !== expectedBit) throw new Error(`Precise mask bit mismatch: resource ${asset.id}, frame ${index}, pixel (${x}, ${y})`);
    }
    if (mask.separate || index === 0) expectedOffset += stride * height;
  }
  return expectedOffset;
}
function verify(source: string, original: string, output: string): void {
  const manifest: Manifest = JSON.parse(readFileSync(join(output, 'manifest.json'), 'utf8'));
  for (const [name, digest] of Object.entries(manifest.generated_files)) if (sha256(join(output, name)) !== digest) throw new Error(`Generated file hash mismatch: ${name}`);
  for (const [name, digest] of Object.entries(manifest.source_files)) if (sha256(join(source, name)) !== digest) throw new Error(`Source file hash mismatch: ${name}`);
  for (const [name, digest] of Object.entries(manifest.source_archives)) if (sha256(join(original, name)) !== digest) throw new Error(`Official archive hash mismatch: ${name}`);
  const metadata: Atlas = JSON.parse(readFileSync(join(output, 'atlas.json'), 'utf8'));
  const { width, height, pixels } = readPng(join(output, 'atlas.png'));
  if (width !== metadata.width || height !== metadata.height) throw new Error('Atlas dimension mismatch');
  if (!pixels.subarray(0, 4).equals(Buffer.from([255, 255, 255, 255]))) throw new Error('Missing white texel');
  if (metadata.collision_file !== 'collision.bin' || metadata.collision_bit_order !== 'lsb0') throw new Error('Unsupported or missing precise collision mask format');
  const collision = readFileSync(join(output, 'collision.bin')); let collisionOffset = 0, checked = 0;
  for (const group of ['Sprites', 'Backgrounds']) for (const path of files(join(source, group), true).filter(path => extname(path) === '.xml')) {
    if (basename(path).startsWith('_')) continue;
    const root = readXml(path); if (!['sprite', 'background'].includes(root.tag)) continue;
    const asset = (group === 'Sprites' ? metadata.sprites : metadata.backgrounds)[stem(path)]!;
    if (asset.id !== integer(root.attrib.id)) throw new Error(`Resource ID mismatch: ${path}`);
    let paths: string[];
    if (root.tag === 'sprite') {
      paths = framePaths(path); const origin = child(root, 'origin');
      if (!same(asset.origin, [integer(origin.attrib.x), integer(origin.attrib.y)])) throw new Error(`Sprite origin mismatch: ${path}`);
      if (!same(asset.mask, maskOf(root))) throw new Error(`Collision mask metadata mismatch: ${path}`);
    } else paths = [suffix(path, '.png')];
    if (paths.length !== asset.frames.length) throw new Error(`Frame count mismatch: ${path}`);
    const images: Image[] = [];
    for (const [index, imagePath] of paths.entries()) {
      const image = readPng(imagePath), record = asset.frames[index]!; images.push(image);
      if (image.width !== record.w || image.height !== record.h) throw new Error(`Frame dimension mismatch: ${imagePath}`);
      if (root.tag === 'sprite' && !same(record.bounds, alphaBounds(image, asset.mask!.alpha_tolerance))) throw new Error(`Collision bounds mismatch: ${imagePath}`);
      for (let row = 0; row < image.height; row++) {
        const target = ((record.y + row) * width + record.x) * 4, start = row * image.width * 4;
        if (!pixels.subarray(target, target + image.width * 4).equals(image.pixels.subarray(start, start + image.width * 4)))
          throw new Error(`Atlas pixel mismatch: ${imagePath}, row ${row}`);
      }
      checked++;
    }
    if (root.tag === 'sprite') {
      if (!same(asset.bounds, unionBounds(asset.frames.map(frame => frame.bounds!)))) throw new Error(`Combined collision bounds mismatch: ${path}`);
      collisionOffset = verifyPreciseMasks(asset, images, collision, collisionOffset);
    }
  }
  if (collisionOffset !== collision.length) throw new Error('Trailing or missing precise collision bytes');
  const audio: Audio = JSON.parse(readFileSync(join(output, 'audio.json'), 'utf8')).sounds;
  for (const record of Object.values(audio)) if (sha256(join(output, record.path)) !== sha256(join(original, 'game/sound', basename(record.path)))) throw new Error(`Audio mismatch: ${record.path}`);
  console.log(`Verified ${checked} images pixel-for-pixel, origins/IDs, ${collision.length} precise collision bytes, ${Object.keys(audio).length} sounds, and all SHA-256 hashes`);
}
function exportAssets(source: string, original: string, output: string, atlasWidth: number): void {
  const frames: { name: string; record: Frame; pixels: Buffer }[] = [], sprites: Record<string, Asset> = {}, backgrounds: Record<string, Asset> = {}, sourceHashes: Record<string, string> = {}, collision: Buffer[] = [];
  for (const path of files(join(source, 'Sprites'), true).filter(path => extname(path) === '.xml')) {
    if (basename(path).startsWith('_')) continue;
    const root = readXml(path); if (root.tag !== 'sprite') continue;
    const origin = child(root, 'origin'), paths = framePaths(path), name = stem(path);
    if (!paths.length) throw new Error(`Sprite has no frames: ${path}`);
    if (Object.hasOwn(sprites, name)) throw new Error(`Duplicate sprite name: ${name}`);
    const asset: Asset = { id: integer(root.attrib.id), origin: [integer(origin.attrib.x), integer(origin.attrib.y)], frames: [], mask: maskOf(root) };
    if (asset.mask!.bounds_mode !== 'AUTO') throw new Error(`Unsupported source mask bounds mode: ${path}`);
    sprites[name] = asset; sourceHashes[portableRelative(source, path)] = sha256(path); const images: Image[] = [];
    for (const [index, imagePath] of paths.entries()) {
      const image = readPng(imagePath); images.push(image);
      const record: Frame = { x: 0, y: 0, w: image.width, h: image.height, bounds: alphaBounds(image, asset.mask!.alpha_tolerance) };
      asset.frames.push(record); frames.push({ name: `sprite:${name}:${String(index).padStart(5, '0')}`, record, pixels: image.pixels });
      sourceHashes[portableRelative(source, imagePath)] = sha256(imagePath);
    }
    asset.bounds = unionBounds(asset.frames.map(frame => frame.bounds!)); appendPreciseMasks(asset, images, collision);
  }
  for (const path of files(join(source, 'Backgrounds'), false).filter(path => extname(path) === '.xml')) {
    if (basename(path).startsWith('_')) continue;
    const root = readXml(path), imagePath = suffix(path, '.png'), image = readPng(imagePath), name = stem(path);
    const record: Frame = { x: 0, y: 0, w: image.width, h: image.height };
    backgrounds[name] = { id: integer(root.attrib.id), origin: [0, 0], frames: [record], use_as_tileset: root.findText('useAsTileset') === 'true' };
    frames.push({ name: `background:${name}`, record, pixels: image.pixels });
    for (const file of [path, imagePath]) sourceHashes[portableRelative(source, file)] = sha256(file);
  }
  let x = 0, y = 2, shelfHeight = 0;
  for (const { name, record } of [...frames].sort((a, b) => b.record.h - a.record.h || b.record.w - a.record.w || compare(a.name, b.name))) {
    if (record.w + 2 > atlasWidth) throw new Error(`Frame too wide for atlas: ${name}`);
    if (x + record.w + 2 > atlasWidth) { x = 0; y += shelfHeight; shelfHeight = 0; }
    record.x = x + 1; record.y = y + 1; x += record.w + 2; shelfHeight = Math.max(shelfHeight, record.h + 2);
  }
  const atlasHeight = 2 ** Math.ceil(Math.log2(y + shelfHeight)), atlas = Buffer.alloc(atlasWidth * atlasHeight * 4);
  atlas.fill(255, 0, 4);
  for (const { record, pixels } of frames) for (let row = 0; row < record.h; row++) {
    pixels.copy(atlas, ((record.y + row) * atlasWidth + record.x) * 4, row * record.w * 4, (row + 1) * record.w * 4);
  }
  mkdirSync(output, { recursive: true }); const collisionBytes = Buffer.concat(collision);
  writeFileSync(join(output, 'atlas.png'), pngBytes(atlasWidth, atlasHeight, atlas)); writeFileSync(join(output, 'collision.bin'), collisionBytes);
  writeJson(join(output, 'atlas.json'), { version: 1, width: atlasWidth, height: atlasHeight, white_pixel: [0, 0], collision_file: 'collision.bin', collision_bit_order: 'lsb0', sprites, backgrounds });
  const audio: Audio = {}, soundDir = join(original, 'game/sound');
  if (!statSync(soundDir).isDirectory()) throw new Error(`Missing official audio directory: ${soundDir}`);
  mkdirSync(join(output, 'audio'), { recursive: true });
  for (const path of files(soundDir, false)) {
    const name = `audio/${basename(path)}`; copyFileSync(path, join(output, name));
    audio[stem(path)] = { path: name, bytes: statSync(path).size, sha256: sha256(path) };
  }
  writeJson(join(output, 'audio.json'), { version: 1, sounds: audio });
  copyFileSync(join(original, 'source/COPYING.txt'), join(output, 'LICENSE'));
  copyFileSync(join(original, 'source/readme.txt'), join(output, 'ORIGINAL_README.txt'));
  const archives = Object.fromEntries(files(original, false).filter(path => extname(path) === '.zip').map(path => [basename(path), sha256(path)]));
  const generatedPaths = ['atlas.png', 'atlas.json', 'collision.bin', 'audio.json', 'LICENSE', 'ORIGINAL_README.txt', ...Object.values(audio).map(record => record.path)].sort(compare);
  writeJson(join(output, 'manifest.json'), { version: 1, source_archives: archives, source_files: sourceHashes,
    generated_files: Object.fromEntries(generatedPaths.map(name => [name, sha256(join(output, name))])), counts: {
      sprites: Object.keys(sprites).length, sprite_frames: Object.values(sprites).reduce((n, asset) => n + asset.frames.length, 0), backgrounds: Object.keys(backgrounds).length,
      audio: Object.keys(audio).length, collision_bytes: collisionBytes.length,
      precise_masks: Object.values(sprites).filter(asset => asset.mask!.shape === 'PRECISE').reduce((n, asset) => n + (asset.mask!.separate ? asset.frames.length : 1), 0) } });
  console.log(`Exported ${Object.keys(sprites).length} sprites, ${frames.length} images, ${collisionBytes.length} precise collision bytes, ${Object.keys(audio).length} sounds; atlas ${atlasWidth}x${atlasHeight}`);
}
function main(): void {
  const { values } = parseArgs({ options: { source: { type: 'string', default: 'references/classic-1.1' }, original: { type: 'string', default: 'references/original' }, output: { type: 'string', default: 'assets' }, 'atlas-width': { type: 'string', default: '2048' }, verify: { type: 'boolean', default: false }, help: { type: 'boolean', short: 'h' } } });
  if (values.help) { console.log('Usage: bun tools/export_assets.ts [--source PATH] [--original PATH] [--output PATH] [--atlas-width 2048] [--verify]\nExport official Classic assets deterministically, or verify all existing pixels, collision bits and hashes without writing.'); return; }
  const width = integer(values['atlas-width']);
  if (width < 2 || !Number.isInteger(Math.log2(width))) throw new Error('--atlas-width must be a power of two greater than one');
  if (values.verify) verify(values.source!, values.original!, values.output!);
  else exportAssets(values.source!, values.original!, values.output!, width);
}
try { main(); } catch (error) { console.error(error instanceof Error ? error.message : String(error)); process.exitCode = 1; }
