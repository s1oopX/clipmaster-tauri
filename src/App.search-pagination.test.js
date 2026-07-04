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
  it('searches within the active day and can clear the query', async () => {
    render(App);

    const search = await screen.findByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenCalledWith('alpha', null, 50, todayDateKey(), 0);
      expect(screen.getByLabelText('当前范围')).toHaveTextContent(`${todayDateKey()} · 已加载 0 条`);
    });

    await fireEvent.click(screen.getByRole('button', { name: '清除搜索' }));

    expect(search).toHaveValue('');
    await waitFor(() => {
      expect(api.getItems).toHaveBeenCalledWith(50, 0);
      expect(screen.getByLabelText('当前范围')).toHaveTextContent('全部日期 · 已加载 0 条');
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
        null,
        50,
        '2026-06-06',
        0
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
      expect(api.searchItems).toHaveBeenCalledWith('alpha', null, 50, todayDateKey(), 0);
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

  it('loads more records with backend pagination and appends without duplicates', async () => {
    api.getItems
      .mockResolvedValueOnce(Array.from({ length: 50 }, (_, index) =>
        textItem({
          id: `page_1_${index}`,
          content: `Page 1 item ${index}`,
          preview: `Page 1 item ${index}`,
          timestamp: 2000 - index,
        })
      ))
      .mockResolvedValueOnce([
        textItem({
          id: 'page_1_0',
          content: 'Page 1 duplicate',
          preview: 'Page 1 duplicate',
          timestamp: 2000,
        }),
        textItem({
          id: 'page_2_0',
          content: 'Page 2 item',
          preview: 'Page 2 item',
          timestamp: 1000,
        }),
      ]);

    render(App);

    expect(await screen.findByText('Page 1 item 0')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '加载更多' })).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '加载更多' }));

    await waitFor(() => {
      expect(api.getItems).toHaveBeenLastCalledWith(50, 50);
    });
    expect(await screen.findByText('Page 2 item')).toBeInTheDocument();
    expect(screen.getByLabelText('当前范围')).toHaveTextContent('全部日期 · 已加载 51 条');
  });

  it('loads more search results with an offset for the active search query', async () => {
    api.searchItems
      .mockResolvedValueOnce(Array.from({ length: 50 }, (_, index) =>
        textItem({
          id: `search_1_${index}`,
          content: `Alpha page 1 item ${index}`,
          preview: `Alpha page 1 item ${index}`,
          timestamp: 2000 - index,
        })
      ))
      .mockResolvedValueOnce([
        textItem({
          id: 'search_2_0',
          content: 'Alpha page 2 item',
          preview: 'Alpha page 2 item',
          timestamp: 1000,
        }),
      ]);

    render(App);

    const search = await screen.findByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    expect(await screen.findByText('Alpha page 1 item 0')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: '加载更多' }));

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenLastCalledWith(
        'alpha',
        null,
        50,
        todayDateKey(),
        50
      );
    });
    expect(await screen.findByText('Alpha page 2 item')).toBeInTheDocument();
  });

  it('passes active sidebar filters to backend list and search calls', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));

    await fireEvent.click(screen.getByRole('button', { name: '收藏' }));
    await waitFor(() => {
      expect(api.getItems).toHaveBeenLastCalledWith(50, 0, { favoriteOnly: true });
    });

    await fireEvent.click(screen.getByRole('button', { name: '图片' }));
    await waitFor(() => {
      expect(api.getItems).toHaveBeenLastCalledWith(50, 0, { itemType: 'image' });
    });

    await fireEvent.click(screen.getByRole('button', { name: '链接' }));
    await waitFor(() => {
      expect(api.getItems).toHaveBeenLastCalledWith(50, 0, { itemType: 'link' });
    });

    const search = screen.getByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });
    await waitFor(() => {
      expect(api.searchItems).toHaveBeenLastCalledWith(
        'alpha',
        null,
        50,
        todayDateKey(),
        0,
        { itemType: 'link' }
      );
    });
  });

  it('keeps the narrow-window shell compact without page-level scrolling', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));

    expect(screen.getByTestId('app-shell')).toHaveAttribute('data-layout', 'compact-ready');
    expect(screen.getByTestId('app-shell')).toHaveAttribute('data-density', 'tool');
    expect(screen.getByTestId('app-shell')).toHaveAttribute(
      'data-reference',
      'figma-utility-grid'
    );
    expect(screen.getByTestId('history-panel')).toHaveAttribute('data-scroll', 'internal');
    expect(screen.getByRole('button', { name: '全部记录' })).toHaveClass('filter-button');
    expect(screen.getByRole('button', { name: '收藏' })).toHaveClass('filter-button');
    expect(screen.getByRole('button', { name: '图片' })).toHaveClass('filter-button');
    expect(document.querySelector('.filter-label-short')).toBeInTheDocument();
    expect(document.querySelector('.filter-button[title]')).not.toBeInTheDocument();
    expect(document.body).toContainElement(screen.getByTestId('app-shell'));
  });

  it('uses plain text month navigation in the date picker without special arrow glyphs', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));

    const picker = document.querySelector('.clipmaster-date-picker');
    const previousMonth = picker?.querySelector('.flatpickr-prev-month');
    const nextMonth = picker?.querySelector('.flatpickr-next-month');

    expect(previousMonth).toHaveTextContent('上月');
    expect(previousMonth).toHaveAttribute('aria-label', '上个月');
    expect(previousMonth).not.toHaveTextContent('‹');
    expect(nextMonth).toHaveTextContent('下月');
    expect(nextMonth).toHaveAttribute('aria-label', '下个月');
    expect(nextMonth).not.toHaveTextContent('›');
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
      expect(api.searchItems).toHaveBeenCalledWith('alpha', null, 50, todayDateKey(), 0);
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });
  });

  it('shows search failures as floating errors without shifting the history layout', async () => {
    api.searchItems.mockRejectedValueOnce('索引暂不可用');

    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));

    const search = screen.getByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('搜索失败: 索引暂不可用');
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);
    expect(document.querySelector('.notice.error')).toBeNull();
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

});
