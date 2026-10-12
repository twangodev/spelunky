import { describe, expect, test } from "bun:test";
import { pythonJson } from "./python_json.ts";

describe("source artifact JSON formatting", () => {
  test("sorts numeric-looking dictionary keys lexically rather than as array indexes", () => {
    expect(pythonJson({ "2": 2, "10": 10, nested: { "3": 3, "11": 11 } }, { sortKeys: true })).toBe('{\n  "10": 10,\n  "2": 2,\n  "nested": {\n    "11": 11,\n    "3": 3\n  }\n}');
  });

  test("escapes non-ASCII text by default and retains UTF-8 when requested", () => {
    expect(pythonJson(["é", "🙂", "\x7f"])).toBe('[\n  "\\u00e9",\n  "\\ud83d\\ude42",\n  "\\u007f"\n]');
    expect(pythonJson({ text: "é🙂" }, { ensureAscii: false })).toBe('{\n  "text": "é🙂"\n}');
  });

  test("keeps empty collections compact and preserves unsorted record field order", () => {
    expect(pythonJson({ z: [], a: {}, b: [null, false, 17] })).toBe('{\n  "z": [],\n  "a": {},\n  "b": [\n    null,\n    false,\n    17\n  ]\n}');
  });

  test("sorts Unicode keys by code point and formats fractional exponents", () => {
    expect(pythonJson({ "𐀀": 1, "\ue000": 2 }, { sortKeys: true })).toBe('{\n  "\\ue000": 2,\n  "\\ud800\\udc00": 1\n}');
    expect(pythonJson([0.0000125, 0.0001])).toBe('[\n  1.25e-05,\n  0.0001\n]');
  });
});
