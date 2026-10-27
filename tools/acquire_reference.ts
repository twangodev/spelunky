#!/usr/bin/env bun
/** Acquire and verify Classic 1.1 reference data; never execute original binaries.
 * Supply --splitter /path/gmksplit.jar --java java for source export.
 * Without --splitter, download/verify originals and extract safe data.
 */
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, statSync } from 'node:fs';
import { dirname, resolve, join, relative, basename, posix, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateRawSync } from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { crc32 } from './lib/crc32.ts';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const DOWNLOADS: Record<string, [string, string]> = {
 'spelunky_1_1_src.zip': ['https://www.derekyu.com/games/spelunky_1_1_src.zip','ad7d23e8de6a1398d245d228869d5c5bbd2c0e72910d66539c21306265cf7014'],
 'spelunky_1_1.zip': ['https://www.derekyu.com/games/spelunky_1_1.zip','572f0a1976e76ebbd295daae651b8b6f00c6b9cf960d3e1c2cc57128d6437727'],
};
const SPLITTER_SHA256 = '319b9b0bf7a724059a18dfbe167b4057ec9d32d8269bf5ff4e0b2a6aed5ad3ed';
export const sha256 = (path: string) => createHash('sha256').update(readFileSync(path)).digest('hex');
function requireZip(condition: boolean, message: string): asserts condition { if (!condition) throw new Error(`Invalid ZIP archive: ${message}`); }
// ZIP's legacy filename encoding is CP437, not the host locale.
const cp437 = 'ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■ ';
function zipName(bytes: Buffer, flags: number): string { return flags & 0x800 ? new TextDecoder('utf-8', {fatal:true}).decode(bytes) : [...bytes].map(byte => byte < 128 ? String.fromCharCode(byte) : cp437[byte - 128]).join(''); }
/** Read stored/deflated ZIP entries without running archive programs or originals. */
export function extractSafe(archive: string, destination: string, suffixes: Set<string>): void {
 const bytes = readFileSync(archive); let eocd = -1;
 for (let i = bytes.length - 22; i >= Math.max(0, bytes.length - 22 - 65535); i--) if (bytes.readUInt32LE(i) === 0x06054b50 && i + 22 + bytes.readUInt16LE(i + 20) === bytes.length) { eocd = i; break; }
 requireZip(eocd >= 0, 'missing end of central directory');
 requireZip(bytes.readUInt16LE(eocd + 4) === 0 && bytes.readUInt16LE(eocd + 6) === 0, 'multipart archives are unsupported');
 const entries = bytes.readUInt16LE(eocd + 10), size = bytes.readUInt32LE(eocd + 12), offset = bytes.readUInt32LE(eocd + 16);
 requireZip(entries === bytes.readUInt16LE(eocd + 8), 'entry count differs between disks');
 requireZip(entries !== 0xffff && size !== 0xffffffff && offset !== 0xffffffff, 'ZIP64 archives are unsupported');
 requireZip(offset + size <= eocd, 'central directory is outside archive');
 let cursor = offset;
 for (let n = 0; n < entries; n++) {
  requireZip(cursor + 46 <= offset + size && bytes.readUInt32LE(cursor) === 0x02014b50, 'invalid central entry');
  const flags = bytes.readUInt16LE(cursor + 8), method = bytes.readUInt16LE(cursor + 10), expectedCrc = bytes.readUInt32LE(cursor + 16), compressedSize = bytes.readUInt32LE(cursor + 20), uncompressedSize = bytes.readUInt32LE(cursor + 24), nameLength = bytes.readUInt16LE(cursor + 28), extraLength = bytes.readUInt16LE(cursor + 30), commentLength = bytes.readUInt16LE(cursor + 32), localOffset = bytes.readUInt32LE(cursor + 42);
  const end = cursor + 46 + nameLength + extraLength + commentLength;
  requireZip(end <= offset + size, 'central entry overruns directory');
  const rawName = bytes.subarray(cursor + 46, cursor + 46 + nameLength), decoded = zipName(rawName, flags);
  // zipfile truncates NULs, but reject them so a malformed name cannot disguise a path.
  requireZip(!decoded.includes('\0'), 'NUL in filename');
  const parts = decoded.split('/').filter(part => part && part !== '.');
  if (decoded.startsWith('/') || parts.includes('..') || decoded.includes('\\') || /^[A-Za-z]:/.test(decoded)) throw new Error(`Unsafe archive path: ${decoded}`);
  cursor = end;
  // PurePosixPath collapses empty/dot components before suffix selection.
  const normalized = parts.join('/');
  if (!suffixes.has(posix.extname(normalized).toLowerCase())) continue;
  requireZip(!(flags & 1), 'encrypted entries are unsupported');
  requireZip(compressedSize !== 0xffffffff && uncompressedSize !== 0xffffffff && localOffset !== 0xffffffff, 'ZIP64 entries are unsupported');
  requireZip(localOffset + 30 <= offset && bytes.readUInt32LE(localOffset) === 0x04034b50, 'invalid local entry');
  const localNameLength = bytes.readUInt16LE(localOffset + 26), localExtraLength = bytes.readUInt16LE(localOffset + 28), dataOffset = localOffset + 30 + localNameLength + localExtraLength;
  requireZip(dataOffset + compressedSize <= offset, 'entry data is outside archive');
  requireZip(bytes.subarray(localOffset + 30, localOffset + 30 + localNameLength).equals(rawName), 'local and central filenames differ');
  requireZip(bytes.readUInt16LE(localOffset + 8) === method && bytes.readUInt16LE(localOffset + 6) === flags, 'local and central compression flags differ');
  const compressed = bytes.subarray(dataOffset, dataOffset + compressedSize);
  requireZip(method === 0 || method === 8, `unsupported compression method ${method}`);
  const data = method === 0 ? compressed : inflateRawSync(compressed, {maxOutputLength: Math.max(1, uncompressedSize)});
  requireZip(data.length === uncompressedSize, 'uncompressed size mismatch'); requireZip(crc32(data) === expectedCrc, 'CRC mismatch');
  const output = join(destination, ...parts); mkdirSync(dirname(output), {recursive:true}); writeFileSync(output, data);
 }
 requireZip(cursor === offset + size, 'central directory size mismatch');
}
function walk(path: string): string[] { return readdirSync(path, {withFileTypes:true}).flatMap(entry => { const child = join(path,entry.name); return entry.isDirectory() ? walk(child) : statSync(child).isFile() ? [child] : []; }).sort(); }
function asciiJson(value: unknown): string { return JSON.stringify(value,null,2).replace(/[\u007f-\uffff]/g, c => '\\u' + c.charCodeAt(0).toString(16).padStart(4,'0')) + '\n'; }
async function main(): Promise<void> {
 let splitter: string | undefined, java = 'java', offline = false, reference = join(ROOT,'references'); const args = process.argv.slice(2);
 for (let i = 0; i < args.length; i++) {
  const arg = args[i], equals = arg.indexOf('='), option = equals < 0 ? arg : arg.slice(0, equals);
  if (arg === '--offline') offline = true;
  else if (['--splitter','--java','--reference-dir'].includes(option)) {
   const inline = equals < 0 ? undefined : arg.slice(equals + 1);
   if (inline === undefined && (!args[i + 1] || args[i + 1].startsWith('-'))) throw new Error(`Missing value for ${option}`);
   const value = inline ?? args[++i];
   if (option === '--java') java = value; else if (option === '--splitter') splitter = resolve(value); else reference = resolve(value);
  } else if (['--help','-h'].includes(arg)) { console.log('Usage: bun tools/acquire_reference.ts [--offline] [--splitter JAR] [--java JAVA] [--reference-dir PATH]\nAcquire and verify Classic 1.1 reference data; never execute original binaries.'); return; }
  else throw new Error(`Unknown or incomplete argument: ${arg}`);
 }
 const original = join(reference,'original'); mkdirSync(original,{recursive:true});
 for (const [name,[url,expected]] of Object.entries(DOWNLOADS)) {
  const output = join(original,name);
  if (!existsSync(output)) { if (offline) throw new Error(`Missing offline archive: ${output}`); const response = await fetch(url,{signal:AbortSignal.timeout(120000)}); if (!response.ok) throw new Error(`Download failed: ${response.status} ${url}`); writeFileSync(output,new Uint8Array(await response.arrayBuffer())); }
  if (sha256(output) !== expected) throw new Error(`Checksum mismatch: ${output}; preserve it for inspection and reacquire explicitly`);
 }
 extractSafe(join(original,'spelunky_1_1_src.zip'),join(original,'source'),new Set(['.gmk','.txt']));
 extractSafe(join(original,'spelunky_1_1.zip'),join(original,'game'),new Set(['.wav','.ogg','.txt','.cfg']));
 if (splitter) {
  if (sha256(splitter) !== SPLITTER_SHA256) throw new Error('Expected the verified GmkSplitter v0.21 JAR; checksum differs');
  const exported = join(reference,'classic-1.1');
  if (existsSync(exported)) console.log(`Keeping existing export: ${exported}`);
  else { const result = spawnSync(java,['-jar',splitter,'--preserve-ids','all','--charset','windows-1252',join(original,'source/spelunky.gmk'),exported],{stdio:'inherit'}); if (result.error) throw result.error; if (result.status !== 0) throw new Error(`GmkSplitter failed (${result.signal ?? result.status})`); }
 }
 const manifest = Object.fromEntries(walk(reference).filter(path => basename(path) !== 'SHA256SUMS.json').map(path => [relative(reference,path).split(sep).join('/'),sha256(path)]));
 // Python sort_keys orders relative paths rather than traversal order.
 const sorted = Object.fromEntries(Object.keys(manifest).sort().map(key => [key,manifest[key]]));
 writeFileSync(join(reference,'SHA256SUMS.json'),asciiJson(sorted)); console.log(`Verified originals and indexed ${Object.keys(manifest).length} reference files`);
}
if (import.meta.main) main().catch(error => { console.error(error instanceof Error ? error.message : error); process.exitCode = 1; });
