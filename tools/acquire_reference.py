#!/usr/bin/env python3
"""Acquire and verify Classic 1.1 reference data; never execute original binaries.

For source export, supply a local GmkSplitter v0.21 JAR and Java executable:
  python3 tools/acquire_reference.py --splitter /path/gmksplit.jar --java java
Without --splitter this downloads/verifies the originals and extracts safe data.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import subprocess
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / 'references'
DOWNLOADS = {
    'spelunky_1_1_src.zip': ('https://www.derekyu.com/games/spelunky_1_1_src.zip', 'ad7d23e8de6a1398d245d228869d5c5bbd2c0e72910d66539c21306265cf7014'),
    'spelunky_1_1.zip': ('https://www.derekyu.com/games/spelunky_1_1.zip', '572f0a1976e76ebbd295daae651b8b6f00c6b9cf960d3e1c2cc57128d6437727'),
}
SPLITTER_SHA256 = '319b9b0bf7a724059a18dfbe167b4057ec9d32d8269bf5ff4e0b2a6aed5ad3ed'

def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def extract_safe(archive: Path, destination: Path, suffixes: set[str]) -> None:
    with zipfile.ZipFile(archive) as source:
        for item in source.infolist():
            path = PurePosixPath(item.filename)
            if path.is_absolute() or '..' in path.parts:
                raise ValueError(f'Unsafe archive path: {path}')
            if path.suffix.lower() not in suffixes:
                continue
            output = destination.joinpath(*path.parts)
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_bytes(source.read(item))

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--splitter', type=Path)
    parser.add_argument('--java', default='java')
    parser.add_argument('--offline', action='store_true', help='Require existing verified archives')
    args = parser.parse_args()
    original = REFERENCE / 'original'
    original.mkdir(parents=True, exist_ok=True)
    for name, (url, expected) in DOWNLOADS.items():
        output = original / name
        if not output.exists():
            if args.offline:
                raise FileNotFoundError(output)
            with urllib.request.urlopen(url, timeout=120) as response:
                output.write_bytes(response.read())
        if sha256(output) != expected:
            raise ValueError(f'Checksum mismatch: {output}; preserve it for inspection and reacquire explicitly')
    extract_safe(original / 'spelunky_1_1_src.zip', original / 'source', {'.gmk', '.txt'})
    extract_safe(original / 'spelunky_1_1.zip', original / 'game', {'.wav', '.ogg', '.txt', '.cfg'})
    if args.splitter:
        if sha256(args.splitter) != SPLITTER_SHA256:
            raise ValueError('Expected the verified GmkSplitter v0.21 JAR; checksum differs')
        exported = REFERENCE / 'classic-1.1'
        if exported.exists():
            print(f'Keeping existing export: {exported}')
        else:
            subprocess.run([args.java, '-jar', str(args.splitter), '--preserve-ids', 'all', '--charset', 'windows-1252', str(original / 'source/spelunky.gmk'), str(exported)], check=True)
    manifest = {str(path.relative_to(REFERENCE)): sha256(path) for path in sorted(REFERENCE.rglob('*')) if path.is_file() and path.name != 'SHA256SUMS.json'}
    (REFERENCE / 'SHA256SUMS.json').write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')
    print(f'Verified originals and indexed {len(manifest)} reference files')

if __name__ == '__main__':
    main()
