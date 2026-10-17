#!/usr/bin/env bun
import { realpath, stat } from 'node:fs/promises';
import { resolve, relative, isAbsolute, sep } from 'node:path';
import { parseArgs } from 'node:util';

const mimeTypes: Record<string, string> = {
  '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript',
  '.mjs': 'text/javascript', '.json': 'application/json',
  '.wasm': 'application/wasm', '.png': 'image/png', '.svg': 'image/svg+xml',
  '.ico': 'image/vnd.microsoft.icon', '.ogg': 'audio/ogg', '.wav': 'audio/wav',
  '.txt': 'text/plain', '.map': 'application/json',
};

function inside(root: string, path: string): boolean {
  const rel = relative(root, path);
  return rel !== '..' && !rel.startsWith(`..${sep}`) && !isAbsolute(rel);
}

/** Serve only the generated bundle, including when a bundle entry is a symlink. */
export async function bundleHandler(directory: string) {
  const root = await realpath(directory);
  return async (request: Request): Promise<Response> => {
    const headers = new Headers({
      'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff',
    });
    const error = (status: number, message: string) => {
      headers.set('Content-Type', 'text/plain; charset=utf-8');
      return new Response(request.method === 'HEAD' ? null : message, { status, headers });
    };
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      headers.set('Allow', 'GET, HEAD');
      return error(405, 'Method not allowed');
    }
    const url = new URL(request.url);
    let pathname: string;
    try { pathname = decodeURIComponent(url.pathname); }
    catch { return error(400, 'Invalid URL encoding'); }
    if (pathname.includes('\0')) return error(400, 'Invalid path');
    let filePath = resolve(root, `.${pathname}`);
    if (!inside(root, filePath)) return error(403, 'Path is outside the web bundle');
    try {
      filePath = await realpath(filePath);
      if (!inside(root, filePath)) return error(403, 'Path is outside the web bundle');
      if ((await stat(filePath)).isDirectory()) {
        // Resolve the index separately: an index symlink can also escape the root.
        try { filePath = await realpath(resolve(filePath, 'index.html')); }
        catch (caught) {
          if ((caught as NodeJS.ErrnoException).code === 'ENOENT') {
            return error(403, 'Directory listing is disabled');
          }
          throw caught;
        }
        if (!inside(root, filePath)) return error(403, 'Path is outside the web bundle');
        if (!url.pathname.endsWith('/')) {
          headers.set('Location', `${url.pathname}/${url.search}`);
          return new Response(null, { status: 301, headers });
        }
      }
      const info = await stat(filePath);
      if (!info.isFile()) return error(403, 'Not a regular file');
      const suffix = /\.[^.\/]+$/.exec(filePath)?.[0].toLowerCase() ?? '';
      headers.set('Content-Type', mimeTypes[suffix] ?? (filePath.endsWith(`${sep}LICENSE`) ? 'text/plain' : 'application/octet-stream'));
      headers.set('Content-Length', String(info.size));
      return new Response(request.method === 'HEAD' ? null : Bun.file(filePath), { headers });
    } catch (caught) {
      const code = (caught as NodeJS.ErrnoException).code;
      if (code === 'ENOENT' || code === 'ENOTDIR') return error(404, 'File not found');
      if (code === 'EACCES' || code === 'EPERM') return error(403, 'Permission denied');
      throw caught;
    }
  };
}

if (import.meta.main) {
  try {
    const { values } = parseArgs({ options: {
      bind: { type: 'string', default: '127.0.0.1' },
      port: { type: 'string', default: '8000' },
      help: { type: 'boolean', short: 'h' },
    } });
    if (values.help) {
      console.log('Usage: bun tools/serve_web.ts [--bind 127.0.0.1] [--port 8000]');
    } else {
      if (!/^\d+$/.test(values.port!) || Number(values.port) > 65535) throw new Error('Port must be between 0 and 65535');
      const directory = resolve(import.meta.dir, '../web/dist');
      if (!(await Bun.file(resolve(directory, 'pkg/module-path.json')).exists())) {
        throw new Error('Missing web/dist/pkg/module-path.json; run ./tools/build_web.sh first');
      }
      const server = Bun.serve({ hostname: values.bind, port: Number(values.port), fetch: await bundleHandler(directory) });
      console.log(`Spelunky: http://${values.bind}:${server.port}/`);
      for (const signal of ['SIGINT', 'SIGTERM'] as const) process.on(signal, () => { server.stop(true); process.exit(0); });
    }
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
