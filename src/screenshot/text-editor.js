import { TEXT_FONT_SIZE } from './constants.js';
import { clamp } from './geometry.js';

export function createTextAnnotationEditor({
  canvas,
  textInput,
  commitAnnotation,
  afterCommit,
}) {
  let pending = null;

  function position() {
    if (!pending) return;
    const inputWidth = Math.min(220, Math.max(120, canvas.width - 24));
    textInput.style.width = `${inputWidth}px`;
    textInput.style.left = `${clamp(pending.x, 8, canvas.width - inputWidth - 8)}px`;
    textInput.style.top = `${clamp(pending.y - TEXT_FONT_SIZE - 12, 8, canvas.height - 38)}px`;
  }

  return {
    get pending() {
      return pending;
    },

    begin(point) {
      this.commit();
      pending = {
        type: 'text',
        x: point.x,
        y: point.y,
      };
      textInput.value = '';
      position();
      textInput.style.display = 'block';
      textInput.focus();
    },

    position,

    commit() {
      if (!pending) return;
      const text = textInput.value.trim();
      const annotation = pending;
      pending = null;
      textInput.value = '';
      textInput.style.display = 'none';

      if (text) {
        commitAnnotation({ ...annotation, text });
        afterCommit();
      }
    },

    cancel() {
      pending = null;
      textInput.value = '';
      textInput.style.display = 'none';
    },
  };
}
