use base64::{engine::general_purpose, Engine as _};
use screenshots::Screen;
use serde::Serialize;
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::time::sleep;

use crate::app_data;
use crate::clipboard::calculate_image_clipboard_hash;
use crate::database::{date_key_now, Database};
use crate::models::{ClipboardItem, ClipboardType, CreateClipboardItem};
use crate::session::SessionManager;
use crate::settings::SettingsStore;

use super::{encode_query_value, forget_next_clipboard_write, mark_next_clipboard_write};

#[derive(Debug, Clone, Serialize)]
struct FrozenScreenSnapshot {
    path: String,
    screen_x: i32,
    screen_y: i32,
    screen_width: u32,
    screen_height: u32,
    pixel_width: u32,
    pixel_height: u32,
    scale_factor: f32,
}

#[derive(Default)]
pub struct ScreenshotWindowState {
    restore_main_window: Mutex<bool>,
}

impl ScreenshotWindowState {
    fn set_restore_main_window(&self, value: bool) {
        if let Ok(mut restore_main_window) = self.restore_main_window.lock() {
            *restore_main_window = value;
        }
    }

    fn take_restore_main_window(&self) -> bool {
        let Ok(mut restore_main_window) = self.restore_main_window.lock() else {
            return false;
        };
        let value = *restore_main_window;
        *restore_main_window = false;
        value
    }
}

/// 开始区域截图
#[tauri::command]
pub async fn start_region_screenshot(
    app: AppHandle,
    settings: State<'_, SettingsStore>,
) -> Result<(), String> {
    // 启动时先冻结当前屏幕，后续选区和标注都基于静态图像合成。
    // 如果窗口已经存在，直接复用并置前，避免关闭后立即重建触发 Tauri label 冲突。
    if let Some(selection_window) = app.get_webview_window("screenshot-selector") {
        if let Err(error) = selection_window.show() {
            return Err(format!("打开已有截图选择窗口失败: {}", error));
        }

        if let Err(error) = selection_window.set_focus() {
            eprintln!("聚焦已有截图选择窗口失败: {}", error);
        }

        return Ok(());
    }

    let should_restore_main_window = prepare_main_window_for_screenshot(&app).await?;
    set_screenshot_restore_main_window(&app, should_restore_main_window);

    let delay_ms = settings.get().capture_delay_ms.max(0) as u64;
    let effective_delay_ms = if should_restore_main_window {
        delay_ms.max(180)
    } else {
        delay_ms
    };
    if effective_delay_ms > 0 {
        sleep(Duration::from_millis(effective_delay_ms)).await;
    }

    let snapshot = match capture_frozen_screen_snapshot(&app) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            restore_main_window_after_failed_screenshot_start(&app, should_restore_main_window);
            return Err(error);
        }
    };
    let url = WebviewUrl::App(
        format!(
            "screenshot.html?snapshotPath={}&screenX={}&screenY={}&screenWidth={}&screenHeight={}&pixelWidth={}&pixelHeight={}&scaleFactor={}&restoreMainWindow={}",
            encode_query_value(&snapshot.path),
            snapshot.screen_x,
            snapshot.screen_y,
            snapshot.screen_width,
            snapshot.screen_height,
            snapshot.pixel_width,
            snapshot.pixel_height,
            snapshot.scale_factor,
            if should_restore_main_window { 1 } else { 0 },
        )
        .into(),
    );

    let selection_window = WebviewWindowBuilder::new(&app, "screenshot-selector", url)
        .title("区域截图")
        .inner_size(snapshot.screen_width as f64, snapshot.screen_height as f64)
        .position(snapshot.screen_x as f64, snapshot.screen_y as f64)
        .decorations(false)
        .transparent(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
        .map_err(|e| {
            restore_main_window_after_failed_screenshot_start(&app, should_restore_main_window);
            e.to_string()
        })?;

    if let Err(error) = selection_window.show() {
        restore_main_window_after_failed_screenshot_start(&app, should_restore_main_window);
        let _ = selection_window.close();
        return Err(format!("打开截图选择窗口失败: {}", error));
    }

    if let Err(error) = selection_window.set_focus() {
        restore_main_window_after_failed_screenshot_start(&app, should_restore_main_window);
        let _ = selection_window.close();
        return Err(format!("聚焦截图选择窗口失败: {}", error));
    }

    Ok(())
}

