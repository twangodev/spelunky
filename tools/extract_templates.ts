#!/usr/bin/env bun
/** Export Classic literal templates and their source conditions, without running GML. */
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative, basename } from "node:path";
import { readXml } from "./lib/xml.ts";
import { pythonJson } from "./lib/python_json.ts";
import { integer, parseArgs, writeOrCheck } from "./lib/common.ts";

type Token = [string, number, number];
type LiteralRecord = { line: number; variable: string; value: string; context: string[]; source_line: string; case?: number; kind: string };
export class LiteralParser {
  private tokens: Token[] = [];
  private i = 0;
  private records: LiteralRecord[] = [];
  constructor(private source: string) {
    const pattern = /(\/\*[\s\S]*?\*\/|\/\/[^\n]*)|("[^"\n]*"|'[^'\n]*')|([A-Za-z_][A-Za-z_0-9]*)|(\s+)|([\s\S])/g;
    for (const match of source.matchAll(pattern)) if (!match[1] && !match[4]) this.tokens.push([match[0], match.index!, match.index! + match[0].length]);
  }
  private peek(): string | null { return this.tokens[this.i]?.[0] ?? null; }
  private consume(): Token {
    if (this.i >= this.tokens.length) throw new Error("Unexpected end of template source");
    return this.tokens[this.i++];
  }
  private condition(): string {
    if (this.peek() !== "(") throw new Error("Expected parenthesized condition");
    const start = this.consume()[1];
    let depth = 1;
    while (this.i < this.tokens.length) {
      const [token, , end] = this.consume();
      depth += Number(token === "(") - Number(token === ")");
      if (depth === 0) return this.source.slice(start, end);
    }
    throw new Error("Unclosed condition");
  }
  private block(context: string[]): void {
    this.consume();
    let caseContext: string[] = [];
    while (this.peek() !== null && this.peek() !== "}") {
      if (this.peek() === "case" || this.peek() === "default") {
        const start = this.consume()[1];
        while (this.peek() !== null && this.peek() !== ":") this.consume();
        const end = this.consume()[2];
        caseContext = [this.source.slice(start, end)];
      } else this.statement([...context, ...caseContext]);
    }
    if (this.peek() === "}") this.consume();
  }
  private statement(context: string[]): void {
    if (this.peek() === "{") { this.block(context); return; }
    if (["if", "switch", "for", "while", "with", "repeat"].includes(this.peek() ?? "")) {
      const name = this.consume()[0];
      const condition = this.condition();
      this.statement([...context, name + condition]);
      if (name === "if" && this.peek() === "else") { this.consume(); this.statement([...context, "else"]); }
      return;
    }
    const start = this.i;
    while (this.peek() !== null && ![";", "}", "{"].includes(this.peek()!)) this.consume();
    const end = this.i;
    if (this.peek() === ";") this.consume();
    else if (this.peek() === "{") this.block([...context, "unknown"]);
    if (end <= start) return;
    const tokens = this.tokens.slice(start, end);
    for (let n = 0; n < tokens.length - 2; n++) {
      const variable = tokens[n][0];
      if (!/^(?:strTemp|strObs[1234]?)$/.test(variable) || tokens[n + 1][0] !== "=") continue;
      const expression = tokens.slice(n + 2);
      if (!expression.length || expression.some(([value]) => value !== "+" && value[0] !== '"' && value[0] !== "'")) continue;
      const value = expression.filter(([value]) => value !== "+").map(([value]) => value.slice(1, -1)).join("");
      const position = tokens[n][1];
      const line = this.source.slice(0, position).split("\n").length;
      const sourceLine = this.source.split(/\r\n|[\n\r\v\f\x1c-\x1e\x85\u2028\u2029]/)[line - 1].trim();
      const caseCondition = [...context].reverse().find(value => value.startsWith("case") && /\d+/.test(value));
      const record: LiteralRecord = { line, variable, value, context, source_line: sourceLine, kind: "" };
      // Match the original insertion order (kind follows the optional case).
      delete (record as Partial<LiteralRecord>).kind;
      if (caseCondition !== undefined) record.case = Number(/\d+/.exec(caseCondition)![0]);
      record.kind = classify(variable, context);
      this.records.push(record);
    }
  }
  parse(): LiteralRecord[] {
    while (this.peek() !== null) {
      const position = this.i;
      this.statement([]);
      if (this.i === position) throw new Error("Unexpected closing brace in template source");
    }
    return this.records;
  }
}

