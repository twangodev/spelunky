import { copyFileSync, mkdirSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { normalizePath, type Plugin, type ResolvedConfig } from 'vite';

const root = normalizePath(fileURLToPath(new URL('../../', import.meta.url))).replace(/\/$/, '');
const pkg = `${root}/target/web-pkg`;
const virtualId = 'virtual:spelunky';
const resolvedId = `\0${virtualId}`;

/** Build the Rust app; Vite bundles the glue JS and hashes the imported Wasm. */
export function spelunky(): Plugin {
  let config: ResolvedConfig;
  let ready: Promise<void> | undefined;

  function compile(): Promise<void> {
    const mode = config.command === 'build' ? '--release' : '--dev';
    config.logger.info(`[spelunky] Building Wasm (${mode.slice(2)})`);
    return new Promise((resolve, reject) => {
      const child = spawn('wasm-pack', [
        'build', 'crates/app', '--target', 'web', '--no-pack', '--out-dir', pkg,
        '--out-name', 'spelunky', mode, '--', '--locked',
      ], { cwd: root, stdio: 'inherit' });
      child.once('error', reject);
      child.once('exit', (code, signal) => {
        if (code === 0) resolve();
        else reject(new Error(`wasm-pack failed (${signal ?? code}); install wasm-pack and the wasm32-unknown-unknown Rust target`));
      });
    });
  }

  return {
    name: 'spelunky',
    enforce: 'pre',
    config() {
      // SvelteKit must discover these public files before its prerender crawl.
      const notices = `${root}/web/public/assets`;
      mkdirSync(notices, { recursive: true });
      for (const name of ['LICENSE', 'ORIGINAL_README.txt']) {
        copyFileSync(`${root}/assets/${name}`, `${notices}/${name}`);
      }
    },
    configResolved(resolved) { config = resolved; },
    async buildStart() {
      // SvelteKit builds the server first, then reuses its Wasm for the client.
      if (config.command === 'serve' || config.build.ssr) await (ready ??= compile());
    },
    resolveId(id) { if (id === virtualId) return resolvedId; },
    async load(id) {
      if (id !== resolvedId) return;
      await ready;
      return `
        import init, { start } from ${JSON.stringify(`${pkg}/spelunky.js`)};
        import wasmUrl from ${JSON.stringify(`${pkg}/spelunky_bg.wasm?url`)};
        export { start };
        export default () => init({ module_or_path: wasmUrl });
      `;
    },
    configureServer(server) {
      server.watcher.add([`${root}/crates`, `${root}/assets`, `${root}/Cargo.toml`, `${root}/Cargo.lock`]);
    },
    async handleHotUpdate({ file, server }) {
      const path = normalizePath(file);
      // Generated modules are invalidated together after compilation, never hot-swapped.
      if (path.startsWith(`${pkg}/`)) return [];
      if (path.startsWith(`${root}/crates/`) || path.startsWith(`${root}/assets/`) ||
          path === `${root}/Cargo.toml` || path === `${root}/Cargo.lock`) {
        // Serialize edits so a change arriving during compilation is not missed.
        ready = (ready ?? Promise.resolve()).catch(() => {}).then(compile);
        await ready;
        server.moduleGraph.invalidateAll();
      }
      // The Rust event loop owns its canvas for the lifetime of the page.
      server.ws.send({ type: 'full-reload' });
      return [];
    },
  };
}
