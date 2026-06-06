use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::settings::{validate_screenshot_hotkey, SettingsStore};

pub struct HotkeyManager;

impl HotkeyManager {
    /// 注册全局快捷键
    pub fn register(app: &AppHandle) -> Result<(), String> {
        let settings_store = app.state::<SettingsStore>();
        let settings = settings_store.get();

        Self::register_screenshot_hotkey(app, &settings.screenshot_hotkey)
    }

    /// 注销所有快捷键
    pub fn unregister_all(app: &AppHandle) -> Result<(), String> {
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| format!("注销快捷键失败: {}", e))
    }

    pub fn re_register_with_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
        Self::unregister_all(app)?;
        Self::register_screenshot_hotkey(app, hotkey)
    }

    fn register_screenshot_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
        let trimmed = hotkey.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        validate_screenshot_hotkey(trimmed)?;
        let shortcut = trimmed
            .parse::<Shortcut>()
            .map_err(|_| "截图快捷键格式无效，请重新录制快捷键".to_string())?;
        let app_handle = app.clone();
        app.global_shortcut()
            .on_shortcut(shortcut, move |_app, _shortcut, _event| {
                let _ = app_handle.emit("hotkey:screenshot", ());
            })
            .map_err(|e| format!("注册截图快捷键失败: {}", e))
    }
}
