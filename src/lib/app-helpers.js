export function activeFilterQuery(activeFilter) {
  if (activeFilter === 'favorite') {
    return { favoriteOnly: true };
  }

  if (activeFilter === 'image') {
    return { itemType: 'image' };
  }

  if (activeFilter === 'link') {
    return { itemType: 'link' };
  }

  return {};
}

export function deleteReasonLabel(item) {
  const reasons = [];
  if (item?.is_favorite) reasons.push('已收藏');
  if (item?.annotation) reasons.push('有标注');
  return reasons.join('、');
}

export function hotkeyFromKeyboardEvent(event) {
  if (['Control', 'Shift', 'Alt', 'Meta', 'Command'].includes(event.key)) {
    return { ignored: true };
  }

  const parts = [];

  if (event.ctrlKey || event.metaKey) {
    parts.push('CommandOrControl');
  }

  if (event.altKey) {
    parts.push('Alt');
  }

  if (event.shiftKey) {
    parts.push('Shift');
  }

  if (parts.length === 0) {
    return { message: '请使用修饰键组合（如 Ctrl+Shift+A）' };
  }

  const keyMap = {
    ' ': 'Space',
    ARROWDOWN: 'Down',
    ARROWLEFT: 'Left',
    ARROWRIGHT: 'Right',
    ARROWUP: 'Up',
  };
  const key = keyMap[event.key.toUpperCase()] || event.key.toUpperCase();

  return { hotkey: [...parts, key].join('+') };
}

export function limitItems(nextItems, pageSize) {
  return nextItems.slice(0, pageSize);
}

export function mergeItems(existingItems, nextItems) {
  const seen = new Set(existingItems.map((item) => item.id));
  return [
    ...existingItems,
    ...nextItems.filter((item) => {
      if (seen.has(item.id)) return false;
      seen.add(item.id);
      return true;
    }),
  ];
}

export function normalizeScreenshotError(errorValue) {
  const message = String(errorValue || '');

  if (message.includes('screenshot-selector') && message.includes('already exists')) {
    return '截图窗口已打开，请完成当前选区或按 Esc 取消后再试';
  }

  if (message.trim()) {
    return '截图失败: ' + message;
  }

  return '截图失败，请稍后再试';
}

export function numberSettingValue(value, fallback) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

export function pageSize(appSettings, defaultSettings) {
  return appSettings.max_items || defaultSettings.max_items;
}

export function requiresDeleteConfirmation(item) {
  return Boolean(item?.is_favorite || item?.annotation);
}
