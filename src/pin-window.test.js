import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const pinHtml = readFileSync('pin.html', 'utf8');
const pinStyles = readFileSync('src/styles/pin-window.css', 'utf8');
const pinScript = readFileSync('src/pin-window.js', 'utf8');
const pinSource = [pinHtml, pinStyles, pinScript].join('\n');
const pinCapability = JSON.parse(readFileSync('src-tauri/capabilities/pin.json', 'utf8'));

describe('Pinned image window', () => {
  it('loads pinned-window behavior from external files for strict CSP', () => {
    expect(pinHtml).toContain('<link rel="stylesheet" href="/src/styles/pin-window.css" />');
    expect(pinHtml).toContain('<script type="module" src="/src/pin-window.js"></script>');
    expect(pinHtml).not.toContain('<style>');
    expect(pinHtml).not.toContain('<script type="module">\n');
  });

  it('keeps the pinned image movable, resizable, and softly rounded', () => {
    expect(pinSource).toContain('startDragging()');
    expect(pinSource).toContain('startResizeDragging(handle.dataset.direction)');
    expect(pinSource.match(/class="resize-handle"/g)).toHaveLength(4);
    expect(pinSource).toContain('data-direction="NorthWest"');
    expect(pinSource).toContain('data-direction="NorthEast"');
    expect(pinSource).toContain('data-direction="SouthWest"');
    expect(pinSource).toContain('data-direction="SouthEast"');
    expect(pinSource).toContain('new LogicalSize(width, height)');
    expect(pinSource).toContain('border-radius: 12px');
    expect(pinSource).toContain('overflow: hidden');
    expect(pinSource).toContain('runWindowAction');
    expect(pinSource).toContain('aria-label="关闭贴图"');
    expect(pinSource).not.toContain('title="关闭"');
  });

  it('keeps the pinned window fitted to the image while zooming', () => {
    expect(pinSource).toContain('width: 100%');
    expect(pinSource).toContain('height: 100%');
    expect(pinSource).toContain('object-fit: fill');
    expect(pinSource).toContain('const MAX_PIN_WIDTH = 720');
    expect(pinSource).toContain('const MAX_PIN_HEIGHT = 520');
    expect(pinSource).toContain('function fitImageWindowSize');
    expect(pinSource).toContain('async function resizePinnedWindow(delta)');
    expect(pinSource).toContain('window.innerWidth * delta');
    expect(pinSource).toContain('window.innerHeight * delta');
    expect(pinSource).toContain('new LogicalSize(nextWidth, nextHeight)');
    expect(pinSource).not.toContain('transform: scale');
    expect(pinSource).not.toContain('imageScale');
    expect(pinSource).not.toContain('--image-scale');
    expect(pinSource).not.toContain('PhysicalSize');
    expect(pinSource).not.toContain('currentWin.innerSize()');
    expect(pinSource).toContain("'同步缩放贴图窗口'");
  });

  it('does not include ctrl-left image panning', () => {
    expect(pinSource).not.toContain('imageOffsetX');
    expect(pinSource).not.toContain('imageOffsetY');
    expect(pinSource).not.toContain('function startImagePan');
    expect(pinSource).not.toContain('setPointerCapture');
    expect(pinSource).not.toContain("'--image-offset-x'");
    expect(pinSource).not.toContain("'--image-offset-y'");
    expect(pinSource).not.toContain('startImagePan(event)');
    expect(pinSource).toContain("currentWin.startDragging(), '移动贴图'");
  });

  it('grants the window APIs required by the pinned image interactions', () => {
    expect(pinCapability.windows).toEqual(['pin-*']);
    expect(pinCapability.permissions).toEqual(
      expect.arrayContaining([
        'core:window:allow-close',
        'core:window:allow-set-size',
        'core:window:allow-start-dragging',
        'core:window:allow-start-resize-dragging',
      ])
    );
    expect(pinCapability.permissions).not.toContain('core:event:allow-listen');
    expect(pinCapability.permissions).not.toContain('core:window:allow-destroy');
  });
});
