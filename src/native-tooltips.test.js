import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const appSource = readFileSync('src/App.svelte', 'utf8');
const pinSource = [
  readFileSync('pin.html', 'utf8'),
  readFileSync('src/styles/pin-window.css', 'utf8'),
  readFileSync('src/pin-window.js', 'utf8'),
].join('\n');
const screenshotHtml = [
  readFileSync('screenshot.html', 'utf8'),
  readFileSync('src/screenshot/screenshot.css', 'utf8'),
  readFileSync('src/screenshot/annotation-history.js', 'utf8'),
  readFileSync('src/screenshot/annotation-utils.js', 'utf8'),
  readFileSync('src/screenshot/capture-actions.js', 'utf8'),
  readFileSync('src/screenshot/constants.js', 'utf8'),
  readFileSync('src/screenshot/cursor.js', 'utf8'),
  readFileSync('src/screenshot/events.js', 'utf8'),
  readFileSync('src/screenshot/final-renderer.js', 'utf8'),
  readFileSync('src/screenshot/geometry.js', 'utf8'),
  readFileSync('src/screenshot/hit-testing.js', 'utf8'),
  readFileSync('src/screenshot/renderer.js', 'utf8'),
  readFileSync('src/screenshot/selection-view.js', 'utf8'),
  readFileSync('src/screenshot/text-editor.js', 'utf8'),
  readFileSync('src/screenshot/window-lifecycle.js', 'utf8'),
  readFileSync('src/screenshot/screenshot.js', 'utf8'),
].join('\n');

describe('Native tooltips', () => {
  it('does not use title attributes for app controls or utility windows', () => {
    for (const source of [appSource, pinSource, screenshotHtml]) {
      expect(source).not.toMatch(/\stitle=/);
    }
  });
});
