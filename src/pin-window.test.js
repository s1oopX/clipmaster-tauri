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
    expect(pinHtml).toContain('border-radius: 12px');
    expect(pinHtml).toContain('overflow: hidden');
    expect(pinHtml).toContain('runWindowAction');
  });

  it('zooms only the image content without resizing the pinned window', () => {
    expect(pinHtml).toContain('transform: scale(var(--image-scale, 1))');
    expect(pinHtml).toContain('function syncImageScale()');
    expect(pinHtml).toContain("image.style.setProperty('--image-scale'");
    expect(pinHtml).toContain('imageScale = Math.max(0.25, Math.min(imageScale * delta, 5))');
    expect(pinHtml).not.toContain('PhysicalSize');
    expect(pinHtml).not.toContain('currentWin.innerSize()');
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
