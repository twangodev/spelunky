import { afterAll, expect, test } from 'bun:test';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { deflateRawSync } from 'node:zlib';
import { extractSafe } from './acquire_reference.ts';

const directory = mkdtempSync(join(tmpdir(), 'spelunky-reference-zip-tests-'));
afterAll(() => rmSync(directory, { recursive: true, force: true }));

// Independent, table-free fixture checksum. ZIP headers are constructed here,
// rather than obtained from the extractor or an external archive executable.
function fixtureCrc32(bytes: Buffer): number {
 let crc = 0xffffffff;
 for (const byte of bytes) {
  crc ^= byte;
  for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
 }
 return (crc ^ 0xffffffff) >>> 0;
}
function zipEntry(name: string, body: string, method = 8, corruptCrc = false): Buffer {
 const filename = Buffer.from(name), data = Buffer.from(body);
 const compressed = method === 8 ? deflateRawSync(data) : data;
 const checksum = (fixtureCrc32(data) + Number(corruptCrc)) >>> 0;
 const local = Buffer.alloc(30);
 local.writeUInt32LE(0x04034b50);
 local.writeUInt16LE(20, 4);
 local.writeUInt16LE(method, 8);
 local.writeUInt32LE(checksum, 14);
 local.writeUInt32LE(compressed.length, 18);
 local.writeUInt32LE(data.length, 22);
 local.writeUInt16LE(filename.length, 26);
 const central = Buffer.alloc(46);
 central.writeUInt32LE(0x02014b50);
 central.writeUInt16LE(20, 4);
 central.writeUInt16LE(20, 6);
 central.writeUInt16LE(method, 10);
 central.writeUInt32LE(checksum, 16);
 central.writeUInt32LE(compressed.length, 20);
 central.writeUInt32LE(data.length, 24);
 central.writeUInt16LE(filename.length, 28);
 const end = Buffer.alloc(22);
 end.writeUInt32LE(0x06054b50);
 end.writeUInt16LE(1, 8);
 end.writeUInt16LE(1, 10);
 end.writeUInt32LE(central.length + filename.length, 12);
 end.writeUInt32LE(local.length + filename.length + compressed.length, 16);
 return Buffer.concat([local, filename, compressed, central, filename, end]);
}
function fixture(name: string, archive: Buffer): { archive: string; destination: string } {
 const path = join(directory, `${name}.zip`);
 writeFileSync(path, archive);
 return { archive: path, destination: join(directory, name) };
}
const allowed = new Set(['.txt']);

test('extracts stored and deflated entries with exact bytes and case-insensitive suffixes', () => {
 for (const method of [0, 8]) {
  const { archive, destination } = fixture(`valid-${method}`, zipEntry('nested/hello.TXT', 'hello\n', method));
  extractSafe(archive, destination, allowed);
  expect(readFileSync(join(destination, 'nested/hello.TXT'), 'utf8')).toBe('hello\n');
 }
 const { archive, destination } = fixture('filtered', zipEntry('original.exe', 'never execute or extract'));
 extractSafe(archive, destination, allowed);
 expect(existsSync(destination)).toBe(false);
});

test('rejects traversal and absolute paths before filtering unselected executable entries', () => {
 const names = ['../outside.exe', '/absolute.exe', 'nested/../../outside.txt', '..\\outside.exe', 'C:/outside.exe', 'C:\\outside.exe'];
 for (let i = 0; i < names.length; i++) {
  const { archive, destination } = fixture(`unsafe-${i}`, zipEntry(names[i], 'unsafe'));
  expect(() => extractSafe(archive, destination, allowed)).toThrow('Unsafe archive path');
  expect(existsSync(destination)).toBe(false);
 }
 expect(existsSync(join(directory, 'outside.exe'))).toBe(false);
 expect(existsSync(join(directory, 'outside.txt'))).toBe(false);
});

test('rejects corrupted CRCs and mismatched local filenames without writing the selected file', () => {
 const mismatch = zipEntry('name.txt', 'test');
 mismatch[30] = 'N'.charCodeAt(0);
 const cases: [string, Buffer, string][] = [
  ['bad-crc', zipEntry('bad.txt', 'corrupt', 8, true), 'CRC mismatch'],
  ['local-name-mismatch', mismatch, 'filenames differ'],
 ];
 for (const [name, bytes, message] of cases) {
  const { archive, destination } = fixture(name, bytes);
  expect(() => extractSafe(archive, destination, allowed)).toThrow(message);
  expect(existsSync(destination)).toBe(false);
 }
});

test('rejects truncated archives and missing end records', () => {
 for (const [name, bytes] of [['truncated', Buffer.from('broken')], ['missing-eocd', zipEntry('name.txt', 'test').subarray(0, 50)]] as const) {
  const { archive, destination } = fixture(name, bytes);
  expect(() => extractSafe(archive, destination, allowed)).toThrow('Invalid ZIP archive');
  expect(existsSync(destination)).toBe(false);
 }
});
