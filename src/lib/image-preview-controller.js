export function createImagePreviewController({
  convertImagePath,
  getItems,
  getImageUrls,
  setImageUrls,
  getThumbnailUrls,
  setThumbnailUrls,
  getImagePreviewErrors,
  setImagePreviewErrors,
  setViewingImageId,
  showActionError,
}) {
  async function resolveFirstImageUrl(paths) {
    const seen = new Set();

    for (const path of paths) {
      if (!path || seen.has(path)) continue;
      seen.add(path);

      try {
        const url = await convertImagePath(path);
        if (url) return url;
      } catch (e) {
        console.error('加载图片预览 URL 失败:', e);
      }
    }

    return null;
  }

  // 状态写入统一走「写入时基于最新状态合并」，避免并发加载各持旧快照互相覆盖
  function commitPreviewResult(itemId, previewUrl) {
    const thumbnailUrls = { ...getThumbnailUrls() };
    const imagePreviewErrors = { ...getImagePreviewErrors() };

    if (previewUrl) {
      thumbnailUrls[itemId] = previewUrl;
      delete imagePreviewErrors[itemId];
    } else {
      delete thumbnailUrls[itemId];
      imagePreviewErrors[itemId] = true;
    }

    setThumbnailUrls(thumbnailUrls);
    setImagePreviewErrors(imagePreviewErrors);
  }

  async function ensureImagePreviewUrl(item) {
    if (item.type !== 'image' || (!item.thumbnail_path && !item.image_path)) {
      return;
    }

    const previewUrl = await resolveFirstImageUrl([item.thumbnail_path, item.image_path]);
    commitPreviewResult(item.id, previewUrl);
  }

  async function loadImageUrls() {
    // 已失败的条目不重复请求（负缓存）；fallbackToOriginalPreview 仍可显式重试
    const pendingItems = getItems().filter(
      (item) =>
        item.type === 'image'
        && !getThumbnailUrls()[item.id]
        && !getImagePreviewErrors()[item.id]
    );

    await Promise.all(pendingItems.map((item) => ensureImagePreviewUrl(item)));
  }

  function pruneImageUrls(nextItems = getItems()) {
    const liveIds = new Set(nextItems.map((item) => item.id));

    setImageUrls(Object.fromEntries(
      Object.entries(getImageUrls()).filter(([itemId]) => liveIds.has(itemId))
    ));

    setThumbnailUrls(Object.fromEntries(
      Object.entries(getThumbnailUrls()).filter(([itemId]) => liveIds.has(itemId))
    ));

    setImagePreviewErrors(Object.fromEntries(
      Object.entries(getImagePreviewErrors()).filter(([itemId]) => liveIds.has(itemId))
    ));
  }

  async function fallbackToOriginalPreview(item) {
    if (!item?.id) return;

    if (!item.image_path || getThumbnailUrls()[item.id] === getImageUrls()[item.id]) {
      commitPreviewResult(item.id, null);
      return;
    }

    try {
      const originalUrl =
        getImageUrls()[item.id] || (await resolveFirstImageUrl([item.image_path]));
      if (!originalUrl) throw new Error('原图 URL 不可用');

      setImageUrls({ ...getImageUrls(), [item.id]: originalUrl });
      commitPreviewResult(item.id, originalUrl);
    } catch (e) {
      console.error('原图预览加载失败:', e);
      commitPreviewResult(item.id, null);
      showActionError('图片预览不可用');
    }
  }

  async function viewFullImage(itemId) {
    const item = getItems().find((entry) => entry.id === itemId);
    if (item?.image_path && !getImageUrls()[itemId]) {
      try {
        const originalUrl = await convertImagePath(item.image_path);
        if (!originalUrl) {
          // 解析成功但文件缺失：不打开空白查看器
          showActionError('原图不可用，文件可能已被删除');
          return;
        }
        setImageUrls({ ...getImageUrls(), [itemId]: originalUrl });
      } catch (e) {
        console.error('加载原图 URL 失败:', e);
        showActionError('加载原图失败: ' + e);
        return;
      }
    }

    setViewingImageId(itemId);
  }

  return {
    ensureImagePreviewUrl,
    fallbackToOriginalPreview,
    loadImageUrls,
    pruneImageUrls,
    viewFullImage,
  };
}
