use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::settings::{validate_screenshot_hotkey, SettingsStore};

pub struct HotkeyManager;

impl HotkeyManager {
    /// 注册全局快捷键
    pub fn register(app: &AppHandle) -> Result<(), String> {
        let settings_store = app.state::<SettingsStore>();
        let settings = settings_store.get();

        // 注册截图快捷键
        if !settings.screenshot_hotkey.is_empty() {
            validate_screenshot_hotkey(&settings.screenshot_hotkey)?;
            let shortcut = settings
                .screenshot_hotkey
                .parse::<Shortcut>()
                .map_err(|_| "截图快捷键格式无效，请重新录制快捷键".to_string())?;
            let app_handle = app.clone();
            app.global_shortcut()
                .on_shortcut(shortcut, move |_app, _shortcut, _event| {
                    let _ = app_handle.emit("hotkey:screenshot", ());
                })
                .map_err(|e| format!("注册截图快捷键失败: {}", e))?;
        }

        Ok(())
    }

    /// 注销所有快捷键
    pub fn unregister_all(app: &AppHandle) -> Result<(), String> {
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| format!("注销快捷键失败: {}", e))
    }

    /// 重新注册快捷键（用于设置更新后）
    pub fn re_register(app: &AppHandle) -> Result<(), String> {
        Self::unregister_all(app)?;
        Self::register(app)
    }
}
