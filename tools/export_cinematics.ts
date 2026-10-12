#!/usr/bin/env bun
/** Export authored Classic ending, credits, and transition rooms exactly. */
import { join } from "node:path";
import { room } from "./export_hub.ts";
import { parseArgs, writeOrCheck } from "./lib/common.ts";
import { pythonJson } from "./lib/python_json.ts";

export const ROOM_NAMES = ["rEnd", "rEnd2", "rEnd3", "rEndCustom", "rCredits1", "rCredits2",
  "rTransition1", "rTransition1x", "rTransition2", "rTransition2x", "rTransition3", "rTransition3x", "rTransition4"];

export function exportCinematics(source: string) {
  return { version: 1, rooms: Object.fromEntries(ROOM_NAMES.map(name => [name, room(join(source, "Rooms", `${name}.xml`))])) };
}

if (import.meta.main) {
  const args = parseArgs({ source: "references/classic-1.1", output: "assets/cinematics.json" }, "Export authored Classic ending, credits, and transition rooms exactly.");
  const data = exportCinematics(String(args.source));
  writeOrCheck(String(args.output), pythonJson(data, { indent: 2, sortKeys: true }) + "\n", Boolean(args.check));
  const count = Object.values(data.rooms).reduce((sum, value) => sum + value.instances.length, 0);
  console.log(`${Object.keys(data.rooms).length} rooms, ${count} instances: ${args.output}`);
}
