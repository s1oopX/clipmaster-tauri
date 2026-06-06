import { spawn } from 'node:child_process';
import { resolve } from 'node:path';
import { readDevServerPort, repoRoot } from './dev-port.mjs';

const port = readDevServerPort();
const viteCliPath = resolve(repoRoot, 'node_modules/vite/bin/vite.js');
const child = spawn(
  process.execPath,
  [viteCliPath, '--host', '127.0.0.1', '--port', String(port), '--strictPort', ...process.argv.slice(2)],
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
