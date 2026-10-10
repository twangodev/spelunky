#!/usr/bin/env bash
# Build the self-contained static bundle under web/dist/ from any working directory.
set -euo pipefail

mode=--release
case "${1:-}" in
  ""|--release) ;;
  --dev) mode=--dev ;;
  -h|--help)
    echo "Usage: $0 [--release|--dev]"
    exit 0
    ;;
  *) echo "Unknown option: $1" >&2; exit 2 ;;
esac
if (( $# > 1 )); then
  echo "Usage: $0 [--release|--dev]" >&2
  exit 2
fi

for tool in cargo rustup wasm-pack python3 bun node; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Required build tool is missing: $tool; install it, then rerun this command" >&2
    exit 1
  fi
done
if ! rustup target list --installed | grep -qx wasm32-unknown-unknown; then
  echo "Install the Wasm target: rustup target add wasm32-unknown-unknown" >&2
  exit 1
fi

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$repo_root"
python3 tools/export_assets.py --verify
# Relative output paths are relative to crates/app, not the shell directory.
wasm-pack build crates/app --target web --out-dir ../../web/public/pkg \
  --out-name spelunky "$mode" -- --locked
python3 - <<'PY'
from pathlib import Path
import shutil

destination = Path("web/public/assets")
if destination.exists():
    shutil.rmtree(destination)
destination.mkdir(parents=True)
for name in ("COPYING.txt", "ORIGINAL_README.txt"):
    shutil.copy2(Path("assets") / name, destination / name)
PY
bun install --cwd web --frozen-lockfile
bun run --cwd web build
echo "Static bundle ready in web/dist/. Serve with python3 tools/serve_web.py."
