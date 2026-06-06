import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const commandsSource = readFileSync('src-tauri/src/commands.rs', 'utf8');

describe('Backend settings commands', () => {
  it('applies fallible runtime settings before persisting the new settings', () => {
    const normalizeIndex = commandsSource.indexOf(
      'let result = SettingsStore::normalize_candidate(settings)'
    );
    const hotkeyIndex = commandsSource.indexOf(
      'HotkeyManager::re_register_with_hotkey(&app, &result.screenshot_hotkey)'
    );
    const saveIndex = commandsSource.indexOf('store.save_normalized(result.clone())');

    expect(normalizeIndex).toBeGreaterThan(-1);
    expect(hotkeyIndex).toBeGreaterThan(normalizeIndex);
    expect(saveIndex).toBeGreaterThan(hotkeyIndex);
    expect(commandsSource).not.toContain('let result = store.save(settings)');
    expect(commandsSource).toContain('rollback_settings_side_effects');
  });
});
