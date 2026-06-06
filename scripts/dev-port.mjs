import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const DEFAULT_DEV_SERVER_PORT = 5174;
export const MIN_DEV_SERVER_PORT = 1;
export const MAX_DEV_SERVER_PORT = 65535;

const scriptDir = dirname(fileURLToPath(import.meta.url));
export const repoRoot = resolve(scriptDir, '..');
export const DEV_PORT_CONFIG_PATH = resolve(repoRoot, '.clipmaster-dev.json');

export function isValidDevServerPort(port) {
  return Number.isInteger(port)
    && port >= MIN_DEV_SERVER_PORT
    && port <= MAX_DEV_SERVER_PORT;
}

export function normalizeDevServerPort(value) {
  const port = Number(value);
  return isValidDevServerPort(port) ? port : DEFAULT_DEV_SERVER_PORT;
}

export function readDevServerPort() {
  const envPort = normalizeDevServerPort(process.env.CLIPMASTER_DEV_PORT);
  if (String(envPort) === String(process.env.CLIPMASTER_DEV_PORT).trim()) {
    return envPort;
  }

  if (!existsSync(DEV_PORT_CONFIG_PATH)) {
    return DEFAULT_DEV_SERVER_PORT;
  }

  try {
    const config = JSON.parse(readFileSync(DEV_PORT_CONFIG_PATH, 'utf8'));
    return normalizeDevServerPort(config.dev_server_port);
  } catch (error) {
    console.warn(`无法读取开发端口配置，使用默认端口 ${DEFAULT_DEV_SERVER_PORT}:`, error);
    return DEFAULT_DEV_SERVER_PORT;
  }
}

export function devServerUrl(port = readDevServerPort()) {
  return `http://127.0.0.1:${port}`;
}
