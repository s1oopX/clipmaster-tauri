export function createAnnotationHistory(annotations, onChange = () => {}) {
  let undoStack = [];
  let redoStack = [];

  function notify() {
    onChange({
      canUndo: undoStack.length > 0,
      canRedo: redoStack.length > 0,
    });
  }

  function removeAnnotationByReference(annotation, fallbackIndex) {
    if (annotations[fallbackIndex] === annotation) {
      annotations.splice(fallbackIndex, 1);
      return;
    }

    const index = annotations.indexOf(annotation);
    if (index >= 0) {
      annotations.splice(index, 1);
    }
  }

  function applyHistoryAction(action) {
    if (action.type === 'add') {
      annotations.splice(Math.min(action.index, annotations.length), 0, action.annotation);
    } else if (action.type === 'remove') {
      removeAnnotationByReference(action.annotation, action.index);
    }
  }

  function revertHistoryAction(action) {
    if (action.type === 'add') {
      removeAnnotationByReference(action.annotation, action.index);
    } else if (action.type === 'remove') {
      annotations.splice(Math.min(action.index, annotations.length), 0, action.annotation);
    }
  }

  return {
    get canUndo() {
      return undoStack.length > 0;
    },

    get canRedo() {
      return redoStack.length > 0;
    },

    clear() {
      undoStack = [];
      redoStack = [];
      notify();
    },

    commit(annotation) {
      const index = annotations.length;
      annotations.push(annotation);
      undoStack.push({ type: 'add', annotation, index });
      redoStack = [];
      notify();
    },

    remove(index) {
      if (index < 0 || index >= annotations.length) return false;
      const [annotation] = annotations.splice(index, 1);
      undoStack.push({ type: 'remove', annotation, index });
      redoStack = [];
      notify();
      return true;
    },

    undo() {
      if (undoStack.length === 0) return false;
      const action = undoStack.pop();
      revertHistoryAction(action);
      redoStack.push(action);
      notify();
      return true;
    },

    redo() {
      if (redoStack.length === 0) return false;
      const action = redoStack.pop();
      applyHistoryAction(action);
      undoStack.push(action);
      notify();
      return true;
    },
  };
}
