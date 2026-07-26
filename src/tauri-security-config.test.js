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
    // $APPDATA 在 Tauri v2 中已解析为「数据目录/应用标识符」，scope 里不能再拼一次
    // 标识符，否则 asset 协议对真实文件路径一律 403。
    expect(security.assetProtocol.scope.allow).toEqual([
      '$APPDATA/images/**',
      '$APPDATA/screenshot-cache/**',
    ]);
    expect(security.assetProtocol.scope.allow).not.toContain('$APPDATA/**');
  });
});
