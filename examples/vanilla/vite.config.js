import { defineConfig } from 'vite';

export default defineConfig({
  optimizeDeps: { exclude: ['@fastpdf/core'] },
  server: { port: 5173 },
});
