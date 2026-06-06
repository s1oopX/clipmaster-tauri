use chrono::NaiveDate;
use screenshots::Screen;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::time::sleep;

use crate::database::{date_key_now, Database};
use crate::models::{
    CleanupPlan, ClipboardDay, ClipboardItem, ClipboardType, CreateClipboardItem, Session,
};
use crate::session::SessionManager;
use crate::settings::{AppSettings, SettingsStore};

/// 获取应用数据目录路径
#[tauri::command]
pub async fn get_app_data_dir(app: AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

/// 复制文本到剪贴板
#[tauri::command]
pub async fn copy_to_clipboard(text: String) -> Result<(), String> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

/// 复制图片到剪贴板
#[tauri::command]
pub async fn copy_image_to_clipboard(app: AppHandle, image_path: String) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};
    use std::borrow::Cow;

    let safe_path = validate_relative_image_path(&image_path)?;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let absolute_path = app_data_dir.join(path_from_forward_slashes(&safe_path));

    if !absolute_path.exists() {
        return Err("图片文件不存在".to_string());
    }

    let rgba_image = image::open(&absolute_path)
        .map_err(|e| format!("读取图片失败: {}", e))?
        .to_rgba8();
    let image_data = ImageData {
        width: rgba_image.width() as usize,
        height: rgba_image.height() as usize,
        bytes: Cow::Owned(rgba_image.into_raw()),
    };

    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_image(image_data).map_err(|e| e.to_string())
}

/// 获取应用设置
#[tauri::command]
pub async fn get_settings(settings: State<'_, SettingsStore>) -> Result<AppSettings, String> {
    Ok(settings.get())
}

/// 保存应用设置
#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    db: State<'_, Database>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let previous = store.get();
    let result = SettingsStore::normalize_candidate(settings).map_err(|e| e.to_string())?;
    let hotkey_changed = previous.screenshot_hotkey != result.screenshot_hotkey;
    let time_zone_changed = previous.time_zone != result.time_zone;

    if hotkey_changed {
        if let Err(error) =
            crate::hotkey::HotkeyManager::re_register_with_hotkey(&app, &result.screenshot_hotkey)
        {
            rollback_settings_hotkey(&app, &previous, hotkey_changed);
            return Err(error);
        }
    }

    if time_zone_changed {
        if let Err(error) = db.rebuild_date_keys(&result.time_zone) {
            let rollback_error =
                rollback_settings_side_effects(&app, &db, &previous, hotkey_changed, false);
            return Err(append_rollback_error(error.to_string(), rollback_error));
        }
    }

    if let Err(error) = store.save_normalized(result.clone()) {
        let rollback_error =
            rollback_settings_side_effects(&app, &db, &previous, hotkey_changed, time_zone_changed);
        return Err(append_rollback_error(error.to_string(), rollback_error));
    }

    Ok(result)
}

fn rollback_settings_hotkey(
    app: &AppHandle,
    previous: &AppSettings,
    hotkey_changed: bool,
) -> Option<String> {
    if !hotkey_changed {
        return None;
    }

    crate::hotkey::HotkeyManager::re_register_with_hotkey(app, &previous.screenshot_hotkey)
        .err()
        .map(|error| format!("快捷键回滚失败: {}", error))
}

fn rollback_settings_side_effects(
    app: &AppHandle,
    db: &Database,
    previous: &AppSettings,
    hotkey_changed: bool,
    time_zone_changed: bool,
) -> Option<String> {
    let mut errors = Vec::new();

    if time_zone_changed {
        if let Err(error) = db.rebuild_date_keys(&previous.time_zone) {
            errors.push(format!("日期规则回滚失败: {}", error));
        }
    }

    if let Some(error) = rollback_settings_hotkey(app, previous, hotkey_changed) {
        errors.push(error);
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors.join("；"))
    }
}

fn append_rollback_error(error: String, rollback_error: Option<String>) -> String {
    match rollback_error {
        Some(rollback_error) => format!("{}（{}）", error, rollback_error),
        None => error,
    }
}

