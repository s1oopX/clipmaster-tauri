import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  api: {
    clearSession: vi.fn(),
    convertImagePath: vi.fn(),
    copyToClipboard: vi.fn(),
    copyImageToClipboard: vi.fn(),
    startRegionScreenshot: vi.fn(),
    deleteItem: vi.fn(),
    checkDevServerPort: vi.fn(),
    getAppDataDir: vi.fn(),
    getSettings: vi.fn(),
    getCurrentSession: vi.fn(),
    getItems: vi.fn(),
    getAvailableDays: vi.fn(),
    getItemsByDay: vi.fn(),
    getItemsBySession: vi.fn(),
    getSessions: vi.fn(),
    clearAllHistory: vi.fn(),
    listen: vi.fn(),
    onNewItem: vi.fn(),
    openExternalUrl: vi.fn(),
    pinImage: vi.fn(),
    previewCustomCleanup: vi.fn(),
    runCustomCleanup: vi.fn(),
    saveSettings: vi.fn(),
    restartApp: vi.fn(),
    searchItems: vi.fn(),
    toggleFavorite: vi.fn(),
    togglePinned: vi.fn(),
    updateItemContent: vi.fn(),
    updateItemAnnotation: vi.fn(),
  },
}));

export const api = mocks.api;

vi.mock('@tauri-apps/api/event', () => ({
  listen: mocks.api.listen,
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    close: vi.fn(),
  })),
}));

vi.mock('../lib/api.js', () => ({
  clipboardApi: {
    getItems: mocks.api.getItems,
    getItemsByDay: mocks.api.getItemsByDay,
    getAvailableDays: mocks.api.getAvailableDays,
    deleteItem: mocks.api.deleteItem,
    toggleFavorite: mocks.api.toggleFavorite,
    togglePinned: mocks.api.togglePinned,
    copyToClipboard: mocks.api.copyToClipboard,
    copyImageToClipboard: mocks.api.copyImageToClipboard,
    updateItemContent: mocks.api.updateItemContent,
    updateItemAnnotation: mocks.api.updateItemAnnotation,
    onNewItem: mocks.api.onNewItem,
  },
  sessionApi: {
    getCurrentSession: mocks.api.getCurrentSession,
    getSessions: mocks.api.getSessions,
    getItemsBySession: mocks.api.getItemsBySession,
    clearSession: mocks.api.clearSession,
  },
  searchApi: {
    searchItems: mocks.api.searchItems,
  },
  toolApi: {
    startRegionScreenshot: mocks.api.startRegionScreenshot,
    pinImage: mocks.api.pinImage,
    openExternalUrl: mocks.api.openExternalUrl,
  },
  settingsApi: {
    getSettings: mocks.api.getSettings,
    getAppDataDir: mocks.api.getAppDataDir,
    saveSettings: mocks.api.saveSettings,
    checkDevServerPort: mocks.api.checkDevServerPort,
    restartApp: mocks.api.restartApp,
    previewCustomCleanup: mocks.api.previewCustomCleanup,
    runCustomCleanup: mocks.api.runCustomCleanup,
    clearAllHistory: mocks.api.clearAllHistory,
  },
  convertImagePath: mocks.api.convertImagePath,
}));

export const session = {
  id: 'session_1',
  start_time: 1780650000000,
  end_time: null,
  item_count: 0,
  is_active: true,
};

export function textItem(overrides = {}) {
  return {
    id: 'text_1',
    type: 'text',
    content: 'Alpha token',
    image_path: null,
    preview: 'Alpha token',
    timestamp: Date.now(),
    date_key: '2026-06-06',
    source_app: null,
    is_favorite: false,
    is_pinned: false,
    annotation: null,
    content_hash: 'hash_text',
    session_id: 'session_1',
    ...overrides,
  };
}

export function linkItem(overrides = {}) {
  const content = 'https://example.com/docs?q=clipmaster';
  return {
    ...textItem({
      id: 'link_1',
      type: 'link',
      content,
      preview: content,
      content_hash: 'hash_link',
      ...overrides,
    }),
  };
}

