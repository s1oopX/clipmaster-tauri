import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import {
  api,
  deferred,
  imageItem,
  linkItem,
  textItem,
  todayDateKey,
} from './test/app-test-utils.js';
import App from './App.svelte';
describe('App UI', () => {
it('renders the desktop app shell with search, filters, and an empty history state', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));

    expect(screen.getByRole('heading', { name: 'ClipMaster' })).toBeInTheDocument();
    expect(screen.getByRole('searchbox', { name: '搜索剪贴板内容' })).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: '剪贴板筛选' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '全部记录' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '收藏' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '图片' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '暂停' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '截图' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '钉住' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '设置' })).toBeInTheDocument();
    expect(screen.getByText('剪贴板历史')).toBeInTheDocument();
    expect(screen.getByText('复制内容后会自动出现在这里')).toBeInTheDocument();
  });

  it('pauses and resumes clipboard monitoring from the toolbar', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));

    await fireEvent.click(screen.getByRole('button', { name: '暂停' }));

    await waitFor(() => {
      expect(api.saveSettings).toHaveBeenCalledWith(
        expect.objectContaining({ clipboard_monitor_enabled: false })
      );
    });
    expect(screen.getByRole('button', { name: '恢复' })).toBeInTheDocument();
    expect(screen.getByText('已暂停剪贴板记录')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '恢复' }));

    await waitFor(() => {
      expect(api.saveSettings).toHaveBeenLastCalledWith(
        expect.objectContaining({ clipboard_monitor_enabled: true })
      );
    });
    expect(screen.getByRole('button', { name: '暂停' })).toBeInTheDocument();
  });

  it('cleans up global event listeners when the app shell unmounts', async () => {
    const unlistenScreenshotHotkey = vi.fn();
    const unlistenSearchHotkey = vi.fn();
    const unlistenNewItem = vi.fn();
    api.listen
      .mockResolvedValueOnce(unlistenScreenshotHotkey)
      .mockResolvedValueOnce(unlistenSearchHotkey);
    api.onNewItem.mockResolvedValueOnce(unlistenNewItem);

    const { unmount } = render(App);

    await waitFor(() => {
      expect(api.listen).toHaveBeenCalledWith('hotkey:screenshot', expect.any(Function));
      expect(api.listen).toHaveBeenCalledWith('hotkey:focus-search', expect.any(Function));
      expect(api.onNewItem).toHaveBeenCalledTimes(1);
    });

    unmount();

    expect(unlistenScreenshotHotkey).toHaveBeenCalledTimes(1);
    expect(unlistenSearchHotkey).toHaveBeenCalledTimes(1);
    expect(unlistenNewItem).toHaveBeenCalledTimes(1);
  });

  it('focuses the search box when the main window hotkey event fires', async () => {
    let focusSearchHandler;
    api.listen.mockImplementation(async (event, handler) => {
      if (event === 'hotkey:focus-search') {
        focusSearchHandler = handler;
      }
      return vi.fn();
    });

    render(App);

    const search = await screen.findByRole('searchbox', { name: '搜索剪贴板内容' });
    await waitFor(() => expect(focusSearchHandler).toEqual(expect.any(Function)));
    await fireEvent.input(search, { target: { value: 'alpha' } });
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));
    expect(screen.getByRole('dialog', { name: '设置' })).toBeInTheDocument();

    focusSearchHandler();

    await waitFor(() => {
      expect(screen.queryByRole('dialog', { name: '设置' })).not.toBeInTheDocument();
      expect(search).toHaveFocus();
    });
    expect(search.selectionStart).toBe(0);
    expect(search.selectionEnd).toBe('alpha'.length);
  });

  it('loads persisted history again when the app shell is mounted after a restart', async () => {
    api.getItems
      .mockResolvedValueOnce([
        textItem({
          id: 'before_restart',
          content: 'Before restart token',
          preview: 'Before restart token',
        }),
      ])
      .mockResolvedValueOnce([
        textItem({
          id: 'after_restart',
          content: 'Persisted after restart',
          preview: 'Persisted after restart',
        }),
      ]);

    const firstRun = render(App);

    expect(await screen.findByText('Before restart token')).toBeInTheDocument();
    firstRun.unmount();

    render(App);

    expect(await screen.findByText('Persisted after restart')).toBeInTheDocument();
    expect(screen.queryByText('Before restart token')).not.toBeInTheDocument();
    expect(api.getItems).toHaveBeenCalledTimes(2);
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

  it('renders link records with hints and opens them from the default browser path', async () => {
    const link = linkItem();
    api.getItems.mockResolvedValue([link]);

    render(App);

    const linkContent = await screen.findByRole('link', { name: /example.com\/docs/ });
    const linkRecord = linkContent.closest('.item');
    expect(within(linkRecord).getByText('链接')).toBeInTheDocument();
    expect(
      screen.getByText('Ctrl/Command+左键打开，Enter 直接打开。复制按钮会复制原始链接。')
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '打开 https://example.com/docs?q=clipmaster' }))
      .toBeInTheDocument();

    await fireEvent.click(linkContent);
    expect(api.openExternalUrl).not.toHaveBeenCalled();

    await fireEvent.click(linkContent, { ctrlKey: true });
    expect(api.openExternalUrl).toHaveBeenCalledWith(link.content);

    await fireEvent.keyDown(linkContent, { key: 'Enter' });
    expect(api.openExternalUrl).toHaveBeenLastCalledWith(link.content);

    await fireEvent.click(screen.getByRole('button', { name: '打开 https://example.com/docs?q=clipmaster' }));
    expect(api.openExternalUrl).toHaveBeenLastCalledWith(link.content);

    await fireEvent.click(screen.getByRole('button', { name: '复制 https://example.com/docs?q=clipmaster' }));
    expect(api.copyToClipboard).toHaveBeenCalledWith(link.content);
  });

});