/// 保存前端基于冻结屏幕合成的最终截图，并复制到系统剪贴板。
#[tauri::command]
pub async fn save_screenshot_image(
    app: AppHandle,
    db: State<'_, Database>,
    session_mgr: State<'_, SessionManager>,
    settings: State<'_, SettingsStore>,
    image_data_url: String,
    snapshot_path: Option<String>,
) -> Result<ClipboardItem, String> {
    let rgba_image = decode_png_data_url(&image_data_url)?;
    let result = save_screenshot_rgba_to_history(
        &app,
        &db,
        &session_mgr,
        &settings,
        &rgba_image,
        "ClipMaster 区域截图",
    )?;

    if let Some(snapshot_path) = snapshot_path {
        cleanup_screenshot_snapshot_file(&app, &snapshot_path)?;
    }

    Ok(result)
}

/// 清理冻结屏幕的临时快照文件。
#[tauri::command]
pub async fn cleanup_screenshot_snapshot(
    app: AppHandle,
    snapshot_path: String,
) -> Result<(), String> {
    cleanup_screenshot_snapshot_file(&app, &snapshot_path)
}

/// 捕获选定区域的截图
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn capture_region_screenshot(
    app: AppHandle,
    db: State<'_, Database>,
    session_mgr: State<'_, SessionManager>,
    settings: State<'_, SettingsStore>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<ClipboardItem, String> {
    if width == 0 || height == 0 {
        return Err("截图区域无效".to_string());
    }

    // 旧 API 仍保留：前端新版会传入冻结图合成后的 PNG，此路径只作为兼容兜底。
    hide_selector_window_for_capture(&app).await?;

    let screen = Screen::from_point(x, y).map_err(|e| format!("未找到选区所在屏幕: {}", e))?;
    let relative_x = x - screen.display_info.x;
    let relative_y = y - screen.display_info.y;
    let captured_image = screen
        .capture_area(relative_x, relative_y, width, height)
        .map_err(|e| format!("捕获截图区域失败: {}", e))?;
    let (captured_width, captured_height) = captured_image.dimensions();
    let rgba_image =
        image::RgbaImage::from_raw(captured_width, captured_height, captured_image.into_raw())
            .ok_or_else(|| "转换截图像素失败".to_string())?;

    save_screenshot_rgba_to_history(
        &app,
        &db,
        &session_mgr,
        &settings,
        &rgba_image,
        "ClipMaster 区域截图",
    )
}

pub fn restore_main_window_after_screenshot(app: &AppHandle) -> Result<(), String> {
    let should_restore = app
        .try_state::<ScreenshotWindowState>()
        .map(|state| state.take_restore_main_window())
        .unwrap_or(true);

    if should_restore {
        restore_main_window(app)?;
    }

    Ok(())
}

fn save_screenshot_rgba_to_history(
    app: &AppHandle,
    db: &Database,
    session_mgr: &SessionManager,
    settings: &SettingsStore,
    rgba_image: &image::RgbaImage,
    source_app: &str,
) -> Result<ClipboardItem, String> {
    let content_hash = calculate_image_clipboard_hash(
        rgba_image.width() as usize,
        rgba_image.height() as usize,
        rgba_image.as_raw(),
    );
    copy_rgba_image_to_clipboard(app, rgba_image, &content_hash)?;

    let time_zone = settings.get().time_zone;

    if let Some(saved_item) = db
        .refresh_duplicate_for_time_zone(&content_hash, &time_zone)
        .map_err(|e| e.to_string())?
    {
        app.emit("clipboard:new-item", &saved_item)
            .map_err(|e| e.to_string())?;
        return Ok(saved_item);
    }

    let (image_path, thumbnail_path) =
        save_cropped_image(app, rgba_image, &content_hash, &time_zone)?;

    let session_id = session_mgr
        .get_current_session_id()
        .ok_or_else(|| "当前没有活动会话".to_string())?;

    let saved_item = db
        .insert_item(
            CreateClipboardItem {
                type_: ClipboardType::Image,
                content: None,
                image_path: Some(image_path),
                thumbnail_path: Some(thumbnail_path),
                source_app: Some(source_app.to_string()),
                content_hash,
                session_id,
            },
            &time_zone,
        )
        .map_err(|e| e.to_string())?;

    app.emit("clipboard:new-item", &saved_item)
        .map_err(|e| e.to_string())?;

    Ok(saved_item)
}

