import { convertFileSrc } from '@tauri-apps/api/core';
import { createAnnotationHistory } from './annotation-history.js';
import {
  nextStepNumber as getNextStepNumber,
  shouldKeepAnnotation,
} from './annotation-utils.js';
import { createCaptureActions } from './capture-actions.js';
import {
  HANDLE_SIZE,
  MIN_SELECTION_SIZE,
  PRIVACY_MASK_MIN_SIZE,
} from './constants.js';
import { updateSelectionCursor } from './cursor.js';
import {
  clamp,
  clampPointToRect,
  clampRectToBounds,
  getSelectionHandles,
  hitSelectionHandle,
  isRectUsable,
  moveRectWithinBounds,
  pointInRect,
  resizeSelectionFromHandle,
} from './geometry.js';
import { renderFinalDataUrl as renderFinalImageDataUrl } from './final-renderer.js';
import { findAnnotationIndexAtPoint as findAnnotationIndexInList } from './hit-testing.js';
import {
  drawScreenshotCanvas,
  positionToolbar as positionSelectionToolbar,
} from './selection-view.js';
import { createTextAnnotationEditor } from './text-editor.js';
import { bindScreenshotEvents } from './events.js';
import { readSnapshotParams } from './snapshot-params.js';
import { closeScreenshotWindow } from './window-lifecycle.js';

const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
const toolbar = document.getElementById('toolbar');
const errorBox = document.getElementById('error');
const sizeInfo = document.getElementById('sizeInfo');
const info = document.getElementById('info');
const confirmBtn = document.getElementById('confirm');
const cancelBtn = document.getElementById('cancel');
const pinBtn = document.getElementById('pinBtn');
const reselectBtn = document.getElementById('reselectBtn');
const undoBtn = document.getElementById('undoBtn');
const redoBtn = document.getElementById('redoBtn');
const textInput = document.getElementById('textInput');
const toolButtons = [...document.querySelectorAll('[data-tool]')];

const { snapshot, shouldRestoreMainWindow } = readSnapshotParams(window.location, window);

const frozenImage = new Image();
let imageReady = false;
let activeTool = 'select';
let isCapturing = false;
let snapshotCleaned = false;
let selection = null;
let dragState = null;
let activeAnnotation = null;
const annotations = [];
const annotationHistory = createAnnotationHistory(annotations, updateHistoryButtons);
const textEditor = createTextAnnotationEditor({
  canvas,
  textInput,
  commitAnnotation,
  afterCommit: () => {
    drawCanvas();
    positionToolbar();
  },
});
const captureActions = createCaptureActions({
  getIsCapturing: () => isCapturing,
  setIsCapturing: (value) => {
    isCapturing = value;
  },
  commitTextInput,
  isUsableSelection,
  showError,
  hideError,
  setToolbarDisabled,
  renderFinalDataUrl: () =>
    renderFinalImageDataUrl({ documentRef: document, frozenImage, selection, annotations, scaleX: pixelScaleX(), scaleY: pixelScaleY() }),
  getSnapshotPath: () => snapshot.path,
  markSnapshotCleaned: () => {
    snapshotCleaned = true;
  },
  closeWindow,
});

function pointFromEvent(event) {
  return {
    x: clamp(event.clientX, 0, canvas.width),
    y: clamp(event.clientY, 0, canvas.height),
  };
}

function clampSelection(rect) {
  return clampRectToBounds(rect, canvas.width, canvas.height);
}

function clampPointToSelection(point) {
  if (!selection) return point;
  return clampPointToRect(point, selection);
}

function isUsableSelection(rect = selection) {
  return isRectUsable(rect, MIN_SELECTION_SIZE);
}

function resizeCanvas() {
  canvas.width = Math.max(1, Math.round(window.innerWidth));
  canvas.height = Math.max(1, Math.round(window.innerHeight));
  if (selection) {
    selection = clampSelection(selection);
  }
  drawCanvas();
  positionToolbar();
  positionTextInput();
}

function showError(message) {
  errorBox.textContent = message;
  errorBox.style.display = 'block';
}

function hideError() {
  errorBox.textContent = '';
  errorBox.style.display = 'none';
}

function setActiveTool(tool) {
  if (tool !== 'text') {
    commitTextInput();
  }
  activeTool = tool;
  toolButtons.forEach((button) => {
    button.classList.toggle('active', button.dataset.tool === tool);
  });
  canvas.style.cursor = tool === 'select' ? 'default' : tool === 'eraser' ? 'cell' : 'crosshair';
}

function resetForReselect() {
  cancelTextInput();
  clearSelectionState();
  activeAnnotation = null;
  dragState = null;
  toolbar.style.display = 'none';
  sizeInfo.style.display = 'none';
  setActiveTool('select');
  drawCanvas();
}

function commitAnnotation(annotation) {
  annotationHistory.commit(annotation);
}

function removeAnnotation(index) {
  return annotationHistory.remove(index);
}

