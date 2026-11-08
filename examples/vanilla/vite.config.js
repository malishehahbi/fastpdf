import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';

const root = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(root, '../..');

export default defineConfig({
  plugins: [wasm()],
  optimizeDeps: {
    exclude: ['@fastpdf/core'],
  },
  server: {
    port: 5173,
    fs: {
      allow: [repoRoot],
    },
  },
  build: {
  /** @fastpdf/core pulls in native .wasm from packages/core/pkg */
    target: 'esnext',
  },
});
