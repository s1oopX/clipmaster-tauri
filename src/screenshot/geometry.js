export function clamp(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

export function normalizeRect(rect) {
  const x = Math.min(rect.x, rect.x + rect.width);
  const y = Math.min(rect.y, rect.y + rect.height);
  return {
    x,
    y,
    width: Math.abs(rect.width),
    height: Math.abs(rect.height),
  };
}

export function clampRectToBounds(rect, widthLimit, heightLimit) {
  const normalized = normalizeRect(rect);
  const width = Math.min(normalized.width, widthLimit);
  const height = Math.min(normalized.height, heightLimit);
  return {
    x: clamp(normalized.x, 0, widthLimit - width),
    y: clamp(normalized.y, 0, heightLimit - height),
    width,
    height,
  };
}

export function clampPointToRect(point, rect) {
  return {
    x: clamp(point.x, rect.x, rect.x + rect.width),
    y: clamp(point.y, rect.y, rect.y + rect.height),
  };
}

export function isRectUsable(rect, minSize) {
  return rect && rect.width >= minSize && rect.height >= minSize;
}

export function getSelectionHandles(rect) {
  const cx = rect.x + rect.width / 2;
  const cy = rect.y + rect.height / 2;
  const right = rect.x + rect.width;
  const bottom = rect.y + rect.height;
  return [
    ['nw', rect.x, rect.y],
    ['n', cx, rect.y],
    ['ne', right, rect.y],
    ['e', right, cy],
    ['se', right, bottom],
    ['s', cx, bottom],
    ['sw', rect.x, bottom],
    ['w', rect.x, cy],
  ];
}

export function hitSelectionHandle(point, rect, handleSize) {
  if (!rect) return null;
  return getSelectionHandles(rect).find(([, x, y]) =>
    Math.abs(point.x - x) <= handleSize && Math.abs(point.y - y) <= handleSize
  )?.[0] || null;
}

export function pointInRect(point, rect) {
  return Boolean(
    rect &&
      point.x >= rect.x &&
      point.x <= rect.x + rect.width &&
      point.y >= rect.y &&
      point.y <= rect.y + rect.height
  );
}

export function resizeSelectionFromHandle(handle, startRect, point) {
  const left = startRect.x;
  const top = startRect.y;
  const right = startRect.x + startRect.width;
  const bottom = startRect.y + startRect.height;
  const next = { x: left, y: top, width: startRect.width, height: startRect.height };

  if (handle.includes('w')) {
    next.x = point.x;
    next.width = right - point.x;
  }
  if (handle.includes('e')) {
    next.width = point.x - left;
  }
  if (handle.includes('n')) {
    next.y = point.y;
    next.height = bottom - point.y;
  }
  if (handle.includes('s')) {
    next.height = point.y - top;
  }

  return normalizeRect(next);
}

export function moveRectWithinBounds(startRect, deltaX, deltaY, widthLimit, heightLimit) {
  return {
    ...startRect,
    x: clamp(startRect.x + deltaX, 0, widthLimit - startRect.width),
    y: clamp(startRect.y + deltaY, 0, heightLimit - startRect.height),
  };
}

export function nudgeRectWithinBounds(rect, deltaX, deltaY, resize, widthLimit, heightLimit) {
  if (!rect) return rect;

  if (resize) {
    return clampRectToBounds(
      {
        ...rect,
        width: Math.max(1, rect.width + deltaX),
        height: Math.max(1, rect.height + deltaY),
      },
      widthLimit,
      heightLimit
    );
  }

  return moveRectWithinBounds(rect, deltaX, deltaY, widthLimit, heightLimit);
}

export function pointInExpandedRect(point, rect, padding) {
  return (
    point.x >= rect.x - padding &&
    point.x <= rect.x + rect.width + padding &&
    point.y >= rect.y - padding &&
    point.y <= rect.y + rect.height + padding
  );
}

export function distanceToSegment(point, start, end) {
  const deltaX = end.x - start.x;
  const deltaY = end.y - start.y;
  const lengthSquared = deltaX * deltaX + deltaY * deltaY;
  if (lengthSquared === 0) {
    return Math.hypot(point.x - start.x, point.y - start.y);
  }

  const t = clamp(
    ((point.x - start.x) * deltaX + (point.y - start.y) * deltaY) / lengthSquared,
    0,
    1
  );
  return Math.hypot(point.x - (start.x + t * deltaX), point.y - (start.y + t * deltaY));
}
