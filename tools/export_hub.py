#!/usr/bin/env python3
"""Export the exact authored Classic introduction, title, and challenge-room layouts.

Room and instance creation code is preserved as data, never executed. Instance,
background, view, and tile ordering matches the source XML. Controller event
behavior lives in the original object events and is not implied by this export.
Only the Python standard library is required.
"""

import argparse
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET


ROOM_NAMES = ("rIntro", "rTitle", "rHighscores", "rSun", "rMoon", "rStars")


def boolean(element: ET.Element, name: str) -> bool:
    value = element.findtext(name, "false")
    if value not in ("true", "false"):
        raise ValueError(f"Invalid {name}: {value!r}")
    return value == "true"


def attributes(element: ET.Element, name: str) -> dict:
    node = element.find(name)
    if node is None:
        raise ValueError(f"Missing {name} in {element.tag}")
    return {key: int(value) for key, value in node.attrib.items()}


def room(path: Path) -> dict:
    raw = path.read_bytes()
    root = ET.fromstring(raw)
    if root.tag != "room":
        raise ValueError(f"Not a room: {path}")
    size = attributes(root, "size")
    instances = []
    ids = set()
    for node in root.findall("instances/instance"):
        instance_id = int(node.attrib["id"])
        if instance_id in ids:
            raise ValueError(f"Duplicate instance id {instance_id} in {path}")
        ids.add(instance_id)
        object_name = node.findtext("object", "")
        if not object_name:
            raise ValueError(f"Missing instance object in {path}")
        instances.append({
            "id": instance_id,
            "object": object_name,
            **attributes(node, "position"),
            "creation_code": node.findtext("creationCode", "") or "",
            "locked": boolean(node, "locked"),
        })
    backgrounds = [{
        "visible_on_room_start": boolean(node, "visibleOnRoomStart"),
        "is_foreground": boolean(node, "isForeground"),
        "image": node.findtext("backgroundImage", "") or "",
        "offset": attributes(node, "offset"),
        "speed": attributes(node, "speed"),
        "tile_horizontally": boolean(node, "tileHorizontally"),
        "tile_vertically": boolean(node, "tileVertically"),
        "stretch": boolean(node, "stretch"),
    } for node in root.findall("backgrounds/backgroundDef")]
    views = [{
        "visible_on_room_start": boolean(node, "visibleOnRoomStart"),
        "view_in_room": attributes(node, "viewInRoom"),
        "port_on_screen": attributes(node, "portOnScreen"),
        "object_following": attributes(node, "objectFollowing"),
        "object_following_name": node.findtext("objectFollowing", "") or "",
    } for node in root.findall("views/view")]
    tiles = []
    for node in root.findall("tiles/tile"):
        background_position = attributes(node, "backgroundPosition")
        tiles.append({
            "id": int(node.attrib["id"]),
            "background": node.findtext("background", "") or "",
            "background_x": background_position["x"],
            "background_y": background_position["y"],
            **attributes(node, "roomPosition"),
            **attributes(node, "size"),
            "depth": int(node.findtext("depth", "0")),
            "locked": boolean(node, "locked"),
        })
    return {
        "source": f"Rooms/{path.name}",
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "id": int(root.attrib["id"]),
        **size,
        "speed": int(root.findtext("speed", "0")),
        "caption": root.findtext("caption", "") or "",
        "persistent": boolean(root, "persistent"),
        "creation_code": root.findtext("creationCode", "") or "",
        "background_color": root.findtext("backgroundColor", "") or "",
        "draw_background_color": boolean(root, "drawBackgroundColor"),
        "backgrounds": backgrounds,
        "enable_views": boolean(root, "enableViews"),
        "views": views,
        "instances": instances,
        "tiles": tiles,
    }


def export(source: Path) -> dict:
    return {"version": 1, "rooms": {
        name: room(source / "Rooms" / f"{name}.xml") for name in ROOM_NAMES
    }}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path("references/classic-1.1"))
    parser.add_argument("--output", type=Path, default=Path("assets/hub.json"))
    parser.add_argument("--check", action="store_true", help="Verify generated data without writing")
    args = parser.parse_args()
    data = export(args.source)
    contents = json.dumps(data, indent=2, sort_keys=True) + "\n"
    if args.check:
        if not args.output.exists() or args.output.read_text() != contents:
            raise SystemExit(f"{args.output} is out of date")
    else:
        args.output.write_text(contents)
    instances = sum(len(value["instances"]) for value in data["rooms"].values())
    print(f"{len(data['rooms'])} rooms, {instances} instances: {args.output}")


if __name__ == "__main__":
    main()
