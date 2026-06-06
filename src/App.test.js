import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';

const api = vi.hoisted(() => ({
  clearSession: vi.fn(),
  convertImagePath: vi.fn(),
  copyToClipboard: vi.fn(),
  copyImageToClipboard: vi.fn(),
  startRegionScreenshot: vi.fn(),
  deleteItem: vi.fn(),
  getSettings: vi.fn(),
  getCurrentSession: vi.fn(),
  getItems: vi.fn(),
  getAvailableDays: vi.fn(),
  getItemsByDay: vi.fn(),
  getItemsBySession: vi.fn(),
  getSessions: vi.fn(),
  listen: vi.fn(),
  onNewItem: vi.fn(),
  pinImage: vi.fn(),
  previewCustomCleanup: vi.fn(),
  runCustomCleanup: vi.fn(),
  saveSettings: vi.fn(),
  searchItems: vi.fn(),
  toggleFavorite: vi.fn(),
  togglePinned: vi.fn(),
  updateItemContent: vi.fn(),
  updateItemAnnotation: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: api.listen,
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    close: vi.fn(),
  })),
}));

vi.mock('./lib/api.js', () => ({
  clipboardApi: {
    getItems: api.getItems,
    getItemsByDay: api.getItemsByDay,
    getAvailableDays: api.getAvailableDays,
    deleteItem: api.deleteItem,
    toggleFavorite: api.toggleFavorite,
    togglePinned: api.togglePinned,
    copyToClipboard: api.copyToClipboard,
    copyImageToClipboard: api.copyImageToClipboard,
    updateItemContent: api.updateItemContent,
    updateItemAnnotation: api.updateItemAnnotation,
    onNewItem: api.onNewItem,
  },
  sessionApi: {
    getCurrentSession: api.getCurrentSession,
    getSessions: api.getSessions,
    getItemsBySession: api.getItemsBySession,
    clearSession: api.clearSession,
  },
  searchApi: {
    searchItems: api.searchItems,
  },
  toolApi: {
    startRegionScreenshot: api.startRegionScreenshot,
    pinImage: api.pinImage,
  },
  settingsApi: {
    getSettings: api.getSettings,
    saveSettings: api.saveSettings,
    previewCustomCleanup: api.previewCustomCleanup,
    runCustomCleanup: api.runCustomCleanup,
  },
  convertImagePath: api.convertImagePath,
}));

const session = {
  id: 'session_1',
  start_time: 1780650000000,
  end_time: null,
  item_count: 0,
  is_active: true,
};

function textItem(overrides = {}) {
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

function imageItem(overrides = {}) {
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

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });

  return { promise, reject, resolve };
}

function todayDateKey(timeZone = 'Asia/Shanghai') {
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).formatToParts(new Date());
  const values = Object.fromEntries(parts.map((part) => [part.type, part.value]));
  return `${values.year}-${values.month}-${values.day}`;
}

