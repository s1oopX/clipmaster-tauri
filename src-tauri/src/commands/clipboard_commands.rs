use std::borrow::Cow;

use tauri::{AppHandle, Manager};

use crate::app_data;
use crate::clipboard::{
    calculate_image_clipboard_hash, calculate_text_clipboard_hash, ClipboardWriteState,
};

use super::image_assets::{path_from_forward_slashes, validate_relative_image_path};

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

pub(super) fn mark_next_clipboard_write(app: &AppHandle, content_hash: String) {
    if let Some(state) = app.try_state::<ClipboardWriteState>() {
        state.suppress_next_hash(content_hash);
    }
}

pub(super) fn forget_next_clipboard_write(app: &AppHandle, content_hash: &str) {
    if let Some(state) = app.try_state::<ClipboardWriteState>() {
        state.forget_pending_hash(content_hash);
    }
}
