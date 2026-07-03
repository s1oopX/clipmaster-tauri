# Screenshot Feature Review

## Mature App Baseline

Sources checked:

- ShareX image editor: https://getsharex.com/docs/image-editor
- Greenshot help: https://getgreenshot.org/help/
- Snipaste: https://www.snipaste.com/
- Microsoft Snipping Tool: https://support.microsoft.com/en-us/windows/apps/use-snipping-tool-to-capture-screenshots

Common mature patterns:

- Capture modes: region, window, full screen, monitor, last region, scrolling capture, and sometimes video recording.
- Fast after-capture actions: copy, save, pin, print, share, upload, or run custom workflows.
- Annotation safety: rectangle, arrow, pen/highlighter, text, step markers, crop, eraser, blur, pixelate/mosaic, and undo/redo.
- Privacy and reuse: local OCR/redaction in newer tools, obfuscation before export, and pinned reference images.
- Keyboard efficiency: global hotkeys, capture delay, copy/save shortcuts, nudge controls, undo/redo.

## ClipMaster Fit

ClipMaster is primarily a local clipboard history and reuse tool, not a full screenshot automation suite. The screenshot workflow should prioritize:

- Capture and paste immediately.
- Save every useful capture into local history.
- Pin images for desktop reference.
- Redact sensitive regions before a screenshot enters clipboard history.
- Keep the selector reliable and recoverable.

## Implemented Coverage

- Region capture on a frozen screen.
- Hide the main window before freezing the screen, then restore only when appropriate.
- Copy final screenshot to clipboard and save it into image history.
- Pin the final screenshot to a desktop image window.
- Selection move, resize handles, keyboard nudging, and reselect.
- Rectangle, arrow, pen, blur, and pixelate annotation tools.
- Undo and redo for screenshot annotations.
- Snapshot cleanup when canceling or completing a screenshot.

## Remaining Gaps

- Full-screen, active-window, monitor, freeform, and last-region capture modes.
- Scrolling screenshot.
- Text labels, step markers, eraser, crop-only export, and color/style controls.
- OCR and automatic text redaction.
- Save-as/export destinations beyond ClipMaster history, clipboard, and pinning.
- Screen recording.

## Priority

P0:

- Keep region capture reliable.
- Keep privacy redaction available before saving/copying.
- Keep undo/redo available for annotation mistakes.

P1:

- Last-region capture.
- Text labels and step markers.
- Basic eraser or delete-selected annotation.

P2:

- Active-window and full-screen capture modes.
- OCR and text redaction.
- Scrolling screenshot.

P3:

- Screen recording and external share/upload workflows.
