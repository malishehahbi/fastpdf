import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';

const root = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(root, '../..');

const corePackage = path.join(repoRoot, 'packages/core');

export default defineConfig({
  plugins: [wasm()],
  resolve: {
    // Always use the workspace build (file: deps in node_modules go stale after wasm-pack).
    alias: {
      '@fastpdf/core': path.join(corePackage, 'dist/index.js'),
    },
  },
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
