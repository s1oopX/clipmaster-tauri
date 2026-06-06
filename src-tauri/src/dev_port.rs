use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};

pub const DEFAULT_DEV_SERVER_PORT: i32 = 5174;
pub const MIN_DEV_SERVER_PORT: i32 = 1;
pub const MAX_DEV_SERVER_PORT: i32 = 65_535;
pub const DEV_PORT_CONFIG_FILE: &str = ".clipmaster-dev.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortCheckResult {
    pub port: i32,
    pub available: bool,
    pub suggested_port: Option<i32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DevPortConfig {
    #[serde(default = "default_dev_server_port")]
    dev_server_port: i32,
}

pub fn default_dev_server_port() -> i32 {
    DEFAULT_DEV_SERVER_PORT
}

pub fn normalize_dev_server_port(port: i32) -> i32 {
    if is_valid_dev_server_port(port) {
        port
    } else {
        DEFAULT_DEV_SERVER_PORT
    }
}

pub fn validate_dev_server_port(port: i32) -> Result<(), String> {
    if is_valid_dev_server_port(port) {
        Ok(())
    } else {
        Err(format!(
            "开发端口必须在 {} 到 {} 之间",
            MIN_DEV_SERVER_PORT, MAX_DEV_SERVER_PORT
        ))
    }
}

pub fn check_dev_server_port(port: i32) -> Result<PortCheckResult, String> {
    validate_dev_server_port(port)?;

    if is_port_available(port) {
        return Ok(PortCheckResult {
            port,
            available: true,
            suggested_port: None,
            message: format!("端口 {} 可用", port),
        });
    }

    let suggested_port = find_available_dev_server_port(port.saturating_add(1));
    let message = match suggested_port {
        Some(suggested_port) => {
            format!("端口 {} 已被占用，可切换到 {}", port, suggested_port)
        }
        None => format!("端口 {} 已被占用，暂未找到可用替代端口", port),
    };

    Ok(PortCheckResult {
        port,
        available: false,
        suggested_port,
        message,
    })
}

pub fn is_port_available(port: i32) -> bool {
    if !is_valid_dev_server_port(port) {
        return false;
    }

    TcpListener::bind((Ipv4Addr::LOCALHOST, port as u16)).is_ok()
}

pub fn find_available_dev_server_port(preferred_port: i32) -> Option<i32> {
    let preferred_port = preferred_port.clamp(MIN_DEV_SERVER_PORT, MAX_DEV_SERVER_PORT);

    for port in preferred_port..=MAX_DEV_SERVER_PORT {
        if is_port_available(port) {
            return Some(port);
        }
    }

    for port in MIN_DEV_SERVER_PORT..preferred_port {
        if is_port_available(port) {
            return Some(port);
        }
    }

    None
}

pub fn read_project_dev_server_port() -> i32 {
    read_dev_server_port_from_path(&project_dev_port_config_path())
        .unwrap_or(DEFAULT_DEV_SERVER_PORT)
}

pub fn write_project_dev_server_port(port: i32) -> Result<i32> {
    write_dev_server_port_to_path(&project_dev_port_config_path(), port)
}

pub fn apply_project_dev_port_to_context<R: tauri::Runtime>(context: &mut tauri::Context<R>) {
    if !cfg!(debug_assertions) {
        return;
    }

    let port = read_project_dev_server_port();
    match tauri::Url::parse(&format!("http://127.0.0.1:{port}")) {
        Ok(url) => {
            context.config_mut().build.dev_url = Some(url);
        }
        Err(error) => {
            eprintln!("开发端口配置无效，使用 Tauri 默认 devUrl: {}", error);
        }
    }
}

fn read_dev_server_port_from_path(path: &Path) -> Option<i32> {
    let raw = fs::read_to_string(path).ok()?;
    let config = serde_json::from_str::<DevPortConfig>(&raw).ok()?;
    Some(normalize_dev_server_port(config.dev_server_port))
}

fn write_dev_server_port_to_path(path: &Path, port: i32) -> Result<i32> {
    validate_dev_server_port(port).map_err(anyhow::Error::msg)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("无法创建开发端口配置目录: {}", parent.to_string_lossy()))?;
    }

    let raw = serde_json::to_string_pretty(&DevPortConfig {
        dev_server_port: port,
    })?;
    fs::write(path, raw)
        .with_context(|| format!("无法写入开发端口配置: {}", path.to_string_lossy()))?;

    Ok(port)
}

fn project_dev_port_config_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(DEV_PORT_CONFIG_FILE)
}

fn is_valid_dev_server_port(port: i32) -> bool {
    (MIN_DEV_SERVER_PORT..=MAX_DEV_SERVER_PORT).contains(&port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_port_range() {
        assert!(validate_dev_server_port(1).is_ok());
        assert!(validate_dev_server_port(65_535).is_ok());
        assert!(validate_dev_server_port(0).is_err());
        assert!(validate_dev_server_port(65_536).is_err());
    }

    #[test]
    fn reports_occupied_ports_and_suggests_an_alternative() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let occupied_port = listener.local_addr().unwrap().port() as i32;

        let result = check_dev_server_port(occupied_port).unwrap();

        assert!(!result.available);
        assert_eq!(result.port, occupied_port);
        assert!(result.message.contains("已被占用"));
        assert_ne!(result.suggested_port, Some(occupied_port));

        drop(listener);
        assert!(is_port_available(occupied_port));
    }

    #[test]
    fn reads_and_writes_project_dev_port_config_shape() {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-dev-port-{}", nanoid::nanoid!()));
        let path = data_dir.join(DEV_PORT_CONFIG_FILE);

        let saved_port = write_dev_server_port_to_path(&path, 6123).unwrap();

        assert_eq!(saved_port, 6123);
        assert_eq!(read_dev_server_port_from_path(&path), Some(6123));

        let _ = fs::remove_dir_all(data_dir);
    }
}
