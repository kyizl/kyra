import { resolve } from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vite';

export default defineConfig({
  root: resolve('src'),
  publicDir: resolve('public'),
  plugins: [tailwindcss(), vue()],
  resolve: {
    alias: {
      '@renderer': resolve('src'),
    },
  },
  build: {
    outDir: resolve('out'),
    emptyOutDir: true,
  },
  server: {
    host: '127.0.0.1',
    port: 5173,
  },
});
