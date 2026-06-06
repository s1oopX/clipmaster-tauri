import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const clipboardSource = readFileSync('src-tauri/src/clipboard.rs', 'utf8');

describe('Backend clipboard service', () => {
  it('retries clipboard initialization instead of panicking the listener task', () => {
    expect(clipboardSource).toContain('match Clipboard::new()');
    expect(clipboardSource).toContain('初始化剪贴板失败，将在 500ms 后重试');
    expect(clipboardSource).toContain('sleep(Duration::from_millis(500)).await');
    expect(clipboardSource).not.toContain('Clipboard::new().expect');
    expect(clipboardSource).not.toContain('Failed to initialize clipboard');
  });
});
