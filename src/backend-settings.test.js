import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const commandsSource = readFileSync('src-tauri/src/commands.rs', 'utf8');
const hotkeySource = readFileSync('src-tauri/src/hotkey.rs', 'utf8');

describe('Backend settings commands', () => {
  it('applies fallible runtime settings before persisting the new settings', () => {
    const normalizeIndex = commandsSource.indexOf(
      'let result = SettingsStore::normalize_candidate(settings)'
    );
    const hotkeyIndex = commandsSource.indexOf(
      'HotkeyManager::re_register_with_settings(&app, &result)'
    );
    const devPortIndex = commandsSource.indexOf(
      'write_project_dev_server_port(result.dev_server_port)'
    );
    const saveIndex = commandsSource.indexOf('store.save_normalized(result.clone())');

    expect(normalizeIndex).toBeGreaterThan(-1);
    expect(hotkeyIndex).toBeGreaterThan(normalizeIndex);
    expect(devPortIndex).toBeGreaterThan(hotkeyIndex);
    expect(saveIndex).toBeGreaterThan(devPortIndex);
    expect(commandsSource).not.toContain('let result = store.save(settings)');
    expect(commandsSource).toContain('rollback_settings_side_effects');
  });

  it('does not run automatic cleanup inside the settings save command', () => {
    const saveStart = commandsSource.indexOf('pub async fn save_settings');
    const previewStart = commandsSource.indexOf('pub async fn preview_custom_cleanup');
    const saveSettingsSource = commandsSource.slice(saveStart, previewStart);

    expect(saveStart).toBeGreaterThan(-1);
    expect(previewStart).toBeGreaterThan(saveStart);
    expect(saveSettingsSource).not.toContain('cleanup_by_settings');
    expect(saveSettingsSource).not.toContain('run_cleanup(');
    expect(saveSettingsSource).not.toContain('auto_cleanup_enabled');
  });

  it('handles global shortcut callbacks only on key press events', () => {
    expect(hotkeySource).toContain('ShortcutState::Pressed');
    expect(hotkeySource.match(/event\.state != ShortcutState::Pressed/g)).toHaveLength(2);
  });
});
