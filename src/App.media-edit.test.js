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

  it('opens and closes image previews with keyboard activation keys', async () => {
    api.getItems.mockResolvedValue([imageItem()]);

    render(App);

    const thumbnail = await screen.findByAltText('剪贴板图片缩略图');
    const previewButton = thumbnail.closest('.image-preview');

    await fireEvent.keyDown(previewButton, { key: ' ' });
    expect(screen.getByAltText('原图')).toBeInTheDocument();

    const overlay = document.querySelector('.image-viewer-overlay');
    await fireEvent.keyDown(overlay, { key: 'Enter' });

    await waitFor(() => {
      expect(screen.queryByAltText('原图')).not.toBeInTheDocument();
    });
  });

  it('falls back to the original image when thumbnail metadata is missing', async () => {
    api.getItems.mockResolvedValue([imageItem({ thumbnail_path: null })]);

    render(App);

    const thumbnail = await screen.findByAltText('剪贴板图片缩略图');

    expect(thumbnail).toHaveAttribute('src', 'asset://localhost/images/2026-06-06/image.png');
    expect(api.convertImagePath).toHaveBeenCalledWith('images/2026-06-06/image.png');
  });

  it('falls back to the original image when the thumbnail file cannot load', async () => {
    api.convertImagePath
      .mockResolvedValueOnce('asset://localhost/images/2026-06-06/image_thumb.png')
      .mockResolvedValueOnce('asset://localhost/images/2026-06-06/image.png');
    api.getItems.mockResolvedValue([imageItem()]);

    render(App);

    const thumbnail = await screen.findByAltText('剪贴板图片缩略图');
    await fireEvent.error(thumbnail);

    await waitFor(() => {
      expect(thumbnail).toHaveAttribute('src', 'asset://localhost/images/2026-06-06/image.png');
    });
    expect(api.convertImagePath).toHaveBeenCalledWith('images/2026-06-06/image.png');
    expect(screen.queryByText('图片预览不可用')).not.toBeInTheDocument();
  });

  it('falls back to the original image when thumbnail URL conversion returns empty', async () => {
    api.convertImagePath
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce('asset://localhost/images/2026-06-06/image.png');
    api.getItems.mockResolvedValue([imageItem()]);

    render(App);

    const thumbnail = await screen.findByAltText('剪贴板图片缩略图');

    expect(thumbnail).toHaveAttribute('src', 'asset://localhost/images/2026-06-06/image.png');
    expect(api.convertImagePath).toHaveBeenNthCalledWith(1, 'images/2026-06-06/image_thumb.png');
    expect(api.convertImagePath).toHaveBeenNthCalledWith(2, 'images/2026-06-06/image.png');
  });

  it('shows a stable unavailable state when image preview loading fails completely', async () => {
    api.convertImagePath
      .mockResolvedValueOnce('asset://localhost/images/2026-06-06/image_thumb.png')
      .mockRejectedValueOnce('原图不存在');
    api.getItems.mockResolvedValue([imageItem()]);

    render(App);

    const thumbnail = await screen.findByAltText('剪贴板图片缩略图');
    await fireEvent.error(thumbnail);

    await waitFor(() => {
      expect(screen.queryByAltText('剪贴板图片缩略图')).not.toBeInTheDocument();
    });
    expect(document.querySelector('.image-loading')).toHaveTextContent('图片预览不可用');
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

  it('copies text from the content area with keyboard activation keys', async () => {
    api.getItems.mockResolvedValue([textItem()]);

    render(App);

    const content = await screen.findByText('Alpha token');

    await fireEvent.keyDown(content, { key: ' ' });
    await waitFor(() => {
      expect(api.copyToClipboard).toHaveBeenCalledWith('Alpha token');
    });

    api.copyToClipboard.mockClear();
    await fireEvent.keyDown(content, { key: 'Enter' });

    await waitFor(() => {
      expect(api.copyToClipboard).toHaveBeenCalledWith('Alpha token');
    });
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
    // 标注不再联动收藏状态
    expect(screen.getByRole('button', { name: '收藏 Alpha token' })).not.toHaveClass('active');

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

  it('turns edited URL content into a link record in the current list', async () => {
    api.getItems.mockResolvedValue([textItem()]);

    render(App);

    const content = await screen.findByText('Alpha token');
    await fireEvent.contextMenu(content, { clientX: 120, clientY: 160 });
    await fireEvent.click(screen.getByRole('menuitem', { name: '编辑原文' }));

    const contentInput = screen.getByLabelText('编辑 Alpha token 的原文');
    await fireEvent.input(contentInput, { target: { value: ' https://example.com/docs ' } });
    await fireEvent.click(screen.getByRole('button', { name: '保存原文' }));

    await waitFor(() => {
      expect(api.updateItemContent).toHaveBeenCalledWith('text_1', ' https://example.com/docs ');
    });
    const linkContent = await screen.findByRole('link', { name: /example\.com\/docs/ });
    expect(linkContent).toBeInTheDocument();
    expect(within(linkContent.closest('.item')).getByText('链接')).toBeInTheDocument();

    await fireEvent.contextMenu(linkContent, {
      clientX: 140,
      clientY: 180,
    });
    await fireEvent.click(screen.getByRole('menuitem', { name: '打开链接' }));

    expect(api.openExternalUrl).toHaveBeenCalledWith('https://example.com/docs');
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

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
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

});
