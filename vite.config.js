import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import { resolve } from 'node:path';
import {
  DEV_PORT_CONFIG_PATH,
  readDevServerPort,
} from './scripts/dev-port.mjs';

const devServerPort = readDevServerPort();

function clipmasterDevPortPlugin() {
  return {
    name: 'clipmaster-dev-port',
    configureServer(server) {
      server.watcher.add(DEV_PORT_CONFIG_PATH);
      const restartWhenDevPortChanges = async (changedPath) => {
        if (resolve(changedPath) === DEV_PORT_CONFIG_PATH) {
          await server.restart();
        }
      };
      server.watcher.on('add', restartWhenDevPortChanges);
      server.watcher.on('change', restartWhenDevPortChanges);
    },
  };
}

export default defineConfig({
  plugins: [svelte(), svelteTesting(), clipmasterDevPortPlugin()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: devServerPort,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'esnext',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    outDir: 'dist',
    rollupOptions: {
      input: {
        main: './index.html',
        screenshot: './screenshot.html',
        pin: './pin.html',
      },
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.js'],
    globals: true,
  },
});
