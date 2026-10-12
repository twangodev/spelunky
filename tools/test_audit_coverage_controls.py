#!/usr/bin/env python3
"""Bounded scanner parser controls, not coverage generation or Rust execution.

Required helper API:
  registered_objects(text, function_name) -> [(name, physical_literal_line)]
  object_match_arms(text) -> [(pattern_name, physical_literal_line)]
Existing function_line/function_body/production_text/without_function_dispatch
provide scope/anchor controls. No function_kind helper is required.

Run this file with --scanner PATH to select a source-only helper
candidate; default is the parent-owned v34 scanner. Only Python FunctionDef AST
nodes are loaded; scanner argparse, file scans, assertions and generator never run.
Fixtures are real Rust snippets; no claim of a Rust parser/compiler or behavior.
"""
import ast
import argparse
import sys
from pathlib import Path
import re
import unittest

DEFAULT_SCANNER = Path(__file__).resolve().with_name('audit_coverage.py')
SCANNER_PATH = DEFAULT_SCANNER


def load_functions(source):
    tree = ast.parse(source)
    definitions = [node for node in tree.body if isinstance(node, ast.FunctionDef)]
    # Verified scanner helpers use re/builtins, with helper-to-helper references.
    namespace = {'re': re, '__builtins__': __builtins__}
    exec(compile(ast.Module(body=definitions, type_ignores=[]), '<coverage-helpers-only>', 'exec'), namespace)
    return namespace


def scanner_functions():
    return load_functions(SCANNER_PATH.read_text())


def literal_line(text, literal, occurrence=0):
    token = '"' + literal + '"'
    start = -1
    for _ in range(occurrence + 1):
        start = text.index(token, start + 1)
    return text.count('\n', 0, start) + 1


class CoverageControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.functions = scanner_functions()

    def helper(self, name):
        self.assertIn(name, self.functions, f'required scanner helper {name} is absent')
        return self.functions[name]

    def registered(self, text, function='handles'):
        return self.helper('registered_objects')(text, function)

    def arms(self, text):
        return self.helper('object_match_arms')(text)

    def test_ast_loading_excludes_all_top_level_execution(self):
        helpers = load_functions('''
def safe_helper(text):
    return len(text)
raise RuntimeError("module startup must not run")
parser.parse_args()
generate_coverage()
''')
        self.assertEqual(helpers['safe_helper']('abc'), 3)
        self.assertNotIn('parser', helpers)
        self.assertNotIn('generate_coverage', helpers)

    def test_negative_registration_does_not_count_excluded_item_objects(self):
        text = '''
pub(crate) fn handles(object: &str) -> bool {
    is_item(object) && !matches!(object, "oBall" | "oBall2")
        || matches!(object, "oWhip")
}
'''
        self.assertEqual(self.registered(text), [('oWhip', literal_line(text, 'oWhip'))])

    def test_multiline_parenthesized_negative_registration_is_not_positive(self):
        text = '''
fn handles(object: &str) -> bool {
    is_item(object)
        && !(matches!(object,
            "oBall" |
            "oBall2"
        ))
        || matches!(object,
            "oWhip" |
            "oWhipPre"
        )
}
'''
        self.assertEqual(self.registered(text), [
            ('oWhip', literal_line(text, 'oWhip')),
            ('oWhipPre', literal_line(text, 'oWhipPre')),
        ])

    def test_registration_excludes_unrelated_body_strings(self):
        text = '''
fn handles(object: &str) -> bool {
    let debug_label = "oBodyOnly";
    let sample = r##"matches!(object, "oRawOnly")"##;
    /* matches!(object, "oCommentOnly") */
    matches!(object, "oLive")
}
fn unrelated() { instance_create("oSpawnOnly"); }
'''
        self.assertEqual(self.registered(text), [('oLive', literal_line(text, 'oLive'))])

    def test_test_function_is_excluded_and_production_anchor_moves(self):
        text = '''#[cfg(test)]
fn handles(object: &str) -> bool { matches!(object, "oTest") }

pub(super) fn handles(object: &str) -> bool {
    matches!(object, "oLive" | "oOther")
}
'''
        expected = [('oLive', literal_line(text, 'oLive')), ('oOther', literal_line(text, 'oOther'))]
        self.assertEqual(self.registered(text), expected)
        prefix = '// unrelated line\nfn independent() {}\n\n'
        self.assertEqual(self.registered(prefix + text), [(n, line + 3) for n, line in expected])
        line = self.helper('function_line')(text, 'handles')
        self.assertEqual(text.splitlines()[line - 1], 'pub(super) fn handles(object: &str) -> bool {')
        self.assertEqual(self.helper('function_line')(prefix + text, 'handles'), line + 3)

    def test_single_guard_counterpart_is_not_dispatch_pattern(self):
        text = '''
fn collision(object: &str, other: &str) {
    match object {
        "oReceiver" if other == "oCounter" => contact(),
        _ => (),
    }
}
'''
        self.assertEqual(self.arms(text), [('oReceiver', literal_line(text, 'oReceiver'))])

    def test_multiline_equals_guard_does_not_hide_receiver_or_register_counterpart(self):
        text = '''
fn collision(object: &str, other: &str) {
    match object {
        "oReceiver"
            if other
                ==
                "oCounter"
            => contact(),
        _ => (),
    }
}
'''
        self.assertEqual(self.arms(text), [('oReceiver', literal_line(text, 'oReceiver'))])

    def test_nested_guard_matches_are_conditions_not_behavior_arms(self):
        text = '''
fn collision(object: &str, other: &str) {
    match object {
        "oReceiver" if (other == "oCounter"
            && matches!(other, "oGuardA" | "oGuardB")
            && ({ let token = "oGuardLocal"; token.len() > 0 }))
            => { instance_create("oBodySpawn"); },
        _ => (),
    }
}
'''
        self.assertEqual(self.arms(text), [('oReceiver', literal_line(text, 'oReceiver'))])

    def test_pattern_alternatives_keep_each_literal_physical_line(self):
        text = '''
fn step(object: &str, other: &str) {
    match object {
        "oFirst" |
        "oSecond" if other == "oCounter" => run(),
        "oThird" => (),
        _ => (),
    }
}
'''
        self.assertEqual(self.arms(text), [
            ('oFirst', literal_line(text, 'oFirst')),
            ('oSecond', literal_line(text, 'oSecond')),
            ('oThird', literal_line(text, 'oThird')),
        ])

    def test_comments_cfg_test_and_raw_rust_text_do_not_create_dispatch(self):
        text = '''
// match object { "oLineComment" => tick() }
/* outer { /* "oNestedComment" => tick() */ } */
const TEXT: &str = r###"fn fake() { match x { "oRaw" => tick() } }"###;
#[cfg(test)]
#[allow(unused)]
fn helper() { match x { "oTestFunction" => tick() } }
#[cfg(test)]
mod tests { fn t() { match x { "oTestModule" => tick() } } }
fn live(object: &str) {
    let escaped = "\\\"oEscaped\\\" => tick()";
    match object { "oLive" => tick(), _ => () }
}
'''
        self.assertEqual(self.arms(text), [('oLive', literal_line(text, 'oLive'))])
        filtered, ranges = self.helper('production_text')(text)
        self.assertEqual(len(filtered), len(text))
        self.assertEqual(filtered.count('\n'), text.count('\n'))
        self.assertEqual(len(ranges), 2)
        prefix = '// moved\n\nfn preliminary() {}\n'
        self.assertEqual(self.arms(prefix + text), [('oLive', literal_line(text, 'oLive') + 3)])

    def test_function_body_ends_before_following_dispatch_despite_literal_braces(self):
        text = '''
fn handles(object: &str) -> bool {
    let close = "}";
    let raw = r#"{"#;
    if object.len() > 0 { matches!(object, "oLive") } else { false }
}
fn later(object: &str) { match object { "oOutside" => tick(), _ => () } }
'''
        body = self.helper('function_body')(text, 'handles')
        self.assertIsNotNone(body)
        start, code = body
        self.assertEqual(text[start:start + len(code)].count('\n'), code.count('\n'))
        self.assertIn('"oLive"', code)
        self.assertNotIn('"oOutside"', code)
        self.assertEqual(self.registered(text), [('oLive', literal_line(text, 'oLive'))])

    def test_metadata_function_is_not_automatic_behavior_and_removal_preserves_offsets(self):
        text = '''
fn source_type(object: &str) -> Option<&'static str> {
    match object { "oMetadataOnly" => Some("tag"), "oDual" => Some("both"), _ => None }
}
fn tick(object: &str) {
    match object { "oDual" => action(), _ => () }
}
fn actual_manual_adapter() { execute_portal_action(); }
'''
        # The arm helper reports syntax; the owner explicitly excludes a known
        # metadata/classifier function before calling it for behavior evidence.
        dispatch = self.helper('without_function_dispatch')(text, 'source_type')
        self.assertEqual(len(dispatch), len(text))
        self.assertEqual(dispatch.count('\n'), text.count('\n'))
        self.assertEqual(self.arms(dispatch), [('oDual', literal_line(text, 'oDual', occurrence=1))])
        anchor = self.helper('function_line')(text, 'actual_manual_adapter')
        self.assertEqual(text.splitlines()[anchor - 1], 'fn actual_manual_adapter() { execute_portal_action(); }')
        self.assertEqual(self.helper('function_line')('// line shift\n' + text, 'actual_manual_adapter'), anchor + 1)
        self.assertEqual(self.arms('fn actual_manual_adapter() { execute_portal_action(); }'), [])

    def test_eligibility_declaration_is_not_automatic_match_arm_behavior(self):
        text = '''
fn handles(object: &str) -> bool { matches!(object, "oEligible") }
fn tick(object: &str) {
    match object { "oImplemented" => update(), _ => () }
}
'''
        self.assertEqual(self.registered(text), [('oEligible', literal_line(text, 'oEligible'))])
        self.assertEqual(self.arms(text), [('oImplemented', literal_line(text, 'oImplemented'))])

    def test_nested_match_inside_guard_is_not_behavior_dispatch(self):
        text = '''
fn collision(object: &str, other: &str) {
    match object {
        "oReceiver" if match other {
            "oGuardCounter" => true,
            _ => false,
        } => contact(),
        _ => (),
    }
}
'''
        self.assertEqual(self.arms(text), [('oReceiver', literal_line(text, 'oReceiver'))])

    def test_turbofish_comma_in_guard_is_not_an_arm_separator(self):
        text = '''
fn check<A, B>(_left: A, _right: B) -> bool { true }
fn collision(object: &str, other: &str) {
    match object {
        "oReceiver" if check::<&str, &str>(other, "oCounter") => contact(),
        _ => (),
    }
}
'''
        self.assertEqual(self.arms(text), [('oReceiver', literal_line(text, 'oReceiver'))])

    def test_matches_pattern_guard_registers_pattern_only(self):
        text = '''
fn handles(object: &str, other: &str) -> bool {
    matches!(object, "oWhip" | "oWhipPre" if other == "oCounter")
}
'''
        self.assertEqual(self.registered(text), [
            ('oWhip', literal_line(text, 'oWhip')),
            ('oWhipPre', literal_line(text, 'oWhipPre')),
        ])

    def test_nested_matches_inside_pattern_guard_is_not_registration(self):
        text = '''
fn handles(object: &str, other: &str) -> bool {
    matches!(object, "oWhip" if matches!(other, "oGuardCounter"))
}
'''
        self.assertEqual(self.registered(text), [('oWhip', literal_line(text, 'oWhip'))])

    def test_negated_matches_guard_never_registers_nested_counterpart(self):
        text = '''
fn handles(object: &str, other: &str) -> bool {
    !matches!(object, "oBall" if matches!(other, "oGuardCounter"))
        || matches!(object, "oWhip")
}
'''
        self.assertEqual(self.registered(text), [('oWhip', literal_line(text, 'oWhip'))])

    def test_missing_function_is_empty_and_duplicate_production_anchor_is_rejected(self):
        self.assertEqual(self.registered('fn other() {}'), [])
        self.assertIsNone(self.helper('function_body')('fn other() {}', 'handles'))
        text = 'fn handles(x: &str) -> bool { true }\nfn handles(x: &str) -> bool { false }\n'
        with self.assertRaises(AssertionError):
            self.helper('function_line')(text, 'handles')
        with self.assertRaises(AssertionError):
            self.helper('function_body')(text, 'handles')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('--scanner', type=Path, default=DEFAULT_SCANNER)
    options, unittest_arguments = parser.parse_known_args()
    SCANNER_PATH = options.scanner
    unittest.main(argv=[sys.argv[0], *unittest_arguments])
