import { describe, expect, test } from "bun:test";
import { parseXml, serializeXml } from "./xml.ts";

describe("GameMaker XML data reader", () => {
  test("preserves creation-code text, mixed content, attributes and source ordering", () => {
    const root = parseXml(`<?xml version="1.0"?><room id="7" note="&quot;&#x1f642;&#10;">before<!-- ignored -->comment<creationCode><![CDATA[if (x < 4) a = "&raw";]]>&amp;&#65;</creationCode>after<empty /><instance id="2" /><instance id="1" /></room>`);
    expect(root.tag).toBe("room");
    expect(root.get("id")).toBe("7");
    expect(root.get("note")).toBe('"🙂\n');
    expect(root.text).toBe("beforecomment");
    expect(root.findText("creationCode")).toBe('if (x < 4) a = "&raw";&A');
    expect(root.find("creationCode")!.tail).toBe("after");
    expect(root.findText("empty", "fallback")).toBe("");
    expect(root.findText("missing", "fallback")).toBe("fallback");
    expect(root.findAll("./instance").map(node => node.get("id"))).toEqual(["2", "1"]);
    expect(root.findAll("instance[@id='1']")).toHaveLength(1);
  });

  test("finds nested STRING action arguments and normalizes XML line endings", () => {
    const root = parseXml('\uFEFF<event><actions>\r\n<action><arguments><argument kind="STRING">line1\r\nline2</argument><argument kind="REAL">4</argument></arguments></action></actions><argument kind="STRING">last</argument></event>');
    expect(root.findAll(".//argument[@kind='STRING']").map(node => node.text)).toEqual(["line1\nline2", "last"]);
    expect(root.findText("actions/action/arguments/argument")).toBe("line1\nline2");
    expect([...root.iter("argument")]).toHaveLength(3);
  });

  test("serializes masks with ElementTree attribute order and empty-element spacing", () => {
    const mask = parseXml('<mask mode="auto" tolerance="0"><shape>RECTANGLE</shape><bounds right="15" left="0" />\n</mask>');
    expect(serializeXml(mask)).toBe('<mask mode="auto" tolerance="0"><shape>RECTANGLE</shape><bounds right="15" left="0" />\n</mask>');
    const escaped = parseXml('<a attr="&quot;&#10;&#09;&#13;&amp;&lt;">&amp;&lt;&gt;</a>');
    expect(serializeXml(escaped)).toBe('<a attr="&quot;&#10;&#09;&#13;&amp;&lt;">&amp;&lt;&gt;</a>');
  });

  test("rejects malformed XML and unsupported document declarations", () => {
    for (const source of [
      '<a>&</a>', '<a>&bogus;</a>', '<a b="raw<text"/>', '<a b="x" b="y"/>',
      '<a></b>', '<a/><b/>', '<a>', '<!DOCTYPE a><a/>', '<a>&#0;</a>',
      '<a><!-- unfinished</a>', '<a><![CDATA[unfinished</a>', '<a><?unfinished</a>',
    ]) expect(() => parseXml(source)).toThrow();
  });
});
