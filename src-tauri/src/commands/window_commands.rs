use std::process::Command;

use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

use crate::app_data;
use crate::link::normalize_web_url;

use super::image_assets::{path_from_forward_slashes, validate_relative_image_path};

/// 在系统默认浏览器打开允许的外部链接
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    let safe_url = validate_external_url(&url)?;
    open_url_with_system(&safe_url)
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
    // 刻意不抢焦点：贴图是参照物，不应打断用户当前输入；
    // 且贴图窗监听 Esc 关闭，抢焦点会让用户在其他程序里的按键误关贴图。

    Ok(())
}

pub(super) fn validate_external_url(url: &str) -> Result<String, String> {
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

pub(super) fn fit_pin_window_size(width: u32, height: u32) -> (f64, f64) {
    let image_width = width.max(1) as f64;
    let image_height = height.max(1) as f64;
    let scale = (720.0 / image_width).min(520.0 / image_height).min(1.0);

    (
        (image_width * scale).max(100.0),
        (image_height * scale).max(100.0),
    )
}

pub(super) fn encode_query_value(value: &str) -> String {
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
