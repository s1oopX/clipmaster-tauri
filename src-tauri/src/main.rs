// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_data;
mod autostart;
mod clipboard;
mod commands;
mod database;
mod dev_port;
mod hotkey;
mod link;
mod models;
mod session;
mod settings;
mod tray;

use clipboard::{ClipboardService, ClipboardWriteState};
use database::Database;
use hotkey::HotkeyManager;
use session::SessionManager;
use settings::SettingsStore;
use std::process;
use tauri::Manager;

struct TrayAvailability {
    available: bool,
}

fn main() {
    let mut context = tauri::generate_context!();
    dev_port::apply_project_dev_port_to_context(&mut context);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // 获取应用数据目录
            let app_data_dir = app_data::resolve_app_data_dir(app.handle())
                .map_err(|error| startup_error("获取应用数据目录失败", error))?;

            if let Err(error) = app_data::migrate_legacy_app_data_dir(&app_data_dir) {
                eprintln!("Failed to migrate legacy app data directory: {}", error);
            }

            // 初始化设置
            let settings_store = SettingsStore::new(&app_data_dir)
                .map_err(|error| startup_error("初始化设置失败，请检查应用数据目录权限", error))?;
            let show_main_window_on_start = settings_store.get().show_main_window_on_start;

            // 初始化数据库
            let db = Database::new(app_data_dir).map_err(|error| {
                startup_error("初始化历史数据库失败，请检查数据文件或磁盘权限", error)
            })?;
            if let Err(error) = db.rebuild_date_keys(&settings_store.get().time_zone) {
                eprintln!("重建日期索引失败，已继续启动: {}", error);
            }

            // 初始化会话管理器
            let session_mgr = SessionManager::new();
            let session_id = session_mgr.start_new_session();

            // 在数据库中创建会话
            db.create_session(&session_id)
                .map_err(|error| startup_error("创建剪贴板会话失败", error))?;

            println!("Session started: {}", session_id);

            // 将服务注入到应用状态
            app.manage(settings_store);
            app.manage(db);
            app.manage(session_mgr);
            app.manage(ClipboardWriteState::default());
            app.manage(commands::ScreenshotWindowState::default());

            // 启动剪贴板监听服务
            let clipboard_service = ClipboardService::new();
            clipboard_service.start(app.handle().clone());

            // 注册全局快捷键
            if let Err(e) = HotkeyManager::register(app.handle()) {
                eprintln!("注册全局快捷键失败: {}", e);
            }

            let tray_available = match tray::setup_tray(app) {
                Ok(()) => true,
                Err(error) => {
                    eprintln!("系统托盘初始化失败，主窗口将保持可见: {}", error);
                    false
                }
            };
            app.manage(TrayAvailability {
                available: tray_available,
            });

            if let Some(window) = app.get_webview_window("main") {
                if tray_available {
                    tray::register_main_window_close_handler(&window);
                }

                if !show_main_window_on_start && tray_available {
                    if let Err(error) = window.hide() {
                        eprintln!("启动时隐藏主窗口失败，已保持窗口可见: {}", error);
                    }
                }
            }

            println!("ClipMaster started successfully!");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_clipboard_items,
            commands::get_items_by_day,
            commands::get_available_days,
            commands::get_items_by_session,
            commands::delete_item,
            commands::toggle_favorite,
            commands::toggle_pinned,
            commands::get_current_session,
            commands::get_sessions,
            commands::clear_session,
            commands::search_items,
            commands::update_item_content,
            commands::update_item_annotation,
            commands::image_assets::get_app_data_dir,
            commands::image_assets::resolve_image_asset,
            commands::copy_to_clipboard,
            commands::copy_image_to_clipboard,
            commands::open_external_url,
            commands::settings_commands::get_settings,
            commands::settings_commands::save_settings,
            commands::settings_commands::check_dev_server_port,
            commands::settings_commands::restart_app,
            commands::cleanup_commands::preview_custom_cleanup,
            commands::cleanup_commands::run_custom_cleanup,
            commands::cleanup_commands::clear_all_history,
            commands::start_region_screenshot,
            commands::capture_region_screenshot,
            commands::save_screenshot_image,
            commands::cleanup_screenshot_snapshot,
            commands::pin_image,
        ])
        .build(context)
        .unwrap_or_else(|error| {
            let message = format!("ClipMaster 启动失败: {}", error);
            eprintln!("{message}");
            show_startup_error_dialog(&message);
            process::exit(1);
        });

    app.run(|app_handle, event| {
        if let tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } = event
        {
            if label == "screenshot-selector" {
                if let Err(error) = commands::restore_main_window_after_screenshot(app_handle) {
                    eprintln!("Failed to restore main window after screenshot: {}", error);
                }
                return;
            }

            if label != "main" {
                return;
            }

            let tray_available = app_handle
                .try_state::<TrayAvailability>()
                .map(|state| state.available)
                .unwrap_or(false);
            if !tray_available {
                return;
            }

            api.prevent_close();
            if let Some(window) = app_handle.get_webview_window("main") {
                tray::hide_main_webview_window_to_tray(&window);
            }
        }
    });
}

fn startup_error<E: std::fmt::Display>(context: &str, error: E) -> Box<dyn std::error::Error> {
    let message = format!("{context}: {error}");
    eprintln!("{message}");
    std::io::Error::other(message).into()
}

#[cfg(target_os = "windows")]
fn show_startup_error_dialog(message: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    let title = wide("ClipMaster 启动失败");
    let message = wide(message);
    unsafe {
        MessageBoxW(0, message.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

#[cfg(not(target_os = "windows"))]
fn show_startup_error_dialog(_message: &str) {}
