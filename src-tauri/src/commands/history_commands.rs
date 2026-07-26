use chrono::NaiveDate;
use tauri::{AppHandle, State};

use crate::database::Database;
use crate::models::{ClipboardDay, ClipboardItem, Session};
use crate::session::SessionManager;

use super::cleanup_commands::{cleanup_file_target_best_effort, cleanup_item_files_best_effort};

#[cfg(test)]
use super::cleanup_commands::cleanup_file_target_in_dir;
#[cfg(test)]
use crate::models::CleanupFileTarget;

#[cfg(test)]
use super::image_assets::{resolve_image_asset_in_dir, validate_relative_image_path};
#[cfg(test)]
use super::window_commands::{fit_pin_window_size, validate_external_url};

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
    query: String,
    session_id: Option<String>,
    date_key: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
    item_type: Option<String>,
    favorite_only: Option<bool>,
) -> Result<Vec<ClipboardItem>, String> {
    // 未指定日期 = 搜索全部历史（FTS 索引使跨天检索代价可控）
    let effective_date_key = date_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if let Some(date_key) = effective_date_key.as_deref() {
        validate_date_key(date_key)?;
    }
    let item_type = normalized_item_type(item_type.as_deref())?;

    db.search_items(
        &query,
        session_id.as_deref(),
        effective_date_key.as_deref(),
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

pub(super) fn validate_date_key(date_key: &str) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ClipboardType;
    use std::fs;
    use std::path::{Path, PathBuf};

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

    #[test]
    fn resolves_existing_image_asset_inside_app_data_images() {
        let data_dir = temp_data_dir();
        let image_dir = data_dir.join("images").join("2026-06-09");
        fs::create_dir_all(&image_dir).unwrap();
        fs::write(image_dir.join("capture.png"), "image").unwrap();

        let asset = resolve_image_asset_in_dir(&data_dir, "images/2026-06-09/capture.png").unwrap();

        let asset = asset.unwrap();
        assert_eq!(asset.path, "images/2026-06-09/capture.png");
        assert!(Path::new(&asset.absolute_path).is_file());

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn resolves_image_asset_by_filename_when_record_path_is_stale() {
        let data_dir = temp_data_dir();
        let old_image_dir = data_dir.join("images").join("2026-06");
        fs::create_dir_all(&old_image_dir).unwrap();
        fs::write(old_image_dir.join("capture.png"), "image").unwrap();

        let asset = resolve_image_asset_in_dir(&data_dir, "images/2026-06-09/capture.png").unwrap();

        let asset = asset.unwrap();
        assert_eq!(asset.path, "images/2026-06/capture.png");
        assert!(Path::new(&asset.absolute_path).is_file());

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn resolve_image_asset_returns_none_for_missing_files() {
        let data_dir = temp_data_dir();
        fs::create_dir_all(data_dir.join("images").join("2026-06-09")).unwrap();

        let asset = resolve_image_asset_in_dir(&data_dir, "images/2026-06-09/missing.png").unwrap();

        assert!(asset.is_none());

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
