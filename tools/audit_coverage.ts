#!/usr/bin/env bun
/** Conservative static object coverage; handler evidence does not establish parity. */
import { readFileSync, writeFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { dirname, resolve, relative, basename, extname, join, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const blank = (text: string) => text.replace(/[^\n]/g, ' ');
export function lexicalShadow(text: string, hideLiterals = true): string {
 const chars = text.split(''); let i = 0;
 const rawPattern = /(?:br|r)(#*)"/y, characterPattern = /'(?:\\.|[^'\\\n])'/y;
 while (i < text.length) {
  const start = i; let comment = false;
  if (text.startsWith('//', i)) { comment = true; const end = text.indexOf('\n', i); i = end < 0 ? text.length : end; }
  else if (text.startsWith('/*', i)) {
   comment = true; let depth = 1; i += 2;
   while (i < text.length && depth) { if (text.startsWith('/*', i)) { depth++; i += 2; } else if (text.startsWith('*/', i)) { depth--; i += 2; } else i++; }
   assert.equal(depth, 0, 'Unterminated Rust comment');
  } else {
   rawPattern.lastIndex = i; const raw = rawPattern.exec(text);
   if (raw) { const end = text.indexOf('"' + raw[1], i + raw[0].length); assert(end >= 0, 'Unterminated Rust raw string'); i = end + 1 + raw[1].length; }
   else if (text[i] === '"') { i++; while (i < text.length) { if (text[i] === '\\') i += 2; else if (text[i] === '"') { i++; break; } else i++; } }
   else if (text[i] === "'") { characterPattern.lastIndex = i; const literal = characterPattern.exec(text); if (literal) i += literal[0].length; else { i++; continue; } }
   else { i++; continue; }
  }
  if (hideLiterals || comment) for (let k = start; k < i; k++) chars[k] = text[k] === '\n' ? '\n' : ' ';
 }
 return chars.join('');
}
export function productionText(text: string): [string, [number, number][]] {
 const shadow = lexicalShadow(text); const ranges: [number, number][] = [];
 for (const match of shadow.matchAll(/#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]/g)) {
  if (ranges.length && match.index! < ranges.at(-1)![1]) continue;
  let i = match.index! + match[0].length;
  while (true) { while (i < text.length && /\s/.test(shadow[i])) i++; if (!shadow.startsWith('#[', i)) break; const end = shadow.indexOf(']', i); assert(end >= 0); i = end + 1; }
  const delimiter = /[;{]/.exec(shadow.slice(i)); assert(delimiter, 'cfg(test) item has no boundary'); i += delimiter.index;
  let end = i + 1;
  if (shadow[i] !== ';') { let depth = 1; while (end < text.length && depth) { if (shadow[end] === '{') depth++; else if (shadow[end] === '}') depth--; end++; } assert.equal(depth, 0, 'Unbalanced cfg(test) item'); }
  ranges.push([match.index!, end]);
 }
 for (const [start, end] of ranges.toReversed()) text = text.slice(0, start) + blank(text.slice(start, end)) + text.slice(end);
 return [text, ranges];
}
const escapeRegex = (name: string) => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const definitions = (shadow: string, name: string) => [...shadow.matchAll(new RegExp('^[ \\t]*(?:pub(?:\\([^)]*\\))?\\s+)?fn\\s+' + escapeRegex(name) + '\\s*(?:<[^;{]*?>\\s*)?\\(', 'gm'))];
const lineAt = (text: string, offset: number) => text.slice(0, offset).split('\n').length;
export function functionLine(text: string, name: string): number { const shadow = lexicalShadow(productionText(text)[0]); const matches = definitions(shadow, name); assert.equal(matches.length, 1, `Expected one production definition of ${name}`); return lineAt(shadow, matches[0].index!); }
export function functionBody(text: string, name: string): [number, string] | null {
 const production = productionText(text)[0], shadow = lexicalShadow(production), matches = definitions(shadow, name);
 if (!matches.length) return null; assert.equal(matches.length, 1, `Expected one production definition of ${name}`);
 const start = matches[0].index!, opening = shadow.indexOf('{', start + matches[0][0].length); assert(opening >= 0);
 let end = opening + 1, depth = 1; while (end < shadow.length && depth) { if (shadow[end] === '{') depth++; else if (shadow[end] === '}') depth--; end++; }
 assert.equal(depth, 0, `Unbalanced function ${name}`); return [start, lexicalShadow(production.slice(start, end), false)];
}
export function registeredObjects(text: string, name: string): [string, number][] {
 // Positive matches! eligibility patterns only, never negation/body/guard strings.
 const body = functionBody(text, name); if (!body) return [];
 const [start, code] = body, shadow = lexicalShadow(code), result: [string, number][] = [], guardRanges: [number, number][] = [];
 for (const match of shadow.matchAll(/\bmatches\s*!\s*\(/g)) {
  if (guardRanges.some(([first,last]) => first <= match.index! && match.index! < last)) continue;
  const prefix = /[!\s(]*$/.exec(shadow.slice(0, match.index!))![0], negated = prefix.includes('!');
  const opening = match.index! + match[0].length - 1; let depth = 1, end = opening + 1, comma: number | null = null;
  while (end < shadow.length && depth) { const char = shadow[end]; if ('([{'.includes(char)) depth++; else if (')]}'.includes(char)) depth--; else if (char === ',' && depth === 1 && comma === null) comma = end; end++; }
  if (depth || comma === null) continue;
  const guard = /\bif\b/.exec(shadow.slice(comma + 1, end - 1));
  const patternEnd = guard ? comma + 1 + guard.index : end - 1;
  if (guard) guardRanges.push([patternEnd,end - 1]);
  if (negated) continue;
  const pattern = code.slice(comma + 1, patternEnd);
  if (!/^\s*"o[A-Za-z0-9]+"(?:\s*\|\s*"o[A-Za-z0-9]+")*\s*,?\s*$/.test(pattern)) continue;
  for (const literal of pattern.matchAll(/"(o[A-Za-z0-9]+)"/g)) result.push([literal[1],lineAt(text,start + comma + 1 + literal.index!)]);
 }
 return result;
}
export function objectMatchArms(text: string): [string, number][] {
 // Parse simple object-string patterns; guards/bodies/metadata are not behavior evidence.
 const production = productionText(text)[0], code = lexicalShadow(production,false), shadow = lexicalShadow(production);
 const result: [string,number][] = [], guardRanges: [number,number][] = [];
 for (const match of shadow.matchAll(/\bmatch\b/g)) {
  if (guardRanges.some(([start,end]) => start <= match.index! && match.index! < end)) continue;
  const opening = shadow.indexOf('{',match.index! + match[0].length);
  if (opening < 0 || shadow.slice(match.index! + match[0].length,opening).includes(';')) continue;
  let braces=1, parens=0, brackets=0, angles=0, i=opening+1, start=i, afterArrow=false;
  while (i < shadow.length && braces) {
   const char=shadow[i];
   if (braces===1 && parens===0 && brackets===0 && angles===0 && shadow.startsWith('=>',i)) {
    const candidate=code.slice(start,i), candidateShadow=shadow.slice(start,i), guard=/\bif\b/.exec(candidateShadow);
    if (guard) guardRanges.push([start+guard.index,i]);
    const pattern=guard ? candidate.slice(0,guard.index) : candidate;
    if (/^\s*"o[A-Za-z0-9]+"(?:\s*\|\s*"o[A-Za-z0-9]+")*\s*$/.test(pattern)) for (const literal of pattern.matchAll(/"(o[A-Za-z0-9]+)"/g)) result.push([literal[1],lineAt(text,start+literal.index!)]);
    afterArrow=true;i+=2;continue;
   }
   if (char==='{') braces++;
   else if (char==='}') { braces--; if (braces===1 && afterArrow && parens===0 && brackets===0) { start=i+1;afterArrow=false; } }
   else if (char==='(') parens++;
   else if (char===')') parens--;
   else if (char==='[') brackets++;
   else if (char===']') brackets--;
   else if (char==='<' && (angles || shadow.slice(Math.max(opening,i-2),i)==='::')) angles++;
   else if (char==='>' && angles) angles--;
   else if (char===',' && braces===1 && parens===0 && brackets===0 && angles===0) { start=i+1;afterArrow=false; }
   i++;
  }
 }
 const unique = new Map(result.map(([name,line]) => [JSON.stringify([name,line]),[name,line] as [string,number]]));
 return [...unique.values()].sort((a,b) => a[1]-b[1] || (a[0]<b[0] ? -1 : a[0]>b[0] ? 1 : 0));
}
function inheritedRubbleLine(text: string): number | null { const code = lexicalShadow(productionText(text)[0], false), match = /_\s+if\s+catalog::is_a\([^,\n]+,\s*"oRubblePiece"\)/.exec(code); return match ? lineAt(code, match.index) : null; }
export function withoutFunctionDispatch(text: string, name: string): string { const body = functionBody(text, name); if (!body) return text; const [start, code] = body; return text.slice(0, start) + blank(text.slice(start, start + code.length)) + text.slice(start + code.length); }

if (import.meta.main) {
// Keep the original scanner regressions and exact line/offset assertions.
const fixture = '#[cfg(test)]\nfn helper() { let s = r#"}"#; /* { */ }\n#[cfg(test)]\n#[path = "traversal.rs"]\nmod traversal;\nfn live() { match x { "oLive" => {} } }\n#[cfg(test)]\nmod tests { fn t() { match x { "oTest" => {} } } }\n';
const [filtered, fixtureRanges] = productionText(fixture);
assert(filtered.includes('"oLive"') && !filtered.includes('"oTest"') && !filtered.includes('fn helper'));
assert.equal(filtered.length, fixture.length); assert.equal(filtered.split('\n').length, fixture.split('\n').length); assert.equal(fixtureRanges.length, 3);
const anchorFixture = '#[cfg(test)]\nfn is_solid() {}\n// pub fn is_solid() {}\nfn helper() {}\npub fn is_solid(name: &str) -> bool { true }\n', anchorLine = functionLine(anchorFixture, 'is_solid');
assert(anchorFixture.split('\n')[anchorLine - 1].startsWith('pub fn is_solid('));
const prefix = '// Unrelated preceding change\nfn another_helper() {}\n\n';
assert.equal(functionLine(prefix + anchorFixture, 'is_solid'), anchorLine + prefix.split('\n').length - 1);
const registrationFixture = '#[cfg(test)]\nfn handles(x: &str) -> bool { matches!(x, "oTest") }\npub(super) fn handles(x: &str) -> bool {\n /* "oComment" { */ let brace = "}";\n matches!(x, "oLive" | "oOther")\n}\nfn later() { let _ = "oUnrelated"; }\n';
assert.deepEqual(registeredObjects(registrationFixture, 'handles'), [['oLive', 5], ['oOther', 5]]);
assert.deepEqual(registeredObjects(prefix + registrationFixture, 'handles'), [['oLive', 8], ['oOther', 8]]);
const rubbleFixture = '// _ if catalog::is_a(&fake, "oRubblePiece")\nfn live() { match object {\n _ if catalog::is_a(&object, "oRubblePiece") => rubble(), _ => {}\n} }\n';
assert.equal(inheritedRubbleLine(rubbleFixture), 3); assert.equal(inheritedRubbleLine(prefix + rubbleFixture.replace('&object,', '&e.object,')), 6);
const labelFixture = 'fn source_type() { let brace = r#"}"#; /* { */ match x { "oLabelOnly" => Some("Label"), "oDual" => Some("Dual"), _ => None } }\nfn live() { match x { "oDual" => tick(), _ => {} } }\n';
const labelShadow = lexicalShadow(labelFixture, false), labelDispatch = withoutFunctionDispatch(labelShadow, 'source_type');
assert.equal(labelDispatch.length, labelShadow.length); assert.equal(labelDispatch.split('\n').length, labelShadow.split('\n').length);
assert(!labelDispatch.includes('"oLabelOnly"') && labelDispatch.match(/"oDual"/g)?.length === 1); assert.equal(labelDispatch.indexOf('"oDual"'), labelShadow.lastIndexOf('"oDual"')); assert.equal(withoutFunctionDispatch(labelShadow, 'absent'), labelShadow);

function walk(path: string): string[] { return readdirSync(path, {withFileTypes:true}).flatMap(entry => { const child = join(path, entry.name); return entry.isDirectory() ? walk(child) : entry.isSymbolicLink() && statSync(child).isDirectory() ? [] : [child]; }).sort(); }
const pathCompare = (a: string, b: string) => { const aa = a.split(sep), bb = b.split(sep); for (let i = 0; i < Math.min(aa.length, bb.length); i++) if (aa[i] !== bb[i]) return aa[i] < bb[i] ? -1 : 1; return aa.length - bb.length; };
const sources = new Map(walk(join(root, 'crates/core/src')).filter(p => p.endsWith('.rs')).sort(pathCompare).map(p => [p, readFileSync(p, 'utf8')]));
const excluded = new Set<string>();
for (const [p, source] of sources) for (const [start, end] of productionText(source)[1]) {
 const item = source.slice(start, end), module = /\bmod\s+(\w+)\s*;/.exec(item); if (!module) continue;
 const path = /#\[path\s*=\s*"([^"]+)"\]/.exec(item);
 if (path) excluded.add(resolve(dirname(p), path[1]));
 else { const base = ['mod.rs','lib.rs','main.rs'].includes(basename(p)) ? dirname(p) : join(dirname(p), basename(p, extname(p))); excluded.add(resolve(base, module[1] + '.rs')); excluded.add(resolve(base, module[1], 'mod.rs')); }
}
const args = process.argv.slice(2); let check = false, output = join(root, 'assets/coverage.json');
for (let i = 0; i < args.length; i++) {
 const arg = args[i];
 if (arg === '--check') check = true;
 else if (arg === '--output' || arg.startsWith('--output=')) {
  const inline = arg.startsWith('--output=') ? arg.slice('--output='.length) : undefined;
  if (inline === undefined && (!args[i + 1] || args[i + 1].startsWith('-'))) throw new Error('Missing value for --output');
  output = resolve(inline ?? args[++i]);
 } else if (['--help','-h'].includes(arg)) { console.log('Usage: bun tools/audit_coverage.ts [--check] [--output PATH]\nConservative static object coverage; handler evidence does not establish parity.'); process.exit(0); }
 else throw new Error(`Unknown or incomplete argument: ${arg}`);
}
const index = JSON.parse(readFileSync(join(root, 'references/source-index.json'), 'utf8'));
type Handler = { module: string; line: number; kind: string };
const handlers = new Map<string, Handler[]>();
const add = (name: string, handler: Handler) => { if (!handlers.has(name)) handlers.set(name, []); handlers.get(name)!.push(handler); };
for (const [p, source] of sources) {
 if (excluded.has(resolve(p))) continue;
 const text = lexicalShadow(productionText(source)[0], false), portablePath = p.split(sep).join('/');
 const dispatchText = p === join(root, 'crates/core/src/items/explosives.rs') ? withoutFunctionDispatch(text, 'source_type') : text;
 for (const [name,line] of objectMatchArms(dispatchText)) add(name, { module:relative(root,p).split(sep).join('/'), line, kind:'explicit_object_match_arm' });
 const registrations: [string, string][] = [];
 if (portablePath.includes('/enemies/')) registrations.push(['matches','enemy_dispatch_registration']);
 if (basename(p) === 'enemies.rs') registrations.push(['is_actor','enemy_dispatch_registration']);
 if (['items.rs','traps.rs','world_events.rs'].includes(basename(p)) || portablePath.includes('/items/') || portablePath.includes('/traps/')) registrations.push(['handles','instance_event_family_registration']);
 if (basename(p) === 'items.rs') registrations.push(['is_item','generic_item_body_registration']);
 if (basename(p) === 'traps.rs') registrations.push(['is_hazard','hazard_dispatch_registration']);
 for (const [fn, kind] of registrations) for (const [name, line] of registeredObjects(text, fn)) add(name, { module: relative(root, p).split(sep).join('/'), line, kind });
}
const equivalents: Record<string, string[]> = {
 'crates/core/src/game.rs': ['oEntrance'],
 'crates/core/src/enemies/temple.rs': ['oOlmec'],
 'crates/core/src/progression.rs': ['oExit','oXMarket','oXGold','oGoldDoor','oTunnelMan','oMoaiInside','oFinalBoss'],
 'crates/core/src/items.rs': ['oWhip','oWhipPre'],
 'crates/app/src/scene.rs': ['oMsgSign','oLamp','oLampRed','oTikiTorch','oHintHand'],
 'crates/core/src/generation/terrain.rs': ['oGold','oGoldBig','oFrozenCaveman'],
 'crates/core/src/traps/curses.rs': ['oBall','oBall2','oChain','oChain2'],
 'crates/core/src/traps.rs': ['oGiantTikiHead'],
 'crates/core/src/hub.rs': ['oBatTarget','oBasket','oTarget','oRim','oRimDeflect'],
 'crates/core/src/items/weapons.rs': ['oShotgunBlastLeft','oShotgunBlastRight'],
 'crates/core/src/player.rs': ['oPlayer1','oParachute','oParaUsed','oCape','oTreeBranchTile'],
};
const tokens: Record<string, string> = {oGold:'gold_chunks',oGoldBig:'gold_nuggets',oFrozenCaveman:'frozen_caveman',oCape:'Cape',oParachute:'Parachute',oParaUsed:'Parachute',oTreeBranchTile:'oPlatform',oFinalBoss:'boss_exit'};
const reviewedFunctions: Record<string,Record<string,string[]>> = {
 'crates/core/src/items.rs': {oWhip:['step_instance_with_motion','step_instance']},
 'crates/core/src/enemies/temple.rs': {oOlmec:['create']},
 'crates/core/src/traps.rs': {oGiantTikiHead:['alarm']},
 'crates/core/src/traps/curses.rs': {oBall:['step_instance_with_motion','step_instance'],oBall2:['step_instance_with_motion','step_instance'],oChain:['chain_step'],oChain2:['chain_step']},
 'crates/core/src/progression.rs': {oXGold:['complete']},
};
for (const [module,names] of Object.entries(equivalents)) {
 const text=productionText(readFileSync(join(root,module),'utf8'))[0];
 for (const name of names) {
  const functions=reviewedFunctions[module]?.[name];
  if (functions) { const present=functions.filter(fn => functionBody(text,fn)!==null); assert(present.length,`No reviewed behavior function in ${module}: ${functions}`);add(name,{module,line:functionLine(text,present[0]),kind:'manually_reviewed_shared_behavior'});continue; }
  const match=new RegExp('"'+(tokens[name]??name)+'"').exec(text);if (match) add(name,{module,line:lineAt(text,match.index),kind:'manually_reviewed_shared_behavior'});
 }
}
const stateAdapters: [string,string,string[]][] = [['crates/core/src/opening.rs','begin_intro',['oIntro']],['crates/core/src/opening.rs','begin_title',['oTitle']],['crates/core/src/opening.rs','actor_step',['oPDummy3','oPDummy4']],['crates/core/src/hub.rs','activate_portal',['oXStart']],['crates/core/src/hub.rs','scores_step',['oHighscores']]];
for (const [module, fn, names] of stateAdapters) { const line = functionLine(sources.get(join(root, module))!, fn); for (const name of names) add(name, { module, line, kind:'manually_reviewed_state_adapter' }); }
const objects = new Map<string, any>(index.objects.map((o: any) => [o.name, o]));
function isa(name: string, parent: string): boolean { for (let i = 0; i < 453; i++) { if (name === parent) return true; if (!objects.has(name)) return false; name = objects.get(name).parent; } return false; }
const abstract = new Set('oBackdrop oCharacter oDetritus oDrawnSprite oEnemy oForeground oFrill oItem oLadder oMenu oMoveableSolid oMovingSolid oPlatform oRubblePiece oSolid oTreasure oWater'.split(' '));
const rootHost = new Set('oCredits1 oCredits2 oCursObj oEnd oEnd2 oEnd3 oEndCustom oGame oGamepad oGlobals oHighscores oIntro oJoyConfig oKeyConfig oLevel oLevelEditor oLoadLevel oMoonRoom oMusic oRoom oScreen oStarsRoom oSunRoom oTitle oTransition'.split(' '));
const rubbleModule = 'crates/core/src/world_events.rs', rubbleLine = inheritedRubbleLine(sources.get(join(root, rubbleModule))!);
if (rubbleLine) for (const name of objects.keys()) if (isa(name, 'oRubblePiece')) add(name, { module:rubbleModule, line:rubbleLine, kind:'inherited_rubble_family_handler' });
const records: any[] = [], terrainAdapterLine = functionLine(sources.get(join(root, 'crates/core/src/catalog.rs'))!, 'is_solid');
for (const o of [...index.objects].sort((a: any,b: any) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0)) {
 const n = o.name, path = o.path; let ev = handlers.get(n) ?? [], category = 'gameplay';
 if (path.includes('/Init/') || path.includes('/Level Editor/') || path.includes('/Other/Title/') || rootHost.has(n)) category = 'ui';
 if (['oBatTarget','oBasket','oTarget','oRim','oRimDeflect'].includes(n)) category = 'gameplay';
 else if (path.includes('/Other/Intro/') || path.includes('/Other/End/') || path.includes('/supersound/')) category = 'visual';
 else if (path.includes('/Effects/') && !ev.length) category = 'visual';
 else if (['oBlack','oBlackBG','oBlackFadeUp','oCaveBG','oCaveBG2','oCaveBGEntrance','oCaveUp','oLeaves','oLeavesTile','oSplash','oSprite','oYellHelp','oDamselKiss','oMattockHead'].includes(n)) category = 'visual';
 else if (n.startsWith('oMenu') || ['oPageDown','oPageUp','oWalDamsel'].includes(n) || n.startsWith('oPDummy')) category = 'ui';
 else if (path.includes('/Basis/') || ['oSprite','oBricks'].includes(n)) category = 'behavior';
 if (n === 'oPlayer1') ev = [['crates/core/src/player.rs','tick','player_tick_replacement'],['crates/core/src/dispatcher.rs','player_step','source_player_phase_adapter']].map(([module,fn,kind]) => ({module,line:functionLine(sources.get(join(root,module))!,fn),kind}));
 let status: string, reason: string;
 if (ev.length) { status = 'partial_handler_present'; reason = 'Direct dispatch or registered generic handler exists; source behavior and runtime acceptance are not complete.'; }
 else if (abstract.has(n)) { status = 'source_abstraction'; reason = 'Source parent/group; requires inherited behavior coverage in concrete descendants.'; }
 else if (category === 'ui') { status = 'host_replacement_unverified'; reason = 'Original object is not directly dispatched; host editor/menu/profile behavior needs separate acceptance.'; }
 else if (category === 'visual') { status = 'asset_only_unverified'; reason = 'Original asset is catalogued; dynamic event/draw behavior has no direct audited handler.'; }
 else if (isa(n,'oSolid') || isa(n,'oLadder') || ['oVine','oVineTop','oLadderTop','oTreeBranch'].includes(n)) { status = 'partial_static_tile_adapter'; reason = 'Generic terrain/ladder collision exists; object-specific source events remain unverified.'; ev = [{module:'crates/core/src/catalog.rs',line:terrainAdapterLine,kind:'source_hierarchy_terrain_adapter'}]; }
 else { status = 'no_direct_handler_found'; reason = 'No audited object dispatch exists; source gameplay behavior needs implementation or documented equivalent.'; }
 if (status === 'no_direct_handler_found' && ['oEntranceCheck','oRopeBag'].includes(n)) reason += ' Preserved Objects/Scripts contain this name only in resource lists, with no producer references; the unused source resource remains unverified.';
 if (status === 'host_replacement_unverified') { const module = path.includes('/Level Editor/') ? 'crates/app/src/editor_ui.rs' : path.includes('/Init/') || ['oJoyConfig','oKeyConfig','oGamepad'].includes(n) ? 'crates/app/src/settings_ui.rs' : 'crates/app/src/host.rs'; ev = [{module,line:1,kind:'host_equivalent_requires_behavior_acceptance'}]; }
 records.push({object:n,source_id:o.id,source_path:path,source_events:o.events,category,status,parity_verified:false,handlers:ev,note:reason});
}
const count = (key: string) => { const counts: Record<string,number> = {}; for (const record of records) counts[record[key]] = (counts[record[key]] ?? 0) + 1; return counts; };
const result = {schema_version:1,reference:'Official Classic 1.1, 453 original objects',review:'Static coverage inventory; handler presence is not feature completion or executable parity. Snapshot of checked-in implementation; regenerate after dispatch changes.',method:'Direct Rust object match arms, explicit enemy/item/hazard/event-family registration, inherited rubble dispatch and manually reviewed room/state adapters; excludes comments, test code, spawn-only strings and arbitrary object mentions. Source Collision declarations establish event selection only and are not implementation evidence. Host replacements and source abstractions are separately classified. Generic callbacks and indirect behavior can require manual follow-up.',counts:{total:records.length,by_status:count('status'),by_category:count('category')},objects:records};
function sortedJson(value: any): any { if (Array.isArray(value)) return value.map(sortedJson); if (value && typeof value === 'object') return Object.fromEntries(Object.keys(value).sort().map(key => [key, sortedJson(value[key])])); return value; }
const encoded = JSON.stringify(sortedJson(result), null, 2).replace(/[\u007f-\uffff]/g, c => '\\u' + c.charCodeAt(0).toString(16).padStart(4,'0')) + '\n';
assert.equal(records.length, 453); assert.equal(new Set(records.map(r => r.object)).size, records.length); assert.equal(new Set(records.map(r => r.source_id)).size, records.length);
for (const record of records) for (const handler of record.handlers) { const module = join(root, handler.module); assert(existsSync(module) && statSync(module).isFile(), JSON.stringify(handler)); const lines = readFileSync(module,'utf8').split(/\r\n|[\n\r\v\f\x1c-\x1e\u0085\u2028\u2029]/); if (lines.at(-1) === '') lines.pop(); assert(handler.line > 0 && handler.line <= lines.length, JSON.stringify(handler)); }
if (check) { if (!existsSync(output) || readFileSync(output,'utf8') !== encoded) { console.error('Coverage inventory is stale; run bun tools/audit_coverage.ts and review the changes.'); process.exitCode = 1; } else console.log('Coverage inventory matches source: 453 unique source objects; handler anchors valid.'); }
else { writeFileSync(output, encoded); console.log(result.counts); }

}