export function imageItem(overrides = {}) {
  return {
    id: 'image_1',
    type: 'image',
    content: null,
    image_path: 'images/2026-06-06/image.png',
    thumbnail_path: 'images/2026-06-06/image_thumb.png',
    preview: null,
    timestamp: Date.now() - 1000,
    date_key: '2026-06-06',
    source_app: null,
    is_favorite: true,
    is_pinned: false,
    annotation: null,
    content_hash: 'hash_image',
    session_id: 'session_1',
    ...overrides,
  };
}

export function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });

  return { promise, reject, resolve };
}

export function todayDateKey(timeZone = 'Asia/Shanghai') {
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).formatToParts(new Date());
  const values = Object.fromEntries(parts.map((part) => [part.type, part.value]));
  return `${values.year}-${values.month}-${values.day}`;
}


beforeEach(() => {
    vi.clearAllMocks();

    api.getCurrentSession.mockResolvedValue(session);
    api.getItems.mockResolvedValue([]);
    api.getItemsByDay.mockResolvedValue([]);
    api.getAvailableDays.mockResolvedValue([{ date_key: '2026-06-06', item_count: 2, start_time: 1780650000000, end_time: 1780653600000 }]);
    api.getSessions.mockResolvedValue([]);
    api.getItemsBySession.mockResolvedValue([]);
    api.getAppDataDir.mockResolvedValue('C:\\Users\\tester\\AppData\\Roaming\\com.clipmaster.desktop');
    api.getSettings.mockResolvedValue({
      clipboard_monitor_enabled: true,
      show_main_window_on_start: true,
      auto_start_enabled: false,
      max_items: 50,
      capture_delay_ms: 150,
      screenshot_hotkey: 'CommandOrControl+Shift+A',
      main_window_hotkey: 'CommandOrControl+Shift+Space',
      time_zone: 'Asia/Shanghai',
      language: 'zh-CN',
      auto_cleanup_enabled: false,
      cleanup_max_items: 200,
      cleanup_keep_days: 30,
      dev_server_port: 5174,
    });
    api.clearSession.mockResolvedValue();
    api.listen.mockResolvedValue(vi.fn());
    api.onNewItem.mockResolvedValue(vi.fn());
    api.searchItems.mockResolvedValue([]);
    api.startRegionScreenshot.mockResolvedValue();
    api.pinImage.mockResolvedValue();
    api.openExternalUrl.mockResolvedValue();
    api.previewCustomCleanup.mockResolvedValue({
      item_count: 0,
      text_count: 0,
      image_count: 0,
      oldest_timestamp: null,
      newest_timestamp: null,
    });
    api.runCustomCleanup.mockResolvedValue({
      item_count: 0,
      text_count: 0,
      image_count: 0,
      oldest_timestamp: null,
      newest_timestamp: null,
    });
    api.clearAllHistory.mockResolvedValue({
      item_count: 0,
      text_count: 0,
      image_count: 0,
      oldest_timestamp: null,
      newest_timestamp: null,
    });
    api.saveSettings.mockImplementation(async (settings) => settings);
    api.checkDevServerPort.mockResolvedValue({
      port: 5174,
      available: true,
      suggested_port: null,
      message: '端口 5174 可用',
    });
    api.restartApp.mockResolvedValue();
    api.deleteItem.mockResolvedValue();
    api.toggleFavorite.mockResolvedValue(true);
    api.togglePinned.mockResolvedValue(true);
    api.copyToClipboard.mockResolvedValue();
    api.copyImageToClipboard.mockResolvedValue();
    api.updateItemContent.mockImplementation(async (itemId, newContent) => {
      const trimmed = newContent.trim();
      const isLink = /^https?:\/\/[^\s/?#]+\.[^\s]*$/i.test(trimmed);
      return textItem({
        id: itemId,
        type: isLink ? 'link' : 'text',
        content: isLink ? trimmed : newContent,
        preview: isLink ? trimmed : newContent,
      });
    });
    api.updateItemAnnotation.mockImplementation(async (_itemId, annotation) => {
      const trimmed = annotation.trim();
      return trimmed ? trimmed : null;
    });
    api.convertImagePath.mockResolvedValue('asset://localhost/images/2026-06-06/image.png');
  });
