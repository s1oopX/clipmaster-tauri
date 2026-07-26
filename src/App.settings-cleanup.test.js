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
  it('loads and saves app settings from the settings panel', async () => {
    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));

    expect(screen.getByRole('dialog', { name: '设置' })).toBeInTheDocument();
    expect(screen.getByRole('tablist', { name: '设置分类' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: '常规' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.queryByRole('tab', { name: '清理' })).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '常规设置' })).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: '监听剪贴板' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: '启动时显示主窗口' })).toBeChecked();

    const maxItems = screen.getByLabelText('保留记录数');
    await fireEvent.input(maxItems, { target: { value: '120' } });
    await fireEvent.input(screen.getByLabelText('截图延迟'), { target: { value: '0' } });
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
      '/github-avatar-display.jpg'
    );
    expect(screen.getByRole('heading', { name: 's1oopX' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '项目简介' })).toBeInTheDocument();
    expect(screen.getByText(/轻巧的本地剪贴板工具/)).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '联系方式' })).toBeInTheDocument();
    const profileLink = screen.getByRole('link', { name: /GitHub 主页/ });
    const issuesLink = screen.getByRole('link', { name: /提交问题或建议/ });
    expect(profileLink).toHaveAttribute(
      'href',
      'https://github.com/s1oopX'
    );
    expect(issuesLink).toHaveAttribute(
      'href',
      'https://github.com/s1oopX/clipmaster-tauri/issues'
    );
    await fireEvent.click(profileLink);
    await fireEvent.click(issuesLink);
    expect(api.openExternalUrl).toHaveBeenCalledWith('https://github.com/s1oopX');
    expect(api.openExternalUrl).toHaveBeenCalledWith(
      'https://github.com/s1oopX/clipmaster-tauri/issues'
    );
    expect(screen.getByText('本地保存')).toBeInTheDocument();
    expect(
      screen.getByText('C:\\Users\\tester\\AppData\\Roaming\\com.clipmaster.desktop')
    ).toBeInTheDocument();
    expect(screen.getByText('纽约（自动夏令时）')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '保存设置' }));

    await waitFor(() => {
      expect(api.saveSettings).toHaveBeenCalledWith({
        clipboard_monitor_enabled: true,
        show_main_window_on_start: false,
        auto_start_enabled: false,
        max_items: 120,
        capture_delay_ms: 0,
        screenshot_hotkey: 'CommandOrControl+Shift+A',
        main_window_hotkey: 'CommandOrControl+Shift+Space',
        time_zone: 'America/New_York',
        language: 'en-US',
        auto_cleanup_enabled: false,
        cleanup_max_items: 200,
        cleanup_keep_days: 30,
        dev_server_port: 5174,
      });
    });
  });

  it('checks custom dev ports, applies a suggestion, and offers app restart after saving', async () => {
    api.checkDevServerPort.mockResolvedValueOnce({
      port: 5175,
      available: false,
      suggested_port: 5176,
      message: '端口 5175 已被占用，可切换到 5176',
    });
    api.saveSettings.mockImplementationOnce(async (settings) => ({
      ...settings,
      dev_server_port: 5176,
    }));

    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));
    await fireEvent.click(screen.getByRole('tab', { name: '高级' }));

    const portInput = screen.getByLabelText('开发端口');
    await fireEvent.input(portInput, { target: { value: '5175' } });
    await fireEvent.click(screen.getByRole('button', { name: '检查端口' }));

    await waitFor(() => {
      expect(api.checkDevServerPort).toHaveBeenCalledWith(5175);
    });
    expect(screen.getByText('端口 5175 已被占用，可切换到 5176')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '使用 5176' }));
    expect(portInput).toHaveValue(5176);
    expect(screen.getByText('端口 5176 可用')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '保存设置' }));

    await waitFor(() => {
      expect(api.saveSettings).toHaveBeenCalledWith(
        expect.objectContaining({ dev_server_port: 5176 })
      );
    });
    expect(screen.getByRole('dialog', { name: '设置' })).toBeInTheDocument();
    expect(screen.getByText('端口 5176 已保存')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '重启应用' }));
    expect(api.restartApp).toHaveBeenCalledTimes(1);
  });

  it('keeps the active search after saving settings', async () => {
    api.searchItems.mockResolvedValue([textItem()]);

    render(App);

    const search = await screen.findByRole('searchbox', { name: '搜索剪贴板内容' });
    await fireEvent.input(search, { target: { value: 'alpha' } });

    await waitFor(() => {
      expect(api.searchItems).toHaveBeenCalledWith('alpha', null, 50, null, 0);
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
        null,
        80,
        null,
        0
      );
    });
    expect(api.getItems).not.toHaveBeenCalled();
  });

  it('shows an error when an about link cannot be opened', async () => {
    api.openExternalUrl.mockRejectedValueOnce('系统浏览器不可用');

    render(App);

    await waitFor(() => expect(api.getSettings).toHaveBeenCalledTimes(1));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));
    await fireEvent.click(screen.getByRole('tab', { name: '关于' }));
    await fireEvent.click(screen.getByRole('link', { name: /GitHub 主页/ }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('打开链接失败: 系统浏览器不可用');
  });

  it('shows settings save failures as floating errors and keeps the dialog open', async () => {
    api.saveSettings.mockRejectedValueOnce('写入失败');

    render(App);

    await waitFor(() => expect(api.getItems).toHaveBeenCalledWith(50, 0));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));
    await fireEvent.click(screen.getByRole('button', { name: '保存设置' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('保存设置失败: 写入失败');
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);
    expect(screen.getByRole('dialog', { name: '设置' })).toBeInTheDocument();
    expect(document.querySelector('.notice.error')).toBeNull();
  });

  it('keeps the hotkey settings open when the screenshot shortcut is rejected', async () => {
    api.saveSettings.mockRejectedValueOnce('截图快捷键格式无效，请重新录制快捷键');

    render(App);

    await waitFor(() => expect(api.getSettings).toHaveBeenCalledTimes(1));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));
    await fireEvent.click(screen.getByRole('button', { name: '保存设置' }));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('保存设置失败: 截图快捷键格式无效，请重新录制快捷键');
    expect(screen.getByRole('heading', { name: '常规设置' })).toBeInTheDocument();
    expect(screen.getByRole('dialog', { name: '设置' })).toBeInTheDocument();
  });

  it('keeps the shortcut value stable while recording feedback changes', async () => {
    render(App);

    await waitFor(() => expect(api.getSettings).toHaveBeenCalledTimes(1));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));

    const shortcut = screen.getByLabelText('截图');
    const mainWindowShortcut = screen.getByLabelText('主窗口');
    expect(shortcut).toHaveValue('CommandOrControl+Shift+A');
    expect(mainWindowShortcut).toHaveValue('CommandOrControl+Shift+Space');

    await fireEvent.focus(shortcut);
    expect(shortcut).toHaveValue('CommandOrControl+Shift+A');
    expect(screen.getByText(/正在录制/)).toBeInTheDocument();

    await fireEvent.keyDown(shortcut, { key: 'A' });
    expect(shortcut).toHaveValue('CommandOrControl+Shift+A');
    expect(screen.getByText(/请使用修饰键组合/)).toBeInTheDocument();

    await fireEvent.blur(shortcut);
    expect(shortcut).toHaveValue('CommandOrControl+Shift+A');
    expect(screen.getByText(/点击输入框后按下组合键自动录制/)).toBeInTheDocument();

    await fireEvent.focus(shortcut);
    await fireEvent.keyDown(shortcut, {
      key: 'k',
      ctrlKey: true,
      shiftKey: true,
    });

    expect(shortcut).toHaveValue('CommandOrControl+Shift+K');
    expect(screen.getByText(/点击输入框后按下组合键自动录制/)).toBeInTheDocument();

    await fireEvent.focus(mainWindowShortcut);
    await fireEvent.keyDown(mainWindowShortcut, {
      key: ' ',
      ctrlKey: true,
      altKey: true,
    });

    expect(mainWindowShortcut).toHaveValue('CommandOrControl+Alt+Space');
  });

  it('reports auto cleanup failures separately after settings are saved', async () => {
    api.runCustomCleanup.mockRejectedValueOnce('清理执行失败');

    render(App);

    await waitFor(() => expect(api.getSettings).toHaveBeenCalledTimes(1));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));
    await fireEvent.click(screen.getByRole('tab', { name: '高级' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: '保存设置后自动清理' }));
    await fireEvent.click(screen.getByRole('button', { name: '保存设置' }));

    await waitFor(() => {
      expect(api.saveSettings).toHaveBeenCalledWith(
        expect.objectContaining({ auto_cleanup_enabled: true })
      );
      expect(api.runCustomCleanup).toHaveBeenCalledWith(200, 30);
    });

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('设置已保存，自动清理失败: 清理执行失败');
    expect(screen.queryByRole('dialog', { name: '设置' })).not.toBeInTheDocument();
    expect(screen.queryByText('保存设置失败')).not.toBeInTheDocument();
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
    await fireEvent.click(screen.getByRole('tab', { name: '高级' }));

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

  it('shows cleanup failures as floating errors without adding a global notice', async () => {
    api.previewCustomCleanup.mockRejectedValueOnce('清理预览失败');
    api.runCustomCleanup.mockRejectedValueOnce('清理执行失败');

    render(App);

    await waitFor(() => expect(api.getSettings).toHaveBeenCalledTimes(1));
    await fireEvent.click(screen.getByRole('button', { name: '设置' }));
    await fireEvent.click(screen.getByRole('tab', { name: '高级' }));

    await fireEvent.click(screen.getByRole('button', { name: '预览清理' }));
    let alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('预览清理失败: 清理预览失败');
    expect(screen.getByTestId('toast-stack')).toContainElement(alert);
    expect(document.querySelector('.notice.error')).toBeNull();

    await fireEvent.click(screen.getByRole('button', { name: '立即清理' }));
    await waitFor(() => {
      alert = screen.getByRole('alert');
      expect(alert).toHaveTextContent('执行清理失败: 清理执行失败');
      expect(screen.getByTestId('toast-stack')).toContainElement(alert);
    });
    expect(document.querySelector('.notice.error')).toBeNull();
  });

  it('requires confirmation before clearing all history and refreshes the visible records', async () => {
    api.getItems
      .mockResolvedValueOnce([textItem(), imageItem({ is_favorite: false })])
      .mockResolvedValue([]);
    api.clearAllHistory.mockResolvedValue({
      item_count: 2,
      text_count: 1,
      image_count: 1,
      oldest_timestamp: 1780640000000,
      newest_timestamp: 1780650000000,
    });

    render(App);

    expect(await screen.findByText('Alpha token')).toBeInTheDocument();
    expect(await screen.findByText('图片记录')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: '设置' }));
    await fireEvent.click(screen.getByRole('tab', { name: '高级' }));
    await fireEvent.click(screen.getByRole('button', { name: '清空全部历史' }));

    expect(api.clearAllHistory).not.toHaveBeenCalled();
    const confirmDialog = screen.getByRole('dialog', { name: '确认清空历史' });
    expect(confirmDialog).toHaveTextContent('所有剪贴板记录、收藏、置顶、标注、图片原图和缩略图都会被删除。');

    await fireEvent.click(within(confirmDialog).getByRole('button', { name: '确认清空' }));

    await waitFor(() => {
      expect(api.clearAllHistory).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(api.getItems).toHaveBeenCalledTimes(2);
    });
    expect(screen.queryByRole('dialog', { name: '确认清空历史' })).not.toBeInTheDocument();
    expect(screen.queryByText('Alpha token')).not.toBeInTheDocument();
    expect(screen.queryByText('图片记录')).not.toBeInTheDocument();
    expect(await screen.findByText('已清空 2 条记录')).toBeInTheDocument();
  });

});
