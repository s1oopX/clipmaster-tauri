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

  async function ensureImagePreviewUrl(item) {
    if (item.type !== 'image' || (!item.thumbnail_path && !item.image_path)) {
      return;
    }

    const thumbnailUrls = { ...getThumbnailUrls() };
    const imagePreviewErrors = { ...getImagePreviewErrors() };
    const previewUrl = await resolveFirstImageUrl([item.thumbnail_path, item.image_path]);
    if (previewUrl) {
      thumbnailUrls[item.id] = previewUrl;
      delete imagePreviewErrors[item.id];
    } else {
      delete thumbnailUrls[item.id];
      imagePreviewErrors[item.id] = true;
    }

    setThumbnailUrls(thumbnailUrls);
    setImagePreviewErrors(imagePreviewErrors);
  }

  async function loadImageUrls() {
    for (const item of getItems()) {
      if (item.type === 'image' && !getThumbnailUrls()[item.id]) {
        await ensureImagePreviewUrl(item);
      }
    }
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
    const thumbnailUrls = { ...getThumbnailUrls() };
    const imagePreviewErrors = { ...getImagePreviewErrors() };
    const imageUrls = { ...getImageUrls() };

    if (!item?.image_path || thumbnailUrls[item.id] === imageUrls[item.id]) {
      delete thumbnailUrls[item.id];
      imagePreviewErrors[item.id] = true;
      setThumbnailUrls(thumbnailUrls);
      setImagePreviewErrors(imagePreviewErrors);
      return;
    }

    try {
      const originalUrl = imageUrls[item.id] || await resolveFirstImageUrl([item.image_path]);
      if (!originalUrl) throw new Error('原图 URL 不可用');

      imageUrls[item.id] = originalUrl;
      thumbnailUrls[item.id] = originalUrl;
      delete imagePreviewErrors[item.id];
      setImageUrls(imageUrls);
      setThumbnailUrls(thumbnailUrls);
      setImagePreviewErrors(imagePreviewErrors);
    } catch (e) {
      console.error('原图预览加载失败:', e);
      delete thumbnailUrls[item.id];
      imagePreviewErrors[item.id] = true;
      setThumbnailUrls(thumbnailUrls);
      setImagePreviewErrors(imagePreviewErrors);
      showActionError('图片预览不可用');
    }
  }

  async function viewFullImage(itemId) {
    const item = getItems().find((entry) => entry.id === itemId);
    const imageUrls = { ...getImageUrls() };
    if (item?.image_path && !imageUrls[itemId]) {
      try {
        imageUrls[itemId] = await convertImagePath(item.image_path);
        setImageUrls(imageUrls);
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
