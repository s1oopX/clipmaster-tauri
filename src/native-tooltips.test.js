import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const appSource = readFileSync('src/App.svelte', 'utf8');

describe('Native tooltips', () => {
  it('does not use title attributes for app controls', () => {
    expect(appSource).not.toMatch(/\stitle=/);
  });
});
