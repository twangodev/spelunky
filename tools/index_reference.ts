#!/usr/bin/env bun
/** Produce a deterministic index of the untouched Classic export. */
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, parse, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { integer, parseArgs, walkFiles, writeOrCheck } from "./lib/common.ts";
import { pythonJson } from "./lib/python_json.ts";
import { readXml, serializeXml } from "./lib/xml.ts";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const SOURCE = join(ROOT, "references/classic-1.1");

export function indexReference() {
  const result: Record<string, any> = { source: "Official Spelunky Classic 1.1 spelunky.gmk", sprites: [], objects: [], rooms: [], scripts: [], backgrounds: [], audio: [] };
  const kinds = [["Sprites", "sprite", "sprites"], ["Objects", "object", "objects"], ["Rooms", "room", "rooms"], ["Backgrounds", "background", "backgrounds"]];
  for (const [kind, tag, key] of kinds) {
    for (const path of walkFiles(join(SOURCE, kind), ".xml")) {
      const node = readXml(path);
      if (node.tag !== tag) continue;
      const record: Record<string, unknown> = { name: parse(path).name, id: integer(node.get("id")), path: relative(SOURCE, path).split(sep).join("/") };
      if (tag === "sprite") {
        const imagePath = path.slice(0, -4) + ".images";
        Object.assign(record, { origin: node.find("origin")!.attrib, mask: serializeXml(node.find("mask")!).trim(), frames: existsSync(imagePath) ? readdirSync(imagePath).filter(name => name.endsWith(".png")).length : 0 });
      } else if (tag === "object") {
        const eventsPath = path.slice(0, -4) + ".events";
        Object.assign(record, { parent: node.findText("parent"), sprite: node.findText("sprite"), mask: node.findText("mask"), depth: node.findText("depth"), events: existsSync(eventsPath) ? readdirSync(eventsPath).filter(name => name.endsWith(".xml")).sort() : [] });
      } else if (tag === "room") {
        const view = node.find("views/view/viewInRoom");
        Object.assign(record, { speed: integer(node.findText("speed")), size: node.find("size")!.attrib, view: view?.attrib ?? null, instances: node.findAll("instances/instance").length });
      }
      result[key].push(record);
    }
  }
  for (const path of walkFiles(join(SOURCE, "Scripts"), ".gml")) {
    const first = readFileSync(path, "utf8").split(/\r\n|\n|\r/)[0];
    if (!first && !readFileSync(path, "utf8").length) throw new Error(`Empty script: ${path}`);
    result.scripts.push({ name: parse(path).name, path: relative(SOURCE, path).split(sep).join("/"), id: first.includes("!scriptId=") ? integer(first.split("=")[1].trim().split(/\s+/)[0]) : null });
  }
  const soundDirectory = join(ROOT, "references/original/game/sound");
  if (existsSync(soundDirectory)) for (const name of readdirSync(soundDirectory).sort()) result.audio.push(relative(join(ROOT, "references"), join(soundDirectory, name)).split(sep).join("/"));
  result.counts = Object.fromEntries(Object.entries(result).filter(([, value]) => Array.isArray(value)).map(([key, value]) => [key, value.length]));
  return result;
}

if (import.meta.main) {
  const args = parseArgs({ output: join(ROOT, "references/source-index.json") }, "Produce a deterministic machine-readable index of the untouched Classic export.");
  writeOrCheck(String(args.output), pythonJson(indexReference(), { indent: 2, sortKeys: true }) + "\n", Boolean(args.check));
  console.log(args.output);
}
