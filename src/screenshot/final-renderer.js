import { drawAnnotation } from './renderer.js';

export function renderFinalDataUrl({
  documentRef,
  frozenImage,
  selection,
  annotations,
  scaleX,
  scaleY,
}) {
  const sourceX = Math.round(selection.x * scaleX);
  const sourceY = Math.round(selection.y * scaleY);
  const sourceWidth = Math.max(1, Math.round(selection.width * scaleX));
  const sourceHeight = Math.max(1, Math.round(selection.height * scaleY));
  const output = documentRef.createElement('canvas');
  output.width = sourceWidth;
  output.height = sourceHeight;
  const outputCtx = output.getContext('2d');

  outputCtx.drawImage(
    frozenImage,
    sourceX,
    sourceY,
    sourceWidth,
    sourceHeight,
    0,
    0,
    sourceWidth,
    sourceHeight
  );

  outputCtx.save();
  outputCtx.beginPath();
  outputCtx.rect(0, 0, sourceWidth, sourceHeight);
  outputCtx.clip();
  const transform = {
    offsetX: selection.x,
    offsetY: selection.y,
    scaleX,
    scaleY,
    avgScale: (scaleX + scaleY) / 2,
  };
  annotations.forEach((annotation) => drawAnnotation(outputCtx, annotation, transform));
  outputCtx.restore();

  return output.toDataURL('image/png');
}
