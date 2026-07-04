import {
  ANNOTATION_COLOR,
  PIXELATE_BLOCK_SIZE,
  STEP_MARKER_RADIUS,
  TEXT_FONT_SIZE,
} from './constants.js';
import { clamp, normalizeRect } from './geometry.js';

export function drawAnnotation(targetCtx, annotation, transform = identityTransform()) {
  const lineWidth = Math.max(2, 3 * transform.avgScale);
  targetCtx.save();
  targetCtx.strokeStyle = ANNOTATION_COLOR;
  targetCtx.fillStyle = ANNOTATION_COLOR;
  targetCtx.lineWidth = lineWidth;
  targetCtx.lineCap = 'round';
  targetCtx.lineJoin = 'round';

  if (annotation.type === 'blur' || annotation.type === 'pixelate') {
    drawPrivacyMask(targetCtx, annotation, transform);
  } else if (annotation.type === 'rect') {
    const rect = normalizeRect(annotation);
    const x = tx(rect.x, transform);
    const y = ty(rect.y, transform);
    targetCtx.strokeRect(
      x,
      y,
      rect.width * transform.scaleX,
      rect.height * transform.scaleY
    );
  } else if (annotation.type === 'arrow') {
    drawArrow(
      targetCtx,
      tx(annotation.x1, transform),
      ty(annotation.y1, transform),
      tx(annotation.x2, transform),
      ty(annotation.y2, transform),
      14 * transform.avgScale
    );
  } else if (annotation.type === 'pen' && annotation.points.length > 1) {
    targetCtx.beginPath();
    targetCtx.moveTo(tx(annotation.points[0].x, transform), ty(annotation.points[0].y, transform));
    for (const point of annotation.points.slice(1)) {
      targetCtx.lineTo(tx(point.x, transform), ty(point.y, transform));
    }
    targetCtx.stroke();
  } else if (annotation.type === 'text') {
    drawTextLabel(targetCtx, annotation, transform);
  } else if (annotation.type === 'step') {
    drawStepMarker(targetCtx, annotation, transform);
  }

  targetCtx.restore();
}

export function drawTextLabel(targetCtx, annotation, transform) {
  const fontSize = Math.max(12, Math.round(TEXT_FONT_SIZE * transform.avgScale));
  const x = tx(annotation.x, transform);
  const y = ty(annotation.y, transform);

  targetCtx.font = `750 ${fontSize}px Inter, ui-sans-serif, system-ui, sans-serif`;
  targetCtx.textBaseline = 'top';
  targetCtx.lineJoin = 'round';
  targetCtx.strokeStyle = 'rgba(255, 255, 255, 0.92)';
  targetCtx.lineWidth = Math.max(3, Math.round(4 * transform.avgScale));
  targetCtx.strokeText(annotation.text, x, y);
  targetCtx.fillStyle = ANNOTATION_COLOR;
  targetCtx.fillText(annotation.text, x, y);
}

export function drawStepMarker(targetCtx, annotation, transform) {
  const x = tx(annotation.x, transform);
  const y = ty(annotation.y, transform);
  const radius = Math.max(9, STEP_MARKER_RADIUS * transform.avgScale);

  targetCtx.save();
  targetCtx.beginPath();
  targetCtx.arc(x, y, radius, 0, Math.PI * 2);
  targetCtx.fillStyle = ANNOTATION_COLOR;
  targetCtx.fill();
  targetCtx.lineWidth = Math.max(2, 2 * transform.avgScale);
  targetCtx.strokeStyle = 'rgba(255, 255, 255, 0.96)';
  targetCtx.stroke();

  targetCtx.fillStyle = '#ffffff';
  targetCtx.font = `800 ${Math.max(11, Math.round(14 * transform.avgScale))}px Inter, ui-sans-serif, system-ui, sans-serif`;
  targetCtx.textAlign = 'center';
  targetCtx.textBaseline = 'middle';
  targetCtx.fillText(String(annotation.value), x, y + 0.5 * transform.avgScale);
  targetCtx.restore();
}

