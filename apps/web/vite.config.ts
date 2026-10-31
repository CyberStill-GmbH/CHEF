import { fileURLToPath, URL } from 'node:url';

import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  plugins: [react()],
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': {
        target: process.env['CHEF_API_PROXY_TARGET'] ?? 'http://127.0.0.1:8787',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api(?=\/|$)/, ''),
      },
    },
  },
  test: {
    environment: 'jsdom',
    passWithNoTests: true,
    setupFiles: ['./src/test/setup.ts'],
  },
});
