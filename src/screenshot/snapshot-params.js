export function readSnapshotParams(locationRef, viewport) {
  const params = new URLSearchParams(locationRef.search);
  const numberParam = (key, fallback) => {
    const parsed = Number(params.get(key));
    return Number.isFinite(parsed) ? parsed : fallback;
  };

  return {
    snapshot: {
      path: params.get('snapshotPath') || '',
      screenX: numberParam('screenX', 0),
      screenY: numberParam('screenY', 0),
      screenWidth: numberParam('screenWidth', viewport.innerWidth),
      screenHeight: numberParam('screenHeight', viewport.innerHeight),
      pixelWidth: numberParam('pixelWidth', viewport.innerWidth),
      pixelHeight: numberParam('pixelHeight', viewport.innerHeight),
      scaleFactor: numberParam('scaleFactor', 1),
    },
    shouldRestoreMainWindow: params.get('restoreMainWindow') !== '0',
  };
}