export function drawPrivacyMask(targetCtx, annotation, transform) {
  const rect = normalizeRect(annotation);
  const x = tx(rect.x, transform);
  const y = ty(rect.y, transform);
  const width = rect.width * transform.scaleX;
  const height = rect.height * transform.scaleY;
  if (width < 1 || height < 1) return;

  targetCtx.save();
  targetCtx.beginPath();
  targetCtx.rect(x, y, width, height);
  targetCtx.clip();

  if (annotation.type === 'blur') {
    drawBlurredRegion(targetCtx, x, y, width, height, transform.avgScale);
  } else {
    drawPixelatedRegion(targetCtx, x, y, width, height, transform.avgScale);
  }

  targetCtx.restore();
}

export function drawBlurredRegion(targetCtx, x, y, width, height, scale) {
  const padding = Math.max(4, Math.round(8 * scale));
  const sourceX = Math.floor(clamp(x - padding, 0, targetCtx.canvas.width));
  const sourceY = Math.floor(clamp(y - padding, 0, targetCtx.canvas.height));
  const sourceWidth = Math.max(1, Math.min(targetCtx.canvas.width - sourceX, Math.ceil(width + padding * 2)));
  const sourceHeight = Math.max(1, Math.min(targetCtx.canvas.height - sourceY, Math.ceil(height + padding * 2)));
  const sample = document.createElement('canvas');
  sample.width = sourceWidth;
  sample.height = sourceHeight;
  const sampleCtx = sample.getContext('2d');

  sampleCtx.drawImage(
    targetCtx.canvas,
    sourceX,
    sourceY,
    sourceWidth,
    sourceHeight,
    0,
    0,
    sourceWidth,
    sourceHeight
  );

  targetCtx.save();
  targetCtx.filter = `blur(${Math.max(4, 8 * scale)}px)`;
  targetCtx.drawImage(sample, 0, 0, sourceWidth, sourceHeight, sourceX, sourceY, sourceWidth, sourceHeight);
  targetCtx.restore();
}

export function drawPixelatedRegion(targetCtx, x, y, width, height, scale) {
  const sourceX = Math.floor(clamp(x, 0, targetCtx.canvas.width));
  const sourceY = Math.floor(clamp(y, 0, targetCtx.canvas.height));
  const sourceWidth = Math.max(1, Math.min(targetCtx.canvas.width - sourceX, Math.ceil(width)));
  const sourceHeight = Math.max(1, Math.min(targetCtx.canvas.height - sourceY, Math.ceil(height)));
  const blockSize = Math.max(4, Math.round(PIXELATE_BLOCK_SIZE * scale));
  const smallWidth = Math.max(1, Math.ceil(sourceWidth / blockSize));
  const smallHeight = Math.max(1, Math.ceil(sourceHeight / blockSize));
  const sample = document.createElement('canvas');
  sample.width = smallWidth;
  sample.height = smallHeight;
  const sampleCtx = sample.getContext('2d');

  sampleCtx.imageSmoothingEnabled = true;
  sampleCtx.drawImage(
    targetCtx.canvas,
    sourceX,
    sourceY,
    sourceWidth,
    sourceHeight,
    0,
    0,
    smallWidth,
    smallHeight
  );

  targetCtx.save();
  targetCtx.imageSmoothingEnabled = false;
  targetCtx.drawImage(
    sample,
    0,
    0,
    smallWidth,
    smallHeight,
    sourceX,
    sourceY,
    sourceWidth,
    sourceHeight
  );
  targetCtx.restore();
}

export function drawArrow(targetCtx, x1, y1, x2, y2, headLength) {
  const angle = Math.atan2(y2 - y1, x2 - x1);
  targetCtx.beginPath();
  targetCtx.moveTo(x1, y1);
  targetCtx.lineTo(x2, y2);
  targetCtx.stroke();

  targetCtx.beginPath();
  targetCtx.moveTo(x2, y2);
  targetCtx.lineTo(
    x2 - headLength * Math.cos(angle - Math.PI / 6),
    y2 - headLength * Math.sin(angle - Math.PI / 6)
  );
  targetCtx.lineTo(
    x2 - headLength * Math.cos(angle + Math.PI / 6),
    y2 - headLength * Math.sin(angle + Math.PI / 6)
  );
  targetCtx.closePath();
  targetCtx.fill();
}

export function identityTransform() {
  return {
    offsetX: 0,
    offsetY: 0,
    scaleX: 1,
    scaleY: 1,
    avgScale: 1,
  };
}

function tx(value, transform) {
  return (value - transform.offsetX) * transform.scaleX;
}

function ty(value, transform) {
  return (value - transform.offsetY) * transform.scaleY;
}
