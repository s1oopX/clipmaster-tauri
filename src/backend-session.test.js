import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const historyCommandsSource = readFileSync(
  'src-tauri/src/commands/history_commands.rs',
  'utf8'
);

describe('Backend session commands', () => {
  it('does not allow clearing the current active session', () => {
    const clearSessionStart = historyCommandsSource.indexOf('pub async fn clear_session');
    const searchStart = historyCommandsSource.indexOf('pub async fn search_items');
    const clearSessionSource = historyCommandsSource.slice(clearSessionStart, searchStart);

    const activeGuardIndex = clearSessionSource.indexOf('get_current_session_id()');
    const deleteSessionIndex = clearSessionSource.indexOf('db.clear_session');

    expect(clearSessionStart).toBeGreaterThan(-1);
    expect(searchStart).toBeGreaterThan(clearSessionStart);
    expect(clearSessionSource).toContain('session_mgr: State<');
    expect(clearSessionSource).toContain('不能清空当前活动会话');
    expect(activeGuardIndex).toBeGreaterThan(-1);
    expect(deleteSessionIndex).toBeGreaterThan(activeGuardIndex);
    expect(clearSessionSource).not.toContain('get_items_by_session(&session_id, i32::MAX, 0)');
  });
});
