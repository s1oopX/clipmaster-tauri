import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const tauriConfig = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));

describe('Tauri security configuration', () => {
  it('uses CSP and limits asset protocol to ClipMaster image data', () => {
    const security = tauriConfig.app.security;

    expect(security.csp).toContain("default-src 'self'");
    expect(security.csp).toContain('connect-src');
    expect(security.csp).toContain('ipc:');
    expect(security.csp).toContain('asset:');
    expect(security.csp).toContain("script-src 'self'");
    expect(security.csp).toContain("style-src 'self'");
    expect(security.csp).toContain("object-src 'none'");
    expect(security.csp).not.toContain("'unsafe-inline'");
    expect(security.assetProtocol.scope.allow).toEqual([
      '$APPDATA/com.clipmaster.desktop/images/**',
      '$APPDATA/com.clipmaster.desktop/screenshot-cache/**',
    ]);
    expect(security.assetProtocol.scope.allow).not.toContain('$APPDATA/**');
  });
});
