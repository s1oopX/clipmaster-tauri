import {
  normalizeScreenshotError,
  requiresDeleteConfirmation,
} from './app-helpers.js';
import { effectiveItemType } from './clipboard-ui.js';

export function createRecordActionsController({
  clipboardApi,
  toolApi,
  getItems,
  setItems,
  getVisibleItems,
  getDeleteCandidate,
  getDeleteConfirmLoading,
  setDeleteCandidate,
  setDeleteConfirmLoading,
  setToolLoading,
  setError,
  clearActionError,
  showActionError,
  showActionNotice,
  showCopyToast,
  loadAvailableDays,
  pruneImageUrls,
  reconcileTransientItemState,
  updateVisibleItem,
}) {
  function sortItems() {
    setItems([...getItems()].sort((a, b) => {
      if (a.is_pinned && !b.is_pinned) return -1;
      if (!a.is_pinned && b.is_pinned) return 1;
      return b.timestamp - a.timestamp;
    }));
  }

  async function performDeleteItem(itemId) {
    try {
      await clipboardApi.deleteItem(itemId);
      const nextItems = getItems().filter((item) => item.id !== itemId);
      setItems(nextItems);
      pruneImageUrls(nextItems);
      reconcileTransientItemState(nextItems);
      await loadAvailableDays();
      setError(null);
      showActionNotice('已删除记录');
      return true;
    } catch (e) {
      console.error('删除失败:', e);
      showActionError('删除失败: ' + e);
      return false;
    }
  }

  function requestDeleteItem(item) {
    if (!requiresDeleteConfirmation(item)) {
      void performDeleteItem(item.id);
      return;
    }

    setDeleteCandidate(item);
  }

  function cancelDeleteConfirmation() {
    if (getDeleteConfirmLoading()) return;
    setDeleteCandidate(null);
  }

  async function confirmDeleteCandidate() {
    const deleteCandidate = getDeleteCandidate();
    if (!deleteCandidate || getDeleteConfirmLoading()) return;

    setDeleteConfirmLoading(true);
    const itemId = deleteCandidate.id;

    try {
      const deleted = await performDeleteItem(itemId);
      if (deleted) {
        setDeleteCandidate(null);
      }
    } finally {
      setDeleteConfirmLoading(false);
    }
  }

  async function toggleFavorite(itemId) {
    try {
      const isFavorite = await clipboardApi.toggleFavorite(itemId);
      updateVisibleItem(itemId, (item) => ({ ...item, is_favorite: isFavorite }));
      setError(null);
      clearActionError();
    } catch (e) {
      console.error('切换收藏失败:', e);
      showActionError('切换收藏失败: ' + e);
    }
  }

  async function togglePinned(itemId) {
    try {
      const isPinned = await clipboardApi.togglePinned(itemId);
      setItems(getItems().map((item) =>
        item.id === itemId ? { ...item, is_pinned: isPinned } : item
      ));
      sortItems();
      setError(null);
      clearActionError();
    } catch (e) {
      console.error('切换置顶失败:', e);
      showActionError('切换置顶失败: ' + e);
    }
  }

  async function copyItem(item) {
    try {
      if ((item.type === 'text' || effectiveItemType(item) === 'link') && item.content) {
        await clipboardApi.copyToClipboard(item.content);
        setError(null);
        showCopyToast();
      } else if (item.type === 'image' && item.image_path) {
        await clipboardApi.copyImageToClipboard(item.image_path);
        setError(null);
        showCopyToast();
      } else if (item.type === 'image') {
        showActionError('图片路径不可用');
      }
    } catch (e) {
      console.error('复制失败:', e);
      showActionError('复制失败: ' + e);
    }
  }

  async function startScreenshot() {
    setToolLoading('screenshot');
    setError(null);

    try {
      await toolApi.startRegionScreenshot();
      clearActionError();
      setToolLoading(null);
    } catch (e) {
      console.error('截图失败:', e);
      showActionError(normalizeScreenshotError(e));
      setToolLoading(null);
    }
  }

  async function pinNewestImage() {
    const image = getVisibleItems().find((item) => item.type === 'image' && item.image_path)
      || getItems().find((item) => item.type === 'image' && item.image_path);

    if (!image) {
      showActionError('当前没有可钉住的图片记录');
      return;
    }

    await pinImageToDesktop(image);
  }

  async function pinImageToDesktop(item) {
    if (!item.image_path) {
      showActionError('图片路径不可用');
      return;
    }

    setToolLoading('pin');
    setError(null);

    try {
      await toolApi.pinImage(item.image_path);
      showActionNotice('已钉到桌面');
    } catch (e) {
      console.error('贴图失败:', e);
      showActionError('贴图失败: ' + e);
    } finally {
      setToolLoading(null);
    }
  }

  return {
    cancelDeleteConfirmation,
    confirmDeleteCandidate,
    copyItem,
    pinImageToDesktop,
    pinNewestImage,
    requestDeleteItem,
    sortItems,
    startScreenshot,
    toggleFavorite,
    togglePinned,
  };
}
