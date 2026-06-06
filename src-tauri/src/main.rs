// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod commands;
mod database;
mod hotkey;
mod models;
mod session;
mod settings;

use clipboard::ClipboardService;
use database::Database;
use hotkey::HotkeyManager;
use session::SessionManager;
use settings::SettingsStore;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // 获取应用数据目录
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            // 初始化设置
            let settings_store =
                SettingsStore::new(&app_data_dir).expect("Failed to initialize settings");
            let show_main_window_on_start = settings_store.get().show_main_window_on_start;

            // 初始化数据库
            let db = Database::new(app_data_dir).expect("Failed to initialize database");

            // 初始化会话管理器
            let session_mgr = SessionManager::new();
            let session_id = session_mgr.start_new_session();

            // 在数据库中创建会话
            db.create_session(&session_id)
                .expect("Failed to create session");

            println!("Session started: {}", session_id);

            // 将服务注入到应用状态
            app.manage(settings_store);
            app.manage(db);
            app.manage(session_mgr);

            // 启动剪贴板监听服务
            let clipboard_service = ClipboardService::new();
            clipboard_service.start(app.handle().clone());

            // 注册全局快捷键
            if let Err(e) = HotkeyManager::register(app.handle()) {
                eprintln!("注册全局快捷键失败: {}", e);
            }

            if !show_main_window_on_start {
                if let Some(window) = app.get_webview_window("main") {
                    window.hide().expect("Failed to hide main window");
                }
            }

            println!("ClipMaster started successfully!");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() != "main" {
                    return;
                }

                // 主窗口关闭时结束会话
                let app_handle = window.app_handle();
                let session_mgr = app_handle.state::<SessionManager>();
                let db = app_handle.state::<Database>();

                if let Some(session_id) = session_mgr.get_current_session_id() {
                    if let Err(e) = db.end_session(&session_id) {
                        eprintln!("Failed to end session: {}", e);
                    } else {
                        session_mgr.end_current_session();
                        println!("Session ended: {}", session_id);
                    }
                }
            }
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
            commands::get_app_data_dir,
            commands::copy_to_clipboard,
            commands::copy_image_to_clipboard,
            commands::get_settings,
            commands::save_settings,
            commands::preview_custom_cleanup,
            commands::run_custom_cleanup,
            commands::start_region_screenshot,
            commands::get_screenshot_temp_path,
            commands::capture_region_screenshot,
            commands::pin_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
