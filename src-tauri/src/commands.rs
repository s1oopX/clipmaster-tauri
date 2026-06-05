use chrono::Local;
use screenshots::Screen;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::time::sleep;

use crate::database::Database;
use crate::models::{ClipboardItem, ClipboardType, CreateClipboardItem, Session};
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

/// 获取应用设置
#[tauri::command]
pub async fn get_settings(settings: State<'_, SettingsStore>) -> Result<AppSettings, String> {
    Ok(settings.get())
}

/// 保存应用设置
#[tauri::command]
pub async fn save_settings(
    store: State<'_, SettingsStore>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    store.save(settings).map_err(|e| e.to_string())
}

/// 捕获当前屏幕截图并作为图片记录保存
#[tauri::command]
pub async fn capture_screenshot(
    app: AppHandle,
    db: State<'_, Database>,
    session_mgr: State<'_, SessionManager>,
    settings: State<'_, SettingsStore>,
) -> Result<ClipboardItem, String> {
    let delay_ms = settings.get().capture_delay_ms.max(0) as u64;
    if delay_ms > 0 {
        sleep(Duration::from_millis(delay_ms)).await;
    }

    let screen = Screen::all()
        .map_err(|e| e.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "未找到可截图的屏幕".to_string())?;

    let image = screen.capture().map_err(|e| e.to_string())?;
    let content_hash = format!("{:x}", md5::compute(image.as_raw()));
    let image_path = save_rgba_image(&app, &image, &content_hash)?;

    let session_id = session_mgr
        .get_current_session_id()
        .ok_or_else(|| "当前没有活动会话".to_string())?;

    let saved_item = db
        .insert_item(CreateClipboardItem {
            type_: ClipboardType::Image,
            content: None,
            image_path: Some(image_path),
            source_app: Some("ClipMaster Screenshot".to_string()),
            content_hash,
            session_id,
        })
        .map_err(|e| e.to_string())?;

    app.emit("clipboard:new-item", &saved_item)
        .map_err(|e| e.to_string())?;

    Ok(saved_item)
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

    let (width, height) = image::image_dimensions(&absolute_path).unwrap_or((360, 260));
    let (window_width, window_height) = fit_pin_window_size(width, height);
    let url = WebviewUrl::App(format!("index.html?pin={}", encode_query_value(&safe_path)).into());
    let label = format!("pin-{}", nanoid::nanoid!(8));

    let window = WebviewWindowBuilder::new(&app, label, url)
        .title("ClipMaster 贴图")
        .inner_size(window_width, window_height)
        .min_inner_size(220.0, 160.0)
        .resizable(true)
        .decorations(false)
        .always_on_top(true)
        .build()
        .map_err(|e| e.to_string())?;

    window.set_always_on_top(true).map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取剪贴板记录列表
#[tauri::command]
pub async fn get_clipboard_items(
    db: State<'_, Database>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.get_items(limit.unwrap_or(100), offset.unwrap_or(0))
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
    db.get_items_by_session(&session_id, limit.unwrap_or(100), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

/// 删除记录
#[tauri::command]
pub async fn delete_item(db: State<'_, Database>, item_id: String) -> Result<(), String> {
    db.delete_item(&item_id).map_err(|e| e.to_string())
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
    db.get_sessions(limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

/// 清空会话
#[tauri::command]
pub async fn clear_session(db: State<'_, Database>, session_id: String) -> Result<(), String> {
    db.clear_session(&session_id).map_err(|e| e.to_string())
}

/// 搜索记录
#[tauri::command]
pub async fn search_items(
    db: State<'_, Database>,
    query: String,
    session_id: Option<String>,
    limit: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.search_items(&query, session_id.as_deref(), limit.unwrap_or(100))
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

fn save_rgba_image(
    app: &AppHandle,
    image: &screenshots::image::RgbaImage,
    content_hash: &str,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let year_month = Local::now().format("%Y-%m").to_string();
    let images_dir = app_data_dir.join("images").join(&year_month);

    fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Utc::now().timestamp_millis();
    let filename = format!(
        "screenshot_{}_{}.png",
        &content_hash[..8.min(content_hash.len())],
        timestamp
    );
    let file_path = images_dir.join(&filename);

    image.save(&file_path).map_err(|e| e.to_string())?;

    Ok(format!("images/{}/{}", year_month, filename))
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
