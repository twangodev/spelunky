#!/usr/bin/env bun
/** Export original object defaults and sprite collision metadata deterministically. */
import { readFileSync } from "node:fs";
import { dirname, join, parse, sep } from "node:path";
import { integer, parseArgs, walkFiles, writeOrCheck } from "./lib/common.ts";
import { pythonJson } from "./lib/python_json.ts";
import { readXml } from "./lib/xml.ts";

interface ObjectRecord {
  name: string; id: number; sprite: string; parent: string; solid: boolean;
  visible: boolean; depth: number; persistent: boolean; mask: string;
}

export function exportCatalog(source: string, atlasPath: string) {
  const atlas = JSON.parse(readFileSync(atlasPath, "utf8"));
  const objects: Record<string, ObjectRecord> = Object.create(null);
  for (const path of walkFiles(join(source, "Objects"), ".xml")) {
    if (dirname(path).split(sep).some(part => part.endsWith(".events"))) continue;
    const root = readXml(path);
    if (root.tag !== "object") continue;
    const name = parse(path).name;
    if (Object.hasOwn(objects, name)) throw new Error(`Duplicate object ${name}`);
    objects[name] = {
      name, id: integer(root.attrib.id),
      sprite: root.findText("sprite", "") || "",
      parent: root.findText("parent", "") || "",
      solid: root.findText("solid") === "true", visible: root.findText("visible") === "true",
      depth: integer(root.findText("depth", "0")), persistent: root.findText("persistent") === "true",
      mask: root.findText("mask", "") || "",
    };
  }
  if (!Object.keys(objects).length) throw new Error(`No objects in ${source}`);
  for (const [name, object] of Object.entries(objects)) {
    const seen = new Set<string>();
    let current = name;
    while (current) {
      if (seen.has(current)) throw new Error(`Inheritance cycle at ${name}: ${current}`);
      seen.add(current);
      if (!Object.hasOwn(objects, current)) throw new Error(`Unknown parent ${current} for ${name}`);
      current = objects[current].parent;
    }
    for (const field of ["sprite", "mask"] as const) {
      if (object[field] && !Object.hasOwn(atlas.sprites, object[field])) throw new Error(`Unknown ${field} ${object[field]} for ${name}`);
    }
  }
  const sprites: Record<string, unknown> = Object.create(null);
  for (const [name, sprite] of Object.entries(atlas.sprites) as [string, any][]) {
    sprites[name] = { id: sprite.id, origin: sprite.origin, bounds: sprite.bounds };
  }
  return { version: 1, objects, sprites };
}

if (import.meta.main) {
  const args = parseArgs({ source: "references/classic-1.1", atlas: "assets/atlas.json", output: "assets/catalog.json" }, "Export original object defaults and sprite collision metadata deterministically.");
  const catalog = exportCatalog(String(args.source), String(args.atlas));
  writeOrCheck(String(args.output), pythonJson(catalog, { indent: 2, sortKeys: true }) + "\n", Boolean(args.check));
  console.log(`${Object.keys(catalog.objects).length} objects, ${Object.keys(catalog.sprites).length} sprites: ${args.output}`);
}
