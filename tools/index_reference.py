#!/usr/bin/env python3
"""Produce a deterministic machine-readable index of the untouched Classic export."""
from pathlib import Path
import json
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'references/classic-1.1'

def index():
    result = {'source': 'Official Spelunky Classic 1.1 spelunky.gmk', 'sprites': [], 'objects': [], 'rooms': [], 'scripts': [], 'backgrounds': [], 'audio': []}
    for kind, tag in [('Sprites', 'sprite'), ('Objects', 'object'), ('Rooms', 'room'), ('Backgrounds', 'background')]:
        for path in sorted((SOURCE / kind).rglob('*.xml')):
            node = ET.parse(path).getroot()
            if node.tag != tag:
                continue
            record = {'name': path.stem, 'id': int(node.get('id')), 'path': path.relative_to(SOURCE).as_posix()}
            if tag == 'sprite':
                record.update(origin=node.find('origin').attrib, mask=ET.tostring(node.find('mask'), encoding='unicode').strip(), frames=len(list(path.with_suffix('.images').glob('*.png'))))
            elif tag == 'object':
                record.update(parent=node.findtext('parent'), sprite=node.findtext('sprite'), mask=node.findtext('mask'), depth=node.findtext('depth'), events=[p.name for p in sorted(path.with_suffix('.events').glob('*.xml'))])
            elif tag == 'room':
                view = node.find('views/view/viewInRoom')
                record.update(speed=int(node.findtext('speed')), size=node.find('size').attrib, view=view.attrib if view is not None else None, instances=len(node.findall('instances/instance')))
            result[{'sprite':'sprites','object':'objects','room':'rooms','background':'backgrounds'}[tag]].append(record)
    for path in sorted((SOURCE / 'Scripts').rglob('*.gml')):
        first = path.read_text().splitlines()[0]
        result['scripts'].append({'name':path.stem,'path':path.relative_to(SOURCE).as_posix(),'id':int(first.split('=')[1].split()[0]) if '!scriptId=' in first else None})
    for path in sorted((ROOT / 'references/original/game/sound').glob('*')):
        result['audio'].append(path.relative_to(ROOT/'references').as_posix())
    result['counts'] = {key:len(value) for key,value in result.items() if isinstance(value,list)}
    return result

if __name__ == '__main__':
    output = ROOT/'references/source-index.json'
    output.write_text(json.dumps(index(), indent=2, sort_keys=True)+'\n')
    print(output)