fn copy_rgba_image_to_clipboard(
    app: &AppHandle,
    rgba_image: &image::RgbaImage,
    content_hash: &str,
) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};

    let image_data = ImageData {
        width: rgba_image.width() as usize,
        height: rgba_image.height() as usize,
        bytes: Cow::Owned(rgba_image.clone().into_raw()),
    };

    mark_next_clipboard_write(app, content_hash.to_string());
    let mut clipboard = match Clipboard::new().map_err(|e| format!("打开剪贴板失败: {}", e))
    {
        Ok(clipboard) => clipboard,
        Err(error) => {
            forget_next_clipboard_write(app, content_hash);
            return Err(error);
        }
    };
    if let Err(error) = clipboard
        .set_image(image_data)
        .map_err(|e| format!("复制截图到剪贴板失败: {}", e))
    {
        forget_next_clipboard_write(app, content_hash);
        return Err(error);
    }

    Ok(())
}

fn capture_frozen_screen_snapshot(app: &AppHandle) -> Result<FrozenScreenSnapshot, String> {
    let screen = select_capture_screen(app)?;
    let captured_image = screen
        .capture()
        .map_err(|e| format!("冻结屏幕失败: {}", e))?;
    let (pixel_width, pixel_height) = captured_image.dimensions();

    let app_data_dir = app_data::resolve_app_data_dir(app).map_err(|e| e.to_string())?;
    let cache_dir = app_data_dir.join("screenshot-cache");
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;

    let filename = format!(
        "freeze_{}_{}.png",
        chrono::Utc::now().timestamp_millis(),
        nanoid::nanoid!(8)
    );
    let file_path = cache_dir.join(filename);
    captured_image
        .save(&file_path)
        .map_err(|e| format!("保存冻结屏幕失败: {}", e))?;

    Ok(FrozenScreenSnapshot {
        path: file_path.to_string_lossy().to_string(),
        screen_x: screen.display_info.x,
        screen_y: screen.display_info.y,
        screen_width: screen.display_info.width,
        screen_height: screen.display_info.height,
        pixel_width,
        pixel_height,
        scale_factor: screen.display_info.scale_factor,
    })
}

fn select_capture_screen(app: &AppHandle) -> Result<Screen, String> {
    if let Some((cursor_x, cursor_y)) = current_cursor_position() {
        if let Ok(screen) = Screen::from_point(cursor_x, cursor_y) {
            return Ok(screen);
        }
    }

    if let Some(main_window) = app.get_webview_window("main") {
        if let (Ok(position), Ok(size)) = (main_window.outer_position(), main_window.outer_size()) {
            let center_x = position.x + (size.width / 2) as i32;
            let center_y = position.y + (size.height / 2) as i32;
            if let Ok(screen) = Screen::from_point(center_x, center_y) {
                return Ok(screen);
            }
        }
    }

    let mut screens = Screen::all().map_err(|e| format!("获取屏幕信息失败: {}", e))?;
    screens
        .iter()
        .find(|screen| screen.display_info.is_primary)
        .copied()
        .or_else(|| screens.pop())
        .ok_or_else(|| "未找到可截图的屏幕".to_string())
}

async fn prepare_main_window_for_screenshot(app: &AppHandle) -> Result<bool, String> {
    let Some(main_window) = app.get_webview_window("main") else {
        return Ok(false);
    };

    let was_visible = main_window
        .is_visible()
        .map_err(|e| format!("读取主窗口可见状态失败: {}", e))?;
    if !was_visible {
        return Ok(false);
    }

    main_window
        .hide()
        .map_err(|e| format!("隐藏主窗口失败，无法安全截图: {}", e))?;
    Ok(true)
}

fn set_screenshot_restore_main_window(app: &AppHandle, should_restore: bool) {
    if let Some(state) = app.try_state::<ScreenshotWindowState>() {
        state.set_restore_main_window(should_restore);
    }
}

