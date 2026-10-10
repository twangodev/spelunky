#!/usr/bin/env python3
"""Conservative static object coverage; handler evidence does not establish parity.

Run after dispatch changes, then manually review the shared-equivalent mappings.
--check validates that the checked-in inventory matches the current source tree.
"""
from pathlib import Path
import argparse, re, json, collections
root = Path(__file__).resolve().parents[1]

def blank(text):
 return re.sub(r'[^\n]', ' ', text)

def lexical_shadow(text):
 """Keep Rust punctuation/offsets while hiding comments and literal contents."""
 chars=list(text);i=0
 while i<len(text):
  start=i
  if text.startswith('//',i):
   end=text.find('\n',i);i=len(text) if end<0 else end
  elif text.startswith('/*',i):
   depth=1;i+=2
   while i<len(text) and depth:
    if text.startswith('/*',i):depth+=1;i+=2
    elif text.startswith('*/',i):depth-=1;i+=2
    else:i+=1
   assert depth==0, 'Unterminated Rust comment'
  else:
   raw=re.match(r'(?:br|r)(#*)"',text[i:])
   if raw:
    end=text.find('"'+raw.group(1),i+len(raw.group()))
    assert end>=0, 'Unterminated Rust raw string'
    i=end+1+len(raw.group(1))
   elif text[i]=='"':
    i+=1
    while i<len(text):
     if text[i]=='\\':i+=2
     elif text[i]=='"':i+=1;break
     else:i+=1
   elif text[i]=="'" and (literal:=re.match(r"'(?:\\.|[^'\\\n])'",text[i:])):
    i+=len(literal.group())
   else:i+=1;continue
  chars[start:i]=blank(text[start:i])
 return ''.join(chars)

def production_text(text):
 """Remove cfg(test) items, preserving later production code and line numbers.

 Handles inline test modules, early helper functions and external module
 declarations. This is a conservative source scanner, not a Rust compiler.
 """
 shadow=lexical_shadow(text);ranges=[]
 for match in re.finditer(r'#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]',shadow):
  if ranges and match.start()<ranges[-1][1]:continue
  i=match.end()
  while True:
   while i<len(text) and shadow[i].isspace():i+=1
   if not shadow.startswith('#[',i):break
   end=shadow.find(']',i);assert end>=0
   i=end+1
  delimiter=re.search(r'[;{]',shadow[i:]);assert delimiter, 'cfg(test) item has no boundary'
  i+=delimiter.start()
  if shadow[i]==';':end=i+1
  else:
   depth=1;end=i+1
   while end<len(text) and depth:
    if shadow[end]=='{':depth+=1
    elif shadow[end]=='}':depth-=1
    end+=1
   assert depth==0, 'Unbalanced cfg(test) item'
  ranges.append((match.start(),end))
 for start,end in reversed(ranges):text=text[:start]+blank(text[start:end])+text[end:]
 return text,ranges

# Regressions: an early helper/external test module must not hide later handlers;
# braces inside literals/comments must not end an excluded item.
fixture='#[cfg(test)]\nfn helper() { let s = r#"}"#; /* { */ }\n#[cfg(test)]\n#[path = "traversal.rs"]\nmod traversal;\nfn live() { match x { "oLive" => {} } }\n#[cfg(test)]\nmod tests { fn t() { match x { "oTest" => {} } } }\n'
filtered,fixture_ranges=production_text(fixture)
assert '"oLive"' in filtered and '"oTest"' not in filtered and 'fn helper' not in filtered
assert len(filtered)==len(fixture) and filtered.count('\n')==fixture.count('\n') and len(fixture_ranges)==3