function undoAnnotation() {
  if (isCapturing || activeAnnotation || textEditor.pending || !annotationHistory.canUndo) return;
  annotationHistory.undo();
  drawCanvas();
  positionToolbar();
}

function redoAnnotation() {
  if (isCapturing || activeAnnotation || textEditor.pending || !annotationHistory.canRedo) return;
  annotationHistory.redo();
  drawCanvas();
  positionToolbar();
}

function updateHistoryButtons() {
  undoBtn.disabled = isCapturing || !annotationHistory.canUndo;
  redoBtn.disabled = isCapturing || !annotationHistory.canRedo;
}

function clearSelectionState() {
  selection = null;
  annotations.length = 0;
  annotationHistory.clear();
}

function getHandles(rect = selection) {
  return rect ? getSelectionHandles(rect) : [];
}

function hitHandle(point) {
  return hitSelectionHandle(point, selection, HANDLE_SIZE);
}

function pointInSelection(point) {
  return pointInRect(point, selection);
}

function selectionFromHandle(handle, startRect, point) {
  return clampSelection(resizeSelectionFromHandle(handle, startRect, point));
}

function moveSelection(startRect, deltaX, deltaY) {
  selection = moveRectWithinBounds(startRect, deltaX, deltaY, canvas.width, canvas.height);
}

function nudgeSelection(deltaX, deltaY, resize = false) {
  if (!selection || isCapturing) return;

  if (resize) {
    selection = clampSelection({
      ...selection,
      width: Math.max(MIN_SELECTION_SIZE, selection.width + deltaX),
      height: Math.max(MIN_SELECTION_SIZE, selection.height + deltaY),
    });
  } else {
    moveSelection(selection, deltaX, deltaY);
  }

  drawCanvas();
  positionToolbar();
}

function drawCanvas() {
  drawScreenshotCanvas({
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
  });
}

function pixelScaleX() {
  return snapshot.pixelWidth / Math.max(1, canvas.width);
}

function pixelScaleY() {
  return snapshot.pixelHeight / Math.max(1, canvas.height);
}

function beginTextAnnotation(point) {
  textEditor.begin(point);
}

function positionTextInput() {
  textEditor.position();
}

function commitTextInput() {
  textEditor.commit();
}

function cancelTextInput() {
  textEditor.cancel();
}

function eraseAnnotationAt(point) {
  const index = findAnnotationIndexAtPoint(point);
  if (index < 0) return false;
  const removed = removeAnnotation(index);
  if (removed) {
    drawCanvas();
    positionToolbar();
  }
  return removed;
}

function findAnnotationIndexAtPoint(point) {
  return findAnnotationIndexInList(annotations, point, ctx);
}

function positionToolbar() {
  positionSelectionToolbar({
    toolbar,
    canvas,
    selection,
    isUsableSelection,
  });
}

function updateCursor(event) {
  updateSelectionCursor({
    activeTool,
    selection,
    dragState,
    event,
    pointFromEvent,
    hitHandle,
    pointInSelection,
    canvas,
  });
}

function onMouseDown(event) {
  if (isCapturing || !imageReady) return;
  hideError();
  const point = pointFromEvent(event);

  if (activeTool === 'select') {
    commitTextInput();
    const handle = hitHandle(point);
    if (selection && handle) {
      dragState = {
        type: 'resize',
        handle,
        startPoint: point,
        startSelection: { ...selection },
      };
    } else if (selection && pointInSelection(point)) {
      dragState = {
        type: 'move',
        startPoint: point,
        startSelection: { ...selection },
      };
    } else {
      selection = { x: point.x, y: point.y, width: 0, height: 0 };
      annotations.length = 0;
      annotationHistory.clear();
      cancelTextInput();
      dragState = {
        type: 'create',
        startPoint: point,
      };
    }
    drawCanvas();
    return;
  }

  if (!selection || !pointInSelection(point)) return;
  const clippedPoint = clampPointToSelection(point);

  if (activeTool === 'rect') {
    activeAnnotation = {
      type: 'rect',
      x: clippedPoint.x,
      y: clippedPoint.y,
      width: 0,
      height: 0,
    };
    dragState = { type: 'annotation', startPoint: clippedPoint };
  } else if (activeTool === 'arrow') {
    activeAnnotation = {
      type: 'arrow',
      x1: clippedPoint.x,
      y1: clippedPoint.y,
      x2: clippedPoint.x,
      y2: clippedPoint.y,
    };
    dragState = { type: 'annotation', startPoint: clippedPoint };
  } else if (activeTool === 'pen') {
    activeAnnotation = {
      type: 'pen',
      points: [clippedPoint],
    };
    dragState = { type: 'annotation', startPoint: clippedPoint };
  } else if (activeTool === 'text') {
    beginTextAnnotation(clippedPoint);
  } else if (activeTool === 'step') {
    commitAnnotation({
      type: 'step',
      x: clippedPoint.x,
      y: clippedPoint.y,
      value: getNextStepNumber(annotations),
    });
    drawCanvas();
    positionToolbar();
  } else if (activeTool === 'blur' || activeTool === 'pixelate') {
    activeAnnotation = {
      type: activeTool,
      x: clippedPoint.x,
      y: clippedPoint.y,
      width: 0,
      height: 0,
    };
    dragState = { type: 'annotation', startPoint: clippedPoint };
  } else if (activeTool === 'eraser') {
    eraseAnnotationAt(clippedPoint);
    dragState = { type: 'erase' };
  }
}

