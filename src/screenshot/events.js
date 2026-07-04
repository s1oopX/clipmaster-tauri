export function bindScreenshotEvents({
  canvas,
  targetWindow,
  documentRef,
  controls,
  actions,
  getState,
}) {
  canvas.addEventListener('mousedown', actions.onMouseDown);
  canvas.addEventListener('mousemove', actions.onMouseMove);
  targetWindow.addEventListener('mouseup', actions.onMouseUp);
  targetWindow.addEventListener('resize', actions.resizeCanvas);

  controls.confirmBtn.addEventListener('click', actions.confirmSelection);
  controls.pinBtn.addEventListener('click', actions.pinSelection);
  controls.cancelBtn.addEventListener('click', () => actions.closeWindow(true));
  controls.reselectBtn.addEventListener('click', actions.resetForReselect);
  controls.undoBtn.addEventListener('click', actions.undoAnnotation);
  controls.redoBtn.addEventListener('click', actions.redoAnnotation);
  controls.textInput.addEventListener('keydown', (event) => {
    event.stopPropagation();
    if (event.key === 'Enter') {
      event.preventDefault();
      actions.commitTextInput();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      actions.cancelTextInput();
      actions.drawCanvas();
      actions.positionToolbar();
      canvas.focus();
    }
  });
  controls.textInput.addEventListener('blur', actions.commitTextInput);

  controls.toolButtons.forEach((button) => {
    button.addEventListener('click', () => actions.setActiveTool(button.dataset.tool));
  });

  documentRef.addEventListener('keydown', (event) => {
    const { pendingTextAnnotation, selection } = getState();

    if (pendingTextAnnotation && event.key === 'Escape') {
      event.preventDefault();
      actions.cancelTextInput();
      actions.drawCanvas();
      actions.positionToolbar();
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      actions.closeWindow(true);
      return;
    }

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z') {
      event.preventDefault();
      if (event.shiftKey) {
        actions.redoAnnotation();
      } else {
        actions.undoAnnotation();
      }
      return;
    }

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'y') {
      event.preventDefault();
      actions.redoAnnotation();
      return;
    }

    if ((event.key === 'Enter' || (event.ctrlKey && event.key.toLowerCase() === 'c')) && selection) {
      event.preventDefault();
      actions.confirmSelection();
      return;
    }

    if ((event.key === 'p' || event.key === 'P') && selection) {
      event.preventDefault();
      actions.pinSelection();
      return;
    }

    const arrows = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    };
    if (selection && arrows[event.key]) {
      event.preventDefault();
      const [deltaX, deltaY] = arrows[event.key];
      actions.nudgeSelection(deltaX, deltaY, event.shiftKey);
    }
  });
}
