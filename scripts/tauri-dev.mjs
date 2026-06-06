import { spawn } from 'node:child_process';
import { resolve } from 'node:path';
import { devServerUrl, readDevServerPort, repoRoot } from './dev-port.mjs';

const port = readDevServerPort();
const tauriCliPath = resolve(repoRoot, 'node_modules/@tauri-apps/cli/tauri.js');
const tauriConfig = {
  build: {
    beforeDevCommand: 'node scripts/vite-dev.mjs',
    devUrl: devServerUrl(port),
    additionalWatchFolders: ['../.clipmaster-dev.json'],
  },
};

const child = spawn(
  process.execPath,
  [tauriCliPath, 'dev', '--config', JSON.stringify(tauriConfig), ...process.argv.slice(2)],
  {
    cwd: repoRoot,
    env: {
      ...process.env,
      CLIPMASTER_DEV_PORT: String(port),
    },
    stdio: 'inherit',
  }
);

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 0);
});
