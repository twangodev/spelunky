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

def lexical_shadow(text, hide_literals=True):
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
  if hide_literals or text.startswith(('//','/*'),start):
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

def function_line(text, name):
 """Resolve a production function definition without depending on line offsets."""
 production,_=production_text(text)
 shadow=lexical_shadow(production)
 definitions=list(re.finditer(
  r'(?m)^[ \t]*(?:pub(?:\([^)]*\))?\s+)?fn\s+'+re.escape(name)+r'\s*\(',
  shadow,
 ))
 assert len(definitions)==1, f'Expected one production definition of {name}'
 return shadow[:definitions[0].start()].count('\n')+1

def function_body(text, name):
 """Find a production function body with nested blocks and literal braces."""
 production,_=production_text(text)
 shadow=lexical_shadow(production)
 definitions=list(re.finditer(
  r'(?m)^[ \t]*(?:pub(?:\([^)]*\))?\s+)?fn\s+'+re.escape(name)+r'\s*\(',shadow,
 ))
 if not definitions:return None
 assert len(definitions)==1, f'Expected one production definition of {name}'
 start=definitions[0].start();opening=shadow.index('{',definitions[0].end())
 end=opening+1;depth=1
 while end<len(shadow) and depth:
  if shadow[end]=='{':depth+=1
  elif shadow[end]=='}':depth-=1
  end+=1
 assert depth==0, f'Unbalanced function {name}'
 return start,lexical_shadow(production[start:end],hide_literals=False)

def registered_objects(text, name):
 body=function_body(text,name)
 if body is None:return []
 start,code=body
 return [(m.group(1),text[:start+m.start()].count('\n')+1)
         for m in re.finditer(r'"(o[A-Za-z0-9]+)"',code)]

def inherited_rubble_line(text):
 production,_=production_text(text)
 code=lexical_shadow(production,hide_literals=False)
 match=re.search(r'_\s+if\s+catalog::is_a\([^,\n]+,\s*"oRubblePiece"\)',code)
 return code[:match.start()].count('\n')+1 if match else None

def without_function_dispatch(text, name):
 """Hide one metadata classifier while retaining all subsequent source offsets."""
 body=function_body(text,name)
 if body is None:return text
 start,code=body
 end=start+len(code)
 return text[:start]+blank(text[start:end])+text[end:]

# Regressions: an early helper/external test module must not hide later handlers;
# braces inside literals/comments must not end an excluded item.
fixture='#[cfg(test)]\nfn helper() { let s = r#"}"#; /* { */ }\n#[cfg(test)]\n#[path = "traversal.rs"]\nmod traversal;\nfn live() { match x { "oLive" => {} } }\n#[cfg(test)]\nmod tests { fn t() { match x { "oTest" => {} } } }\n'
filtered,fixture_ranges=production_text(fixture)
assert '"oLive"' in filtered and '"oTest"' not in filtered and 'fn helper' not in filtered
assert len(filtered)==len(fixture) and filtered.count('\n')==fixture.count('\n') and len(fixture_ranges)==3

# Adapter evidence must follow the actual definition when unrelated code moves.
anchor_fixture='#[cfg(test)]\nfn is_solid() {}\n// pub fn is_solid() {}\nfn helper() {}\npub fn is_solid(name: &str) -> bool { true }\n'
anchor_line=function_line(anchor_fixture,'is_solid')
assert anchor_fixture.splitlines()[anchor_line-1].startswith('pub fn is_solid(')
prefix='// Unrelated preceding change\nfn another_helper() {}\n\n'
assert function_line(prefix+anchor_fixture,'is_solid')==anchor_line+prefix.count('\n')

# Family registration may be pub(super), nested, and followed by unrelated code.
registration_fixture='#[cfg(test)]\nfn handles(x: &str) -> bool { matches!(x, "oTest") }\npub(super) fn handles(x: &str) -> bool {\n /* "oComment" { */ let brace = "}";\n matches!(x, "oLive" | "oOther")\n}\nfn later() { let _ = "oUnrelated"; }\n'
assert registered_objects(registration_fixture,'handles')==[('oLive',5),('oOther',5)]
assert registered_objects(prefix+registration_fixture,'handles')==[('oLive',8),('oOther',8)]
rubble_fixture='// _ if catalog::is_a(&fake, "oRubblePiece")\nfn live() { match object {\n _ if catalog::is_a(&object, "oRubblePiece") => rubble(), _ => {}\n} }\n'
assert inherited_rubble_line(rubble_fixture)==3
assert inherited_rubble_line(prefix+rubble_fixture.replace('&object,','&e.object,'))==6

