use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
    pub max_items: i32,
    pub capture_delay_ms: i32,
    pub screenshot_hotkey: String,
    pub time_zone: String,
    pub language: String,
    pub auto_cleanup_enabled: bool,
    pub cleanup_max_items: i32,
    pub cleanup_keep_days: i32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            clipboard_monitor_enabled: true,
            show_main_window_on_start: true,
            max_items: 50,
            capture_delay_ms: 150,
            screenshot_hotkey: "CommandOrControl+Shift+A".to_string(),
            time_zone: DEFAULT_TIME_ZONE.to_string(),
            language: DEFAULT_LANGUAGE.to_string(),
            auto_cleanup_enabled: false,
            cleanup_max_items: 200,
            cleanup_keep_days: 30,
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
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            AppSettings::default()
        };
        let settings = Self::normalize(settings);

        Ok(Self {
            path,
            current: Mutex::new(settings),
        })
    }

    pub fn get(&self) -> AppSettings {
        self.current.lock().unwrap().clone()
    }

    pub fn save(&self, settings: AppSettings) -> Result<AppSettings> {
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
            max_items: settings.max_items.clamp(10, 500),
            capture_delay_ms: settings.capture_delay_ms.clamp(0, 3000),
            screenshot_hotkey: settings.screenshot_hotkey,
            time_zone: normalize_choice(
                &settings.time_zone,
                SUPPORTED_TIME_ZONES,
                DEFAULT_TIME_ZONE,
            ),
            language: normalize_choice(&settings.language, SUPPORTED_LANGUAGES, DEFAULT_LANGUAGE),
            auto_cleanup_enabled: settings.auto_cleanup_enabled,
            cleanup_max_items: settings.cleanup_max_items.clamp(10, 5000),
            cleanup_keep_days: settings.cleanup_keep_days.clamp(1, 3650),
        }
    }
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
            .save(AppSettings {
                clipboard_monitor_enabled: false,
                show_main_window_on_start: false,
                max_items: 900,
                capture_delay_ms: -30,
                screenshot_hotkey: "CommandOrControl+Alt+S".to_string(),
                time_zone: "America/New_York".to_string(),
                language: "en-US".to_string(),
                auto_cleanup_enabled: true,
                cleanup_max_items: 9000,
                cleanup_keep_days: -5,
            })
            .unwrap();

        assert_eq!(saved.max_items, 500);
        assert_eq!(saved.capture_delay_ms, 0);
        assert_eq!(saved.time_zone, "America/New_York");
        assert_eq!(saved.language, "en-US");
        assert_eq!(saved.cleanup_max_items, 5000);
        assert_eq!(saved.cleanup_keep_days, 1);

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
            .save(AppSettings {
                time_zone: "Mars/Base".to_string(),
                language: "pirate".to_string(),
                ..AppSettings::default()
            })
            .unwrap();

        assert_eq!(saved.time_zone, DEFAULT_TIME_ZONE);
        assert_eq!(saved.language, DEFAULT_LANGUAGE);

        let _ = fs::remove_dir_all(data_dir);
    }
}
