export function createContextMenuController({
  getContextMenu,
  setContextMenu,
  getWindow,
}) {
  function closeContextMenu() {
    setContextMenu({ open: false, x: 0, y: 0, itemId: null });
  }

  function handleDocumentClick(event) {
    if (!getContextMenu().open) return;
    if (event.target?.closest?.('.context-menu')) return;
    closeContextMenu();
  }

  function handleDocumentKeyDown(event) {
    if (event.key === 'Escape') {
      closeContextMenu();
    }
  }

  function openContextMenu(event, item) {
    event.preventDefault();

    const estimatedWidth = 178;
    const estimatedHeight = item.type === 'text' ? 188 : 144;
    const currentWindow = getWindow();
    const x = Math.min(
      Math.max(8, event.clientX),
      Math.max(8, currentWindow.innerWidth - estimatedWidth - 8)
    );
    const y = Math.min(
      Math.max(8, event.clientY),
      Math.max(8, currentWindow.innerHeight - estimatedHeight - 8)
    );

    setContextMenu({ open: true, x, y, itemId: item.id });
  }

  function runContextAction(action) {
    closeContextMenu();
    action();
  }

  return {
    closeContextMenu,
    handleDocumentClick,
    handleDocumentKeyDown,
    openContextMenu,
    runContextAction,
  };
}
