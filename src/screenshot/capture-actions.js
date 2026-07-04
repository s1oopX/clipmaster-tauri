import { invoke } from '@tauri-apps/api/core';

export function createCaptureActions({
  getIsCapturing,
  setIsCapturing,
  commitTextInput,
  isUsableSelection,
  showError,
  hideError,
  setToolbarDisabled,
  renderFinalDataUrl,
  getSnapshotPath,
  markSnapshotCleaned,
  closeWindow,
}) {
  async function saveSelection() {
    if (getIsCapturing()) return null;
    commitTextInput();
    if (!isUsableSelection()) {
      showError('请先选择截图区域');
      return null;
    }

    setIsCapturing(true);
    hideError();
    setToolbarDisabled(true);

    try {
      const imageDataUrl = renderFinalDataUrl();
      const item = await invoke('save_screenshot_image', {
        imageDataUrl,
        snapshotPath: getSnapshotPath(),
      });
      markSnapshotCleaned();
      return item;
    } catch (error) {
      console.error('截图失败:', error);
      showError('截图失败: ' + error);
      setIsCapturing(false);
      setToolbarDisabled(false);
      return null;
    }
  }

  return {
    saveSelection,

    async confirmSelection() {
      const item = await saveSelection();
      if (item) {
        await closeWindow(false);
      }
    },

    async pinSelection() {
      const item = await saveSelection();
      if (!item) return;

      try {
        if (item.image_path) {
          await invoke('pin_image', { imagePath: item.image_path });
        }
        await closeWindow(false);
      } catch (error) {
        console.error('钉住截图失败:', error);
        showError('钉住截图失败: ' + error);
        setIsCapturing(false);
        setToolbarDisabled(false);
      }
    },
  };
}
