use base64::{engine::general_purpose, Engine as _};
use chrono::NaiveDate;
use screenshots::Screen;
use serde::Serialize;
use std::borrow::Cow;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::time::sleep;

use crate::app_data;
use crate::clipboard::{
    calculate_image_clipboard_hash, calculate_text_clipboard_hash, ClipboardWriteState,
};
use crate::database::{date_key_now, Database};
use crate::dev_port::{check_dev_server_port as check_port, PortCheckResult};
use crate::link::normalize_web_url;
use crate::models::{
    CleanupFileTarget, CleanupPlan, ClipboardDay, ClipboardItem, ClipboardType,
    CreateClipboardItem, Session,
};
use crate::session::SessionManager;
use crate::settings::{AppSettings, SettingsStore};

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

/// 获取应用数据目录路径
#[tauri::command]
pub async fn get_app_data_dir(app: AppHandle) -> Result<String, String> {
    app_data::resolve_app_data_dir(&app)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

/// 复制文本到剪贴板
#[tauri::command]
pub async fn copy_to_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    use arboard::Clipboard;
    let content_hash = calculate_text_clipboard_hash(&text);
    mark_next_clipboard_write(&app, content_hash.clone());

    let mut clipboard = match Clipboard::new().map_err(|e| e.to_string()) {
        Ok(clipboard) => clipboard,
        Err(error) => {
            forget_next_clipboard_write(&app, &content_hash);
            return Err(error);
        }
    };
    if let Err(error) = clipboard.set_text(text).map_err(|e| e.to_string()) {
        forget_next_clipboard_write(&app, &content_hash);
        return Err(error);
    }

    Ok(())
}

/// 复制图片到剪贴板
#[tauri::command]
pub async fn copy_image_to_clipboard(app: AppHandle, image_path: String) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};
    use std::borrow::Cow;

    let safe_path = validate_relative_image_path(&image_path)?;
    let app_data_dir = app_data::resolve_app_data_dir(&app).map_err(|e| e.to_string())?;
    let absolute_path = app_data_dir.join(path_from_forward_slashes(&safe_path));

    if !absolute_path.exists() {
        return Err("图片文件不存在".to_string());
    }

    let rgba_image = image::open(&absolute_path)
        .map_err(|e| format!("读取图片失败: {}", e))?
        .to_rgba8();
    let content_hash = calculate_image_clipboard_hash(
        rgba_image.width() as usize,
        rgba_image.height() as usize,
        rgba_image.as_raw(),
    );
    let image_data = ImageData {
        width: rgba_image.width() as usize,
        height: rgba_image.height() as usize,
        bytes: Cow::Owned(rgba_image.into_raw()),
    };

    mark_next_clipboard_write(&app, content_hash.clone());
    let mut clipboard = match Clipboard::new().map_err(|e| e.to_string()) {
        Ok(clipboard) => clipboard,
        Err(error) => {
            forget_next_clipboard_write(&app, &content_hash);
            return Err(error);
        }
    };
    if let Err(error) = clipboard.set_image(image_data).map_err(|e| e.to_string()) {
        forget_next_clipboard_write(&app, &content_hash);
        return Err(error);
    }

    Ok(())
}

/// 在系统默认浏览器打开允许的外部链接
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    let safe_url = validate_external_url(&url)?;
    open_url_with_system(&safe_url)
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
    let dev_server_port_changed = previous.dev_server_port != result.dev_server_port;
    let auto_start_changed = previous.auto_start_enabled != result.auto_start_enabled;

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
            let rollback_error = rollback_settings_side_effects(
                &app,
                &db,
                &previous,
                hotkey_changed,
                false,
                false,
                false,
            );
            return Err(append_rollback_error(error.to_string(), rollback_error));
        }
    }

    if dev_server_port_changed {
        let port_check = check_port(result.dev_server_port)?;
        if !port_check.available {
            let rollback_error = rollback_settings_side_effects(
                &app,
                &db,
                &previous,
                hotkey_changed,
                time_zone_changed,
                false,
                false,
            );
            return Err(append_rollback_error(port_check.message, rollback_error));
        }

        if let Err(error) = crate::dev_port::write_project_dev_server_port(result.dev_server_port) {
            let rollback_error = rollback_settings_side_effects(
                &app,
                &db,
                &previous,
                hotkey_changed,
                time_zone_changed,
                true,
                false,
            );
            return Err(append_rollback_error(error.to_string(), rollback_error));
        }
    }

    if auto_start_changed {
        if let Err(error) = apply_autostart_setting(&app, result.auto_start_enabled) {
            let rollback_error = rollback_settings_side_effects(
                &app,
                &db,
                &previous,
                hotkey_changed,
                time_zone_changed,
                dev_server_port_changed,
                false,
            );
            return Err(append_rollback_error(error.to_string(), rollback_error));
        }
    }

    if let Err(error) = store.save_normalized(result.clone()) {
        let rollback_error = rollback_settings_side_effects(
            &app,
            &db,
            &previous,
            hotkey_changed,
            time_zone_changed,
            dev_server_port_changed,
            auto_start_changed,
        );
        return Err(append_rollback_error(error.to_string(), rollback_error));
    }

    Ok(result)
}

