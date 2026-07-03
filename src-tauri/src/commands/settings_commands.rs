use tauri::{AppHandle, State};

use crate::database::Database;
use crate::dev_port::{check_dev_server_port as check_port, PortCheckResult};
use crate::settings::{AppSettings, SettingsStore};

/// 获取应用设置
#[tauri::command]
pub async fn get_settings(settings: State<'_, SettingsStore>) -> Result<AppSettings, String> {
    Ok(settings.get())
}

/// 保存应用设置
#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    db: State<'_, Database>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let previous = store.get();
    let result = SettingsStore::normalize_candidate(settings).map_err(|e| e.to_string())?;
    let hotkey_changed = previous.screenshot_hotkey != result.screenshot_hotkey
        || previous.main_window_hotkey != result.main_window_hotkey;
    let time_zone_changed = previous.time_zone != result.time_zone;
    let dev_server_port_changed = previous.dev_server_port != result.dev_server_port;
    let auto_start_changed = previous.auto_start_enabled != result.auto_start_enabled;

    if hotkey_changed {
        if let Err(error) = crate::hotkey::HotkeyManager::re_register_with_settings(&app, &result) {
            rollback_settings_hotkey(&app, &previous, hotkey_changed);
            return Err(error);
        }
    }

    if time_zone_changed {
        if let Err(error) = db.rebuild_date_keys(&result.time_zone) {
            let rollback_error = rollback_settings_side_effects(
                &app,
                &db,
                &previous,
                hotkey_changed,
                false,
                false,
                false,
            );
            return Err(append_rollback_error(error.to_string(), rollback_error));
        }
    }

    if dev_server_port_changed {
        let port_check = check_port(result.dev_server_port)?;
        if !port_check.available {
            let rollback_error = rollback_settings_side_effects(
                &app,
                &db,
                &previous,
                hotkey_changed,
                time_zone_changed,
                false,
                false,
            );
            return Err(append_rollback_error(port_check.message, rollback_error));
        }

        if let Err(error) = crate::dev_port::write_project_dev_server_port(result.dev_server_port) {
            let rollback_error = rollback_settings_side_effects(
                &app,
                &db,
                &previous,
                hotkey_changed,
                time_zone_changed,
                true,
                false,
            );
            return Err(append_rollback_error(error.to_string(), rollback_error));
        }
    }

    if auto_start_changed {
        if let Err(error) = apply_autostart_setting(&app, result.auto_start_enabled) {
            let rollback_error = rollback_settings_side_effects(
                &app,
                &db,
                &previous,
                hotkey_changed,
                time_zone_changed,
                dev_server_port_changed,
                false,
            );
            return Err(append_rollback_error(error.to_string(), rollback_error));
        }
    }

    if let Err(error) = store.save_normalized(result.clone()) {
        let rollback_error = rollback_settings_side_effects(
            &app,
            &db,
            &previous,
            hotkey_changed,
            time_zone_changed,
            dev_server_port_changed,
            auto_start_changed,
        );
        return Err(append_rollback_error(error.to_string(), rollback_error));
    }

    Ok(result)
}

fn apply_autostart_setting(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        crate::autostart::enable_autostart(app).map_err(|e| format!("启用开机自启动失败: {}", e))
    } else {
        crate::autostart::disable_autostart(app).map_err(|e| format!("禁用开机自启动失败: {}", e))
    }
}

fn rollback_settings_hotkey(
    app: &AppHandle,
    previous: &AppSettings,
    hotkey_changed: bool,
) -> Option<String> {
    if !hotkey_changed {
        return None;
    }

    crate::hotkey::HotkeyManager::re_register_with_settings(app, previous)
        .err()
        .map(|error| format!("快捷键回滚失败: {}", error))
}

fn rollback_settings_side_effects(
    app: &AppHandle,
    db: &Database,
    previous: &AppSettings,
    hotkey_changed: bool,
    time_zone_changed: bool,
    dev_server_port_changed: bool,
    auto_start_changed: bool,
) -> Option<String> {
    let mut errors = Vec::new();

    if auto_start_changed {
        if let Err(error) = apply_autostart_setting(app, previous.auto_start_enabled) {
            errors.push(format!("开机自启动回滚失败: {}", error));
        }
    }

    if time_zone_changed {
        if let Err(error) = db.rebuild_date_keys(&previous.time_zone) {
            errors.push(format!("日期规则回滚失败: {}", error));
        }
    }

    if let Some(error) = rollback_settings_hotkey(app, previous, hotkey_changed) {
        errors.push(error);
    }

    if dev_server_port_changed {
        if let Err(error) = crate::dev_port::write_project_dev_server_port(previous.dev_server_port)
        {
            errors.push(format!("开发端口回滚失败: {}", error));
        }
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors.join("；"))
    }
}

fn append_rollback_error(error: String, rollback_error: Option<String>) -> String {
    match rollback_error {
        Some(rollback_error) => format!("{}（{}）", error, rollback_error),
        None => error,
    }
}

/// 检查开发端口是否可用，并在占用时给出替代端口
#[tauri::command]
pub async fn check_dev_server_port(port: i32) -> Result<PortCheckResult, String> {
    check_port(port)
}

/// 重启应用，让需要重启的设置立即进入下一次启动流程
#[tauri::command]
pub async fn restart_app(app: AppHandle) -> Result<(), String> {
    app.request_restart();
    Ok(())
}
