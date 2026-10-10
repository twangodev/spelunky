#!/usr/bin/env python3
"""Export Classic Collision event declarations from the preserved source XML.

Uses only the Python standard library. Run after export_catalog.py. Declarations
retain their action counts: the runner ignores an empty local event when looking
for inherited Collision handlers. Source paths are relative to --source, and
records follow the original owner/target resource ordering. The source label is
the canonical repository provenance, independent of the physical --source path.
"""

import argparse
import json
from pathlib import Path
import xml.etree.ElementTree as ET


def export(source: Path, catalog_path: Path) -> dict:
    catalog = json.loads(catalog_path.read_text())["objects"]
    object_ids = {}
    event_paths = []
    for path in sorted((source / "Objects").rglob("*.xml")):
        root = ET.parse(path).getroot()
        if root.tag == "object":
            name = path.stem
            if name in object_ids:
                raise ValueError(f"Duplicate object {name}")
            object_ids[name] = int(root.attrib["id"])
            if name not in catalog or catalog[name]["id"] != object_ids[name]:
                raise ValueError(f"Catalog resource ID differs from source for {name}")
        elif root.tag == "event" and root.get("category") == "COLLISION":
            event_paths.append((path, root))
    if not object_ids:
        raise ValueError(f"No objects in {source}")

    events = []
    seen = set()
    for path, root in event_paths:
        if not path.parent.name.endswith(".events"):
            raise ValueError(f"Collision event outside an object event directory: {path}")
        owner = path.parent.name.removesuffix(".events")
        target = root.attrib["with"]
        if owner not in object_ids or target not in object_ids:
            raise ValueError(f"Unknown Collision owner or target: {owner} -> {target}")
        if (owner, target) in seen:
            raise ValueError(f"Duplicate Collision declaration: {owner} -> {target}")
        seen.add((owner, target))
        events.append({
            "owner": owner,
            "owner_id": object_ids[owner],
            "target": target,
            "target_id": object_ids[target],
            "source": path.relative_to(source).as_posix(),
            "action_count": len(root.findall("./actions/action")),
        })
    if not events:
        raise ValueError(f"No Collision events in {source}")
    events.sort(key=lambda event: (event["owner_id"], event["target_id"]))
    return {"version": 1, "source": "references/classic-1.1", "events": events}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path("references/classic-1.1"))
    parser.add_argument("--catalog", type=Path, default=Path("assets/catalog.json"))
    parser.add_argument("--output", type=Path, default=Path("assets/collision-events.json"))
    parser.add_argument("--check", action="store_true", help="Verify generated data without writing")
    args = parser.parse_args()
    metadata = export(args.source, args.catalog)
    contents = json.dumps(metadata, indent=2) + "\n"
    if args.check:
        if not args.output.exists() or args.output.read_text() != contents:
            raise SystemExit(f"{args.output} is out of date")
    else:
        args.output.write_text(contents)
    print(f"{len(metadata['events'])} Collision events: {args.output}")


if __name__ == "__main__":
    main()
