// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_data;
mod autostart;
mod clipboard;
mod commands;
mod database;
mod dev_port;
mod hotkey;
mod models;
mod session;
mod settings;
mod tray;

use clipboard::ClipboardService;
use database::Database;
use hotkey::HotkeyManager;
use session::SessionManager;
use settings::SettingsStore;
use tauri::Manager;

fn main() {
    let mut context = tauri::generate_context!();
    dev_port::apply_project_dev_port_to_context(&mut context);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // 获取应用数据目录
            let app_data_dir =
                app_data::resolve_app_data_dir(app.handle()).expect("Failed to get app data dir");

            if let Err(error) = app_data::migrate_legacy_app_data_dir(&app_data_dir) {
                eprintln!("Failed to migrate legacy app data directory: {}", error);
            }

            // 初始化设置
            let settings_store =
                SettingsStore::new(&app_data_dir).expect("Failed to initialize settings");
            let show_main_window_on_start = settings_store.get().show_main_window_on_start;

            // 初始化数据库
            let db = Database::new(app_data_dir).expect("Failed to initialize database");
            db.rebuild_date_keys(&settings_store.get().time_zone)
                .expect("Failed to rebuild date keys");

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

            tray::setup_tray(app).expect("Failed to setup tray");

            if let Some(window) = app.get_webview_window("main") {
                tray::register_main_window_close_handler(&window);

                if !show_main_window_on_start {
                    window.hide().expect("Failed to hide main window");
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
            commands::get_app_data_dir,
            commands::copy_to_clipboard,
            commands::copy_image_to_clipboard,
            commands::open_external_url,
            commands::get_settings,
            commands::save_settings,
            commands::check_dev_server_port,
            commands::restart_app,
            commands::preview_custom_cleanup,
            commands::run_custom_cleanup,
            commands::clear_all_history,
            commands::start_region_screenshot,
            commands::capture_region_screenshot,
            commands::pin_image,
        ])
        .build(context)
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::WindowEvent { label, event, .. } = event {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if label == "screenshot-selector" {
                    if let Err(error) = commands::restore_main_window(app_handle) {
                        eprintln!("Failed to restore main window after screenshot: {}", error);
                    }
                    return;
                }

                if label != "main" {
                    return;
                }

                api.prevent_close();
                if let Some(window) = app_handle.get_webview_window("main") {
                    tray::hide_main_webview_window_to_tray(&window);
                }
            }
        }
    });
}
