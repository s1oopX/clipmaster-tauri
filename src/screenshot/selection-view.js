import { HANDLE_SIZE } from './constants.js';
import { clamp } from './geometry.js';
import { drawAnnotation } from './renderer.js';

export function drawScreenshotCanvas({
  ctx,
  canvas,
  frozenImage,
  imageReady,
  selection,
  activeAnnotation,
  annotations,
  getHandles,
  isUsableSelection,
  pixelScaleX,
  pixelScaleY,
  sizeInfo,
}) {
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  if (imageReady) {
    ctx.drawImage(frozenImage, 0, 0, canvas.width, canvas.height);
  } else {
    ctx.fillStyle = '#05070c';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
  }

  if (!selection) {
    ctx.fillStyle = 'rgba(2, 6, 23, 0.28)';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    return;
  }

  drawOutsideOverlay(ctx, canvas, selection);

  ctx.save();
  ctx.beginPath();
  ctx.rect(selection.x, selection.y, selection.width, selection.height);
  ctx.clip();
  annotations.forEach((annotation) => drawAnnotation(ctx, annotation));
  if (activeAnnotation) {
    drawAnnotation(ctx, activeAnnotation);
  }
  ctx.restore();

  drawSelectionFrame(ctx, selection, getHandles);
  drawSizeInfo({
    canvas,
    sizeInfo,
    selection,
    isUsableSelection,
    pixelScaleX,
    pixelScaleY,
  });
}

export function positionToolbar({ toolbar, canvas, selection, isUsableSelection }) {
  if (!isUsableSelection()) {
    toolbar.style.display = 'none';
    return;
  }

  toolbar.style.display = 'flex';
  const maxLeft = Math.max(8, canvas.width - toolbar.offsetWidth - 8);
  const left = clamp(selection.x + selection.width - toolbar.offsetWidth, 8, maxLeft);
  let top = selection.y + selection.height + 10;

  if (top + toolbar.offsetHeight > canvas.height) {
    top = selection.y - toolbar.offsetHeight - 10;
  }
  toolbar.style.left = `${left}px`;
  toolbar.style.top = `${clamp(top, 8, Math.max(8, canvas.height - toolbar.offsetHeight - 8))}px`;
}

function drawOutsideOverlay(ctx, canvas, rect) {
  ctx.fillStyle = 'rgba(2, 6, 23, 0.48)';
  ctx.fillRect(0, 0, canvas.width, rect.y);
  ctx.fillRect(0, rect.y + rect.height, canvas.width, canvas.height - rect.y - rect.height);
  ctx.fillRect(0, rect.y, rect.x, rect.height);
  ctx.fillRect(rect.x + rect.width, rect.y, canvas.width - rect.x - rect.width, rect.height);
}

function drawSelectionFrame(ctx, rect, getHandles) {
  ctx.save();
  ctx.strokeStyle = '#38bdf8';
  ctx.lineWidth = 2;
  ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.width, rect.height);

  ctx.strokeStyle = 'rgba(255, 255, 255, 0.9)';
  ctx.lineWidth = 1;
  ctx.strokeRect(rect.x + 3.5, rect.y + 3.5, Math.max(0, rect.width - 6), Math.max(0, rect.height - 6));

  for (const [, x, y] of getHandles(rect)) {
    ctx.fillStyle = '#38bdf8';
    ctx.fillRect(x - HANDLE_SIZE / 2, y - HANDLE_SIZE / 2, HANDLE_SIZE, HANDLE_SIZE);
    ctx.strokeStyle = 'rgba(15, 23, 42, 0.88)';
    ctx.strokeRect(x - HANDLE_SIZE / 2 + 0.5, y - HANDLE_SIZE / 2 + 0.5, HANDLE_SIZE - 1, HANDLE_SIZE - 1);
  }
  ctx.restore();
}

function drawSizeInfo({
  canvas,
  sizeInfo,
  selection,
  isUsableSelection,
  pixelScaleX,
  pixelScaleY,
}) {
  if (!isUsableSelection()) return;
  const logicalWidth = Math.round(selection.width);
  const logicalHeight = Math.round(selection.height);
  const pixelWidth = Math.round(selection.width * pixelScaleX());
  const pixelHeight = Math.round(selection.height * pixelScaleY());
  sizeInfo.textContent = `${logicalWidth} × ${logicalHeight} / ${pixelWidth} × ${pixelHeight}`;
  sizeInfo.style.display = 'block';
  sizeInfo.style.left = `${Math.min(selection.x + selection.width + 10, canvas.width - 148)}px`;
  sizeInfo.style.top = `${Math.max(8, selection.y)}px`;
}