/// 预览自定义清理结果
#[tauri::command]
pub async fn preview_custom_cleanup(
    db: State<'_, Database>,
    max_items: i32,
    keep_days: i32,
) -> Result<CleanupPlan, String> {
    let max_items = max_items.clamp(10, 5000);
    let keep_days = keep_days.clamp(1, 3650);
    db.cleanup_plan(max_items, keep_days)
        .map_err(|e| e.to_string())
}

/// 执行自定义清理
#[tauri::command]
pub async fn run_custom_cleanup(
    app: AppHandle,
    db: State<'_, Database>,
    max_items: i32,
    keep_days: i32,
) -> Result<CleanupPlan, String> {
    let max_items = max_items.clamp(10, 5000);
    let keep_days = keep_days.clamp(1, 3650);
    run_cleanup(&app, &db, max_items, keep_days)
}

/// 将图片记录以置顶小窗打开
#[tauri::command]
pub async fn pin_image(app: AppHandle, image_path: String) -> Result<(), String> {
    let safe_path = validate_relative_image_path(&image_path)?;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let absolute_path = app_data_dir.join(path_from_forward_slashes(&safe_path));

    if !absolute_path.exists() {
        return Err("图片文件不存在".to_string());
    }

    let (width, height) = image::image_dimensions(&absolute_path).unwrap_or((400, 300));
    let (window_width, window_height) = fit_pin_window_size(width, height);

    // 使用新的 pin.html，通过 path 参数传递完整路径
    let url = WebviewUrl::App(
        format!(
            "pin.html?path={}",
            encode_query_value(&absolute_path.to_string_lossy())
        )
        .into(),
    );
    let label = format!("pin-{}", nanoid::nanoid!(8));

    let window = WebviewWindowBuilder::new(&app, label, url)
        .title("钉住的图片")
        .inner_size(window_width, window_height)
        .min_inner_size(100.0, 100.0)
        .resizable(true)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(false)
        .visible(true)
        .accept_first_mouse(true)
        .build()
        .map_err(|e| e.to_string())?;

    window.set_always_on_top(true).map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取剪贴板记录列表
#[tauri::command]
pub async fn get_clipboard_items(
    db: State<'_, Database>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.get_items(bounded_limit(limit, 100, 500), bounded_offset(offset))
        .map_err(|e| e.to_string())
}

/// 按会话获取记录
#[tauri::command]
pub async fn get_items_by_session(
    db: State<'_, Database>,
    session_id: String,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.get_items_by_session(
        &session_id,
        bounded_limit(limit, 100, 500),
        bounded_offset(offset),
    )
    .map_err(|e| e.to_string())
}

/// 按日期获取记录
#[tauri::command]
pub async fn get_items_by_day(
    db: State<'_, Database>,
    date_key: String,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    validate_date_key(&date_key)?;
    db.get_items_by_day(
        &date_key,
        bounded_limit(limit, 100, 500),
        bounded_offset(offset),
    )
    .map_err(|e| e.to_string())
}

/// 获取可用日期列表
#[tauri::command]
pub async fn get_available_days(
    db: State<'_, Database>,
    limit: Option<i32>,
) -> Result<Vec<ClipboardDay>, String> {
    db.get_available_days(bounded_limit(limit, 365, 3650))
        .map_err(|e| e.to_string())
}

/// 删除记录
#[tauri::command]
pub async fn delete_item(
    app: AppHandle,
    db: State<'_, Database>,
    item_id: String,
) -> Result<(), String> {
    let item = db.get_item(&item_id).map_err(|e| e.to_string())?;
    db.delete_item(&item_id).map_err(|e| e.to_string())?;

    if let Some(item) = item {
        cleanup_item_files_best_effort(&app, &item);
    }

    Ok(())
}

/// 切换收藏状态
#[tauri::command]
pub async fn toggle_favorite(db: State<'_, Database>, item_id: String) -> Result<bool, String> {
    db.toggle_favorite(&item_id).map_err(|e| e.to_string())
}

/// 切换置顶状态
#[tauri::command]
pub async fn toggle_pinned(db: State<'_, Database>, item_id: String) -> Result<bool, String> {
    db.toggle_pinned(&item_id).map_err(|e| e.to_string())
}

/// 获取当前会话
#[tauri::command]
pub async fn get_current_session(db: State<'_, Database>) -> Result<Option<Session>, String> {
    db.get_current_session().map_err(|e| e.to_string())
}

/// 获取会话列表
#[tauri::command]
pub async fn get_sessions(
    db: State<'_, Database>,
    limit: Option<i32>,
) -> Result<Vec<Session>, String> {
    db.get_sessions(bounded_limit(limit, 50, 500))
        .map_err(|e| e.to_string())
}

/// 清空会话
#[tauri::command]
pub async fn clear_session(
    app: AppHandle,
    db: State<'_, Database>,
    session_id: String,
) -> Result<(), String> {
    let items = db
        .get_items_by_session(&session_id, i32::MAX, 0)
        .map_err(|e| e.to_string())?;

    db.clear_session(&session_id).map_err(|e| e.to_string())?;

    for item in &items {
        cleanup_item_files_best_effort(&app, item);
    }

    Ok(())
}

/// 搜索记录
#[tauri::command]
pub async fn search_items(
    db: State<'_, Database>,
    settings: State<'_, SettingsStore>,
    query: String,
    session_id: Option<String>,
    date_key: Option<String>,
    limit: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    let effective_date_key = date_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| date_key_now(&settings.get().time_zone));

    validate_date_key(&effective_date_key)?;

    db.search_items(
        &query,
        session_id.as_deref(),
        &effective_date_key,
        bounded_limit(limit, 100, 500),
    )
    .map_err(|e| e.to_string())
}

