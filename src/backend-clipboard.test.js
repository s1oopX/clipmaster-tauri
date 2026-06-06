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

  it('marks clipboard hash and sequence only after a successful save', () => {
    const skipIndex = clipboardSource.indexOf('Self::should_skip_sequence(&last_sequence, clipboard_sequence)');
    const readIndex = clipboardSource.indexOf('Self::get_clipboard_content(&mut clipboard)');
    const saveIndex = clipboardSource.indexOf('Self::save_clipboard_item(&app_handle, content, hash.clone()).await');
    const okIndex = clipboardSource.indexOf('Ok(()) =>');
    const markIndex = clipboardSource.indexOf('Self::mark_clipboard_item_saved(');
    const errIndex = clipboardSource.indexOf('Err(e) =>');

    expect(skipIndex).toBeGreaterThan(-1);
    expect(readIndex).toBeGreaterThan(skipIndex);
    expect(saveIndex).toBeGreaterThan(readIndex);
    expect(okIndex).toBeGreaterThan(saveIndex);
    expect(markIndex).toBeGreaterThan(okIndex);
    expect(errIndex).toBeGreaterThan(markIndex);
    expect(clipboardSource).toContain('fn should_skip_sequence');
    expect(clipboardSource).toContain('fn mark_clipboard_item_saved');
    expect(clipboardSource).not.toContain('*last = Some(sequence);');
  });
});
