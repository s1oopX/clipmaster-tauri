import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const screenshotHtml = readFileSync('screenshot.html', 'utf8');
const commandsSource = readFileSync('src-tauri/src/commands.rs', 'utf8');
const mainSource = readFileSync('src-tauri/src/main.rs', 'utf8');
const capabilities = JSON.parse(
  readFileSync('src-tauri/capabilities/default.json', 'utf8')
);

describe('Screenshot selector window', () => {
  it('does not trap the fullscreen selector when window recovery fails', () => {
    expect(screenshotHtml).toContain('runWindowAction');
    expect(screenshotHtml).toContain("cleanup_screenshot_snapshot");
    expect(screenshotHtml).toContain("mainWindow.show(), '恢复主窗口显示'");
    expect(screenshotHtml).toContain("mainWindow.setFocus(), '恢复主窗口焦点'");
    expect(screenshotHtml).toContain("console.warn('主窗口恢复失败，继续关闭截图窗口')");
    expect(screenshotHtml).toContain("currentWin.close(), '关闭截图窗口'");
    expect(screenshotHtml).toContain("currentWin.destroy(), '强制关闭截图窗口'");
    expect(screenshotHtml).not.toContain("showError('无法恢复主窗口，请从任务栏重新打开 ClipMaster')");
  });

  it('captures a frozen screen before opening the selector and saves the final PNG', () => {
    expect(commandsSource).toContain('prepare_main_window_for_screenshot(&app).await?');
    expect(commandsSource).toContain('隐藏主窗口失败，无法安全截图');
    expect(commandsSource).toContain('capture_frozen_screen_snapshot(&app)');
    expect(commandsSource).toContain('restoreMainWindow={}');
    expect(commandsSource).toContain('pub async fn save_screenshot_image');
    expect(commandsSource).toContain('copy_rgba_image_to_clipboard(app, rgba_image, &content_hash)?');
    expect(screenshotHtml).toContain("invoke('save_screenshot_image'");
    expect(screenshotHtml).toContain('renderFinalDataUrl');
  });

  it('restores the main window only when the screenshot start hid it', () => {
    expect(commandsSource).toContain('pub struct ScreenshotWindowState');
    expect(commandsSource).toContain('take_restore_main_window');
    expect(commandsSource).toContain('restore_main_window_after_screenshot');
    expect(mainSource).toContain('app.manage(commands::ScreenshotWindowState::default())');
    expect(mainSource).toContain('commands::restore_main_window_after_screenshot(app_handle)');
    expect(screenshotHtml).toContain("const shouldRestoreMainWindow = params.get('restoreMainWindow') !== '0'");
    expect(screenshotHtml).toContain('if (!shouldRestoreMainWindow)');
  });

  it('supports selection adjustment, annotation tools, and reselecting', () => {
    expect(screenshotHtml).toContain('id="rectTool"');
    expect(screenshotHtml).toContain('id="arrowTool"');
    expect(screenshotHtml).toContain('id="penTool"');
    expect(screenshotHtml).toContain('id="textTool"');
    expect(screenshotHtml).toContain('id="stepTool"');
    expect(screenshotHtml).toContain('id="blurTool"');
    expect(screenshotHtml).toContain('id="pixelateTool"');
    expect(screenshotHtml).toContain('id="eraserTool"');
    expect(screenshotHtml).toContain('id="undoBtn"');
    expect(screenshotHtml).toContain('id="redoBtn"');
    expect(screenshotHtml).toContain('id="reselectBtn"');
    expect(screenshotHtml).toContain("['nw', rect.x, rect.y]");
    expect(screenshotHtml).toContain('function selectionFromHandle');
    expect(screenshotHtml).toContain('function nudgeSelection');
    expect(screenshotHtml).toContain('function drawPrivacyMask');
    expect(screenshotHtml).toContain('function drawBlurredRegion');
    expect(screenshotHtml).toContain('function drawPixelatedRegion');
    expect(screenshotHtml).toContain('function drawTextLabel');
    expect(screenshotHtml).toContain('function drawStepMarker');
    expect(screenshotHtml).toContain('function beginTextAnnotation');
    expect(screenshotHtml).toContain('function eraseAnnotationAt');
    expect(screenshotHtml).toContain('function hitTestAnnotation');
    expect(screenshotHtml).toContain('function undoAnnotation');
    expect(screenshotHtml).toContain('function redoAnnotation');
    expect(screenshotHtml).toContain("event.key.toLowerCase() === 'z'");
    expect(screenshotHtml).toContain("event.key.toLowerCase() === 'y'");
    expect(screenshotHtml).toContain('ArrowLeft');
  });

  it('grants the window APIs required by screenshot capture recovery', () => {
    expect(capabilities.windows).toContain('screenshot-selector');
    expect(capabilities.permissions).toEqual(
      expect.arrayContaining([
        'core:window:allow-close',
        'core:window:allow-destroy',
        'core:window:allow-hide',
        'core:window:allow-set-focus',
        'core:window:allow-show',
      ])
    );
  });
});