/// 更新记录内容
#[tauri::command]
pub async fn update_item_content(
    db: State<'_, Database>,
    item_id: String,
    new_content: String,
) -> Result<(), String> {
    db.update_item_content(&item_id, &new_content)
        .map_err(|e| e.to_string())
}

/// 更新记录标注，不改变原始剪贴板内容
#[tauri::command]
pub async fn update_item_annotation(
    db: State<'_, Database>,
    item_id: String,
    annotation: String,
) -> Result<Option<String>, String> {
    let trimmed = annotation.trim();
    if trimmed.chars().count() > 2000 {
        return Err("标注不能超过 2000 字".to_string());
    }

    let normalized = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };

    db.update_item_annotation(&item_id, normalized.as_deref())
        .map_err(|e| e.to_string())?;

    Ok(normalized)
}

/// 开始区域截图
#[tauri::command]
pub async fn start_region_screenshot(
    app: AppHandle,
    settings: State<'_, SettingsStore>,
) -> Result<(), String> {
    let delay_ms = settings.get().capture_delay_ms.max(0) as u64;
    if delay_ms > 0 {
        sleep(Duration::from_millis(delay_ms)).await;
    }

    // 1. 创建截图选择窗口。窗口自身只显示灰色底版，最终截图在用户确认后再抓取。
    // 如果窗口已经存在，直接复用并置前，避免关闭后立即重建触发 Tauri label 冲突。
    if let Some(selection_window) = app.get_webview_window("screenshot-selector") {
        if let Some(main_window) = app.get_webview_window("main") {
            if let Err(error) = main_window.hide() {
                return Err(format!("隐藏主窗口失败: {}", error));
            }
        }

        if let Err(error) = selection_window.show() {
            let _ = restore_main_window(&app);
            return Err(format!("打开已有截图选择窗口失败: {}", error));
        }

        if let Err(error) = selection_window.set_focus() {
            eprintln!("聚焦已有截图选择窗口失败: {}", error);
        }

        return Ok(());
    }

    let selection_window = WebviewWindowBuilder::new(
        &app,
        "screenshot-selector",
        WebviewUrl::App("screenshot.html".into()),
    )
    .title("区域截图")
    .fullscreen(true)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()
    .map_err(|e| e.to_string())?;

    // 2. 先隐藏主窗口，再显示灰色底版，避免主窗口出现在用户要截取的内容里。
    if let Some(main_window) = app.get_webview_window("main") {
        if let Err(error) = main_window.hide() {
            let _ = selection_window.close();
            let _ = restore_main_window(&app);
            return Err(format!("隐藏主窗口失败: {}", error));
        }
    }

    if let Err(error) = selection_window.show() {
        let _ = selection_window.close();
        let _ = restore_main_window(&app);
        return Err(format!("打开截图选择窗口失败: {}", error));
    }

    if let Err(error) = selection_window.set_focus() {
        let _ = selection_window.close();
        let _ = restore_main_window(&app);
        return Err(format!("聚焦截图选择窗口失败: {}", error));
    }

    Ok(())
}

