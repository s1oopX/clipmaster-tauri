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
    // 从 save 调用之后开始找，避免命中监督循环里的 Ok(()) => break
    const okIndex = clipboardSource.indexOf('Ok(()) =>', saveIndex);
    const markIndex = clipboardSource.indexOf('Self::mark_clipboard_item_saved(', okIndex);
    const errIndex = clipboardSource.indexOf('Err(e) =>', okIndex);

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

  it('skips self writes from history copy without saving them as newer records', () => {
    expect(clipboardSource).toContain('ClipboardWriteState');
    expect(clipboardSource).toContain('fn should_skip_self_write');
    expect(clipboardSource).toContain('state.consume_pending_hash(hash)');

    const skipIndex = clipboardSource.indexOf('if Self::should_skip_self_write(&app_handle, &hash)');
    const saveIndex = clipboardSource.indexOf('Self::save_clipboard_item(&app_handle, content, hash.clone()).await');

    expect(skipIndex).toBeGreaterThan(-1);
    expect(saveIndex).toBeGreaterThan(skipIndex);
  });
});
