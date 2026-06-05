use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub clipboard_monitor_enabled: bool,
    pub show_main_window_on_start: bool,
    pub max_items: i32,
    pub capture_delay_ms: i32,
    pub screenshot_hotkey: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            clipboard_monitor_enabled: true,
            show_main_window_on_start: true,
            max_items: 50,
            capture_delay_ms: 150,
            screenshot_hotkey: "CommandOrControl+Shift+A".to_string(),
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
            region_screenshot_hotkey: settings.region_screenshot_hotkey,
        }
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
            })
            .unwrap();

        assert_eq!(saved.max_items, 500);
        assert_eq!(saved.capture_delay_ms, 0);

        let reloaded = SettingsStore::new(&data_dir).unwrap();
        assert_eq!(reloaded.get(), saved);

        let _ = fs::remove_dir_all(data_dir);
    }
}