/// 捕获选定区域的截图
#[tauri::command]
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

    // 1. 前端会先隐藏选择窗口；这里再兜底等待一次，避免把灰色底版和工具条拍进结果。
    hide_selector_window_for_capture(&app).await?;

    // 2. 用户在灰色底版上完成框选后，抓取真实屏幕区域。
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

    // 3. 计算 hash
    let content_hash = format!("{:x}", md5::compute(rgba_image.as_raw()));
    let time_zone = settings.get().time_zone;

    if let Some(saved_item) = db
        .refresh_duplicate_for_time_zone(&content_hash, &time_zone)
        .map_err(|e| e.to_string())?
    {
        app.emit("clipboard:new-item", &saved_item)
            .map_err(|e| e.to_string())?;
        return Ok(saved_item);
    }

    // 4. 保存图片和缩略图
    let (image_path, thumbnail_path) =
        save_cropped_image(&app, &rgba_image, &content_hash, &time_zone)?;

    // 5. 获取会话
    let session_id = session_mgr
        .get_current_session_id()
        .ok_or_else(|| "当前没有活动会话".to_string())?;

    // 6. 创建记录
    let saved_item = db
        .insert_item(
            CreateClipboardItem {
                type_: ClipboardType::Image,
                content: None,
                image_path: Some(image_path),
                thumbnail_path: Some(thumbnail_path),
                source_app: Some("ClipMaster 区域截图".to_string()),
                content_hash,
                session_id,
            },
            &time_zone,
        )
        .map_err(|e| e.to_string())?;

    // 7. 通知前端
    app.emit("clipboard:new-item", &saved_item)
        .map_err(|e| e.to_string())?;

    Ok(saved_item)
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

fn validate_date_key(date_key: &str) -> Result<(), String> {
    NaiveDate::parse_from_str(date_key, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| "日期格式必须为 YYYY-MM-DD".to_string())
}

fn bounded_limit(limit: Option<i32>, default: i32, max: i32) -> i32 {
    limit.unwrap_or(default).clamp(1, max)
}

fn bounded_offset(offset: Option<i32>) -> i32 {
    offset.unwrap_or(0).max(0)
}

