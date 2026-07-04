import {
  ERASER_HIT_RADIUS,
  STEP_MARKER_RADIUS,
  TEXT_FONT_SIZE,
} from './constants.js';
import {
  distanceToSegment,
  normalizeRect,
  pointInExpandedRect,
} from './geometry.js';

export function findAnnotationIndexAtPoint(annotations, point, ctx) {
  for (let index = annotations.length - 1; index >= 0; index -= 1) {
    if (hitTestAnnotation(annotations[index], point, ctx)) {
      return index;
    }
  }
  return -1;
}

export function hitTestAnnotation(annotation, point, ctx) {
  if (annotation.type === 'rect') {
    return hitTestRectOutline(annotation, point);
  }
  if (annotation.type === 'blur' || annotation.type === 'pixelate') {
    return pointInExpandedRect(point, normalizeRect(annotation), ERASER_HIT_RADIUS);
  }
  if (annotation.type === 'arrow') {
    return distanceToSegment(point, { x: annotation.x1, y: annotation.y1 }, { x: annotation.x2, y: annotation.y2 }) <= ERASER_HIT_RADIUS;
  }
  if (annotation.type === 'pen') {
    return annotation.points.some((current, index) => {
      if (index === 0) return false;
      return distanceToSegment(point, annotation.points[index - 1], current) <= ERASER_HIT_RADIUS;
    });
  }
  if (annotation.type === 'text') {
    return pointInExpandedRect(point, textAnnotationBounds(annotation, ctx), ERASER_HIT_RADIUS);
  }
  if (annotation.type === 'step') {
    return Math.hypot(point.x - annotation.x, point.y - annotation.y) <= STEP_MARKER_RADIUS + ERASER_HIT_RADIUS;
  }
  return false;
}

export function hitTestRectOutline(annotation, point) {
  const rect = normalizeRect(annotation);
  const outer = pointInExpandedRect(point, rect, ERASER_HIT_RADIUS);
  const inner = pointInExpandedRect(point, {
    x: rect.x + ERASER_HIT_RADIUS,
    y: rect.y + ERASER_HIT_RADIUS,
    width: Math.max(0, rect.width - ERASER_HIT_RADIUS * 2),
    height: Math.max(0, rect.height - ERASER_HIT_RADIUS * 2),
  }, 0);
  return outer && !inner;
}

export function textAnnotationBounds(annotation, ctx) {
  ctx.save();
  ctx.font = `750 ${TEXT_FONT_SIZE}px Inter, ui-sans-serif, system-ui, sans-serif`;
  const width = Math.max(12, ctx.measureText(annotation.text).width);
  ctx.restore();
  return {
    x: annotation.x,
    y: annotation.y,
    width,
    height: TEXT_FONT_SIZE + 4,
  };
}
