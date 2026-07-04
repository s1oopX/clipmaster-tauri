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

  it('clears stale action errors after successful favorite or pinned actions', async () => {
    api.getItems.mockResolvedValue([textItem()]);
    api.toggleFavorite.mockRejectedValueOnce('收藏失败').mockResolvedValueOnce(true);
    api.togglePinned.mockRejectedValueOnce('置顶失败').mockResolvedValueOnce(true);

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '收藏 Alpha token' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('切换收藏失败: 收藏失败');

    await fireEvent.click(screen.getByRole('button', { name: '收藏 Alpha token' }));
    await waitFor(() => {
      expect(api.toggleFavorite).toHaveBeenCalledTimes(2);
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });

    await fireEvent.click(screen.getByRole('button', { name: '置顶 Alpha token' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('切换置顶失败: 置顶失败');

    await fireEvent.click(screen.getByRole('button', { name: '置顶 Alpha token' }));
    await waitFor(() => {
      expect(api.togglePinned).toHaveBeenCalledTimes(2);
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });
  });

  it('updates favorite and pinned UI state after successful actions', async () => {
    api.getItems.mockResolvedValue([
      textItem({
        id: 'newer_text',
        content: 'Newer token',
        preview: 'Newer token',
        timestamp: 2000,
      }),
      textItem({
        id: 'older_text',
        content: 'Older token',
        preview: 'Older token',
        timestamp: 1000,
      }),
    ]);
    api.toggleFavorite.mockResolvedValueOnce(true);
    api.togglePinned.mockResolvedValueOnce(true);

    render(App);

    expect(await screen.findByText('Newer token')).toBeInTheDocument();
    expect(screen.getByText('Older token')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '收藏 Newer token' }));
    await waitFor(() => {
      expect(api.toggleFavorite).toHaveBeenCalledWith('newer_text');
      expect(screen.getByRole('button', { name: '收藏 Newer token' })).toHaveClass('active');
    });
    expect(screen.getByText('Newer token').closest('.item')).toHaveTextContent('收藏');

    await fireEvent.click(screen.getByRole('button', { name: '置顶 Older token' }));
    await waitFor(() => {
      expect(api.togglePinned).toHaveBeenCalledWith('older_text');
      expect(screen.getByRole('button', { name: '置顶 Older token' })).toHaveClass('active');
    });

    const renderedItems = Array.from(document.querySelectorAll('.item'));
    expect(renderedItems).toHaveLength(2);
    expect(renderedItems[0]).toHaveTextContent('Older token');
    expect(renderedItems[0]).toHaveTextContent('置顶');
    expect(renderedItems[1]).toHaveTextContent('Newer token');
  });

});