pub fn restore_main_window(app: &AppHandle) -> Result<(), String> {
    if let Some(main_window) = app.get_webview_window("main") {
        main_window.show().map_err(|e| e.to_string())?;
        main_window.set_focus().map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn cleanup_item_files(app: &AppHandle, item: &ClipboardItem) -> Result<(), String> {
    if !matches!(item.type_, ClipboardType::Image) {
        return Ok(());
    }

    if let Some(path) = &item.image_path {
        remove_app_data_file(app, path)?;
    }

    if let Some(path) = &item.thumbnail_path {
        remove_app_data_file(app, path)?;
    }

    Ok(())
}

fn run_cleanup(
    app: &AppHandle,
    db: &Database,
    max_items: i32,
    keep_days: i32,
) -> Result<CleanupPlan, String> {
    let items = db
        .get_cleanup_candidates(max_items, keep_days)
        .map_err(|e| e.to_string())?;
    let plan = CleanupPlan::from_items(items.clone());
    let item_ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();

    db.delete_items(&item_ids).map_err(|e| e.to_string())?;

    for item in &items {
        cleanup_item_files_best_effort(app, item);
    }

    Ok(plan)
}

fn cleanup_item_files_best_effort(app: &AppHandle, item: &ClipboardItem) {
    if let Err(error) = cleanup_item_files(app, item) {
        eprintln!("清理记录文件失败（{}）: {}", item.id, error);
    }
}

fn remove_app_data_file(app: &AppHandle, relative_path: &str) -> Result<(), String> {
    let safe_path = validate_relative_image_path(relative_path)?;
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let absolute_path = app_data_dir.join(path_from_forward_slashes(&safe_path));

    match fs::remove_file(&absolute_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("删除图片文件失败: {}", error)),
    }
}

fn save_cropped_image(
    app: &AppHandle,
    image: &image::RgbaImage,
    content_hash: &str,
    time_zone: &str,
) -> Result<(String, String), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
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

    // 保存原图
    image.save(&file_path).map_err(|e| e.to_string())?;

    // 生成缩略图
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

fn validate_relative_image_path(image_path: &str) -> Result<String, String> {
    let normalized = image_path.trim().replace('\\', "/");

    if normalized.is_empty() {
        return Err("图片路径不能为空".to_string());
    }

    if normalized.starts_with('/') || Path::new(&normalized).is_absolute() {
        return Err("图片路径必须是相对路径".to_string());
    }

    let mut parts = Vec::new();
    for part in normalized.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err("图片路径不合法".to_string());
        }
        parts.push(part);
    }

    if parts.len() != 3 || parts[0] != "images" {
        return Err("图片路径必须位于 images 日期目录".to_string());
    }

    validate_date_key(parts[1])?;

    Ok(parts.join("/"))
}

fn path_from_forward_slashes(path: &str) -> PathBuf {
    let mut path_buf = PathBuf::new();
    for part in path.split('/') {
        path_buf.push(part);
    }
    path_buf
}

fn fit_pin_window_size(width: u32, height: u32) -> (f64, f64) {
    let image_width = width.max(1) as f64;
    let image_height = height.max(1) as f64;
    let scale = (720.0 / image_width).min(520.0 / image_height).min(1.0);

    (
        (image_width * scale).max(260.0),
        (image_height * scale + 34.0).max(180.0),
    )
}

fn encode_query_value(value: &str) -> String {
    let mut encoded = String::new();

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{byte:02X}"));
            }
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_image_paths_inside_daily_images_directory() {
        assert_eq!(
            validate_relative_image_path("images/2026-06-06/capture.png").unwrap(),
            "images/2026-06-06/capture.png"
        );
        assert_eq!(
            validate_relative_image_path("images\\2026-06-06\\capture.png").unwrap(),
            "images/2026-06-06/capture.png"
        );
    }

    #[test]
    fn rejects_image_paths_outside_daily_images_directory() {
        for path in [
            "",
            "settings.json",
            "../settings.json",
            "images/2026-06/capture.png",
            "images/2026-06-06/nested/capture.png",
            "images/2026-06-06/../settings.json",
        ] {
            assert!(validate_relative_image_path(path).is_err(), "{path}");
        }
    }

    #[test]
    fn bounds_public_query_pagination_inputs() {
        assert_eq!(bounded_limit(None, 100, 500), 100);
        assert_eq!(bounded_limit(Some(-1), 100, 500), 1);
        assert_eq!(bounded_limit(Some(0), 100, 500), 1);
        assert_eq!(bounded_limit(Some(42), 100, 500), 42);
        assert_eq!(bounded_limit(Some(50_000), 100, 500), 500);

        assert_eq!(bounded_offset(None), 0);
        assert_eq!(bounded_offset(Some(-20)), 0);
        assert_eq!(bounded_offset(Some(30)), 30);
    }
}
