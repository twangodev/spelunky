#!/usr/bin/env python3
"""Export Classic room/obstacle literals with their exact source conditions.

Only the Python standard library is required. The original source remains the
authority for random selection and side effects; this file extracts data, not a
replacement implementation of the GameMaker runtime.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET


TOKEN = re.compile(r'(?P<comment>/\*.*?\*/|//[^\n]*)|(?P<string>"[^"\n]*"|\'[^\'\n]*\')|(?P<word>[A-Za-z_][A-Za-z_0-9]*)|(?P<space>\s+)|(?P<other>.)', re.S)


class LiteralParser:
    """Walk statements sufficiently to retain nested if/switch/case provenance."""

    def __init__(self, source):
        self.source = source
        self.tokens = [(m.group(), m.start(), m.end()) for m in TOKEN.finditer(source)
                       if m.lastgroup not in ("comment", "space")]
        self.i = 0
        self.records = []

    def peek(self):
        return self.tokens[self.i][0] if self.i < len(self.tokens) else None

    def consume(self):
        token = self.tokens[self.i]
        self.i += 1
        return token

    def condition(self):
        if self.peek() != "(":
            raise ValueError("Expected parenthesized condition")
        start = self.consume()[1]
        depth = 1
        while self.i < len(self.tokens):
            token, _, end = self.consume()
            depth += (token == "(") - (token == ")")
            if depth == 0:
                return self.source[start:end]
        raise ValueError("Unclosed condition")

    def block(self, context):
        self.consume()  # opening brace
        case = []
        while self.peek() not in (None, "}"):
            if self.peek() in ("case", "default"):
                start = self.consume()[1]
                while self.peek() not in (None, ":"):
                    self.consume()
                end = self.consume()[2]
                case = [self.source[start:end]]
            else:
                self.statement(context + case)
        if self.peek() == "}":
            self.consume()

    def statement(self, context):
        if self.peek() == "{":
            self.block(context)
            return
        if self.peek() in ("if", "switch", "for", "while", "with", "repeat"):
            name = self.consume()[0]
            condition = self.condition()
            self.statement(context + [name + condition])
            if name == "if" and self.peek() == "else":
                self.consume()
                self.statement(context + ["else"])
            return
        start = self.i
        while self.peek() not in (None, ";", "}", "{"):
            self.consume()
        end = self.i
        if self.peek() == ";":
            self.consume()
        elif self.peek() == "{":
            # Unknown block-form statements remain explicitly marked.
            self.block(context + ["unknown"])
        if end <= start:
            return
        tokens = self.tokens[start:end]
        for n in range(len(tokens) - 2):
            variable = tokens[n][0]
            if not re.fullmatch(r"strTemp|strObs[1234]?", variable):
                continue
            if tokens[n + 1][0] != "=":
                continue
            expression = tokens[n + 2:]
            # Export literal concatenations only; mutations using string_insert
            # are represented by the original script, not fabricated literals.
            if not expression or any(t[0] != "+" and t[0][:1] not in ('"', "'") for t in expression):
                continue
            value = "".join(t[0][1:-1] for t in expression if t[0] != "+")
            position = tokens[n][1]
            line = self.source.count("\n", 0, position) + 1
            source_line = self.source.splitlines()[line - 1].strip()
            case = next((int(re.search(r"\d+", c).group()) for c in reversed(context)
                         if c.startswith("case") and re.search(r"\d+", c)), None)
            record = {"line": line, "variable": variable, "value": value,
                      "context": context, "source_line": source_line}
            if case is not None:
                record["case"] = case
            record["kind"] = classify(variable, context)
            self.records.append(record)

    def parse(self):
        while self.peek() is not None:
            self.statement([])
        return self.records


def classify(variable, context):
    if variable.startswith("strObs"):
        for c in context:
            match = re.search(r'tile\s*==\s*"(.)"', c)
            if match:
                return "obstacle_" + match[1]
        return "obstacle_default"
    for c in context:
        if "global.startRoomX" in c:
            return "start"
        if "global.endRoomX" in c:
            return "end"
        if re.search(r"roomPath\s*==\s*0", c):
            return "main" if re.search(r"roomPath\s*==\s*1", c) else "side"
        match = re.search(r"roomPath\s*==\s*(\d+)", c)
        if match:
            return {1: "main", 3: "landing", 4: "shop_left", 5: "shop_right"}.get(
                int(match[1]), "special_" + match[1])
        if re.search(r"y\s*<\s*480", c):
            return "boss_upper"
    return "drop" if context else "default"


def export(reference):
    scripts = {}
    for path in sorted((reference / "Scripts" / "Level Generation").glob("scrRoomGen*.gml")):
        raw = path.read_bytes()
        source = raw.decode("utf-8-sig").replace("\r\n", "\n")
        # Room literals and obstacle substitutions precede this marker in all
        # original scripts. Tile materialization contains no template literals.
        data_source = source.split("// Generate the tiles", 1)[0]
        records = LiteralParser(data_source).parse()
        for record in records:
            if record["variable"] == "strTemp" and len(record["value"]) != 80:
                raise ValueError(f"Invalid room literal in {path}:{record['line']}")
            if path.stem == "scrRoomGen5" and record["kind"] == "drop":
                record["kind"] = "boss_lower"
        scripts[path.stem] = {"source": path.relative_to(reference).as_posix(),
                              "source_sha256": hashlib.sha256(raw).hexdigest(),
                              "selection_source": data_source,
                              "records": records}
    title_path = reference / "Rooms" / "rTitle.xml"
    title_xml = ET.parse(title_path).getroot()
    title = {"source": "Rooms/rTitle.xml", "width": int(title_xml.find("size").get("width")),
             "height": int(title_xml.find("size").get("height")),
             "speed": int(title_xml.findtext("speed")), "instances": []}
    for instance in title_xml.findall("instances/instance"):
        position = instance.find("position")
        title["instances"].append({"object": instance.findtext("object"),
                                   "x": int(position.get("x")), "y": int(position.get("y")),
                                   "creation_code": instance.findtext("creationCode") or ""})
    create = ET.parse(reference / "Objects" / "oTitle.events" / "Create.xml").getroot()
    title["create_code"] = "\n".join(arg.text or "" for arg in create.findall(".//argument[@kind='STRING']"))
    return {"schema_version": 1, "tile_size": 16, "room_width": 10,
            "room_height": 8, "obstacle_width": 5, "obstacle_height": 3,
            "scripts": scripts, "title": title}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, default=Path("references/classic-1.1"))
    parser.add_argument("--output", type=Path, default=Path("assets/generation.json"))
    args = parser.parse_args()
    data = export(args.reference)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"Exported {sum(len(s['records']) for s in data['scripts'].values())} literals "
          f"from {len(data['scripts'])} scripts and {len(data['title']['instances'])} title instances")


if __name__ == "__main__":
    main()
