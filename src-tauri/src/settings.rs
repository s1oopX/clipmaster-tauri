use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri_plugin_global_shortcut::Shortcut;

use crate::dev_port::{
    default_dev_server_port, normalize_dev_server_port, validate_dev_server_port,
};

pub const DEFAULT_TIME_ZONE: &str = "Asia/Shanghai";
pub const DEFAULT_LANGUAGE: &str = "zh-CN";

const SUPPORTED_TIME_ZONES: &[&str] = &[
    "Asia/Shanghai",
    "America/New_York",
    "Europe/London",
    "Asia/Tokyo",
];

const SUPPORTED_LANGUAGES: &[&str] = &["zh-CN", "en-US"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppSettings {
    pub clipboard_monitor_enabled: bool,
    pub show_main_window_on_start: bool,
    pub auto_start_enabled: bool,
    pub max_items: i32,
    pub capture_delay_ms: i32,
    pub screenshot_hotkey: String,
    pub time_zone: String,
    pub language: String,
    pub auto_cleanup_enabled: bool,
    pub cleanup_max_items: i32,
    pub cleanup_keep_days: i32,
    pub dev_server_port: i32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            clipboard_monitor_enabled: true,
            show_main_window_on_start: true,
            auto_start_enabled: false,
            max_items: 50,
            capture_delay_ms: 150,
            screenshot_hotkey: "CommandOrControl+Shift+A".to_string(),
            time_zone: DEFAULT_TIME_ZONE.to_string(),
            language: DEFAULT_LANGUAGE.to_string(),
            auto_cleanup_enabled: false,
            cleanup_max_items: 200,
            cleanup_keep_days: 30,
            dev_server_port: default_dev_server_port(),
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<AppSettings>,
}

impl SettingsStore {
    pub fn new(data_dir: &Path) -> Result<Self> {
        fs::create_dir_all(data_dir)?;
        let path = data_dir.join("settings.json");
        let settings = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            match serde_json::from_str(&raw) {
                Ok(settings) => settings,
                Err(error) => {
                    eprintln!("设置文件解析失败，已进入安全默认配置: {}", error);
                    if let Err(backup_error) = backup_corrupt_settings_file(&path) {
                        eprintln!("备份损坏设置文件失败，已继续使用安全默认配置: {backup_error}");
                    }
                    safe_fallback_settings()
                }
            }
        } else {
            AppSettings::default()
        };
        let settings = Self::normalize(settings);
        if !path.exists() {
            if let Err(error) = fs::write(&path, serde_json::to_string_pretty(&settings)?) {
                eprintln!("写入默认设置失败，已继续启动: {}", error);
            }
        }

        Ok(Self {
            path,
            current: Mutex::new(settings),
        })
    }

    pub fn get(&self) -> AppSettings {
        self.current.lock().unwrap().clone()
    }

    pub fn normalize_candidate(settings: AppSettings) -> Result<AppSettings> {
        validate_screenshot_hotkey(&settings.screenshot_hotkey).map_err(anyhow::Error::msg)?;
        validate_dev_server_port(settings.dev_server_port).map_err(anyhow::Error::msg)?;
        Ok(Self::normalize(settings))
    }

    pub fn save_normalized(&self, settings: AppSettings) -> Result<AppSettings> {
        validate_screenshot_hotkey(&settings.screenshot_hotkey).map_err(anyhow::Error::msg)?;
        validate_dev_server_port(settings.dev_server_port).map_err(anyhow::Error::msg)?;
        let normalized = Self::normalize(settings);
        let raw = serde_json::to_string_pretty(&normalized)?;
        fs::write(&self.path, raw)?;
        *self.current.lock().unwrap() = normalized.clone();
        Ok(normalized)
    }

    fn normalize(settings: AppSettings) -> AppSettings {
        AppSettings {
            clipboard_monitor_enabled: settings.clipboard_monitor_enabled,
            show_main_window_on_start: settings.show_main_window_on_start,
            auto_start_enabled: settings.auto_start_enabled,
            max_items: settings.max_items.clamp(10, 500),
            capture_delay_ms: settings.capture_delay_ms.clamp(0, 3000),
            screenshot_hotkey: normalize_screenshot_hotkey(&settings.screenshot_hotkey),
            time_zone: normalize_choice(
                &settings.time_zone,
                SUPPORTED_TIME_ZONES,
                DEFAULT_TIME_ZONE,
            ),
            language: normalize_choice(&settings.language, SUPPORTED_LANGUAGES, DEFAULT_LANGUAGE),
            auto_cleanup_enabled: settings.auto_cleanup_enabled,
            cleanup_max_items: settings.cleanup_max_items.clamp(10, 5000),
            cleanup_keep_days: settings.cleanup_keep_days.clamp(1, 3650),
            dev_server_port: normalize_dev_server_port(settings.dev_server_port),
        }
    }
}

