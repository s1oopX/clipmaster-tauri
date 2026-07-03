import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const commandsSource = readFileSync('src-tauri/src/commands.rs', 'utf8');
const cleanupCommandsSource = readFileSync(
  'src-tauri/src/commands/cleanup_commands.rs',
  'utf8'
);

describe('Backend cleanup commands', () => {
  it('does not report record deletion as failed when post-delete file cleanup fails', () => {
    expect(cleanupCommandsSource).toContain('fn cleanup_item_files_best_effort');
    expect(cleanupCommandsSource).toContain('fn cleanup_file_target_best_effort');
    expect(commandsSource).toContain('cleanup_item_files_best_effort(&app, &item);');
    expect(cleanupCommandsSource).toContain('cleanup_item_files_best_effort(app, item);');
    expect(commandsSource).toContain('cleanup_file_target_best_effort(&app, target);');
    expect(commandsSource).not.toContain('cleanup_item_files(&app, &item)?');
    expect(cleanupCommandsSource).not.toContain('cleanup_item_files(&app, item)?');
    expect(cleanupCommandsSource).not.toContain('cleanup_item_files(app, item)?');
    expect(commandsSource).not.toContain('cleanup_file_target(&app, target)?');
  });
});
