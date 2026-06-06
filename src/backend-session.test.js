import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const commandsSource = readFileSync('src-tauri/src/commands.rs', 'utf8');

describe('Backend session commands', () => {
  it('does not allow clearing the current active session', () => {
    const clearSessionStart = commandsSource.indexOf('pub async fn clear_session');
    const searchStart = commandsSource.indexOf('pub async fn search_items');
    const clearSessionSource = commandsSource.slice(clearSessionStart, searchStart);

    const activeGuardIndex = clearSessionSource.indexOf('get_current_session_id()');
    const loadItemsIndex = clearSessionSource.indexOf('get_items_by_session');
    const deleteSessionIndex = clearSessionSource.indexOf('db.clear_session');

    expect(clearSessionStart).toBeGreaterThan(-1);
    expect(searchStart).toBeGreaterThan(clearSessionStart);
    expect(clearSessionSource).toContain('session_mgr: State<');
    expect(clearSessionSource).toContain('不能清空当前活动会话');
    expect(activeGuardIndex).toBeGreaterThan(-1);
    expect(loadItemsIndex).toBeGreaterThan(activeGuardIndex);
    expect(deleteSessionIndex).toBeGreaterThan(activeGuardIndex);
  });
});