fn restore_main_window_after_failed_screenshot_start(app: &AppHandle, should_restore: bool) {
    set_screenshot_restore_main_window(app, false);
    if should_restore {
        if let Err(error) = restore_main_window(app) {
            eprintln!("恢复主窗口失败: {}", error);
        }
    }
}

#[cfg(target_os = "windows")]
fn current_cursor_position() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    if ok == 0 {
        None
    } else {
        Some((point.x, point.y))
    }
}

#[cfg(not(target_os = "windows"))]
fn current_cursor_position() -> Option<(i32, i32)> {
    None
}

fn decode_png_data_url(image_data_url: &str) -> Result<image::RgbaImage, String> {
    let payload = image_data_url
        .trim()
        .strip_prefix("data:image/png;base64,")
        .ok_or_else(|| "截图数据格式无效".to_string())?;
    let bytes = general_purpose::STANDARD
        .decode(payload)
        .map_err(|e| format!("解析截图数据失败: {}", e))?;
    image::load_from_memory(&bytes)
        .map_err(|e| format!("读取截图数据失败: {}", e))
        .map(|image| image.to_rgba8())
}

fn cleanup_screenshot_snapshot_file(app: &AppHandle, snapshot_path: &str) -> Result<(), String> {
    let app_data_dir = app_data::resolve_app_data_dir(app).map_err(|e| e.to_string())?;
    let cache_dir = app_data_dir.join("screenshot-cache");
    let snapshot_path = PathBuf::from(snapshot_path);

    if !snapshot_path.exists() {
        return Ok(());
    }

    let cache_dir = fs::canonicalize(&cache_dir).map_err(|e| e.to_string())?;
    let snapshot_path = fs::canonicalize(&snapshot_path).map_err(|e| e.to_string())?;

    if !snapshot_path.starts_with(&cache_dir) {
        return Err("冻结截图路径不合法".to_string());
    }

    fs::remove_file(&snapshot_path).map_err(|e| format!("清理冻结截图失败: {}", e))
}

async fn hide_selector_window_for_capture(app: &AppHandle) -> Result<(), String> {
    if let Some(selection_window) = app.get_webview_window("screenshot-selector") {
        selection_window
            .hide()
            .map_err(|error| format!("隐藏截图选择窗口失败，无法安全截图: {}", error))?;
        sleep(Duration::from_millis(260)).await;
    }

    Ok(())
}

fn restore_main_window(app: &AppHandle) -> Result<(), String> {
    if let Some(main_window) = app.get_webview_window("main") {
        main_window.show().map_err(|e| e.to_string())?;
        main_window.set_focus().map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn save_cropped_image(
    app: &AppHandle,
    image: &image::RgbaImage,
    content_hash: &str,
    time_zone: &str,
) -> Result<(String, String), String> {
    let app_data_dir = app_data::resolve_app_data_dir(app).map_err(|e| e.to_string())?;
    let date_key = date_key_now(time_zone);
    let images_dir = app_data_dir.join("images").join(&date_key);

    fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Utc::now().timestamp_millis();
    let filename = format!(
        "region_{}_{}.png",
        &content_hash[..8.min(content_hash.len())],
        timestamp
    );
    let file_path = images_dir.join(&filename);

    image.save(&file_path).map_err(|e| e.to_string())?;

    let thumb_filename = format!(
        "region_{}_{}_thumb.png",
        &content_hash[..8.min(content_hash.len())],
        timestamp
    );
    let thumb_path = images_dir.join(&thumb_filename);

    let dynamic_img = image::DynamicImage::ImageRgba8(image.clone());
    let thumb_image = dynamic_img.resize(200, 200, image::imageops::FilterType::Lanczos3);
    thumb_image.save(&thumb_path).map_err(|e| e.to_string())?;

    let relative_path = format!("images/{}/{}", date_key, filename);
    let relative_thumb_path = format!("images/{}/{}", date_key, thumb_filename);

    Ok((relative_path, relative_thumb_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_restore_state_is_consumed_once() {
        let state = ScreenshotWindowState::default();

        assert!(!state.take_restore_main_window());
        state.set_restore_main_window(true);
        assert!(state.take_restore_main_window());
        assert!(!state.take_restore_main_window());
    }
}
