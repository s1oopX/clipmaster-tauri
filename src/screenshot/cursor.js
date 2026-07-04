export function updateSelectionCursor({
  activeTool,
  selection,
  dragState,
  event,
  pointFromEvent,
  hitHandle,
  pointInSelection,
  canvas,
}) {
  if (activeTool !== 'select' || !selection || dragState) return;
  const point = pointFromEvent(event);
  const handle = hitHandle(point);
  const cursors = {
    n: 'ns-resize',
    s: 'ns-resize',
    e: 'ew-resize',
    w: 'ew-resize',
    nw: 'nwse-resize',
    se: 'nwse-resize',
    ne: 'nesw-resize',
    sw: 'nesw-resize',
  };
  canvas.style.cursor = handle ? cursors[handle] : pointInSelection(point) ? 'move' : 'crosshair';
}