function onMouseMove(event) {
  if (!dragState) {
    updateCursor(event);
    return;
  }

  const point = pointFromEvent(event);

  if (dragState.type === 'create') {
    selection = clampSelection({
      x: dragState.startPoint.x,
      y: dragState.startPoint.y,
      width: point.x - dragState.startPoint.x,
      height: point.y - dragState.startPoint.y,
    });
  } else if (dragState.type === 'move') {
    moveSelection(
      dragState.startSelection,
      point.x - dragState.startPoint.x,
      point.y - dragState.startPoint.y
    );
  } else if (dragState.type === 'resize') {
    selection = selectionFromHandle(
      dragState.handle,
      dragState.startSelection,
      point
    );
  } else if (dragState.type === 'annotation' && activeAnnotation) {
    const clippedPoint = clampPointToSelection(point);
    if (activeAnnotation.type === 'rect') {
      activeAnnotation.width = clippedPoint.x - dragState.startPoint.x;
      activeAnnotation.height = clippedPoint.y - dragState.startPoint.y;
    } else if (activeAnnotation.type === 'blur' || activeAnnotation.type === 'pixelate') {
      activeAnnotation.width = clippedPoint.x - dragState.startPoint.x;
      activeAnnotation.height = clippedPoint.y - dragState.startPoint.y;
    } else if (activeAnnotation.type === 'arrow') {
      activeAnnotation.x2 = clippedPoint.x;
      activeAnnotation.y2 = clippedPoint.y;
    } else if (activeAnnotation.type === 'pen') {
      activeAnnotation.points.push(clippedPoint);
    }
  } else if (dragState.type === 'erase' && selection && pointInSelection(point)) {
    eraseAnnotationAt(clampPointToSelection(point));
  }

  drawCanvas();
  positionToolbar();
}

function onMouseUp() {
  if (!dragState) return;

  if (dragState.type === 'create') {
    if (!isUsableSelection()) {
      selection = null;
      toolbar.style.display = 'none';
      sizeInfo.style.display = 'none';
    }
  } else if (dragState.type === 'annotation' && activeAnnotation) {
    if (shouldKeepAnnotation(activeAnnotation, PRIVACY_MASK_MIN_SIZE)) {
      commitAnnotation(activeAnnotation);
    }
    activeAnnotation = null;
  }

  dragState = null;
  drawCanvas();
  positionToolbar();
}

function setToolbarDisabled(disabled) {
  [...toolbar.querySelectorAll('button')].forEach((button) => {
    button.disabled = disabled;
  });
  textInput.disabled = disabled;
  if (!disabled) {
    updateHistoryButtons();
  }
}

async function closeWindow(shouldCleanup = true) {
  await closeScreenshotWindow({
    shouldCleanup,
    snapshotPath: snapshot.path,
    snapshotCleaned,
    markSnapshotCleaned: () => {
      snapshotCleaned = true;
    },
    shouldRestoreMainWindow,
    onCloseFailed: () => {
      isCapturing = false;
      setToolbarDisabled(false);
      showError('关闭截图窗口失败，请按 Alt+F4 退出');
    },
  });
}

bindScreenshotEvents({
  canvas,
  targetWindow: window,
  documentRef: document,
  controls: {
    confirmBtn,
    pinBtn,
    cancelBtn,
    reselectBtn,
    undoBtn,
    redoBtn,
    textInput,
    toolButtons,
  },
  actions: {
    onMouseDown,
    onMouseMove,
    onMouseUp,
    resizeCanvas,
    confirmSelection: captureActions.confirmSelection,
    pinSelection: captureActions.pinSelection,
    closeWindow,
    resetForReselect,
    undoAnnotation,
    redoAnnotation,
    commitTextInput,
    cancelTextInput,
    drawCanvas,
    positionToolbar,
    setActiveTool,
    nudgeSelection,
  },
  getState: () => ({
    pendingTextAnnotation: textEditor.pending,
    selection,
  }),
});

resizeCanvas();

if (!snapshot.path) {
  showError('冻结屏幕失败: 缺少快照路径');
} else {
  frozenImage.onload = () => {
    imageReady = true;
    info.style.display = 'block';
    drawCanvas();
  };
  frozenImage.onerror = () => {
    showError('冻结屏幕加载失败，请重试');
  };
  frozenImage.src = convertFileSrc(snapshot.path);
}