sources={p:p.read_text() for p in sorted((root/'crates/core/src').rglob('*.rs'))}
excluded=set()
for p,source in sources.items():
 _,ranges=production_text(source)
 for start,end in ranges:
  item=source[start:end]
  module=re.search(r'\bmod\s+(\w+)\s*;',item)
  if not module:continue
  path=re.search(r'#\[path\s*=\s*"([^"]+)"\]',item)
  if path:excluded.add((p.parent/path.group(1)).resolve())
  else:
   base=p.parent if p.name in {'mod.rs','lib.rs','main.rs'} else p.parent/p.stem
   excluded.update({(base/(module.group(1)+'.rs')).resolve(),(base/module.group(1)/'mod.rs').resolve()})

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
index=json.loads((root/'references/source-index.json').read_text())
handlers=collections.defaultdict(list)
for p,source in sources.items():
 if p.resolve() in excluded:continue
 text,_=production_text(source)
 text=re.sub(r'//[^\n]*','',text)
 # Explicit object-valued match arms are behavior dispatch evidence; arbitrary
 # mentions, spawn strings, comments and tests are intentionally excluded.
 for m in re.finditer(r'((?:"o[A-Za-z0-9]+"\s*\|\s*)*"o[A-Za-z0-9]+")\s*(?:if\s+[^\n=>]+)?\s*=>',text):
  for name in re.findall(r'"(o[A-Za-z0-9]+)"',m.group(1)):
   handlers[name].append({'module':p.relative_to(root).as_posix(),'line':text[:m.start()].count('\n')+1,'kind':'explicit_object_match_arm'})
 # Enemy matcher is the parent dispatch registration contract. An empty tick
 # arm remains partial, never parity verified.
 if '/enemies/' in str(p):
  m=re.search(r'pub fn matches\(object: &str\).*?\n}',text,re.S)
  if m:
   for name in re.findall(r'"(o[A-Za-z0-9]+)"',m.group()):
    handlers[name].append({'module':p.relative_to(root).as_posix(),'line':text[:m.start()].count('\n')+1,'kind':'enemy_dispatch_registration'})
 # Base item-body predicates are actual ownership of common motion/pickup.
 if p.name=='items.rs':
  m=re.search(r'pub fn is_item\(.*?\n}',text,re.S)
  if m:
   for name in re.findall(r'"(o[A-Za-z0-9]+)"',m.group()):
    handlers[name].append({'module':p.relative_to(root).as_posix(),'line':text[:m.start()].count('\n')+1,'kind':'generic_item_body_registration'})
 if p.name=='traps.rs':
  m=re.search(r'fn is_hazard\(.*?\n}',text,re.S)
  if m:
   for name in re.findall(r'"(o[A-Za-z0-9]+)"',m.group()):
    handlers[name].append({'module':p.relative_to(root).as_posix(),'line':text[:m.start()].count('\n')+1,'kind':'hazard_dispatch_registration'})
# Manually audited equivalents that use if-based selectors or shared tick code.
equivalents = {
 'crates/core/src/game.rs': ['oEntrance'],
 'crates/core/src/progression.rs': ['oExit','oXMarket','oXGold','oGoldDoor','oTunnelMan','oMoaiInside','oFinalBoss'],
 'crates/core/src/items.rs': ['oWhip','oWhipPre'],
 'crates/app/src/scene.rs': ['oMsgSign','oLamp','oLampRed','oTikiTorch','oHintHand'],
 'crates/core/src/generation/terrain.rs': ['oGold','oGoldBig','oFrozenCaveman'],
 'crates/core/src/traps/curses.rs': ['oBall','oBall2','oChain','oChain2'],
 'crates/core/src/traps.rs': ['oGiantTikiHead'],
 'crates/core/src/hub.rs': ['oBatTarget','oBasket','oTarget','oRim','oRimDeflect'],
 'crates/core/src/items/weapons.rs': ['oShotgunBlastLeft','oShotgunBlastRight'],
 'crates/core/src/player.rs': ['oPlayer1','oParachute','oParaUsed','oCape','oTreeBranchTile'],
}
for module,names in equivalents.items():
 text,_=production_text((root/module).read_text())
 for name in names:
  token={'oGold':'gold_chunks','oGoldBig':'gold_nuggets','oFrozenCaveman':'frozen_caveman','oCape':'Cape','oParachute':'Parachute','oParaUsed':'Parachute','oTreeBranchTile':'oPlatform','oFinalBoss':'boss_exit'}.get(name,name)
  match=re.search(r'"'+token+r'"',text)
  if match:handlers[name].append({'module':module,'line':text[:match.start()].count('\n')+1,'kind':'manually_reviewed_shared_behavior'})

objects={o['name']:o for o in index['objects']}
def isa(name,parent):
 for _ in range(453):
  if name==parent:return True
  if name not in objects:return False
  name=objects[name]['parent']
 return False
abstract=set('oBackdrop oCharacter oDetritus oDrawnSprite oEnemy oForeground oFrill oItem oLadder oMenu oMoveableSolid oMovingSolid oPlatform oRubblePiece oSolid oTreasure oWater'.split())
root_host=set('oCredits1 oCredits2 oCursObj oEnd oEnd2 oEnd3 oEndCustom oGame oGamepad oGlobals oHighscores oIntro oJoyConfig oKeyConfig oLevel oLevelEditor oLoadLevel oMoonRoom oMusic oRoom oScreen oStarsRoom oSunRoom oTitle oTransition'.split())
# Original shared rubble-family update is a real inheritance dispatcher.
rubble_module = 'crates/core/src/world_events.rs'
rubble_text,_ = production_text((root / rubble_module).read_text())
rubble_dispatch = re.search(r'_ if catalog::is_a\(&e.object, "oRubblePiece"\)', rubble_text)
if rubble_dispatch:
 for name in objects:
  if isa(name,'oRubblePiece'):
   handlers[name].append({'module':rubble_module,'line':rubble_text[:rubble_dispatch.start()].count('\n')+1,'kind':'inherited_rubble_family_handler'})

