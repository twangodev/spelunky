import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    {
      name: 'spelunky-full-page-reload',
      enforce: 'pre',
      handleHotUpdate({ server }) {
        // The Rust event loop owns its canvas for the lifetime of the page.
        server.ws.send({ type: 'full-reload' });
        return [];
      },
    },
    sveltekit({
      adapter: adapter({ pages: 'dist', assets: 'dist' }),
      files: { assets: 'public' },
      paths: { relative: true },
      compilerOptions: { hmr: false },
    }),
  ],
});