# Source type labels describe Create metadata, not direct event dispatch.
# A later real handler for the same object must retain its exact source offset.
label_fixture='fn source_type() { let brace = r#"}"#; /* { */ match x { "oLabelOnly" => Some("Label"), "oDual" => Some("Dual"), _ => None } }\nfn live() { match x { "oDual" => tick(), _ => {} } }\n'
label_shadow=lexical_shadow(label_fixture,hide_literals=False)
label_dispatch=without_function_dispatch(label_shadow,'source_type')
assert len(label_dispatch)==len(label_shadow)
assert label_dispatch.count('\n')==label_shadow.count('\n')
assert '"oLabelOnly"' not in label_dispatch and label_dispatch.count('"oDual"')==1
assert label_dispatch.index('"oDual"')==label_shadow.rindex('"oDual"')
assert without_function_dispatch(label_shadow,'absent')==label_shadow

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
 text=lexical_shadow(text,hide_literals=False)
 # Explicit object-valued match arms are behavior dispatch evidence; arbitrary
 # mentions, spawn strings, comments and tests are intentionally excluded.
 dispatch_text=(without_function_dispatch(text,'source_type')
                if p==root/'crates/core/src/items/explosives.rs' else text)
 for m in re.finditer(r'((?:"o[A-Za-z0-9]+"\s*\|\s*)*"o[A-Za-z0-9]+")\s*(?:if\s+[^\n=>]+)?\s*=>',dispatch_text):
  for name in re.findall(r'"(o[A-Za-z0-9]+)"',m.group(1)):
   handlers[name].append({'module':p.relative_to(root).as_posix(),'line':text[:m.start()].count('\n')+1,'kind':'explicit_object_match_arm'})
 # Enemy matcher is the parent dispatch registration contract. An empty tick
 # arm remains partial, never parity verified.
 registrations=[]
 if '/enemies/' in str(p):registrations.append(('matches','enemy_dispatch_registration'))
 if p.name=='enemies.rs':registrations.append(('is_actor','enemy_dispatch_registration'))
 if p.name in {'items.rs','traps.rs','world_events.rs'} or '/items/' in str(p) or '/traps/' in str(p):
  registrations.append(('handles','instance_event_family_registration'))
 # Base item-body predicates are actual ownership of common motion/pickup.
 if p.name=='items.rs':
  registrations.append(('is_item','generic_item_body_registration'))
 if p.name=='traps.rs':
  registrations.append(('is_hazard','hazard_dispatch_registration'))
 for function,kind in registrations:
  for name,line in registered_objects(text,function):
   handlers[name].append({'module':p.relative_to(root).as_posix(),'line':line,'kind':kind})
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

# State adapters dispatch by source room/stage rather than object string arms.
# These anchors establish partial implementation evidence, never parity.
state_adapters=[
 ('crates/core/src/opening.rs','begin_intro',['oIntro']),
 ('crates/core/src/opening.rs','begin_title',['oTitle']),
 ('crates/core/src/opening.rs','actor_step',['oPDummy3','oPDummy4']),
 ('crates/core/src/hub.rs','activate_portal',['oXStart']),
 ('crates/core/src/hub.rs','scores_step',['oHighscores']),
]
for module,function,names in state_adapters:
 line=function_line(sources[root/module],function)
 for name in names:
  handlers[name].append({'module':module,'line':line,'kind':'manually_reviewed_state_adapter'})

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
rubble_line = inherited_rubble_line(sources[root / rubble_module])
if rubble_line:
 for name in objects:
  if isa(name,'oRubblePiece'):
   handlers[name].append({'module':rubble_module,'line':rubble_line,'kind':'inherited_rubble_family_handler'})

records=[]
terrain_adapter_line=function_line(sources[root/'crates/core/src/catalog.rs'],'is_solid')
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
  ev=[{'module':module,'line':function_line(sources[root/module],function),'kind':kind}
      for module,function,kind in [
       ('crates/core/src/player.rs','tick','player_tick_replacement'),
       ('crates/core/src/dispatcher.rs','player_step','source_player_phase_adapter'),
      ]]
 if ev:status='partial_handler_present';reason='Direct dispatch or registered generic handler exists; source behavior and runtime acceptance are not complete.'
 elif n in abstract:status='source_abstraction';reason='Source parent/group; requires inherited behavior coverage in concrete descendants.'
 elif category=='ui':status='host_replacement_unverified';reason='Original object is not directly dispatched; host editor/menu/profile behavior needs separate acceptance.'
 elif category=='visual':status='asset_only_unverified';reason='Original asset is catalogued; dynamic event/draw behavior has no direct audited handler.'
 elif isa(n,'oSolid') or isa(n,'oLadder') or n in {'oVine','oVineTop','oLadderTop','oTreeBranch'}:status='partial_static_tile_adapter';reason='Generic terrain/ladder collision exists; object-specific source events remain unverified.';ev=[{'module':'crates/core/src/catalog.rs','line':terrain_adapter_line,'kind':'source_hierarchy_terrain_adapter'}]
 else:status='no_direct_handler_found';reason='No audited object dispatch exists; source gameplay behavior needs implementation or documented equivalent.'
 if status=='no_direct_handler_found' and n in {'oEntranceCheck','oRopeBag'}:
  reason+=' Preserved Objects/Scripts contain this name only in resource lists, with no producer references; the unused source resource remains unverified.'
 if status=='host_replacement_unverified':
  module='crates/app/src/editor_ui.rs' if '/Level Editor/' in path else ('crates/app/src/settings_ui.rs' if '/Init/' in path or n in {'oJoyConfig','oKeyConfig','oGamepad'} else 'crates/app/src/host.rs')
  ev=[{'module':module,'line':1,'kind':'host_equivalent_requires_behavior_acceptance'}]
 records.append({'object':n,'source_id':o['id'],'source_path':path,'source_events':o['events'],'category':category,'status':status,'parity_verified':False,'handlers':ev,'note':reason})
result={'schema_version':1,'reference':'Official Classic 1.1, 453 original objects','review':'Static coverage inventory; handler presence is not feature completion or executable parity. Snapshot of checked-in implementation; regenerate after dispatch changes.','method':'Direct Rust object match arms, explicit enemy/item/hazard/event-family registration, inherited rubble dispatch and manually reviewed room/state adapters; excludes comments, test code, spawn-only strings and arbitrary object mentions. Source Collision declarations establish event selection only and are not implementation evidence. Host replacements and source abstractions are separately classified. Generic callbacks and indirect behavior can require manual follow-up.','counts':{'total':len(records),'by_status':dict(collections.Counter(x['status'] for x in records)),'by_category':dict(collections.Counter(x['category'] for x in records))},'objects':records}
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
