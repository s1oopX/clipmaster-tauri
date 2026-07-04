import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, Window } from '@tauri-apps/api/window';

export async function runWindowAction(action, label) {
  try {
    await action();
    return true;
  } catch (error) {
    console.error(`${label}失败:`, error);
    return false;
  }
}

export async function restoreMainWindow(shouldRestoreMainWindow) {
  if (!shouldRestoreMainWindow) {
    return true;
  }

  try {
    const mainWindow = await Window.getByLabel('main');
    if (!mainWindow) {
      console.warn('未找到主窗口');
      return false;
    }

    const shown = await runWindowAction(() => mainWindow.show(), '恢复主窗口显示');
    const focused = await runWindowAction(() => mainWindow.setFocus(), '恢复主窗口焦点');
    return shown && focused;
  } catch (error) {
    console.error('恢复主窗口失败:', error);
    return false;
  }
}

export async function cleanupSnapshot(snapshotPath, snapshotCleaned, markSnapshotCleaned) {
  if (!snapshotPath || snapshotCleaned) return;
  markSnapshotCleaned();
  try {
    await invoke('cleanup_screenshot_snapshot', { snapshotPath });
  } catch (error) {
    console.warn('清理冻结截图失败:', error);
  }
}

export async function closeScreenshotWindow({
  shouldCleanup = true,
  snapshotPath,
  snapshotCleaned,
  markSnapshotCleaned,
  shouldRestoreMainWindow,
  onCloseFailed,
}) {
  const currentWin = getCurrentWindow();
  if (shouldCleanup) {
    await cleanupSnapshot(snapshotPath, snapshotCleaned, markSnapshotCleaned);
  }

  const restored = await restoreMainWindow(shouldRestoreMainWindow);
  if (!restored) {
    console.warn('主窗口恢复失败，继续关闭截图窗口');
  }

  const closed = await runWindowAction(() => currentWin.close(), '关闭截图窗口');
  if (closed) return;

  const destroyed = await runWindowAction(() => currentWin.destroy(), '强制关闭截图窗口');
  if (!destroyed) {
    onCloseFailed();
  }
}
