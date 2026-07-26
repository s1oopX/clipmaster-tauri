use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::settings::{
    validate_main_window_hotkey, validate_screenshot_hotkey, AppSettings, SettingsStore,
};

pub struct HotkeyManager;

impl HotkeyManager {
    /// 注册全局快捷键
    pub fn register(app: &AppHandle) -> Result<(), String> {
        let settings_store = app.state::<SettingsStore>();
        let settings = settings_store.get();

        Self::register_hotkeys(app, &settings)
    }

    /// 注销所有快捷键
    pub fn unregister_all(app: &AppHandle) -> Result<(), String> {
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| format!("注销快捷键失败: {}", e))
    }

    pub fn re_register_with_settings(
        app: &AppHandle,
        settings: &AppSettings,
    ) -> Result<(), String> {
        Self::unregister_all(app)?;
        Self::register_hotkeys(app, settings)
    }

    fn register_hotkeys(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
        // 独立注册：一个快捷键被其他程序占用不应连累另一个，
        // 部分可用优于全部静默失效；失败信息合并上报由调用方展示。
        let mut errors = Vec::new();

        if let Err(error) = Self::register_screenshot_hotkey(app, &settings.screenshot_hotkey) {
            errors.push(error);
        }

        if let Err(error) = Self::register_main_window_hotkey(app, &settings.main_window_hotkey) {
            errors.push(error);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("；"))
        }
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
            .on_shortcut(shortcut, move |_app, _shortcut, event| {
                if event.state != ShortcutState::Pressed {
                    return;
                }

                let _ = app_handle.emit("hotkey:screenshot", ());
            })
            .map_err(|e| format!("注册截图快捷键失败: {}", e))
    }

    fn register_main_window_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
        let trimmed = hotkey.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        validate_main_window_hotkey(trimmed)?;
        let shortcut = trimmed
            .parse::<Shortcut>()
            .map_err(|_| "主窗口快捷键格式无效，请重新录制快捷键".to_string())?;
        let app_handle = app.clone();
        app.global_shortcut()
            .on_shortcut(shortcut, move |_app, _shortcut, event| {
                if event.state != ShortcutState::Pressed {
                    return;
                }

                if let Err(error) = toggle_main_window(&app_handle) {
                    eprintln!("处理主窗口快捷键失败: {}", error);
                }
            })
            .map_err(|e| format!("注册主窗口快捷键失败: {}", e))
    }
}

fn toggle_main_window(app: &AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    let visible = window
        .is_visible()
        .map_err(|e| format!("读取主窗口可见状态失败: {}", e))?;
    let focused = window
        .is_focused()
        .map_err(|e| format!("读取主窗口焦点状态失败: {}", e))?;

    if visible && focused {
        window
            .hide()
            .map_err(|e| format!("隐藏主窗口失败: {}", e))?;
        return Ok(());
    }

    window
        .show()
        .map_err(|e| format!("显示主窗口失败: {}", e))?;
    window
        .set_focus()
        .map_err(|e| format!("聚焦主窗口失败: {}", e))?;
    app.emit("hotkey:focus-search", ())
        .map_err(|e| format!("发送搜索聚焦事件失败: {}", e))
}
