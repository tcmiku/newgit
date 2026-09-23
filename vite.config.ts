import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { sveltePhosphorOptimize } from 'phosphor-svelte/vite';

export default defineConfig({
  plugins: [svelte(), sveltePhosphorOptimize()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: ['es2022', 'safari15'], sourcemap: false },
});