fn safe_fallback_settings() -> AppSettings {
    AppSettings {
        clipboard_monitor_enabled: false,
        show_main_window_on_start: true,
        ..AppSettings::default()
    }
}

fn backup_corrupt_settings_file(path: &Path) -> Result<()> {
    let backup_path = path.with_file_name(format!(
        "settings.corrupt-{}.json",
        chrono::Utc::now().timestamp_millis()
    ));
    fs::rename(path, backup_path)?;
    Ok(())
}

pub fn validate_screenshot_hotkey(value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("截图快捷键不能为空，请重新录制快捷键".to_string());
    }

    let parts: Vec<&str> = trimmed.split('+').map(str::trim).collect();
    if parts.len() < 2
        || !parts[..parts.len() - 1]
            .iter()
            .any(|part| is_hotkey_modifier(part))
    {
        return Err("截图快捷键需要包含 Ctrl、Alt 或 Shift 等修饰键".to_string());
    }

    trimmed
        .parse::<Shortcut>()
        .map(|_| ())
        .map_err(|_| "截图快捷键格式无效，请重新录制快捷键".to_string())
}

fn normalize_screenshot_hotkey(value: &str) -> String {
    let trimmed = value.trim();
    if validate_screenshot_hotkey(trimmed).is_ok() {
        trimmed.to_string()
    } else {
        AppSettings::default().screenshot_hotkey
    }
}

fn is_hotkey_modifier(value: &str) -> bool {
    matches!(
        value,
        "CommandOrControl" | "Control" | "Ctrl" | "Alt" | "Shift" | "Meta" | "Super" | "Command"
    )
}

