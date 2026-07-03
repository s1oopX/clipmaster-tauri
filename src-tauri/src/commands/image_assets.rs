use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

use crate::app_data;

#[derive(Debug, Clone, Serialize)]
pub struct ImageAsset {
    pub(super) path: String,
    pub(super) absolute_path: String,
    pub(super) data_url: Option<String>,
}

/// 获取应用数据目录路径
#[tauri::command]
pub async fn get_app_data_dir(app: AppHandle) -> Result<String, String> {
    app_data::resolve_app_data_dir(&app)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

/// 解析图片记录路径，返回前端可直接渲染的图片资源。
#[tauri::command]
pub async fn resolve_image_asset(
    app: AppHandle,
    image_path: String,
) -> Result<Option<ImageAsset>, String> {
    let app_data_dir = app_data::resolve_app_data_dir(&app).map_err(|e| e.to_string())?;
    resolve_image_asset_in_dir(&app_data_dir, &image_path)
}

pub(super) fn remove_app_data_file_in_dir(
    app_data_dir: &Path,
    relative_path: &str,
) -> Result<(), String> {
    let safe_path = validate_relative_image_path(relative_path)?;
    let absolute_path = app_data_dir.join(path_from_forward_slashes(&safe_path));

    match fs::remove_file(&absolute_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("删除图片文件失败: {}", error)),
    }
}

pub(super) fn resolve_image_asset_in_dir(
    app_data_dir: &Path,
    image_path: &str,
) -> Result<Option<ImageAsset>, String> {
    let safe_path = validate_relative_image_path(image_path)?;
    let absolute_path = app_data_dir.join(path_from_forward_slashes(&safe_path));

    if absolute_path.is_file() {
        return Ok(Some(ImageAsset {
            path: safe_path,
            absolute_path: absolute_path.to_string_lossy().to_string(),
            data_url: image_file_data_url(&absolute_path),
        }));
    }

    let file_name = Path::new(&safe_path)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "图片路径缺少文件名".to_string())?;
    let images_dir = app_data_dir.join("images");
    let Some(fallback_path) = find_image_file_by_name(&images_dir, file_name)? else {
        return Ok(None);
    };

    let relative_path = fallback_path
        .strip_prefix(app_data_dir)
        .map_err(|_| "图片文件不在应用数据目录内".to_string())?
        .to_string_lossy()
        .replace('\\', "/");

    Ok(Some(ImageAsset {
        path: relative_path,
        absolute_path: fallback_path.to_string_lossy().to_string(),
        data_url: image_file_data_url(&fallback_path),
    }))
}

fn image_file_data_url(path: &Path) -> Option<String> {
    let mime_type = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("tif") | Some("tiff") => "image/tiff",
        Some("avif") => "image/avif",
        _ => return None,
    };

    let bytes = fs::read(path).ok()?;
    Some(format!(
        "data:{};base64,{}",
        mime_type,
        general_purpose::STANDARD.encode(bytes)
    ))
}

fn find_image_file_by_name(images_dir: &Path, file_name: &str) -> Result<Option<PathBuf>, String> {
    if !images_dir.is_dir() {
        return Ok(None);
    }

    let mut stack = vec![images_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| format!("读取图片目录失败: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("读取图片目录失败: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if path.file_name().and_then(|name| name.to_str()) == Some(file_name) {
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

pub(super) fn validate_relative_image_path(image_path: &str) -> Result<String, String> {
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

    super::validate_date_key(parts[1])?;

    Ok(parts.join("/"))
}

pub(super) fn path_from_forward_slashes(path: &str) -> PathBuf {
    let mut path_buf = PathBuf::new();
    for part in path.split('/') {
        path_buf.push(part);
    }
    path_buf
}
