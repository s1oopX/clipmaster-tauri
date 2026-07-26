import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import {
  api,
  deferred,
  imageItem,
  linkItem,
  textItem,
} from './test/app-test-utils.js';
import App from './App.svelte';
describe('App UI', () => {
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
      main_window_hotkey: 'CommandOrControl+Shift+Space',
      time_zone: 'Asia/Shanghai',
      language: 'zh-CN',
      auto_cleanup_enabled: false,
      cleanup_max_items: 200,
      cleanup_keep_days: 30,
      dev_server_port: 5174,
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

  it('closes item-bound confirmation when live records replace the item', async () => {
    let newItemHandler;
    api.getSettings.mockResolvedValue({
      clipboard_monitor_enabled: true,
      show_main_window_on_start: true,
      max_items: 1,
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
    api.getItems.mockResolvedValue([
      textItem({ id: 'old_item', content: 'Old token', preview: 'Old token', is_favorite: true }),
    ]);
    api.onNewItem.mockImplementation(async (handler) => {
      newItemHandler = handler;
      return vi.fn();
    });

    render(App);

    expect(await screen.findByText('Old token')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: '删除 Old token' }));
    expect(screen.getByRole('dialog', { name: '确认删除' })).toBeInTheDocument();

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
    expect(screen.queryByRole('dialog', { name: '确认删除' })).not.toBeInTheDocument();
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
      expect(api.searchItems).toHaveBeenCalledWith('alpha', null, 50, null, 0);
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

    await newItemHandler(
      textItem({
        id: 'old_alpha_item',
        content: 'Alpha from another day',
        preview: 'Alpha from another day',
        date_key: '2026-06-05',
      })
    );

    expect(screen.getByText('Alpha token')).toBeInTheDocument();
    // 搜索现在覆盖全部日期：其他日期的匹配项应实时加入结果
    expect(screen.getByText('Alpha from another day')).toBeInTheDocument();
  });

});
