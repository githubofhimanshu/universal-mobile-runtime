import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      '/rpc': 'http://127.0.0.1:9090',
      '/health': 'http://127.0.0.1:9090',
    },
  },
});

