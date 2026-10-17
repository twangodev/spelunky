import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { spelunky } from './plugins/spelunky';

export default defineConfig({
  plugins: [
    spelunky(),
    sveltekit({
      adapter: adapter({ pages: 'dist', assets: 'dist' }),
      files: { assets: 'public' },
      paths: { relative: true },
      compilerOptions: { hmr: false },
    }),
  ],
});
