import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const appSource = readFileSync('src/App.svelte', 'utf8');

function cssBlock(selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = appSource.match(new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`));
  return match?.[1] || '';
}

describe('Toast layering', () => {
  it('keeps toast errors above modal overlays', () => {
    const toastStack = cssBlock('.toast-stack');
    const confirmDialog = cssBlock('.confirm-dialog');
    const contextMenu = cssBlock('.context-menu');

    expect(toastStack).toContain('position: fixed');
    expect(toastStack).toContain('z-index: 1300');
    expect(confirmDialog).toContain('z-index: 31');
    expect(contextMenu).toContain('z-index: 1200');
  });
});
