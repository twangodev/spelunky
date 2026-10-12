#!/usr/bin/env bun
/** Nineteen real Rust-snippet scanner controls; no Rust compilation or parity claim. */
import assert from 'node:assert/strict';
import { readFileSync,writeFileSync,mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname,join,resolve } from 'node:path';
import { pathToFileURL,fileURLToPath } from 'node:url';
const args=process.argv.slice(2); assert(args.length===0 || (args.length===2 && args[0]==='--scanner'),'Usage: bun tools/test_audit_coverage_controls.ts [--scanner PATH]');
const scanner=args.length ? resolve(args[1]) : join(dirname(fileURLToPath(import.meta.url)),'audit_coverage.ts');
const h=await import(pathToFileURL(scanner).href);
const fixtures: Record<string,string> = {
  "negative_registration_does_not_count_excluded_item_objects": "\npub(crate) fn handles(object: &str) -> bool {\n    is_item(object) && !matches!(object, \"oBall\" | \"oBall2\")\n        || matches!(object, \"oWhip\")\n}\n",
  "multiline_parenthesized_negative_registration_is_not_positive": "\nfn handles(object: &str) -> bool {\n    is_item(object)\n        && !(matches!(object,\n            \"oBall\" |\n            \"oBall2\"\n        ))\n        || matches!(object,\n            \"oWhip\" |\n            \"oWhipPre\"\n        )\n}\n",
  "registration_excludes_unrelated_body_strings": "\nfn handles(object: &str) -> bool {\n    let debug_label = \"oBodyOnly\";\n    let sample = r##\"matches!(object, \"oRawOnly\")\"##;\n    /* matches!(object, \"oCommentOnly\") */\n    matches!(object, \"oLive\")\n}\nfn unrelated() { instance_create(\"oSpawnOnly\"); }\n",
  "test_function_is_excluded_and_production_anchor_moves": "#[cfg(test)]\nfn handles(object: &str) -> bool { matches!(object, \"oTest\") }\n\npub(super) fn handles(object: &str) -> bool {\n    matches!(object, \"oLive\" | \"oOther\")\n}\n",
  "single_guard_counterpart_is_not_dispatch_pattern": "\nfn collision(object: &str, other: &str) {\n    match object {\n        \"oReceiver\" if other == \"oCounter\" => contact(),\n        _ => (),\n    }\n}\n",
  "multiline_equals_guard_does_not_hide_receiver_or_register_counterpart": "\nfn collision(object: &str, other: &str) {\n    match object {\n        \"oReceiver\"\n            if other\n                ==\n                \"oCounter\"\n            => contact(),\n        _ => (),\n    }\n}\n",
  "nested_guard_matches_are_conditions_not_behavior_arms": "\nfn collision(object: &str, other: &str) {\n    match object {\n        \"oReceiver\" if (other == \"oCounter\"\n            && matches!(other, \"oGuardA\" | \"oGuardB\")\n            && ({ let token = \"oGuardLocal\"; token.len() > 0 }))\n            => { instance_create(\"oBodySpawn\"); },\n        _ => (),\n    }\n}\n",
  "pattern_alternatives_keep_each_literal_physical_line": "\nfn step(object: &str, other: &str) {\n    match object {\n        \"oFirst\" |\n        \"oSecond\" if other == \"oCounter\" => run(),\n        \"oThird\" => (),\n        _ => (),\n    }\n}\n",
  "comments_cfg_test_and_raw_rust_text_do_not_create_dispatch": "\n// match object { \"oLineComment\" => tick() }\n/* outer { /* \"oNestedComment\" => tick() */ } */\nconst TEXT: &str = r###\"fn fake() { match x { \"oRaw\" => tick() } }\"###;\n#[cfg(test)]\n#[allow(unused)]\nfn helper() { match x { \"oTestFunction\" => tick() } }\n#[cfg(test)]\nmod tests { fn t() { match x { \"oTestModule\" => tick() } } }\nfn live(object: &str) {\n    let escaped = \"\\\"oEscaped\\\" => tick()\";\n    match object { \"oLive\" => tick(), _ => () }\n}\n",
  "function_body_ends_before_following_dispatch_despite_literal_braces": "\nfn handles(object: &str) -> bool {\n    let close = \"}\";\n    let raw = r#\"{\"#;\n    if object.len() > 0 { matches!(object, \"oLive\") } else { false }\n}\nfn later(object: &str) { match object { \"oOutside\" => tick(), _ => () } }\n",
  "metadata_function_is_not_automatic_behavior_and_removal_preserves_offsets": "\nfn source_type(object: &str) -> Option<&'static str> {\n    match object { \"oMetadataOnly\" => Some(\"tag\"), \"oDual\" => Some(\"both\"), _ => None }\n}\nfn tick(object: &str) {\n    match object { \"oDual\" => action(), _ => () }\n}\nfn actual_manual_adapter() { execute_portal_action(); }\n",
  "eligibility_declaration_is_not_automatic_match_arm_behavior": "\nfn handles(object: &str) -> bool { matches!(object, \"oEligible\") }\nfn tick(object: &str) {\n    match object { \"oImplemented\" => update(), _ => () }\n}\n",
  "nested_match_inside_guard_is_not_behavior_dispatch": "\nfn collision(object: &str, other: &str) {\n    match object {\n        \"oReceiver\" if match other {\n            \"oGuardCounter\" => true,\n            _ => false,\n        } => contact(),\n        _ => (),\n    }\n}\n",
  "turbofish_comma_in_guard_is_not_an_arm_separator": "\nfn check<A, B>(_left: A, _right: B) -> bool { true }\nfn collision(object: &str, other: &str) {\n    match object {\n        \"oReceiver\" if check::<&str, &str>(other, \"oCounter\") => contact(),\n        _ => (),\n    }\n}\n",
  "matches_pattern_guard_registers_pattern_only": "\nfn handles(object: &str, other: &str) -> bool {\n    matches!(object, \"oWhip\" | \"oWhipPre\" if other == \"oCounter\")\n}\n",
  "nested_matches_inside_pattern_guard_is_not_registration": "\nfn handles(object: &str, other: &str) -> bool {\n    matches!(object, \"oWhip\" if matches!(other, \"oGuardCounter\"))\n}\n",
  "negated_matches_guard_never_registers_nested_counterpart": "\nfn handles(object: &str, other: &str) -> bool {\n    !matches!(object, \"oBall\" if matches!(other, \"oGuardCounter\"))\n        || matches!(object, \"oWhip\")\n}\n",
  "missing_function_is_empty_and_duplicate_production_anchor_is_rejected": "fn handles(x: &str) -> bool { true }\nfn handles(x: &str) -> bool { false }\n"
};
const line=(text:string,name:string,occurrence=0) => {let at=-1;for(let i=0;i<=occurrence;i++)at=text.indexOf('"'+name+'"',at+1);assert(at>=0);return text.slice(0,at).split('\n').length;};
const expected=(text:string,names:string[])=>names.map(name=>[name,line(text,name)]);
const results:string[]=[];
function check(name:string,body:()=>void){body();results.push(name);console.log('PASS '+name);}
const registration=(key:string,names:string[])=>{const text=fixtures[key];assert.deepEqual(h.registeredObjects(text,'handles'),expected(text,names));};
const arms=(key:string,names:string[])=>{const text=fixtures[key];assert.deepEqual(h.objectMatchArms(text),expected(text,names));};
check('import_loading_excludes_all_generator_execution',()=>{
 const dir=mkdtempSync(join(tmpdir(),'spelunky-bun-coverage-import-control-'));const copy=join(dir,'scanner.ts');writeFileSync(copy,readFileSync(scanner));
 const command=[process.execPath,'-e',`await import(${JSON.stringify(pathToFileURL(copy).href)}); console.log('IMPORTED_HELPERS_ONLY');`];const r=Bun.spawnSync(command,{stdout:'pipe',stderr:'pipe'});assert.equal(r.exitCode,0,new TextDecoder().decode(r.stderr));assert.equal(new TextDecoder().decode(r.stdout),'IMPORTED_HELPERS_ONLY\n');
});
check('negative_registration_excludes_items',()=>registration('negative_registration_does_not_count_excluded_item_objects',['oWhip']));
check('multiline_parenthesized_negative',()=>registration('multiline_parenthesized_negative_registration_is_not_positive',['oWhip','oWhipPre']));
check('body_raw_comment_spawn_exclusions',()=>registration('registration_excludes_unrelated_body_strings',['oLive']));
check('cfgtest_function_exclusion_and_anchor_shift',()=>{
 const text=fixtures.test_function_is_excluded_and_production_anchor_moves,prefix='// unrelated line\nfn independent() {}\n\n';const e=expected(text,['oLive','oOther']);assert.deepEqual(h.registeredObjects(text,'handles'),e);assert.deepEqual(h.registeredObjects(prefix+text,'handles'),e.map(([n,l])=>[n,Number(l)+3]));const anchor=h.functionLine(text,'handles');assert.equal(text.split('\n')[anchor-1],'pub(super) fn handles(object: &str) -> bool {');assert.equal(h.functionLine(prefix+text,'handles'),anchor+3);
});
check('single_equality_guard_counterpart',()=>arms('single_guard_counterpart_is_not_dispatch_pattern',['oReceiver']));
check('multiline_equals_guard',()=>arms('multiline_equals_guard_does_not_hide_receiver_or_register_counterpart',['oReceiver']));
check('nested_matches_block_guard',()=>arms('nested_guard_matches_are_conditions_not_behavior_arms',['oReceiver']));
check('alternatives_physical_literal_lines',()=>arms('pattern_alternatives_keep_each_literal_physical_line',['oFirst','oSecond','oThird']));
check('comments_cfgtests_raw_literals_and_shift',()=>{
 const key='comments_cfg_test_and_raw_rust_text_do_not_create_dispatch',text=fixtures[key];arms(key,['oLive']);const [filtered,ranges]=h.productionText(text);assert.equal(filtered.length,text.length);assert.equal(filtered.split('\n').length,text.split('\n').length);assert.equal(ranges.length,2);assert.deepEqual(h.objectMatchArms('// moved\n\nfn preliminary() {}\n'+text),[['oLive',line(text,'oLive')+3]]);
});
check('function_body_exact_boundary',()=>{
 const text=fixtures.function_body_ends_before_following_dispatch_despite_literal_braces;const body=h.functionBody(text,'handles');assert(body);const [start,code]=body;assert.equal(text.slice(start,start+code.length).split('\n').length,code.split('\n').length);assert(code.includes('"oLive"')&&!code.includes('"oOutside"'));assert.deepEqual(h.registeredObjects(text,'handles'),expected(text,['oLive']));
});
check('metadata_exclusion_offsets_and_manual_anchor',()=>{
 const text=fixtures.metadata_function_is_not_automatic_behavior_and_removal_preserves_offsets,dispatch=h.withoutFunctionDispatch(text,'source_type');assert.equal(dispatch.length,text.length);assert.equal(dispatch.split('\n').length,text.split('\n').length);assert.deepEqual(h.objectMatchArms(dispatch),[['oDual',line(text,'oDual',1)]]);const a=h.functionLine(text,'actual_manual_adapter');assert.equal(text.split('\n')[a-1],'fn actual_manual_adapter() { execute_portal_action(); }');assert.equal(h.functionLine('// line shift\n'+text,'actual_manual_adapter'),a+1);assert.deepEqual(h.objectMatchArms('fn actual_manual_adapter() { execute_portal_action(); }'),[]);
});
check('eligibility_separate_from_behavior',()=>{registration('eligibility_declaration_is_not_automatic_match_arm_behavior',['oEligible']);arms('eligibility_declaration_is_not_automatic_match_arm_behavior',['oImplemented']);});
check('nested_match_guard_not_behavior',()=>arms('nested_match_inside_guard_is_not_behavior_dispatch',['oReceiver']));
check('turbofish_guard_comma',()=>arms('turbofish_comma_in_guard_is_not_an_arm_separator',['oReceiver']));
check('matches_pattern_guard_only',()=>registration('matches_pattern_guard_registers_pattern_only',['oWhip','oWhipPre']));
check('nested_matches_pattern_guard_excluded',()=>registration('nested_matches_inside_pattern_guard_is_not_registration',['oWhip']));
check('negated_matches_guard_counterpart_excluded',()=>registration('negated_matches_guard_never_registers_nested_counterpart',['oWhip']));
check('missing_duplicate_and_generic_function_anchors',()=>{
 assert.deepEqual(h.registeredObjects('fn other() {}','handles'),[]);assert.equal(h.functionBody('fn other() {}','handles'),null);const text='fn handles(x: &str) -> bool { true }\nfn handles(x: &str) -> bool { false }\n';assert.throws(()=>h.functionLine(text,'handles'));assert.throws(()=>h.functionBody(text,'handles'));const generic="pub(crate) fn handles<'a, T>(object: &'a str) -> bool { matches!(object, \"oLive\") }";assert.deepEqual(h.registeredObjects(generic,'handles'),[['oLive',1]]);assert.equal(h.functionLine(generic,'handles'),1);
});
assert.equal(results.length,19);console.log(JSON.stringify({status:'PASS_19_SOURCE_PARSER_CONTROLS',controls:results,scanner,scope:'Parser snippets only; no source generation, Rust compilation, runtime or parity acceptance.'}));