fn normalize_choice(value: &str, supported: &[&str], fallback: &str) -> String {
    let trimmed = value.trim();
    if supported.contains(&trimmed) {
        trimmed.to_string()
    } else {
        fallback.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_reloads_settings() {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-settings-{}", nanoid::nanoid!()));
        let store = SettingsStore::new(&data_dir).unwrap();
        assert_eq!(store.get(), AppSettings::default());

        let saved = store
            .save_normalized(AppSettings {
                clipboard_monitor_enabled: false,
                show_main_window_on_start: false,
                auto_start_enabled: true,
                max_items: 900,
                capture_delay_ms: -30,
                screenshot_hotkey: "CommandOrControl+Alt+S".to_string(),
                time_zone: "America/New_York".to_string(),
                language: "en-US".to_string(),
                auto_cleanup_enabled: true,
                cleanup_max_items: 9000,
                cleanup_keep_days: -5,
                dev_server_port: 6123,
            })
            .unwrap();

        assert_eq!(saved.max_items, 500);
        assert_eq!(saved.capture_delay_ms, 0);
        assert_eq!(saved.time_zone, "America/New_York");
        assert_eq!(saved.language, "en-US");
        assert_eq!(saved.cleanup_max_items, 5000);
        assert_eq!(saved.cleanup_keep_days, 1);
        assert_eq!(saved.dev_server_port, 6123);
        assert_eq!(saved.screenshot_hotkey, "CommandOrControl+Alt+S");
        assert!(saved.auto_start_enabled);

        let reloaded = SettingsStore::new(&data_dir).unwrap();
        assert_eq!(reloaded.get(), saved);

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn normalizes_unknown_time_zone_and_language_to_defaults() {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-settings-{}", nanoid::nanoid!()));
        let store = SettingsStore::new(&data_dir).unwrap();

        let saved = store
            .save_normalized(AppSettings {
                time_zone: "Mars/Base".to_string(),
                language: "pirate".to_string(),
                ..AppSettings::default()
            })
            .unwrap();

        assert_eq!(saved.time_zone, DEFAULT_TIME_ZONE);
        assert_eq!(saved.language, DEFAULT_LANGUAGE);

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn normalizes_candidate_without_mutating_current_settings() {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-settings-{}", nanoid::nanoid!()));
        let store = SettingsStore::new(&data_dir).unwrap();

        let normalized = SettingsStore::normalize_candidate(AppSettings {
            max_items: 900,
            capture_delay_ms: -30,
            screenshot_hotkey: " CommandOrControl+Alt+S ".to_string(),
            time_zone: "America/New_York".to_string(),
            ..AppSettings::default()
        })
        .unwrap();

        assert_eq!(normalized.max_items, 500);
        assert_eq!(normalized.capture_delay_ms, 0);
        assert_eq!(normalized.screenshot_hotkey, "CommandOrControl+Alt+S");
        assert_eq!(normalized.time_zone, "America/New_York");
        assert_eq!(store.get(), AppSettings::default());

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn rejects_invalid_screenshot_hotkeys_on_save() {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-settings-{}", nanoid::nanoid!()));
        let store = SettingsStore::new(&data_dir).unwrap();

        for hotkey in ["", "A", "CommandOrControl+NotAKey"] {
            let err = store
                .save_normalized(AppSettings {
                    screenshot_hotkey: hotkey.to_string(),
                    ..AppSettings::default()
                })
                .unwrap_err()
                .to_string();
            assert!(err.contains("截图快捷键"), "{hotkey}: {err}");
        }

        assert_eq!(
            store.get().screenshot_hotkey,
            AppSettings::default().screenshot_hotkey
        );

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn rejects_invalid_dev_server_ports_on_save() {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-settings-{}", nanoid::nanoid!()));
        let store = SettingsStore::new(&data_dir).unwrap();

        for dev_server_port in [0, 65_536] {
            let err = store
                .save_normalized(AppSettings {
                    dev_server_port,
                    ..AppSettings::default()
                })
                .unwrap_err()
                .to_string();
            assert!(err.contains("开发端口"), "{dev_server_port}: {err}");
        }

        assert_eq!(
            store.get().dev_server_port,
            AppSettings::default().dev_server_port
        );

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn normalizes_invalid_persisted_hotkey_to_default_on_load() {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-settings-{}", nanoid::nanoid!()));
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(
            data_dir.join("settings.json"),
            r#"{"screenshot_hotkey":"CommandOrControl+NotAKey"}"#,
        )
        .unwrap();

        let store = SettingsStore::new(&data_dir).unwrap();

        assert_eq!(
            store.get().screenshot_hotkey,
            AppSettings::default().screenshot_hotkey
        );

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn corrupt_settings_file_falls_back_to_visible_window_and_disabled_monitor() {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-settings-{}", nanoid::nanoid!()));
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("settings.json"), "{not valid json").unwrap();

        let store = SettingsStore::new(&data_dir).unwrap();
        let settings = store.get();

        assert!(!settings.clipboard_monitor_enabled);
        assert!(settings.show_main_window_on_start);
        assert!(fs::read_dir(&data_dir).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("settings.corrupt-")));
        assert!(data_dir.join("settings.json").exists());

        let _ = fs::remove_dir_all(data_dir);
    }
}
