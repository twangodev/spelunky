#!/usr/bin/env python3
"""Serve the generated web bundle locally without exposing other repo files."""

import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class BundleHandler(SimpleHTTPRequestHandler):
    extensions_map = {
        **SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".json": "application/json",
        ".ogg": "audio/ogg",
        ".wav": "audio/wav",
    }

    def send_head(self):
        # SimpleHTTPRequestHandler follows symlinks by default. Reject paths
        # outside this bundle even if a symlink was created under web/dist/.
        resolved = Path(self.translate_path(self.path)).resolve()
        if not resolved.is_relative_to(Path(self.directory).resolve()):
            self.send_error(403, "Path is outside the web bundle")
            return None
        if resolved.is_dir() and not (resolved / "index.html").is_file():
            self.send_error(403, "Directory listing is disabled")
            return None
        return super().send_head()

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        super().end_headers()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bind", default="127.0.0.1", help="Address to bind (default: localhost only)")
    parser.add_argument("--port", type=int, default=8000)
    args = parser.parse_args()
    bundle = Path(__file__).resolve().parent.parent / "web" / "dist"
    if not (bundle / "pkg" / "spelunky.js").is_file():
        parser.error("Missing web/dist/pkg/spelunky.js; run ./tools/build_web.sh first")
    handler = partial(BundleHandler, directory=str(bundle))
    with ThreadingHTTPServer((args.bind, args.port), handler) as server:
        print(f"Spelunky: http://{args.bind}:{server.server_port}/", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
