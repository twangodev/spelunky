#!/usr/bin/env python3
"""Export original object defaults and sprite collision metadata deterministically.

Uses only the Python standard library. Run after export_assets.py. GameMaker's
`solid` property is preserved verbatim; gameplay collision groups are determined
by object inheritance, independently of that property.
"""

import argparse
import json
from pathlib import Path
import xml.etree.ElementTree as ET


def export(source: Path, atlas_path: Path) -> dict:
    atlas = json.loads(atlas_path.read_text())
    objects = {}
    for path in sorted((source / "Objects").rglob("*.xml")):
        if any(part.endswith(".events") for part in path.parts):
            continue
        root = ET.parse(path).getroot()
        if root.tag != "object":
            continue
        name = path.stem
        if name in objects:
            raise ValueError(f"Duplicate object {name}")
        objects[name] = {
            "name": name,
            "id": int(root.attrib["id"]),
            "sprite": root.findtext("sprite", "") or "",
            "parent": root.findtext("parent", "") or "",
            "solid": root.findtext("solid") == "true",
            "visible": root.findtext("visible") == "true",
            "depth": int(root.findtext("depth", "0")),
            "persistent": root.findtext("persistent") == "true",
            "mask": root.findtext("mask", "") or "",
        }
    if not objects:
        raise ValueError(f"No objects in {source}")
    for name, obj in objects.items():
        seen = set()
        current = name
        while current:
            if current in seen:
                raise ValueError(f"Inheritance cycle at {name}: {current}")
            seen.add(current)
            if current not in objects:
                raise ValueError(f"Unknown parent {current} for {name}")
            current = objects[current]["parent"]
        for field in ("sprite", "mask"):
            if obj[field] and obj[field] not in atlas["sprites"]:
                raise ValueError(f"Unknown {field} {obj[field]} for {name}")
    sprites = {
        name: {"id": sprite["id"], "origin": sprite["origin"], "bounds": sprite["bounds"]}
        for name, sprite in atlas["sprites"].items()
    }
    return {"version": 1, "objects": objects, "sprites": sprites}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path("references/classic-1.1"))
    parser.add_argument("--atlas", type=Path, default=Path("assets/atlas.json"))
    parser.add_argument("--output", type=Path, default=Path("assets/catalog.json"))
    parser.add_argument("--check", action="store_true", help="Verify generated data without writing")
    args = parser.parse_args()
    catalog = export(args.source, args.atlas)
    contents = json.dumps(catalog, indent=2, sort_keys=True) + "\n"
    if args.check:
        if not args.output.exists() or args.output.read_text() != contents:
            raise SystemExit(f"{args.output} is out of date")
    else:
        args.output.write_text(contents)
    print(f"{len(catalog['objects'])} objects, {len(catalog['sprites'])} sprites: {args.output}")


if __name__ == "__main__":
    main()
