#!/usr/bin/env python3
"""Export authored Classic ending, credits, and transition rooms exactly."""

import argparse
import json
from pathlib import Path

from export_hub import room


ROOM_NAMES = (
    "rEnd", "rEnd2", "rEnd3", "rEndCustom", "rCredits1", "rCredits2",
    "rTransition1", "rTransition1x", "rTransition2", "rTransition2x",
    "rTransition3", "rTransition3x", "rTransition4",
)


def export(source: Path) -> dict:
    return {"version": 1, "rooms": {
        name: room(source / "Rooms" / f"{name}.xml") for name in ROOM_NAMES
    }}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path("references/classic-1.1"))
    parser.add_argument("--output", type=Path, default=Path("assets/cinematics.json"))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = export(args.source)
    contents = json.dumps(data, indent=2, sort_keys=True) + "\n"
    if args.check:
        if not args.output.exists() or args.output.read_text() != contents:
            raise SystemExit(f"{args.output} is out of date")
    else:
        args.output.write_text(contents)
    count = sum(len(value["instances"]) for value in data["rooms"].values())
    print(f"{len(data['rooms'])} rooms, {count} instances: {args.output}")


if __name__ == "__main__":
    main()
