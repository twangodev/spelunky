#!/usr/bin/env bun
/** Export Classic Collision event declarations, including empty local events. */
import { readFileSync } from "node:fs";
import { basename, dirname, join, parse, relative, sep } from "node:path";
import { integer, parseArgs, walkFiles, writeOrCheck } from "./lib/common.ts";
import { pythonJson } from "./lib/python_json.ts";
import { readXml, type XmlElement } from "./lib/xml.ts";

export function exportCollisionEvents(source: string, catalogPath: string) {
  const catalog = JSON.parse(readFileSync(catalogPath, "utf8")).objects;
  const objectIds: Record<string, number> = Object.create(null);
  const eventPaths: [string, XmlElement][] = [];
  for (const path of walkFiles(join(source, "Objects"), ".xml")) {
    const root = readXml(path);
    if (root.tag === "object") {
      const name = parse(path).name;
      if (Object.hasOwn(objectIds, name)) throw new Error(`Duplicate object ${name}`);
      objectIds[name] = integer(root.attrib.id);
      if (!Object.hasOwn(catalog, name) || catalog[name].id !== objectIds[name]) throw new Error(`Catalog resource ID differs from source for ${name}`);
    } else if (root.tag === "event" && root.get("category") === "COLLISION") eventPaths.push([path, root]);
  }
  if (!Object.keys(objectIds).length) throw new Error(`No objects in ${source}`);
  const events = [];
  const seen = new Set<string>();
  for (const [path, root] of eventPaths) {
    const directory = basename(dirname(path));
    if (!directory.endsWith(".events")) throw new Error(`Collision event outside an object event directory: ${path}`);
    const owner = directory.slice(0, -".events".length);
    const target = root.attrib.with;
    if (!Object.hasOwn(objectIds, owner) || !Object.hasOwn(objectIds, target)) throw new Error(`Unknown Collision owner or target: ${owner} -> ${target}`);
    const key = JSON.stringify([owner, target]);
    if (seen.has(key)) throw new Error(`Duplicate Collision declaration: ${owner} -> ${target}`);
    seen.add(key);
    events.push({ owner, owner_id: objectIds[owner], target, target_id: objectIds[target],
      source: relative(source, path).split(sep).join("/"), action_count: root.findAll("./actions/action").length });
  }
  if (!events.length) throw new Error(`No Collision events in ${source}`);
  events.sort((left, right) => left.owner_id - right.owner_id || left.target_id - right.target_id);
  return { version: 1, source: "references/classic-1.1", events };
}

if (import.meta.main) {
  const args = parseArgs({ source: "references/classic-1.1", catalog: "assets/catalog.json", output: "assets/collision-events.json" }, "Export Classic Collision event declarations from the preserved source XML.");
  const metadata = exportCollisionEvents(String(args.source), String(args.catalog));
  writeOrCheck(String(args.output), pythonJson(metadata, { indent: 2 }) + "\n", Boolean(args.check));
  console.log(`${metadata.events.length} Collision events: ${args.output}`);
}
