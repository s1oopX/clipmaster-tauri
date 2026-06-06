import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const pinHtml = readFileSync('pin.html', 'utf8');
const capabilities = JSON.parse(
  readFileSync('src-tauri/capabilities/default.json', 'utf8')
);

describe('Pinned image window', () => {
  it('keeps the pinned image movable, resizable, and softly rounded', () => {
    expect(pinHtml).toContain('startDragging()');
    expect(pinHtml).toContain('startResizeDragging(handle.dataset.direction)');
    expect(pinHtml.match(/class="resize-handle"/g)).toHaveLength(4);
    expect(pinHtml).toContain('data-direction="NorthWest"');
    expect(pinHtml).toContain('data-direction="NorthEast"');
    expect(pinHtml).toContain('data-direction="SouthWest"');
    expect(pinHtml).toContain('data-direction="SouthEast"');
    expect(pinHtml).toContain('new LogicalSize(width, height)');
    expect(pinHtml).toContain('new PhysicalSize(');
    expect(pinHtml).toContain('border-radius: 12px');
    expect(pinHtml).toContain('overflow: hidden');
    expect(pinHtml).toContain('runWindowAction');
  });

  it('grants the window APIs required by the pinned image interactions', () => {
    expect(capabilities.windows).toContain('pin-*');
    expect(capabilities.permissions).toEqual(
      expect.arrayContaining([
        'core:window:allow-close',
        'core:window:allow-set-size',
        'core:window:allow-start-dragging',
        'core:window:allow-start-resize-dragging',
      ])
    );
  });
});