export function classify(variable: string, context: string[]): string {
  if (variable.startsWith("strObs")) {
    for (const condition of context) {
      const match = /tile\s*==\s*"(.)"/.exec(condition);
      if (match) return `obstacle_${match[1]}`;
    }
    return "obstacle_default";
  }
  for (const condition of context) {
    if (condition.includes("global.startRoomX")) return "start";
    if (condition.includes("global.endRoomX")) return "end";
    if (/roomPath\s*==\s*0/.test(condition)) return /roomPath\s*==\s*1/.test(condition) ? "main" : "side";
    const match = /roomPath\s*==\s*(\d+)/.exec(condition);
    if (match) return ({ 1: "main", 3: "landing", 4: "shop_left", 5: "shop_right" } as Record<number, string>)[Number(match[1])] ?? `special_${match[1]}`;
    if (/y\s*<\s*480/.test(condition)) return "boss_upper";
  }
  return context.length ? "drop" : "default";
}

export function exportTemplates(reference: string) {
  const scripts: Record<string, unknown> = {};
  const directory = join(reference, "Scripts", "Level Generation");
  for (const filename of readdirSync(directory).filter(value => /^scrRoomGen.*\.gml$/.test(value)).sort()) {
    const path = join(directory, filename);
    const raw = readFileSync(path);
    const source = new TextDecoder("utf-8", { fatal: true }).decode(raw).replace(/^\uFEFF/, "").replace(/\r\n/g, "\n");
    const dataSource = source.split("// Generate the tiles", 1)[0];
    const records = new LiteralParser(dataSource).parse();
    const name = basename(path, ".gml");
    for (const record of records) {
      if (record.variable === "strTemp" && record.value.length !== 80) throw new Error(`Invalid room literal in ${path}:${record.line}`);
      if (name === "scrRoomGen5" && record.kind === "drop") record.kind = "boss_lower";
    }
    scripts[name] = { source: relative(reference, path).replaceAll("\\", "/"), source_sha256: createHash("sha256").update(raw).digest("hex"), selection_source: dataSource, records };
  }
  const root = readXml(join(reference, "Rooms", "rTitle.xml"));
  const size = root.find("size")!;
  const title = {
    source: "Rooms/rTitle.xml", width: integer(size.get("width")), height: integer(size.get("height")), speed: integer(root.findText("speed")),
    instances: root.findAll("instances/instance").map(node => ({ object: node.findText("object"), x: integer(node.find("position")!.get("x")), y: integer(node.find("position")!.get("y")), creation_code: node.findText("creationCode") || "" })),
    create_code: readXml(join(reference, "Objects", "oTitle.events", "Create.xml")).findAll(".//argument[@kind='STRING']").map(node => node.text || "").join("\n"),
  };
  return { schema_version: 1, tile_size: 16, room_width: 10, room_height: 8, obstacle_width: 5, obstacle_height: 3, scripts, title };
}

if (import.meta.main) {
  try {
    const values = parseArgs({ reference: "references/classic-1.1", output: "assets/generation.json" }, "Export Classic room/obstacle literals with source conditions");
      const data = exportTemplates(String(values.reference));
      const contents = pythonJson(data, { ensureAscii: false }) + "\n";
      if (!values.check) mkdirSync(dirname(String(values.output)), { recursive: true });
      writeOrCheck(String(values.output), contents, Boolean(values.check));
      const records = Object.values(data.scripts) as { records: unknown[] }[];
      console.log(`${values.check ? "Verified" : "Exported"} ${records.reduce((sum, script) => sum + script.records.length, 0)} literals from ${records.length} scripts and ${data.title.instances.length} title instances`);
  } catch (error) { console.error(error instanceof Error ? error.message : error); process.exitCode = 1; }
}
