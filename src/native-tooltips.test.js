import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const appSource = readFileSync('src/App.svelte', 'utf8');
const pinHtml = readFileSync('pin.html', 'utf8');
const screenshotHtml = readFileSync('screenshot.html', 'utf8');

describe('Native tooltips', () => {
  it('does not use title attributes for app controls or utility windows', () => {
    for (const source of [appSource, pinHtml, screenshotHtml]) {
      expect(source).not.toMatch(/\stitle=/);
    }
  });
});