describe('App UI', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    api.getCurrentSession.mockResolvedValue(session);
    api.getItems.mockResolvedValue([]);
    api.getItemsByDay.mockResolvedValue([]);
    api.getAvailableDays.mockResolvedValue([{ date_key: '2026-06-06', item_count: 2, start_time: 1780650000000, end_time: 1780653600000 }]);
    api.getSessions.mockResolvedValue([]);
    api.getItemsBySession.mockResolvedValue([]);
    api.getSettings.mockResolvedValue({
      clipboard_monitor_enabled: true,
      show_main_window_on_start: true,
      max_items: 50,
      capture_delay_ms: 150,
      screenshot_hotkey: 'CommandOrControl+Shift+A',
      time_zone: 'Asia/Shanghai',
      language: 'zh-CN',
      auto_cleanup_enabled: false,
      cleanup_max_items: 200,
      cleanup_keep_days: 30,
    });
    api.clearSession.mockResolvedValue();
    api.listen.mockResolvedValue(vi.fn());
    api.onNewItem.mockResolvedValue(vi.fn());
    api.searchItems.mockResolvedValue([]);
    api.startRegionScreenshot.mockResolvedValue();
    api.pinImage.mockResolvedValue();
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
    api.saveSettings.mockImplementation(async (settings) => settings);
    api.deleteItem.mockResolvedValue();
    api.toggleFavorite.mockResolvedValue(true);
    api.togglePinned.mockResolvedValue(true);
    api.copyToClipboard.mockResolvedValue();
    api.copyImageToClipboard.mockResolvedValue();
    api.updateItemContent.mockResolvedValue();
    api.updateItemAnnotation.mockImplementation(async (_itemId, annotation) => {
      const trimmed = annotation.trim();
      return trimmed ? trimmed : null;
    });
    api.convertImagePath.mockResolvedValue('asset://localhost/images/2026-06-06/image.png');
  });

  it('renders the desktop app shell with search, filters, and an empty history state', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));

    expect(screen.getByRole('heading', { name: 'ClipMaster' })).toBeInTheDocument();
    expect(screen.getByRole('searchbox', { name: '搜索剪贴板内容' })).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: '剪贴板筛选' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '全部记录' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '收藏' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '图片' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '截图' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '钉住' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '设置' })).toBeInTheDocument();
    expect(screen.getByText('剪贴板历史')).toBeInTheDocument();
    expect(screen.getByText('复制内容后会自动出现在这里')).toBeInTheDocument();
  });

  it('cleans up global event listeners when the app shell unmounts', async () => {
    const unlistenHotkey = vi.fn();
    const unlistenNewItem = vi.fn();
    api.listen.mockResolvedValueOnce(unlistenHotkey);
    api.onNewItem.mockResolvedValueOnce(unlistenNewItem);

    const { unmount } = render(App);

    await waitFor(() => {
      expect(api.listen).toHaveBeenCalledWith('hotkey:screenshot', expect.any(Function));
      expect(api.onNewItem).toHaveBeenCalledTimes(1);
    });

    unmount();

    expect(unlistenHotkey).toHaveBeenCalledTimes(1);
    expect(unlistenNewItem).toHaveBeenCalledTimes(1);
  });

  it('shows clipboard items with accessible action buttons and image previews', async () => {
    api.getItems.mockResolvedValue([textItem(), imageItem()]);

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();
    expect(await screen.findByText('图片记录')).toBeInTheDocument();
    expect(await screen.findByAltText('剪贴板图片缩略图')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '复制 Alpha token' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '置顶 Alpha token' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '收藏 Alpha token' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '删除 Alpha token' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '复制 图片记录' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '钉到桌面 图片记录' })).toBeInTheDocument();
  });

  it('deletes ordinary records immediately without a confirmation dialog', async () => {
    api.getItems.mockResolvedValue([textItem()]);

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '删除 Alpha token' }));

    await waitFor(() => {
      expect(api.deleteItem).toHaveBeenCalledWith('text_1');
    });
    await waitFor(() => {
      expect(api.getAvailableDays).toHaveBeenCalledTimes(2);
    });
    expect(screen.queryByRole('dialog', { name: '确认删除' })).not.toBeInTheDocument();
    expect(screen.queryByText('Alpha token')).not.toBeInTheDocument();
  });

  it('shows a toast error when ordinary record deletion fails', async () => {
    api.getItems.mockResolvedValue([textItem()]);
    api.deleteItem.mockRejectedValueOnce('数据库忙');

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '删除 Alpha token' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('删除失败: 数据库忙');
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);
    expect(screen.getByText('Alpha token')).toBeInTheDocument();
  });

  it('asks for confirmation before deleting favorite records', async () => {
    api.getItems.mockResolvedValue([textItem({ is_favorite: true })]);

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '删除 Alpha token' }));

    const dialog = screen.getByRole('dialog', { name: '确认删除' });
    expect(within(dialog).getByText(/已收藏/)).toBeInTheDocument();
    expect(api.deleteItem).not.toHaveBeenCalled();
  });

  it('asks for confirmation before deleting annotated records', async () => {
    api.getItems.mockResolvedValue([textItem({ annotation: '用于发票核对' })]);

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '删除 Alpha token' }));

    const dialog = screen.getByRole('dialog', { name: '确认删除' });
    expect(within(dialog).getByText(/有标注/)).toBeInTheDocument();
    expect(api.deleteItem).not.toHaveBeenCalled();
  });

  it('cancels protected record deletion without removing the item', async () => {
    api.getItems.mockResolvedValue([textItem({ is_favorite: true })]);

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '删除 Alpha token' }));
    await fireEvent.click(screen.getByRole('button', { name: '取消' }));

    expect(screen.queryByRole('dialog', { name: '确认删除' })).not.toBeInTheDocument();
    expect(api.deleteItem).not.toHaveBeenCalled();
    expect(screen.getByText('Alpha token')).toBeInTheDocument();
  });

  it('deletes a protected record after explicit confirmation', async () => {
    api.getItems.mockResolvedValue([textItem({ annotation: '用于发票核对' })]);

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '删除 Alpha token' }));
    await fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(api.deleteItem).toHaveBeenCalledWith('text_1');
    });
    expect(screen.queryByRole('dialog', { name: '确认删除' })).not.toBeInTheDocument();
    expect(screen.queryByText('Alpha token')).not.toBeInTheDocument();
  });

  it('keeps the confirmation dialog open when protected record deletion fails', async () => {
    api.getItems.mockResolvedValue([textItem({ annotation: '用于发票核对' })]);
    api.deleteItem.mockRejectedValueOnce('数据库忙');

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '删除 Alpha token' }));
    await fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('删除失败: 数据库忙');
    const dialog = screen.getByRole('dialog', { name: '确认删除' });
    expect(dialog).toBeInTheDocument();
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);
    expect(screen.getAllByText('Alpha token').length).toBeGreaterThanOrEqual(2);
  });

  it('shows toast errors when favorite or pinned actions fail', async () => {
    api.getItems.mockResolvedValue([textItem()]);
    api.toggleFavorite.mockRejectedValueOnce('收藏失败');
    api.togglePinned.mockRejectedValueOnce('置顶失败');

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '收藏 Alpha token' }));
    let alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('切换收藏失败: 收藏失败');
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);

    await fireEvent.click(screen.getByRole('button', { name: '置顶 Alpha token' }));
    alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('切换置顶失败: 置顶失败');
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);
    expect(screen.getByText('Alpha token')).toBeInTheDocument();
  });

  it('searches within the current session and can clear the query', async () => {
    render(App);

    const search = await screen.findByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenCalledWith('alpha', 'session_1', 50, todayDateKey());
    });

    await fireEvent.click(screen.getByRole('button', { name: '清除搜索' }));

    expect(search).toHaveValue('');
    await waitFor(() => {
      expect(api.getItems).toHaveBeenCalledWith(50, 0);
    });
  });

  it('searches within the selected calendar day', async () => {
    api.getItemsByDay.mockResolvedValue([textItem()]);

    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
    await fireEvent.click(screen.getByRole('button', { name: '06-06 · 2' }));

    await waitFor(() => {
      expect(api.getItemsByDay).toHaveBeenCalledWith('2026-06-06', 50, 0);
    });

    const search = screen.getByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenLastCalledWith(
        'alpha',
        'session_1',
        50,
        '2026-06-06'
      );
    });
  });

  it('ignores stale search results after the query is cleared', async () => {
    const pendingSearch = deferred();
    api.getItems
      .mockResolvedValueOnce([
        textItem({ id: 'initial', content: 'Initial token', preview: 'Initial token' }),
      ])
      .mockResolvedValueOnce([
        textItem({ id: 'fresh', content: 'Fresh list', preview: 'Fresh list' }),
      ]);
    api.searchItems.mockReturnValueOnce(pendingSearch.promise);

    render(App);

    expect(await screen.findByText('Initial token')).toBeInTheDocument();

    const search = screen.getByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenCalledWith('alpha', 'session_1', 50, todayDateKey());
    });

    await fireEvent.click(screen.getByRole('button', { name: '清除搜索' }));

    expect(await screen.findByText('Fresh list')).toBeInTheDocument();
    expect(search).toHaveValue('');

    pendingSearch.resolve([
      textItem({ id: 'stale', content: 'Stale alpha result', preview: 'Stale alpha result' }),
    ]);

    await waitFor(() => {
      expect(screen.queryByText('Stale alpha result')).not.toBeInTheDocument();
      expect(screen.getByText('Fresh list')).toBeInTheDocument();
    });
  });

  it('keeps the narrow-window shell compact without page-level scrolling', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));

    expect(screen.getByTestId('app-shell')).toHaveAttribute('data-layout', 'compact-ready');
    expect(screen.getByTestId('app-shell')).toHaveAttribute('data-density', 'tool');
    expect(screen.getByTestId('history-panel')).toHaveAttribute('data-scroll', 'internal');
    expect(screen.getByRole('button', { name: '全部记录' })).toHaveClass('filter-button');
    expect(screen.getByRole('button', { name: '收藏' })).toHaveClass('filter-button');
    expect(screen.getByRole('button', { name: '图片' })).toHaveClass('filter-button');
    expect(document.body).toContainElement(screen.getByTestId('app-shell'));
  });

  it('starts region screenshot selection from the toolbar', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));

    await fireEvent.click(screen.getByRole('button', { name: '截图' }));

    await waitFor(() => expect(api.startRegionScreenshot).toHaveBeenCalledTimes(1));
  });

  it('shows an inline error when screenshot selection cannot start', async () => {
    api.startRegionScreenshot.mockRejectedValueOnce('权限不足');

    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
    await fireEvent.click(screen.getByRole('button', { name: '截图' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('截图失败: 权限不足');
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);
  });

  it('clears stale action errors after a successful search', async () => {
    api.startRegionScreenshot.mockRejectedValueOnce('权限不足');

    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
    await fireEvent.click(screen.getByRole('button', { name: '截图' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('截图失败: 权限不足');

    const search = screen.getByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenCalledWith('alpha', 'session_1', 50, todayDateKey());
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });
  });

  it('explains duplicate screenshot selector windows without exposing internal labels', async () => {
    api.startRegionScreenshot.mockRejectedValueOnce(
      'a webview with label `screenshot-selector` already exists'
    );

    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
    await fireEvent.click(screen.getByRole('button', { name: '截图' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('截图窗口已打开，请完成当前选区或按 Esc 取消后再试');
    expect(alert).not.toHaveTextContent('screenshot-selector');
  });

  it('pins the newest image globally and supports pinning an image item to desktop', async () => {
    api.getItems.mockResolvedValue([textItem(), imageItem()]);

    render(App);

    await screen.findByText('图片记录');

    await fireEvent.click(screen.getByRole('button', { name: '钉住' }));
    expect(api.pinImage).toHaveBeenCalledWith('images/2026-06-06/image.png');

    await fireEvent.click(screen.getByRole('button', { name: '钉到桌面 图片记录' }));
    expect(api.pinImage).toHaveBeenCalledTimes(2);
  });

  it('shows a toast error when there is no image to pin', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
    await fireEvent.click(screen.getByRole('button', { name: '钉住' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('当前没有可钉住的图片记录');
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);
  });

  it('copies image items to the system clipboard', async () => {
    api.getItems.mockResolvedValue([imageItem()]);

    render(App);

    await screen.findByText('图片记录');
    await fireEvent.click(screen.getByRole('button', { name: '复制 图片记录' }));

    await waitFor(() => {
      expect(api.copyImageToClipboard).toHaveBeenCalledWith('images/2026-06-06/image.png');
    });
    expect(screen.getByRole('status')).toHaveTextContent('已复制到剪贴板');
    expect(screen.getByTestId('toast-stack')).toHaveTextContent('已复制到剪贴板');
  });

  it('shows a toast error when copying text fails', async () => {
    api.getItems.mockResolvedValue([textItem()]);
    api.copyToClipboard.mockRejectedValueOnce('剪贴板被占用');

    render(App);

    const content = await screen.findByText('Alpha token');
    await fireEvent.dblClick(content);

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('复制失败: 剪贴板被占用');
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);
  });

  it('replaces a copy success toast with the next error toast', async () => {
    api.getItems.mockResolvedValue([textItem()]);

    render(App);

    const content = await screen.findByText('Alpha token');
    await fireEvent.dblClick(content);

    expect(await screen.findByRole('status')).toHaveTextContent('已复制到剪贴板');

    api.copyToClipboard.mockRejectedValueOnce('剪贴板被占用');
    await fireEvent.dblClick(content);

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('复制失败: 剪贴板被占用');
    expect(screen.queryByText('已复制到剪贴板')).not.toBeInTheDocument();
  });

  it('copies text quickly from the content area on double click', async () => {
    api.getItems.mockResolvedValue([textItem()]);

    render(App);

    const content = await screen.findByText('Alpha token');
    await fireEvent.dblClick(content);

    await waitFor(() => {
      expect(api.copyToClipboard).toHaveBeenCalledWith('Alpha token');
    });
    expect(screen.getByRole('status')).toHaveTextContent('已复制到剪贴板');
    expect(screen.getByTestId('toast-stack')).toHaveTextContent('已复制到剪贴板');
  });

  it('saves annotations without changing the original clipboard content', async () => {
    api.getItems.mockResolvedValue([
      textItem({ annotation: '旧标注' }),
    ]);

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '标注 Alpha token' }));
    const annotationInput = screen.getByLabelText('编辑 Alpha token 的标注');

    expect(annotationInput).toHaveValue('旧标注');

    await fireEvent.input(annotationInput, { target: { value: '用于发票核对' } });
    await fireEvent.click(screen.getByRole('button', { name: '保存标注' }));

    await waitFor(() => {
      expect(api.updateItemAnnotation).toHaveBeenCalledWith('text_1', '用于发票核对');
    });
    expect(screen.getByText('用于发票核对')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '复制 Alpha token' }));

    expect(api.copyToClipboard).toHaveBeenCalledWith('Alpha token');
    expect(api.copyToClipboard).not.toHaveBeenCalledWith('用于发票核对');
  });

  it('offers edit and annotation actions from the item context menu', async () => {
    api.getItems.mockResolvedValue([textItem()]);

    render(App);

    const content = await screen.findByText('Alpha token');
    await fireEvent.contextMenu(content, { clientX: 120, clientY: 160 });

    expect(screen.getByRole('menu')).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: '编辑原文' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: '添加标注' })).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('menuitem', { name: '编辑原文' }));
    const contentInput = screen.getByLabelText('编辑 Alpha token 的原文');
    await fireEvent.input(contentInput, { target: { value: 'Beta token' } });
    await fireEvent.click(screen.getByRole('button', { name: '保存原文' }));

    await waitFor(() => {
      expect(api.updateItemContent).toHaveBeenCalledWith('text_1', 'Beta token');
    });
    expect(screen.getByText('Beta token')).toBeInTheDocument();

    await fireEvent.contextMenu(screen.getByText('Beta token'), { clientX: 140, clientY: 180 });
    await fireEvent.click(screen.getByRole('menuitem', { name: '添加标注' }));

    const annotationInput = screen.getByLabelText('编辑 Beta token 的标注');
    await fireEvent.input(annotationInput, { target: { value: '来自右键菜单' } });
    await fireEvent.click(screen.getByRole('button', { name: '保存标注' }));

    await waitFor(() => {
      expect(api.updateItemAnnotation).toHaveBeenCalledWith('text_1', '来自右键菜单');
    });
    expect(screen.getByText('已标注')).toBeInTheDocument();
    expect(screen.getByText('来自右键菜单')).toBeInTheDocument();
  });

  it('shows an error toast when content edit fails', async () => {
    api.getItems.mockResolvedValue([textItem()]);
    api.updateItemContent.mockRejectedValueOnce('当日已存在相同内容');

    render(App);

    const content = await screen.findByText('Alpha token');
    await fireEvent.contextMenu(content, { clientX: 120, clientY: 160 });
    await fireEvent.click(screen.getByRole('menuitem', { name: '编辑原文' }));

    const contentInput = screen.getByLabelText('编辑 Alpha token 的原文');
    await fireEvent.input(contentInput, { target: { value: 'Beta token' } });
    await fireEvent.click(screen.getByRole('button', { name: '保存原文' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('保存原文失败: 当日已存在相同内容');
    expect(alert).toHaveClass('error');
    expect(screen.getByRole('button', { name: '保存原文' })).toBeInTheDocument();
    expect(screen.queryByText('原文已更新')).not.toBeInTheDocument();
  });

  it('shows an error toast when annotation save fails', async () => {
    api.getItems.mockResolvedValue([textItem()]);
    api.updateItemAnnotation.mockRejectedValueOnce('权限不足');

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '标注 Alpha token' }));
    const annotationInput = screen.getByLabelText('编辑 Alpha token 的标注');
    await fireEvent.input(annotationInput, { target: { value: '来自右键菜单' } });
    await fireEvent.click(screen.getByRole('button', { name: '保存标注' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('保存标注失败: 权限不足');
    expect(alert).toHaveClass('error');
    expect(screen.queryByText('来自右键菜单')).not.toBeInTheDocument();
  });

  it('removes edited content from active search results when it no longer matches', async () => {
    api.searchItems.mockResolvedValue([textItem()]);

    render(App);

    const search = await screen.findByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    const content = await screen.findByText('Alpha token');
    await fireEvent.contextMenu(content, { clientX: 120, clientY: 160 });
    await fireEvent.click(screen.getByRole('menuitem', { name: '编辑原文' }));

    const contentInput = screen.getByLabelText('编辑 Alpha token 的原文');
    await fireEvent.input(contentInput, { target: { value: 'Beta token' } });
    await fireEvent.click(screen.getByRole('button', { name: '保存原文' }));

    await waitFor(() => {
      expect(api.updateItemContent).toHaveBeenCalledWith('text_1', 'Beta token');
    });
    expect(screen.queryByText('Beta token')).not.toBeInTheDocument();
    expect(screen.getByText('未找到匹配的记录')).toBeInTheDocument();
  });

  it('removes cleared annotations from active search results when the note was the match', async () => {
    api.searchItems.mockResolvedValue([
      textItem({ annotation: 'invoice note' }),
    ]);

    render(App);

    const search = await screen.findByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'invoice' } });

    expect(await screen.findByText('invoice note')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '标注 Alpha token' }));
    const annotationInput = screen.getByLabelText('编辑 Alpha token 的标注');
    await fireEvent.input(annotationInput, { target: { value: '' } });
    await fireEvent.click(screen.getByRole('button', { name: '保存标注' }));

    await waitFor(() => {
      expect(api.updateItemAnnotation).toHaveBeenCalledWith('text_1', '');
    });
    expect(screen.queryByText('Alpha token')).not.toBeInTheDocument();
    expect(screen.getByText('未找到匹配的记录')).toBeInTheDocument();
  });

  it('loads and saves app settings from the settings panel', async () => {
    render(App);

    await waitFor(() => expect(api.getSettings).toHaveBeenCalledTimes(1));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));

    expect(screen.getByRole('dialog', { name: '设置' })).toBeInTheDocument();
    expect(screen.getByRole('tablist', { name: '设置分类' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: '基础' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('heading', { name: '基础设置' })).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: '监听剪贴板' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: '启动时显示主窗口' })).toBeChecked();

    const maxItems = screen.getByLabelText('保留记录数');
    await fireEvent.input(maxItems, { target: { value: '120' } });
    await fireEvent.click(screen.getByRole('checkbox', { name: '启动时显示主窗口' }));

    await fireEvent.click(screen.getByRole('tab', { name: '日期语言' }));
    expect(screen.getByRole('heading', { name: '界面与日期' })).toBeInTheDocument();
    expect(screen.getByLabelText('日期划分时区')).toHaveValue('Asia/Shanghai');
    expect(screen.getByLabelText('应用语言')).toHaveValue('zh-CN');
    await fireEvent.change(screen.getByLabelText('日期划分时区'), {
      target: { value: 'America/New_York' },
    });
    await fireEvent.change(screen.getByLabelText('应用语言'), {
      target: { value: 'en-US' },
    });

    await fireEvent.click(screen.getByRole('tab', { name: '关于' }));
    expect(screen.getByRole('img', { name: 's1oopX GitHub 头像' })).toHaveAttribute(
      'src',
      '/github-avatar.jpg'
    );
    expect(screen.getByRole('heading', { name: 's1oopX' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '项目简介' })).toBeInTheDocument();
    expect(screen.getByText(/轻巧的本地剪贴板工具/)).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '联系方式' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /GitHub 主页/ })).toHaveAttribute(
      'href',
      'https://github.com/s1oopX'
    );
    expect(screen.getByRole('link', { name: /提交问题或建议/ })).toHaveAttribute(
      'href',
      'https://github.com/s1oopX/clipmaster-tauri/issues'
    );
    expect(screen.getByText('本地保存')).toBeInTheDocument();
    expect(screen.getByText('纽约（自动夏令时）')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '保存设置' }));

    await waitFor(() => {
      expect(api.saveSettings).toHaveBeenCalledWith({
        clipboard_monitor_enabled: true,
        show_main_window_on_start: false,
        max_items: 120,
        capture_delay_ms: 150,
        screenshot_hotkey: 'CommandOrControl+Shift+A',
        time_zone: 'America/New_York',
        language: 'en-US',
        auto_cleanup_enabled: false,
        cleanup_max_items: 200,
        cleanup_keep_days: 30,
      });
    });
  });

  it('keeps the active search after saving settings', async () => {
    api.searchItems.mockResolvedValue([textItem()]);

    render(App);

    const search = await screen.findByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenCalledWith('alpha', 'session_1', 50, todayDateKey());
    });

    await fireEvent.click(screen.getByRole('button', { name: '设置' }));

    const maxItems = screen.getByLabelText('保留记录数');
    await fireEvent.input(maxItems, { target: { value: '80' } });
    await fireEvent.click(screen.getByRole('button', { name: '保存设置' }));

    await waitFor(() => {
      expect(api.saveSettings).toHaveBeenCalledWith(expect.objectContaining({ max_items: 80 }));
    });

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenLastCalledWith(
        'alpha',
        'session_1',
        80,
        todayDateKey()
      );
    });
    expect(api.getItems).not.toHaveBeenCalled();
  });

  it('previews and runs custom cleanup from the settings panel', async () => {
    api.previewCustomCleanup.mockResolvedValue({
      item_count: 3,
      text_count: 2,
      image_count: 1,
      oldest_timestamp: 1780640000000,
      newest_timestamp: 1780650000000,
    });
    api.runCustomCleanup.mockResolvedValue({
      item_count: 3,
      text_count: 2,
      image_count: 1,
      oldest_timestamp: 1780640000000,
      newest_timestamp: 1780650000000,
    });

    render(App);

    await waitFor(() => expect(api.getSettings).toHaveBeenCalledTimes(1));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));
    await fireEvent.click(screen.getByRole('tab', { name: '清理' }));

    const maxItems = screen.getByLabelText('普通记录最多保留');
    const keepDays = screen.getByLabelText('普通记录保留天数');
    await fireEvent.input(maxItems, { target: { value: '80' } });
    await fireEvent.input(keepDays, { target: { value: '7' } });

    await fireEvent.click(screen.getByRole('button', { name: '预览清理' }));

    await waitFor(() => {
      expect(api.previewCustomCleanup).toHaveBeenCalledWith(80, 7);
    });
    expect(screen.getByRole('status')).toHaveTextContent('将清理 3 条记录');

    await fireEvent.click(screen.getByRole('button', { name: '立即清理' }));

    await waitFor(() => {
      expect(api.runCustomCleanup).toHaveBeenCalledWith(80, 7);
    });
    expect(api.getItems).toHaveBeenCalledTimes(2);
  });

  it('loads records by precisely selected calendar day', async () => {
    api.getItemsByDay.mockResolvedValue([imageItem()]);

    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
    await fireEvent.click(screen.getByRole('button', { name: '06-06 · 2' }));

    await waitFor(() => {
      expect(api.getItemsByDay).toHaveBeenCalledWith('2026-06-06', 50, 0);
    });
    expect(await screen.findByText('图片记录')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '清除日期筛选' }));
    await waitFor(() => {
      expect(api.getItems).toHaveBeenCalledTimes(2);
    });
  });

  it('keeps live clipboard events within the configured item limit', async () => {
    let newItemHandler;
    api.getSettings.mockResolvedValue({
      clipboard_monitor_enabled: true,
      show_main_window_on_start: true,
      max_items: 1,
      capture_delay_ms: 150,
      screenshot_hotkey: 'CommandOrControl+Shift+A',
      time_zone: 'Asia/Shanghai',
      language: 'zh-CN',
      auto_cleanup_enabled: false,
      cleanup_max_items: 200,
      cleanup_keep_days: 30,
    });
    api.getItems.mockResolvedValue([
      textItem({ id: 'old_item', content: 'Old token', preview: 'Old token' }),
    ]);
    api.onNewItem.mockImplementation(async (handler) => {
      newItemHandler = handler;
      return vi.fn();
    });

    render(App);

    expect(await screen.findByText('Old token')).toBeInTheDocument();
    await waitFor(() => expect(api.onNewItem).toHaveBeenCalledTimes(1));

    await newItemHandler(
      textItem({
        id: 'new_item',
        content: 'New token',
        preview: 'New token',
        timestamp: Date.now() + 1000,
      })
    );

    expect(await screen.findByText('New token')).toBeInTheDocument();
    expect(screen.queryByText('Old token')).not.toBeInTheDocument();
  });

  it('replaces refreshed live clipboard records instead of duplicating them', async () => {
    let newItemHandler;
    api.getItems.mockResolvedValue([
      textItem({
        id: 'same_item',
        content: 'Old token',
        preview: 'Old token',
        timestamp: 1000,
      }),
    ]);
    api.onNewItem.mockImplementation(async (handler) => {
      newItemHandler = handler;
      return vi.fn();
    });

    render(App);

    expect(await screen.findByText('Old token')).toBeInTheDocument();
    await waitFor(() => expect(api.onNewItem).toHaveBeenCalledTimes(1));

    await newItemHandler(
      textItem({
        id: 'same_item',
        content: 'New token',
        preview: 'New token',
        timestamp: 2000,
      })
    );

    expect(await screen.findByText('New token')).toBeInTheDocument();
    expect(screen.queryByText('Old token')).not.toBeInTheDocument();
  });

  it('keeps live clipboard events from polluting active search results', async () => {
    let newItemHandler;
    api.getItems.mockResolvedValue([
      textItem({ id: 'alpha_item', content: 'Alpha token', preview: 'Alpha token' }),
    ]);
    api.searchItems.mockResolvedValue([
      textItem({ id: 'alpha_item', content: 'Alpha token', preview: 'Alpha token' }),
    ]);
    api.onNewItem.mockImplementation(async (handler) => {
      newItemHandler = handler;
      return vi.fn();
    });

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    const search = screen.getByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenCalledWith('alpha', 'session_1', 50, todayDateKey());
    });

    await newItemHandler(
      textItem({
        id: 'beta_item',
        content: 'Beta token',
        preview: 'Beta token',
      })
    );

    expect(screen.getByText('Alpha token')).toBeInTheDocument();
    expect(screen.queryByText('Beta token')).not.toBeInTheDocument();
  });
});
