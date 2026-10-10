import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

export default defineConfig({
  root: 'client',
  base: './',
  publicDir: '../public',
  build: { outDir: '../dist', emptyOutDir: true },
  // Winit owns the canvas and event loop; reload the page instead of remounting it.
  plugins: [svelte({ compilerOptions: { hmr: false } })],
});