fn apply_autostart_setting(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        crate::autostart::enable_autostart(app).map_err(|e| format!("启用开机自启动失败: {}", e))
    } else {
        crate::autostart::disable_autostart(app).map_err(|e| format!("禁用开机自启动失败: {}", e))
    }
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
    dev_server_port_changed: bool,
    auto_start_changed: bool,
) -> Option<String> {
    let mut errors = Vec::new();

    if auto_start_changed {
        if let Err(error) = apply_autostart_setting(app, previous.auto_start_enabled) {
            errors.push(format!("开机自启动回滚失败: {}", error));
        }
    }

    if time_zone_changed {
        if let Err(error) = db.rebuild_date_keys(&previous.time_zone) {
            errors.push(format!("日期规则回滚失败: {}", error));
        }
    }

    if let Some(error) = rollback_settings_hotkey(app, previous, hotkey_changed) {
        errors.push(error);
    }

    if dev_server_port_changed {
        if let Err(error) = crate::dev_port::write_project_dev_server_port(previous.dev_server_port)
        {
            errors.push(format!("开发端口回滚失败: {}", error));
        }
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

/// 检查开发端口是否可用，并在占用时给出替代端口
#[tauri::command]
pub async fn check_dev_server_port(port: i32) -> Result<PortCheckResult, String> {
    check_port(port)
}

/// 重启应用，让需要重启的设置立即进入下一次启动流程
#[tauri::command]
pub async fn restart_app(app: AppHandle) -> Result<(), String> {
    app.request_restart();
    Ok(())
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

/// 清空全部剪贴板历史，包括收藏、置顶、标注记录和图片文件。
#[tauri::command]
pub async fn clear_all_history(
    app: AppHandle,
    db: State<'_, Database>,
) -> Result<CleanupPlan, String> {
    let (plan, file_targets) = db.clear_all_history().map_err(|e| e.to_string())?;

    for target in &file_targets {
        cleanup_file_target_best_effort(&app, target);
    }

    Ok(plan)
}

/// 将图片记录以置顶小窗打开
#[tauri::command]
pub async fn pin_image(app: AppHandle, image_path: String) -> Result<(), String> {
    let safe_path = validate_relative_image_path(&image_path)?;
    let app_data_dir = app_data::resolve_app_data_dir(&app).map_err(|e| e.to_string())?;
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
    item_type: Option<String>,
    favorite_only: Option<bool>,
) -> Result<Vec<ClipboardItem>, String> {
    let item_type = normalized_item_type(item_type.as_deref())?;
    db.get_items_filtered(
        bounded_limit(limit, 100, 500),
        bounded_offset(offset),
        item_type,
        favorite_only.unwrap_or(false),
    )
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
    item_type: Option<String>,
    favorite_only: Option<bool>,
) -> Result<Vec<ClipboardItem>, String> {
    validate_date_key(&date_key)?;
    let item_type = normalized_item_type(item_type.as_deref())?;
    db.get_items_by_day_filtered(
        &date_key,
        bounded_limit(limit, 100, 500),
        bounded_offset(offset),
        item_type,
        favorite_only.unwrap_or(false),
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
    session_mgr: State<'_, SessionManager>,
    session_id: String,
) -> Result<(), String> {
    if session_mgr.get_current_session_id().as_deref() == Some(session_id.as_str()) {
        return Err("不能清空当前活动会话".to_string());
    }

    let file_targets = db.clear_session(&session_id).map_err(|e| e.to_string())?;

    for target in &file_targets {
        cleanup_file_target_best_effort(&app, target);
    }

    Ok(())
}

/// 搜索记录
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn search_items(
    db: State<'_, Database>,
    settings: State<'_, SettingsStore>,
    query: String,
    session_id: Option<String>,
    date_key: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
    item_type: Option<String>,
    favorite_only: Option<bool>,
) -> Result<Vec<ClipboardItem>, String> {
    let effective_date_key = date_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| date_key_now(&settings.get().time_zone));

    validate_date_key(&effective_date_key)?;
    let item_type = normalized_item_type(item_type.as_deref())?;

    db.search_items(
        &query,
        session_id.as_deref(),
        &effective_date_key,
        bounded_limit(limit, 100, 500),
        bounded_offset(offset),
        item_type,
        favorite_only.unwrap_or(false),
    )
    .map_err(|e| e.to_string())
}

/// 更新记录内容
#[tauri::command]
pub async fn update_item_content(
    db: State<'_, Database>,
    item_id: String,
    new_content: String,
) -> Result<ClipboardItem, String> {
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

    // 1. 创建截图选择窗口。启动时先冻结当前屏幕，后续选区和标注都基于静态图像合成。
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

    let snapshot = capture_frozen_screen_snapshot(&app)?;
    let url = WebviewUrl::App(
        format!(
            "screenshot.html?snapshotPath={}&screenX={}&screenY={}&screenWidth={}&screenHeight={}&pixelWidth={}&pixelHeight={}&scaleFactor={}",
            encode_query_value(&snapshot.path),
            snapshot.screen_x,
            snapshot.screen_y,
            snapshot.screen_width,
            snapshot.screen_height,
            snapshot.pixel_width,
            snapshot.pixel_height,
            snapshot.scale_factor,
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
        .map_err(|e| e.to_string())?;

    if let Err(error) = selection_window.show() {
        let _ = selection_window.close();
        return Err(format!("打开截图选择窗口失败: {}", error));
    }

    if let Err(error) = selection_window.set_focus() {
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

fn mark_next_clipboard_write(app: &AppHandle, content_hash: String) {
    if let Some(state) = app.try_state::<ClipboardWriteState>() {
        state.suppress_next_hash(content_hash);
    }
}

fn forget_next_clipboard_write(app: &AppHandle, content_hash: &str) {
    if let Some(state) = app.try_state::<ClipboardWriteState>() {
        state.forget_pending_hash(content_hash);
    }
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

fn normalized_item_type(item_type: Option<&str>) -> Result<Option<&str>, String> {
    match item_type.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("all") => Ok(None),
        Some("text") => Ok(Some("text")),
        Some("link") => Ok(Some("link")),
        Some("image") => Ok(Some("image")),
        Some("file") => Ok(Some("file")),
        Some(_) => Err("记录类型筛选无效".to_string()),
    }
}

pub fn restore_main_window(app: &AppHandle) -> Result<(), String> {
    if let Some(main_window) = app.get_webview_window("main") {
        main_window.show().map_err(|e| e.to_string())?;
        main_window.set_focus().map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn cleanup_item_files(app: &AppHandle, item: &ClipboardItem) -> Result<(), String> {
    let Some(target) = CleanupFileTarget::from_item(item) else {
        return Ok(());
    };
    cleanup_file_target(app, &target)
}

fn cleanup_file_target(app: &AppHandle, target: &CleanupFileTarget) -> Result<(), String> {
    let app_data_dir = app_data::resolve_app_data_dir(app).map_err(|e| e.to_string())?;
    cleanup_file_target_in_dir(&app_data_dir, target)
}

fn cleanup_file_target_in_dir(
    app_data_dir: &Path,
    target: &CleanupFileTarget,
) -> Result<(), String> {
    if let Some(path) = &target.image_path {
        remove_app_data_file_in_dir(app_data_dir, path)?;
    }

    if let Some(path) = &target.thumbnail_path {
        remove_app_data_file_in_dir(app_data_dir, path)?;
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

fn cleanup_file_target_best_effort(app: &AppHandle, target: &CleanupFileTarget) {
    if let Err(error) = cleanup_file_target(app, target) {
        eprintln!("清理记录文件失败（{}）: {}", target.id, error);
    }
}

fn remove_app_data_file_in_dir(app_data_dir: &Path, relative_path: &str) -> Result<(), String> {
    let safe_path = validate_relative_image_path(relative_path)?;
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

fn validate_external_url(url: &str) -> Result<String, String> {
    normalize_web_url(url).ok_or_else(|| "只能打开安全的 http 或 https 链接".to_string())
}

#[cfg(target_os = "windows")]
fn open_url_with_system(url: &str) -> Result<(), String> {
    Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开链接失败: {}", e))
}

#[cfg(target_os = "macos")]
fn open_url_with_system(url: &str) -> Result<(), String> {
    Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开链接失败: {}", e))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_url_with_system(url: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开链接失败: {}", e))
}

fn fit_pin_window_size(width: u32, height: u32) -> (f64, f64) {
    let image_width = width.max(1) as f64;
    let image_height = height.max(1) as f64;
    let scale = (720.0 / image_width).min(520.0 / image_height).min(1.0);

    (
        (image_width * scale).max(100.0),
        (image_height * scale).max(100.0),
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
    use std::path::PathBuf;

    fn temp_data_dir() -> PathBuf {
        std::env::temp_dir().join(format!("clipmaster-commands-{}", nanoid::nanoid!()))
    }

    fn clipboard_item(
        type_: ClipboardType,
        image_path: Option<&str>,
        thumbnail_path: Option<&str>,
    ) -> ClipboardItem {
        ClipboardItem {
            id: "item_1".to_string(),
            type_,
            content: None,
            image_path: image_path.map(str::to_string),
            thumbnail_path: thumbnail_path.map(str::to_string),
            preview: None,
            timestamp: 1_780_000_000_000,
            date_key: "2026-06-07".to_string(),
            source_app: None,
            is_favorite: false,
            is_pinned: false,
            annotation: None,
            content_hash: "hash".to_string(),
            session_id: "session_1".to_string(),
        }
    }

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
    fn validates_safe_external_links() {
        assert_eq!(
            validate_external_url(" https://github.com/s1oopX ").unwrap(),
            "https://github.com/s1oopX"
        );
        assert_eq!(
            validate_external_url("https://github.com/s1oopX/clipmaster-tauri/issues").unwrap(),
            "https://github.com/s1oopX/clipmaster-tauri/issues"
        );
        assert_eq!(
            validate_external_url("https://example.com/docs?q=clipmaster#install").unwrap(),
            "https://example.com/docs?q=clipmaster#install"
        );

        for url in [
            "",
            "localhost",
            "https://localhost",
            "https://example",
            "javascript:alert(1)",
            "file:///C:/Windows/System32/calc.exe",
            "https://example.com/a b",
        ] {
            assert!(validate_external_url(url).is_err(), "{url}");
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

    #[test]
    fn fits_pin_window_to_image_without_chrome_padding() {
        assert_eq!(fit_pin_window_size(400, 300), (400.0, 300.0));
        assert_eq!(fit_pin_window_size(1440, 1040), (720.0, 520.0));
        assert_eq!(fit_pin_window_size(1000, 800), (650.0, 520.0));
    }

    #[test]
    fn keeps_tiny_pin_window_usable_without_extra_content_area() {
        assert_eq!(fit_pin_window_size(48, 48), (100.0, 100.0));
        assert_eq!(fit_pin_window_size(320, 60), (320.0, 100.0));
    }

    #[test]
    fn cleanup_item_files_removes_image_and_thumbnail_files() {
        let data_dir = temp_data_dir();
        let image_dir = data_dir.join("images").join("2026-06-07");
        fs::create_dir_all(&image_dir).unwrap();
        fs::write(image_dir.join("capture.png"), "image").unwrap();
        fs::write(image_dir.join("capture_thumb.png"), "thumbnail").unwrap();
        fs::write(image_dir.join("kept.png"), "kept").unwrap();

        let item = clipboard_item(
            ClipboardType::Image,
            Some("images/2026-06-07/capture.png"),
            Some("images/2026-06-07/capture_thumb.png"),
        );

        let target = CleanupFileTarget::from_item(&item).unwrap();
        cleanup_file_target_in_dir(&data_dir, &target).unwrap();

        assert!(!image_dir.join("capture.png").exists());
        assert!(!image_dir.join("capture_thumb.png").exists());
        assert!(image_dir.join("kept.png").exists());

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn cleanup_item_files_ignores_non_image_records() {
        let data_dir = temp_data_dir();
        let image_dir = data_dir.join("images").join("2026-06-07");
        fs::create_dir_all(&image_dir).unwrap();
        fs::write(image_dir.join("capture.png"), "image").unwrap();

        let item = clipboard_item(
            ClipboardType::Text,
            Some("images/2026-06-07/capture.png"),
            None,
        );

        cleanup_item_files_in_dir_for_test(&data_dir, &item).unwrap();

        assert!(image_dir.join("capture.png").exists());

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn cleanup_item_files_rejects_paths_outside_images_directory() {
        let data_dir = temp_data_dir();
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("settings.json"), "settings").unwrap();

        let item = clipboard_item(ClipboardType::Image, Some("../settings.json"), None);
        let target = CleanupFileTarget::from_item(&item).unwrap();
        let error = cleanup_file_target_in_dir(&data_dir, &target).unwrap_err();

        assert!(error.contains("图片路径"), "{error}");
        assert_eq!(
            fs::read_to_string(data_dir.join("settings.json")).unwrap(),
            "settings"
        );

        let _ = fs::remove_dir_all(data_dir);
    }

    fn cleanup_item_files_in_dir_for_test(
        app_data_dir: &Path,
        item: &ClipboardItem,
    ) -> Result<(), String> {
        let Some(target) = CleanupFileTarget::from_item(item) else {
            return Ok(());
        };
        cleanup_file_target_in_dir(app_data_dir, &target)
    }
}
