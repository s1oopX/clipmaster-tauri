export function nextStepNumber(annotations) {
  return annotations
    .filter((annotation) => annotation.type === 'step')
    .reduce((maxValue, annotation) => Math.max(maxValue, Number(annotation.value) || 0), 0) + 1;
}

export function shouldKeepAnnotation(annotation, minRectSize) {
  if (annotation.type === 'pen') {
    return annotation.points.length > 1;
  }
  if (annotation.type === 'rect' || annotation.type === 'blur' || annotation.type === 'pixelate') {
    return Math.abs(annotation.width) >= minRectSize && Math.abs(annotation.height) >= minRectSize;
  }
  return Math.hypot(annotation.x2 - annotation.x1, annotation.y2 - annotation.y1) > 3;
}