records=[]
for o in sorted(index['objects'],key=lambda x:x['name']):
 n=o['name'];path=o['path'];ev=handlers[n]
 category='gameplay'
 if '/Init/' in path or '/Level Editor/' in path or '/Other/Title/' in path or n in root_host:category='ui'
 if n in {'oBatTarget','oBasket','oTarget','oRim','oRimDeflect'}:category='gameplay'
 elif '/Other/Intro/' in path or '/Other/End/' in path or '/supersound/' in path:category='visual'
 elif '/Effects/' in path and not ev:category='visual'
 elif n in {'oBlack','oBlackBG','oBlackFadeUp','oCaveBG','oCaveBG2','oCaveBGEntrance','oCaveUp','oLeaves','oLeavesTile','oSplash','oSprite','oYellHelp','oDamselKiss','oMattockHead'}:category='visual'
 elif n.startswith('oMenu') or n in {'oPageDown','oPageUp','oWalDamsel'} or n.startswith('oPDummy'):category='ui'
 elif '/Basis/' in path or n in {'oSprite','oBricks'}:category='behavior'
 if n=='oPlayer1':
  ev=[{'module':'crates/core/src/player.rs','line':1,'kind':'player_tick_replacement'}]
 if ev:status='partial_handler_present';reason='Direct dispatch or registered generic handler exists; source behavior and runtime acceptance are not complete.'
 elif n in abstract:status='source_abstraction';reason='Source parent/group; requires inherited behavior coverage in concrete descendants.'
 elif category=='ui':status='host_replacement_unverified';reason='Original object is not directly dispatched; host editor/menu/profile behavior needs separate acceptance.'
 elif category=='visual':status='asset_only_unverified';reason='Original asset is catalogued; dynamic event/draw behavior has no direct audited handler.'
 elif isa(n,'oSolid') or isa(n,'oLadder') or n in {'oVine','oVineTop','oLadderTop','oTreeBranch'}:status='partial_static_tile_adapter';reason='Generic terrain/ladder collision exists; object-specific source events remain unverified.';ev=[{'module':'crates/core/src/catalog.rs','line':78,'kind':'source_hierarchy_terrain_adapter'}]
 else:status='no_direct_handler_found';reason='No audited object dispatch exists; source gameplay behavior needs implementation or documented equivalent.'
 if status=='host_replacement_unverified':
  module='crates/app/src/editor_ui.rs' if '/Level Editor/' in path else ('crates/app/src/settings_ui.rs' if '/Init/' in path or n in {'oJoyConfig','oKeyConfig','oGamepad'} else 'crates/app/src/host.rs')
  ev=[{'module':module,'line':1,'kind':'host_equivalent_requires_behavior_acceptance'}]
 records.append({'object':n,'source_id':o['id'],'source_path':path,'source_events':o['events'],'category':category,'status':status,'parity_verified':False,'handlers':ev,'note':reason})
result={'schema_version':1,'reference':'Official Classic 1.1, 453 original objects','review':'Static coverage inventory; handler presence is not feature completion or executable parity. Snapshot of checked-in implementation; regenerate after dispatch changes.','method':'Direct Rust object match arms and explicit enemy/item/hazard dispatch registration; excludes comments, test code, spawn-only strings and arbitrary object mentions. Host replacements and source abstractions are separately classified. Generic callbacks and indirect behavior can require manual follow-up.','counts':{'total':len(records),'by_status':dict(collections.Counter(x['status'] for x in records)),'by_category':dict(collections.Counter(x['category'] for x in records))},'objects':records}
output = root / 'assets/coverage.json'
encoded = json.dumps(result, indent=2, sort_keys=True) + '\n'
assert len(records) == 453
assert len({r['object'] for r in records}) == len(records)
assert len({r['source_id'] for r in records}) == len(records)
for record in records:
 for handler in record['handlers']:
  module = root / handler['module']
  assert module.is_file(), handler
  assert 0 < handler['line'] <= len(module.read_text().splitlines()), handler
if args.check:
 if not output.exists() or output.read_text() != encoded:
  parser.exit(1, 'Coverage inventory is stale; run python3 tools/audit_coverage.py and review the changes.\n')
 print('Coverage inventory matches source: 453 unique source objects; handler anchors valid.')
else:
 output.write_text(encoded)
 print(result['counts'])
