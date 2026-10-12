#!/usr/bin/env bun
/** Export authored Classic rooms as data; creation code is never executed. */
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { parseXml, XmlElement } from "./lib/xml.ts";
import { pythonJson } from "./lib/python_json.ts";
import { integer, parseArgs, writeOrCheck } from "./lib/common.ts";

export const ROOM_NAMES = ["rIntro", "rTitle", "rHighscores", "rSun", "rMoon", "rStars"];

function boolean(element: XmlElement, name: string): boolean {
  const value = element.findText(name, "false");
  if (value !== "true" && value !== "false") throw new Error(`Invalid ${name}: ${JSON.stringify(value)}`);
  return value === "true";
}
function attributes(element: XmlElement, name: string): Record<string, number> {
  const node = element.find(name);
  if (!node) throw new Error(`Missing ${name} in ${element.tag}`);
  return Object.fromEntries(Object.entries(node.attrib).map(([key, value]) => [key, integer(value)]));
}

export function room(path: string) {
  const raw = readFileSync(path);
  const root = parseXml(raw);
  if (root.tag !== "room") throw new Error(`Not a room: ${path}`);
  const ids = new Set<number>();
  const instances = root.findAll("instances/instance").map(node => {
    const id = integer(node.attrib.id);
    if (ids.has(id)) throw new Error(`Duplicate instance id ${id} in ${path}`);
    ids.add(id);
    const object = node.findText("object", "");
    if (!object) throw new Error(`Missing instance object in ${path}`);
    return { id, object, ...attributes(node, "position"), creation_code: node.findText("creationCode", "") || "", locked: boolean(node, "locked") };
  });
  const backgrounds = root.findAll("backgrounds/backgroundDef").map(node => ({
    visible_on_room_start: boolean(node, "visibleOnRoomStart"),
    is_foreground: boolean(node, "isForeground"),
    image: node.findText("backgroundImage", "") || "",
    offset: attributes(node, "offset"), speed: attributes(node, "speed"),
    tile_horizontally: boolean(node, "tileHorizontally"), tile_vertically: boolean(node, "tileVertically"), stretch: boolean(node, "stretch"),
  }));
  const views = root.findAll("views/view").map(node => ({
    visible_on_room_start: boolean(node, "visibleOnRoomStart"),
    view_in_room: attributes(node, "viewInRoom"), port_on_screen: attributes(node, "portOnScreen"),
    object_following: attributes(node, "objectFollowing"), object_following_name: node.findText("objectFollowing", "") || "",
  }));
  const tiles = root.findAll("tiles/tile").map(node => {
    const position = attributes(node, "backgroundPosition");
    return { id: integer(node.attrib.id), background: node.findText("background", "") || "",
      background_x: position.x, background_y: position.y,
      ...attributes(node, "roomPosition"), ...attributes(node, "size"),
      depth: integer(node.findText("depth", "0")), locked: boolean(node, "locked") };
  });
  return {
    source: `Rooms/${basename(path)}`, source_sha256: createHash("sha256").update(raw).digest("hex"),
    id: integer(root.attrib.id), ...attributes(root, "size"), speed: integer(root.findText("speed", "0")),
    caption: root.findText("caption", "") || "", persistent: boolean(root, "persistent"),
    creation_code: root.findText("creationCode", "") || "", background_color: root.findText("backgroundColor", "") || "",
    draw_background_color: boolean(root, "drawBackgroundColor"), backgrounds,
    enable_views: boolean(root, "enableViews"), views, instances, tiles,
  };
}
export function exportHub(source: string) {
  return { version: 1, rooms: Object.fromEntries(ROOM_NAMES.map(name => [name, room(join(source, "Rooms", `${name}.xml`))])) };
}

if (import.meta.main) {
  try {
    const values = parseArgs({ source: "references/classic-1.1", output: "assets/hub.json" }, "Export authored Classic rooms");
      const data = exportHub(String(values.source));
      const contents = pythonJson(data, { sortKeys: true }) + "\n";
      writeOrCheck(String(values.output), contents, Boolean(values.check));
      console.log(`${ROOM_NAMES.length} rooms, ${Object.values(data.rooms).reduce((sum, value) => sum + value.instances.length, 0)} instances: ${values.output}`);
  } catch (error) { console.error(error instanceof Error ? error.message : error); process.exitCode = 1; }
}
